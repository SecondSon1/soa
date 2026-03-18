package com.example.flight.grpc

import com.example.flight.contract.Flight
import com.example.flight.contract.FlightServiceGrpc
import com.example.flight.contract.FlightStatus
import com.example.flight.contract.GetFlightRequest
import com.example.flight.contract.ReleaseReservationRequest
import com.example.flight.contract.ReleaseReservationResponse
import com.example.flight.contract.ReserveSeatsRequest
import com.example.flight.contract.ReserveSeatsResponse
import com.example.flight.contract.SearchFlightsRequest
import com.example.flight.contract.SearchFlightsResponse
import com.example.flight.contract.SeatReservationStatus
import com.example.flight.model.ConflictException
import com.example.flight.model.DomainException
import com.example.flight.model.FlightRecord
import com.example.flight.model.InsufficientSeatsException
import com.example.flight.model.InvalidRequestException
import com.example.flight.model.NotFoundException
import com.example.flight.model.ReleaseResult
import com.example.flight.model.ReservationResult
import com.example.flight.model.ReservationStatus
import com.example.flight.model.SearchFlightsQuery
import com.example.flight.service.FlightDomainService
import com.google.protobuf.Timestamp
import io.grpc.Status
import io.grpc.stub.StreamObserver
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.util.UUID

class FlightGrpcService(
    private val flightService: FlightDomainService,
) : FlightServiceGrpc.FlightServiceImplBase() {
    override fun searchFlights(
        request: SearchFlightsRequest,
        responseObserver: StreamObserver<SearchFlightsResponse>,
    ) {
        execute(responseObserver) {
            require(request.origin.isNotBlank()) { "`origin` must not be blank" }
            require(request.destination.isNotBlank()) { "`destination` must not be blank" }

            val result =
                flightService.searchFlights(
                    SearchFlightsQuery(
                        origin = request.origin,
                        destination = request.destination,
                        departureDate = request.departureDateOrNull()?.toLocalDateUtc(),
                    ),
                )

            SearchFlightsResponse
                .newBuilder()
                .addAllFlights(result.map { it.toProto() })
                .build()
        }
    }

    override fun getFlight(
        request: GetFlightRequest,
        responseObserver: StreamObserver<Flight>,
    ) {
        execute(responseObserver) {
            val flightId = parseUuid(request.flightId, "flight_id")
            flightService.getFlight(flightId)?.toProto()
                ?: throw NotFoundException("Flight $flightId was not found")
        }
    }

    override fun reserveSeats(
        request: ReserveSeatsRequest,
        responseObserver: StreamObserver<ReserveSeatsResponse>,
    ) {
        execute(responseObserver) {
            val flightId = parseUuid(request.flightId, "flight_id")
            val bookingId = parseUuid(request.bookingId, "booking_id")
            if (request.seatCount <= 0) {
                throw InvalidRequestException("`seat_count` must be positive")
            }

            flightService
                .reserveSeats(flightId = flightId, seatCount = request.seatCount, bookingId = bookingId)
                .toProto()
        }
    }

    override fun releaseReservation(
        request: ReleaseReservationRequest,
        responseObserver: StreamObserver<ReleaseReservationResponse>,
    ) {
        execute(responseObserver) {
            val bookingId = parseUuid(request.bookingId, "booking_id")
            flightService.releaseReservation(bookingId).toProto()
        }
    }

    private fun <T> execute(
        responseObserver: StreamObserver<T>,
        block: () -> T,
    ) {
        try {
            responseObserver.onNext(block())
            responseObserver.onCompleted()
        } catch (exception: DomainException) {
            responseObserver.onError(exception.toStatusException())
        } catch (exception: IllegalArgumentException) {
            responseObserver.onError(Status.INVALID_ARGUMENT.withDescription(exception.message).asRuntimeException())
        } catch (exception: Exception) {
            responseObserver.onError(Status.INTERNAL.withDescription(exception.message).asRuntimeException())
        }
    }

    private fun parseUuid(
        rawValue: String,
        fieldName: String,
    ): UUID =
        try {
            UUID.fromString(rawValue)
        } catch (_: IllegalArgumentException) {
            throw InvalidRequestException("`$fieldName` must be a valid UUID")
        }
}

private fun SearchFlightsRequest.departureDateOrNull(): Timestamp? = if (hasDepartureDate()) departureDate else null

private fun Timestamp.toLocalDateUtc(): LocalDate = Instant.ofEpochSecond(seconds, nanos.toLong()).atZone(ZoneOffset.UTC).toLocalDate()

private fun FlightRecord.toProto(): Flight =
    Flight
        .newBuilder()
        .setId(id.toString())
        .setAirlineCode(airlineCode)
        .setFlightNumber(flightNumber)
        .setOrigin(origin)
        .setDestination(destination)
        .setDepartureTime(departureTime.toTimestamp())
        .setArrivalTime(arrivalTime.toTimestamp())
        .setTotalSeats(totalSeats)
        .setAvailableSeats(availableSeats)
        .setPriceMinorUnits(priceMinorUnits)
        .setCurrency(currency)
        .setStatus(status.toProto())
        .build()

private fun ReservationResult.toProto(): ReserveSeatsResponse =
    ReserveSeatsResponse
        .newBuilder()
        .setReservationId(reservation.id.toString())
        .setStatus(reservation.status.toProto())
        .setRemainingAvailableSeats(remainingAvailableSeats)
        .build()

private fun ReleaseResult.toProto(): ReleaseReservationResponse =
    ReleaseReservationResponse
        .newBuilder()
        .setReservationId(reservation.id.toString())
        .setStatus(reservation.status.toProto())
        .setRestoredAvailableSeats(restoredAvailableSeats)
        .build()

private fun com.example.flight.model.FlightStatus.toProto(): FlightStatus =
    when (this) {
        com.example.flight.model.FlightStatus.SCHEDULED -> FlightStatus.FLIGHT_STATUS_SCHEDULED
        com.example.flight.model.FlightStatus.DEPARTED -> FlightStatus.FLIGHT_STATUS_DEPARTED
        com.example.flight.model.FlightStatus.CANCELLED -> FlightStatus.FLIGHT_STATUS_CANCELLED
        com.example.flight.model.FlightStatus.COMPLETED -> FlightStatus.FLIGHT_STATUS_COMPLETED
    }

private fun ReservationStatus.toProto(): SeatReservationStatus =
    when (this) {
        ReservationStatus.ACTIVE -> SeatReservationStatus.SEAT_RESERVATION_STATUS_ACTIVE
        ReservationStatus.RELEASED -> SeatReservationStatus.SEAT_RESERVATION_STATUS_RELEASED
        ReservationStatus.EXPIRED -> SeatReservationStatus.SEAT_RESERVATION_STATUS_EXPIRED
    }

private fun Instant.toTimestamp(): Timestamp =
    Timestamp
        .newBuilder()
        .setSeconds(epochSecond)
        .setNanos(nano)
        .build()

private fun DomainException.toStatusException() =
    when (this) {
        is NotFoundException -> Status.NOT_FOUND.withDescription(message).asRuntimeException()
        is InvalidRequestException -> Status.INVALID_ARGUMENT.withDescription(message).asRuntimeException()
        is InsufficientSeatsException -> Status.RESOURCE_EXHAUSTED.withDescription(message).asRuntimeException()
        is ConflictException -> Status.FAILED_PRECONDITION.withDescription(message).asRuntimeException()
    }
