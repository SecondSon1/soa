package com.example.booking.config

import java.time.Duration

data class BookingServiceConfig(
    val httpPort: Int,
    val database: DatabaseConfig,
    val flightGrpc: GrpcConfig,
)

data class DatabaseConfig(
    val jdbcUrl: String,
    val username: String,
    val password: String,
)

data class GrpcConfig(
    val host: String,
    val port: Int,
    val apiKey: String,
    val retry: RetryConfig,
    val circuitBreaker: CircuitBreakerConfig,
)

data class RetryConfig(
    val maxAttempts: Int,
    val initialBackoff: Duration,
    val maxBackoff: Duration,
    val backoffMultiplier: Double,
)

data class CircuitBreakerConfig(
    val failureThreshold: Int,
    val windowSize: Duration,
    val openStateTimeout: Duration,
)

fun loadBookingServiceConfig(): BookingServiceConfig =
    BookingServiceConfig(
        httpPort = env("BOOKING_HTTP_PORT", "8080").toInt(),
        database =
            DatabaseConfig(
                jdbcUrl = env("BOOKING_DB_URL"),
                username = env("BOOKING_DB_USER"),
                password = env("BOOKING_DB_PASSWORD"),
            ),
        flightGrpc =
            GrpcConfig(
                host = env("FLIGHT_GRPC_HOST", "flight-service"),
                port = env("FLIGHT_GRPC_PORT", "9090").toInt(),
                apiKey = env("FLIGHT_GRPC_API_KEY"),
                retry =
                    RetryConfig(
                        maxAttempts = env("FLIGHT_GRPC_RETRY_MAX_ATTEMPTS", "3").toInt(),
                        initialBackoff = Duration.ofMillis(env("FLIGHT_GRPC_RETRY_INITIAL_BACKOFF_MS", "100").toLong()),
                        maxBackoff = Duration.ofMillis(env("FLIGHT_GRPC_RETRY_MAX_BACKOFF_MS", "400").toLong()),
                        backoffMultiplier = env("FLIGHT_GRPC_RETRY_BACKOFF_MULTIPLIER", "2.0").toDouble(),
                    ),
                circuitBreaker =
                    CircuitBreakerConfig(
                        failureThreshold = env("FLIGHT_GRPC_CIRCUIT_BREAKER_FAILURE_THRESHOLD", "5").toInt(),
                        windowSize = Duration.ofSeconds(env("FLIGHT_GRPC_CIRCUIT_BREAKER_WINDOW_SECONDS", "30").toLong()),
                        openStateTimeout = Duration.ofSeconds(env("FLIGHT_GRPC_CIRCUIT_BREAKER_OPEN_SECONDS", "15").toLong()),
                    ),
            ),
    )

private fun env(name: String, default: String? = null): String =
    System.getenv(name) ?: default ?: error("Missing required environment variable: $name")
