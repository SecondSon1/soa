package com.example.booking.repository

import com.example.booking.model.BookingRecord
import com.example.booking.model.BookingStatus
import com.example.booking.model.CreateBookingCommand
import java.sql.PreparedStatement
import java.sql.ResultSet
import java.sql.Timestamp
import java.time.Instant
import java.util.UUID
import javax.sql.DataSource

class BookingRepository(
    private val dataSource: DataSource,
) {
    fun create(command: CreateBookingCommand): BookingRecord {
        val now = Instant.now()
        dataSource.connection.use { connection ->
            connection.prepareStatement(
                """
                insert into bookings (
                    id,
                    user_id,
                    flight_id,
                    passenger_name,
                    passenger_email,
                    seat_count,
                    total_price_minor_units,
                    currency,
                    status,
                    created_at,
                    updated_at
                ) values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """.trimIndent(),
            ).use { statement ->
                statement.setObject(1, command.id)
                statement.setString(2, command.userId)
                statement.setObject(3, command.flightId)
                statement.setString(4, command.passengerName)
                statement.setString(5, command.passengerEmail)
                statement.setInt(6, command.seatCount)
                statement.setLong(7, command.totalPriceMinorUnits)
                statement.setString(8, command.currency)
                statement.setString(9, BookingStatus.CONFIRMED.name)
                statement.setInstant(10, now)
                statement.setInstant(11, now)
                statement.executeUpdate()
            }
        }

        return getById(command.id) ?: error("Created booking ${command.id} is missing")
    }

    fun getById(id: UUID): BookingRecord? =
        dataSource.connection.use { connection ->
            connection.prepareStatement(
                """
                select
                    id,
                    user_id,
                    flight_id,
                    passenger_name,
                    passenger_email,
                    seat_count,
                    total_price_minor_units,
                    currency,
                    status,
                    created_at,
                    updated_at
                from bookings
                where id = ?
                """.trimIndent(),
            ).use { statement ->
                statement.setObject(1, id)
                statement.executeQuery().use { resultSet ->
                    if (resultSet.next()) resultSet.toBookingRecord() else null
                }
            }
        }

    fun listByUserId(userId: String): List<BookingRecord> =
        dataSource.connection.use { connection ->
            connection.prepareStatement(
                """
                select
                    id,
                    user_id,
                    flight_id,
                    passenger_name,
                    passenger_email,
                    seat_count,
                    total_price_minor_units,
                    currency,
                    status,
                    created_at,
                    updated_at
                from bookings
                where user_id = ?
                order by created_at desc
                """.trimIndent(),
            ).use { statement ->
                statement.setString(1, userId)
                statement.executeQuery().use { resultSet ->
                    buildList {
                        while (resultSet.next()) {
                            add(resultSet.toBookingRecord())
                        }
                    }
                }
            }
        }

    fun cancel(id: UUID): BookingRecord? {
        val now = Instant.now()
        dataSource.connection.use { connection ->
            connection.prepareStatement(
                """
                update bookings
                set status = ?, updated_at = ?
                where id = ?
                """.trimIndent(),
            ).use { statement ->
                statement.setString(1, BookingStatus.CANCELLED.name)
                statement.setInstant(2, now)
                statement.setObject(3, id)
                statement.executeUpdate()
            }
        }

        return getById(id)
    }

    private fun ResultSet.toBookingRecord(): BookingRecord =
        BookingRecord(
            id = getObject("id", UUID::class.java),
            userId = getString("user_id"),
            flightId = getObject("flight_id", UUID::class.java),
            passengerName = getString("passenger_name"),
            passengerEmail = getString("passenger_email"),
            seatCount = getInt("seat_count"),
            totalPriceMinorUnits = getLong("total_price_minor_units"),
            currency = getString("currency"),
            status = BookingStatus.valueOf(getString("status")),
            createdAt = getTimestamp("created_at").toInstant(),
            updatedAt = getTimestamp("updated_at").toInstant(),
        )

    private fun PreparedStatement.setInstant(
        parameterIndex: Int,
        value: Instant,
    ) {
        setTimestamp(parameterIndex, Timestamp.from(value))
    }
}
