package com.example.flight.model

import java.time.Instant
import java.time.LocalDate
import java.util.UUID

enum class FlightStatus {
    SCHEDULED,
    DEPARTED,
    CANCELLED,
    COMPLETED,
}

enum class ReservationStatus {
    ACTIVE,
    RELEASED,
    EXPIRED,
}

data class FlightRecord(
    val id: UUID,
    val airlineCode: String,
    val flightNumber: String,
    val origin: String,
    val destination: String,
    val departureTime: Instant,
    val arrivalTime: Instant,
    val totalSeats: Int,
    val availableSeats: Int,
    val priceMinorUnits: Long,
    val currency: String,
    val status: FlightStatus,
)

data class SeatReservationRecord(
    val id: UUID,
    val bookingId: UUID,
    val flightId: UUID,
    val seatCount: Int,
    val status: ReservationStatus,
)

data class SearchFlightsQuery(
    val origin: String,
    val destination: String,
    val departureDate: LocalDate?,
)

data class ReservationResult(
    val reservation: SeatReservationRecord,
    val remainingAvailableSeats: Int,
)

data class ReleaseResult(
    val reservation: SeatReservationRecord,
    val restoredAvailableSeats: Int,
)

sealed class DomainException(
    message: String,
) : RuntimeException(message)

class NotFoundException(message: String) : DomainException(message)

class InvalidRequestException(message: String) : DomainException(message)

class InsufficientSeatsException(message: String) : DomainException(message)

class ConflictException(message: String) : DomainException(message)
