package com.example.booking

import com.example.booking.config.loadBookingServiceConfig
import com.example.booking.db.DatabaseFactory
import com.example.booking.grpc.FlightGrpcClient
import com.example.booking.model.BookingStatus
import com.example.booking.model.CreateBookingCommand
import com.example.booking.repository.BookingRepository
import com.example.booking.web.ApiException
import com.example.booking.web.CreateBookingRequest
import com.example.booking.web.configureHttp
import com.example.booking.web.toResponse
import io.grpc.Status
import io.grpc.StatusRuntimeException
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.Application
import io.ktor.server.application.call
import io.ktor.server.engine.embeddedServer
import io.ktor.server.netty.Netty
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.routing
import io.ktor.server.plugins.calllogging.CallLogging
import io.ktor.server.application.install
import java.time.LocalDate
import java.time.format.DateTimeParseException
import java.util.UUID

fun main() {
    val config = loadBookingServiceConfig()
    val dataSource = DatabaseFactory.createDataSource(config.database)
    val repository = BookingRepository(dataSource)
    val flightClient =
        FlightGrpcClient(
            host = config.flightGrpc.host,
            port = config.flightGrpc.port,
            apiKey = config.flightGrpc.apiKey,
            retryConfig = config.flightGrpc.retry,
            circuitBreakerConfig = config.flightGrpc.circuitBreaker,
        )

    Runtime.getRuntime().addShutdownHook(
        Thread {
            flightClient.close()
            dataSource.close()
        },
    )

    embeddedServer(Netty, port = config.httpPort) {
        bookingModule(repository, flightClient)
    }.start(wait = true)
}

fun Application.bookingModule(
    repository: BookingRepository,
    flightClient: FlightGrpcClient,
) {
    install(CallLogging)
    configureHttp()

    routing {
        get("/flights") {
            val origin =
                call.request.queryParameters["origin"]
                    ?.takeIf { it.isNotBlank() }
                    ?: throw ApiException(HttpStatusCode.BadRequest, "`origin` is required")
            val destination =
                call.request.queryParameters["destination"]
                    ?.takeIf { it.isNotBlank() }
                    ?: throw ApiException(HttpStatusCode.BadRequest, "`destination` is required")
            val date =
                call.request.queryParameters["date"]
                    ?.let(::parseDateQuery)

            val flights = flightClient.searchFlights(origin = origin, destination = destination, departureDate = date)
            call.respond(flights.map { it.toResponse() })
        }

        get("/flights/{id}") {
            val id = call.parameters["id"]?.let(UUID::fromString)
                ?: throw ApiException(HttpStatusCode.BadRequest, "Flight id is invalid")
            call.respond(flightClient.getFlight(id).toResponse())
        }

        post("/bookings") {
            val request = call.receive<CreateBookingRequest>()
            validateCreateBooking(request)

            val bookingId = UUID.randomUUID()
            val flight = flightClient.getFlight(request.flightId)
            val totalPriceMinorUnits = Math.multiplyExact(flight.priceMinorUnits, request.seatCount.toLong())

            flightClient.reserveSeats(
                flightId = request.flightId,
                seatCount = request.seatCount,
                bookingId = bookingId,
            )

            val booking =
                try {
                    repository.create(
                        CreateBookingCommand(
                            id = bookingId,
                            userId = request.userId,
                            flightId = request.flightId,
                            passengerName = request.passengerName,
                            passengerEmail = request.passengerEmail,
                            seatCount = request.seatCount,
                            totalPriceMinorUnits = totalPriceMinorUnits,
                            currency = flight.currency,
                        ),
                    )
                } catch (exception: Exception) {
                    runCatching {
                        flightClient.releaseReservation(bookingId)
                    }
                    throw exception
                }

            call.respond(HttpStatusCode.Created, booking.toResponse())
        }

        get("/bookings/{id}") {
            val bookingId = call.parameters["id"]?.let(UUID::fromString)
                ?: throw ApiException(HttpStatusCode.BadRequest, "Booking id is invalid")
            val booking = repository.getById(bookingId)
                ?: throw ApiException(HttpStatusCode.NotFound, "Booking $bookingId was not found")
            call.respond(booking.toResponse())
        }

        post("/bookings/{id}/cancel") {
            val bookingId = call.parameters["id"]?.let(UUID::fromString)
                ?: throw ApiException(HttpStatusCode.BadRequest, "Booking id is invalid")
            val booking = repository.getById(bookingId)
                ?: throw ApiException(HttpStatusCode.NotFound, "Booking $bookingId was not found")

            if (booking.status != BookingStatus.CONFIRMED) {
                throw ApiException(HttpStatusCode.Conflict, "Booking $bookingId is already cancelled")
            }

            try {
                flightClient.releaseReservation(bookingId)
            } catch (exception: StatusRuntimeException) {
                if (exception.status.code != Status.Code.NOT_FOUND) {
                    throw exception
                }
            }

            val cancelled = repository.cancel(bookingId)
                ?: throw ApiException(HttpStatusCode.NotFound, "Booking $bookingId was not found")
            call.respond(cancelled.toResponse())
        }

        get("/bookings") {
            val userId =
                call.request.queryParameters["user_id"]
                    ?.takeIf { it.isNotBlank() }
                    ?: throw ApiException(HttpStatusCode.BadRequest, "`user_id` is required")
            call.respond(repository.listByUserId(userId).map { it.toResponse() })
        }
    }
}

private fun validateCreateBooking(request: CreateBookingRequest) {
    require(request.userId.isNotBlank()) { "`user_id` must not be blank" }
    require(request.passengerName.isNotBlank()) { "`passenger_name` must not be blank" }
    require(request.passengerEmail.isNotBlank()) { "`passenger_email` must not be blank" }
    require(request.seatCount > 0) { "`seat_count` must be positive" }
}

private fun parseDateQuery(rawValue: String): LocalDate =
    try {
        LocalDate.parse(rawValue)
    } catch (_: DateTimeParseException) {
        throw ApiException(HttpStatusCode.BadRequest, "`date` must be in YYYY-MM-DD format")
    }
