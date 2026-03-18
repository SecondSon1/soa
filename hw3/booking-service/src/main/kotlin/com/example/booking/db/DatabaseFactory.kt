package com.example.booking.db

import com.example.booking.config.DatabaseConfig
import com.zaxxer.hikari.HikariConfig
import com.zaxxer.hikari.HikariDataSource
import org.flywaydb.core.Flyway
object DatabaseFactory {
    fun createDataSource(config: DatabaseConfig): HikariDataSource {
        migrate(config)

        val hikariConfig =
            HikariConfig().apply {
                jdbcUrl = config.jdbcUrl
                username = config.username
                password = config.password
                maximumPoolSize = 10
                minimumIdle = 2
                connectionTimeout = 10_000
                validationTimeout = 5_000
                initializationFailTimeout = -1
            }

        return HikariDataSource(hikariConfig)
    }

    private fun migrate(config: DatabaseConfig) {
        Flyway
            .configure()
            .dataSource(config.jdbcUrl, config.username, config.password)
            .connectRetries(30)
            .connectRetriesInterval(2)
            .locations("classpath:db/migration")
            .load()
            .migrate()
    }
}
