package com.example.booking.model

import java.time.Instant
import java.util.UUID

enum class BookingStatus {
    CONFIRMED,
    CANCELLED,
}

data class BookingRecord(
    val id: UUID,
    val userId: String,
    val flightId: UUID,
    val passengerName: String,
    val passengerEmail: String,
    val seatCount: Int,
    val totalPriceMinorUnits: Long,
    val currency: String,
    val status: BookingStatus,
    val createdAt: Instant,
    val updatedAt: Instant,
)

data class CreateBookingCommand(
    val id: UUID,
    val userId: String,
    val flightId: UUID,
    val passengerName: String,
    val passengerEmail: String,
    val seatCount: Int,
    val totalPriceMinorUnits: Long,
    val currency: String,
)

data class FlightDetails(
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
    val status: String,
)
