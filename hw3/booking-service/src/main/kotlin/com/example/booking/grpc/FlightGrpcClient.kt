package com.example.booking.grpc

import com.example.booking.config.CircuitBreakerConfig
import com.example.booking.config.RetryConfig
import com.example.booking.grpc.auth.apiKeyMetadataInterceptor
import com.example.booking.grpc.resilience.CircuitBreakerClientInterceptor
import com.example.booking.grpc.resilience.GrpcCircuitBreaker
import com.example.booking.grpc.resilience.nextBackoff
import com.example.booking.grpc.resilience.shouldRetry
import com.example.booking.model.FlightDetails
import com.example.flight.contract.Flight
import com.example.flight.contract.FlightServiceGrpc
import com.example.flight.contract.GetFlightRequest
import com.example.flight.contract.ReleaseReservationRequest
import com.example.flight.contract.ReleaseReservationResponse
import com.example.flight.contract.ReserveSeatsRequest
import com.example.flight.contract.ReserveSeatsResponse
import com.example.flight.contract.SearchFlightsRequest
import com.google.protobuf.Timestamp
import io.grpc.ManagedChannel
import io.grpc.ManagedChannelBuilder
import io.grpc.StatusRuntimeException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import org.slf4j.LoggerFactory
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.util.UUID
import java.util.concurrent.TimeUnit

class FlightGrpcClient(
    host: String,
    port: Int,
    apiKey: String,
    private val retryConfig: RetryConfig,
    circuitBreakerConfig: CircuitBreakerConfig,
) : AutoCloseable {
    private val logger = LoggerFactory.getLogger(javaClass)
    private val circuitBreaker = GrpcCircuitBreaker(circuitBreakerConfig)
    private val channel: ManagedChannel =
        ManagedChannelBuilder
            .forAddress(host, port)
            .usePlaintext()
            .build()

    private val stub: FlightServiceGrpc.FlightServiceBlockingStub =
        FlightServiceGrpc
            .newBlockingStub(channel)
            .withInterceptors(
                CircuitBreakerClientInterceptor(circuitBreaker),
                apiKeyMetadataInterceptor(apiKey),
            )

    suspend fun searchFlights(
        origin: String,
        destination: String,
        departureDate: LocalDate?,
    ): List<FlightDetails> =
        executeUnaryCall("SearchFlights") {
            val builder =
                SearchFlightsRequest
                    .newBuilder()
                    .setOrigin(origin)
                    .setDestination(destination)

            departureDate?.let {
                builder.departureDate =
                    Timestamp
                        .newBuilder()
                        .setSeconds(it.atStartOfDay().toEpochSecond(ZoneOffset.UTC))
                        .build()
            }

            stub.searchFlights(builder.build()).flightsList.map { it.toFlightDetails() }
        }

    suspend fun getFlight(flightId: UUID): FlightDetails =
        executeUnaryCall("GetFlight") {
            stub
                .getFlight(GetFlightRequest.newBuilder().setFlightId(flightId.toString()).build())
                .toFlightDetails()
        }

    suspend fun reserveSeats(
        flightId: UUID,
        seatCount: Int,
        bookingId: UUID,
    ): ReserveSeatsResponse =
        executeUnaryCall("ReserveSeats") {
            stub.reserveSeats(
                ReserveSeatsRequest
                    .newBuilder()
                    .setFlightId(flightId.toString())
                    .setSeatCount(seatCount)
                    .setBookingId(bookingId.toString())
                    .build(),
            )
        }

    suspend fun releaseReservation(bookingId: UUID): ReleaseReservationResponse =
        executeUnaryCall("ReleaseReservation") {
            stub.releaseReservation(
                ReleaseReservationRequest
                    .newBuilder()
                    .setBookingId(bookingId.toString())
                    .build(),
            )
        }

    override fun close() {
        channel.shutdown()
        channel.awaitTermination(5, TimeUnit.SECONDS)
    }

    private suspend fun <T> executeUnaryCall(
        methodName: String,
        block: () -> T,
    ): T =
        withContext(Dispatchers.IO) {
            var attempt = 1
            var backoff = retryConfig.initialBackoff
            var lastException: StatusRuntimeException? = null

            while (attempt <= retryConfig.maxAttempts) {
                try {
                    return@withContext block()
                } catch (exception: StatusRuntimeException) {
                    lastException = exception
                    if (!shouldRetry(exception.status.code) || attempt >= retryConfig.maxAttempts) {
                        throw exception
                    }

                    logger.info(
                        "retrying grpc call method={} nextAttempt={} status={} backoffMs={}",
                        methodName,
                        attempt + 1,
                        exception.status.code,
                        backoff.toMillis(),
                    )
                    delay(backoff.toMillis())
                    attempt += 1
                    backoff = nextBackoff(backoff, retryConfig)
                }
            }

            throw lastException ?: error("Retry loop exited without a result for method $methodName")
        }
}

private fun Flight.toFlightDetails(): FlightDetails =
    FlightDetails(
        id = UUID.fromString(id),
        airlineCode = airlineCode,
        flightNumber = flightNumber,
        origin = origin,
        destination = destination,
        departureTime = departureTime.toInstant(),
        arrivalTime = arrivalTime.toInstant(),
        totalSeats = totalSeats,
        availableSeats = availableSeats,
        priceMinorUnits = priceMinorUnits,
        currency = currency,
        status = status.name.removePrefix("FLIGHT_STATUS_"),
    )

private fun Timestamp.toInstant(): Instant = Instant.ofEpochSecond(seconds, nanos.toLong())
