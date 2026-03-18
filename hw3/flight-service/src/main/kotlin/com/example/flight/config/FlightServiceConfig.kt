package com.example.flight.config

import java.time.Duration

data class FlightServiceConfig(
    val grpcPort: Int,
    val database: DatabaseConfig,
    val auth: AuthConfig,
    val cache: CacheConfig,
)

data class DatabaseConfig(
    val jdbcUrl: String,
    val username: String,
    val password: String,
)

data class AuthConfig(
    val apiKey: String,
)

data class CacheConfig(
    val connection: RedisConnectionConfig,
    val ttl: Duration,
)

sealed interface RedisConnectionConfig {
    data class Standalone(
        val uri: String,
    ) : RedisConnectionConfig

    data class Sentinel(
        val masterId: String,
        val sentinels: List<RedisNode>,
        val database: Int,
    ) : RedisConnectionConfig
}

data class RedisNode(
    val host: String,
    val port: Int,
)

fun loadFlightServiceConfig(): FlightServiceConfig =
    FlightServiceConfig(
        grpcPort = env("FLIGHT_GRPC_PORT", "9090").toInt(),
        database =
            DatabaseConfig(
                jdbcUrl = env("FLIGHT_DB_URL"),
                username = env("FLIGHT_DB_USER"),
                password = env("FLIGHT_DB_PASSWORD"),
            ),
        auth =
            AuthConfig(
                apiKey = env("FLIGHT_GRPC_API_KEY"),
            ),
        cache =
            CacheConfig(
                connection = loadRedisConnectionConfig(),
                ttl = Duration.ofSeconds(env("FLIGHT_CACHE_TTL_SECONDS", "300").toLong()),
            ),
    )

private fun env(name: String, default: String? = null): String =
    System.getenv(name) ?: default ?: error("Missing required environment variable: $name")

private fun envOrNull(name: String): String? = System.getenv(name)

private fun loadRedisConnectionConfig(): RedisConnectionConfig {
    val sentinelNodes =
        envOrNull("FLIGHT_REDIS_SENTINELS")
            ?.takeIf { it.isNotBlank() }
            ?.split(",")
            ?.map(::parseRedisNode)

    return if (sentinelNodes.isNullOrEmpty()) {
        RedisConnectionConfig.Standalone(env("FLIGHT_REDIS_URI", "redis://redis:6379/0"))
    } else {
        RedisConnectionConfig.Sentinel(
            masterId = env("FLIGHT_REDIS_MASTER_ID", "flight-cache-master"),
            sentinels = sentinelNodes,
            database = env("FLIGHT_REDIS_DATABASE", "0").toInt(),
        )
    }
}

private fun parseRedisNode(rawValue: String): RedisNode {
    val (host, port) =
        rawValue.split(":", limit = 2).let { parts ->
            require(parts.size == 2) { "Redis node must be in host:port format: $rawValue" }
            parts[0] to parts[1]
        }

    return RedisNode(
        host = host,
        port = port.toInt(),
    )
}
