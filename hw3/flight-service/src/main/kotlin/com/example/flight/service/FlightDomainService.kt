package com.example.flight.service

import com.example.flight.cache.FlightCache
import com.example.flight.model.FlightRecord
import com.example.flight.model.ReleaseResult
import com.example.flight.model.ReservationResult
import com.example.flight.model.SearchFlightsQuery
import com.example.flight.repository.FlightRepository
import java.util.UUID

class FlightDomainService(
    private val repository: FlightRepository,
    private val cache: FlightCache,
) {
    fun searchFlights(query: SearchFlightsQuery): List<FlightRecord> =
        cache.getSearch(query) ?: repository.search(query).also { cache.putSearch(query, it) }

    fun getFlight(flightId: UUID): FlightRecord? =
        cache.getFlight(flightId) ?: repository.getById(flightId)?.also { cache.putFlight(it) }

    fun reserveSeats(
        flightId: UUID,
        seatCount: Int,
        bookingId: UUID,
    ): ReservationResult =
        repository.reserveSeats(flightId, seatCount, bookingId).also {
            cache.invalidateFlightRelated(flightId)
        }

    fun releaseReservation(bookingId: UUID): ReleaseResult =
        repository.releaseReservation(bookingId).also {
            cache.invalidateFlightRelated(it.reservation.flightId)
        }
}
