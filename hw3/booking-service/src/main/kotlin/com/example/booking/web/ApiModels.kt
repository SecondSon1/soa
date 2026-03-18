package com.example.booking.web

import com.example.booking.model.BookingRecord
import com.example.booking.model.FlightDetails
import java.math.BigDecimal
import java.time.Instant
import java.util.UUID

data class CreateBookingRequest(
    val userId: String,
    val flightId: UUID,
    val passengerName: String,
    val passengerEmail: String,
    val seatCount: Int,
)

data class BookingResponse(
    val id: UUID,
    val userId: String,
    val flightId: UUID,
    val passengerName: String,
    val passengerEmail: String,
    val seatCount: Int,
    val totalPrice: String,
    val currency: String,
    val status: String,
    val createdAt: Instant,
    val updatedAt: Instant,
)

data class FlightResponse(
    val id: UUID,
    val airlineCode: String,
    val flightNumber: String,
    val origin: String,
    val destination: String,
    val departureTime: Instant,
    val arrivalTime: Instant,
    val totalSeats: Int,
    val availableSeats: Int,
    val price: String,
    val currency: String,
    val status: String,
)

data class ErrorResponse(
    val message: String,
)

fun BookingRecord.toResponse(): BookingResponse =
    BookingResponse(
        id = id,
        userId = userId,
        flightId = flightId,
        passengerName = passengerName,
        passengerEmail = passengerEmail,
        seatCount = seatCount,
        totalPrice = totalPriceMinorUnits.toMoneyString(),
        currency = currency,
        status = status.name,
        createdAt = createdAt,
        updatedAt = updatedAt,
    )

fun FlightDetails.toResponse(): FlightResponse =
    FlightResponse(
        id = id,
        airlineCode = airlineCode,
        flightNumber = flightNumber,
        origin = origin,
        destination = destination,
        departureTime = departureTime,
        arrivalTime = arrivalTime,
        totalSeats = totalSeats,
        availableSeats = availableSeats,
        price = priceMinorUnits.toMoneyString(),
        currency = currency,
        status = status,
    )

private fun Long.toMoneyString(): String = BigDecimal.valueOf(this, 2).toPlainString()
