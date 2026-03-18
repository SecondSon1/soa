package com.example.flight.repository

import com.example.flight.model.ConflictException
import com.example.flight.model.FlightRecord
import com.example.flight.model.FlightStatus
import com.example.flight.model.InsufficientSeatsException
import com.example.flight.model.NotFoundException
import com.example.flight.model.ReleaseResult
import com.example.flight.model.ReservationResult
import com.example.flight.model.ReservationStatus
import com.example.flight.model.SearchFlightsQuery
import com.example.flight.model.SeatReservationRecord
import java.sql.Connection
import java.sql.PreparedStatement
import java.sql.ResultSet
import java.sql.Timestamp
import java.time.Instant
import java.util.UUID
import javax.sql.DataSource

class FlightRepository(
    private val dataSource: DataSource,
) {
    fun search(query: SearchFlightsQuery): List<FlightRecord> =
        dataSource.connection.use { connection ->
            val sql =
                buildString {
                    append(
                        """
                        select
                            id,
                            airline_code,
                            flight_number,
                            origin_iata,
                            destination_iata,
                            departure_time,
                            arrival_time,
                            total_seats,
                            available_seats,
                            price_minor_units,
                            currency,
                            status
                        from flights
                        where upper(origin_iata) = upper(?)
                          and upper(destination_iata) = upper(?)
                          and status = 'SCHEDULED'
                        """.trimIndent(),
                    )
                    if (query.departureDate != null) {
                        append("\n  and departure_time::date = ?")
                    }
                    append("\norder by departure_time asc")
                }

            connection.prepareStatement(sql).use { statement ->
                statement.setString(1, query.origin)
                statement.setString(2, query.destination)
                if (query.departureDate != null) {
                    statement.setObject(3, query.departureDate)
                }

                statement.executeQuery().use { resultSet ->
                    buildList {
                        while (resultSet.next()) {
                            add(resultSet.toFlightRecord())
                        }
                    }
                }
            }
        }

    fun getById(id: UUID): FlightRecord? =
        dataSource.connection.use { connection ->
            connection.prepareStatement(
                """
                select
                    id,
                    airline_code,
                    flight_number,
                    origin_iata,
                    destination_iata,
                    departure_time,
                    arrival_time,
                    total_seats,
                    available_seats,
                    price_minor_units,
                    currency,
                    status
                from flights
                where id = ?
                """.trimIndent(),
            ).use { statement ->
                statement.setObject(1, id)
                statement.executeQuery().use { resultSet ->
                    if (resultSet.next()) resultSet.toFlightRecord() else null
                }
            }
        }

    fun reserveSeats(
        flightId: UUID,
        seatCount: Int,
        bookingId: UUID,
    ): ReservationResult =
        inTransaction { connection ->
            val existingReservation = findReservationForUpdate(connection, bookingId)
            if (existingReservation != null) {
                if (existingReservation.status == ReservationStatus.ACTIVE &&
                    existingReservation.flightId == flightId &&
                    existingReservation.seatCount == seatCount
                ) {
                    val flight = findFlightForUpdate(connection, flightId)
                    connection.commit()
                    return@inTransaction ReservationResult(existingReservation, flight.availableSeats)
                }

                throw ConflictException("Booking $bookingId already has a reservation")
            }

            val flight = findFlightForUpdate(connection, flightId)
            if (flight.status != FlightStatus.SCHEDULED) {
                throw ConflictException("Flight $flightId is not available for booking")
            }
            if (flight.availableSeats < seatCount) {
                throw InsufficientSeatsException("Not enough seats on flight $flightId")
            }

            val remainingSeats = flight.availableSeats - seatCount
            val reservationId = UUID.randomUUID()
            val now = Instant.now()

            connection.prepareStatement(
                """
                update flights
                set available_seats = ?, updated_at = ?
                where id = ?
                """.trimIndent(),
            ).use { statement ->
                statement.setInt(1, remainingSeats)
                statement.setInstant(2, now)
                statement.setObject(3, flightId)
                statement.executeUpdate()
            }

            connection.prepareStatement(
                """
                insert into seat_reservations (
                    id,
                    booking_id,
                    flight_id,
                    seat_count,
                    status,
                    created_at,
                    updated_at
                ) values (?, ?, ?, ?, ?, ?, ?)
                """.trimIndent(),
            ).use { statement ->
                statement.setObject(1, reservationId)
                statement.setObject(2, bookingId)
                statement.setObject(3, flightId)
                statement.setInt(4, seatCount)
                statement.setString(5, ReservationStatus.ACTIVE.name)
                statement.setInstant(6, now)
                statement.setInstant(7, now)
                statement.executeUpdate()
            }

            connection.commit()
            ReservationResult(
                reservation =
                    SeatReservationRecord(
                        id = reservationId,
                        bookingId = bookingId,
                        flightId = flightId,
                        seatCount = seatCount,
                        status = ReservationStatus.ACTIVE,
                    ),
                remainingAvailableSeats = remainingSeats,
            )
        }

    fun releaseReservation(bookingId: UUID): ReleaseResult =
        inTransaction { connection ->
            val reservation =
                findReservationForUpdate(connection, bookingId)
                    ?: throw NotFoundException("Reservation for booking $bookingId was not found")
            val flight = findFlightForUpdate(connection, reservation.flightId)

            if (reservation.status == ReservationStatus.RELEASED) {
                connection.commit()
                return@inTransaction ReleaseResult(reservation, flight.availableSeats)
            }
            if (reservation.status != ReservationStatus.ACTIVE) {
                throw ConflictException("Reservation ${reservation.id} cannot be released from status ${reservation.status}")
            }

            val restoredSeats = flight.availableSeats + reservation.seatCount
            val now = Instant.now()

            connection.prepareStatement(
                """
                update flights
                set available_seats = ?, updated_at = ?
                where id = ?
                """.trimIndent(),
            ).use { statement ->
                statement.setInt(1, restoredSeats)
                statement.setInstant(2, now)
                statement.setObject(3, reservation.flightId)
                statement.executeUpdate()
            }

            connection.prepareStatement(
                """
                update seat_reservations
                set status = ?, updated_at = ?
                where id = ?
                """.trimIndent(),
            ).use { statement ->
                statement.setString(1, ReservationStatus.RELEASED.name)
                statement.setInstant(2, now)
                statement.setObject(3, reservation.id)
                statement.executeUpdate()
            }

            connection.commit()
            ReleaseResult(
                reservation = reservation.copy(status = ReservationStatus.RELEASED),
                restoredAvailableSeats = restoredSeats,
            )
        }

    private fun <T> inTransaction(block: (Connection) -> T): T =
        dataSource.connection.use { connection ->
            connection.autoCommit = false
            try {
                block(connection)
            } catch (exception: Exception) {
                connection.rollback()
                throw exception
            } finally {
                connection.autoCommit = true
            }
        }

    private fun findFlightForUpdate(
        connection: Connection,
        flightId: UUID,
    ): FlightRecord =
        connection.prepareStatement(
            """
            select
                id,
                airline_code,
                flight_number,
                origin_iata,
                destination_iata,
                departure_time,
                arrival_time,
                total_seats,
                available_seats,
                price_minor_units,
                currency,
                status
            from flights
            where id = ?
            for update
            """.trimIndent(),
        ).use { statement ->
            statement.setObject(1, flightId)
            statement.executeQuery().use { resultSet ->
                if (resultSet.next()) resultSet.toFlightRecord()
                else throw NotFoundException("Flight $flightId was not found")
            }
        }

    private fun findReservationForUpdate(
        connection: Connection,
        bookingId: UUID,
    ): SeatReservationRecord? =
        connection.prepareStatement(
            """
            select
                id,
                booking_id,
                flight_id,
                seat_count,
                status
            from seat_reservations
            where booking_id = ?
            for update
            """.trimIndent(),
        ).use { statement ->
            statement.setObject(1, bookingId)
            statement.executeQuery().use { resultSet ->
                if (resultSet.next()) resultSet.toSeatReservationRecord() else null
            }
        }

    private fun ResultSet.toFlightRecord(): FlightRecord =
        FlightRecord(
            id = getObject("id", UUID::class.java),
            airlineCode = getString("airline_code"),
            flightNumber = getString("flight_number"),
            origin = getString("origin_iata"),
            destination = getString("destination_iata"),
            departureTime = getTimestamp("departure_time").toInstant(),
            arrivalTime = getTimestamp("arrival_time").toInstant(),
            totalSeats = getInt("total_seats"),
            availableSeats = getInt("available_seats"),
            priceMinorUnits = getLong("price_minor_units"),
            currency = getString("currency"),
            status = FlightStatus.valueOf(getString("status")),
        )

    private fun ResultSet.toSeatReservationRecord(): SeatReservationRecord =
        SeatReservationRecord(
            id = getObject("id", UUID::class.java),
            bookingId = getObject("booking_id", UUID::class.java),
            flightId = getObject("flight_id", UUID::class.java),
            seatCount = getInt("seat_count"),
            status = ReservationStatus.valueOf(getString("status")),
        )

    private fun PreparedStatement.setInstant(
        parameterIndex: Int,
        value: Instant,
    ) {
        setTimestamp(parameterIndex, Timestamp.from(value))
    }
}
