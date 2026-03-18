package com.example.flight.cache

import com.example.flight.config.RedisConnectionConfig
import com.example.flight.model.FlightRecord
import com.example.flight.model.SearchFlightsQuery
import com.fasterxml.jackson.core.type.TypeReference
import com.fasterxml.jackson.databind.ObjectMapper
import io.lettuce.core.RedisClient
import io.lettuce.core.RedisURI
import io.lettuce.core.api.StatefulRedisConnection
import org.slf4j.LoggerFactory
import java.time.Duration
import java.util.UUID

class FlightCache(
    connectionConfig: RedisConnectionConfig,
    private val ttl: Duration,
    private val objectMapper: ObjectMapper,
) : AutoCloseable {
    private val logger = LoggerFactory.getLogger(javaClass)
    private val redisClient: RedisClient = RedisClient.create(redisUri(connectionConfig))
    private val connection: StatefulRedisConnection<String, String> = redisClient.connect()
    private val commands = connection.sync()
    private val flightListType = object : TypeReference<List<FlightRecord>>() {}

    fun getFlight(flightId: UUID): FlightRecord? =
        readValue(
            key = flightKey(flightId),
            cacheLabel = "flight",
        ) { json ->
            objectMapper.readValue(json, FlightRecord::class.java)
        }

    fun putFlight(flight: FlightRecord) {
        commands.setex(flightKey(flight.id), ttl.seconds, objectMapper.writeValueAsString(flight))
    }

    fun getSearch(query: SearchFlightsQuery): List<FlightRecord>? =
        readValue(
            key = searchKey(query),
            cacheLabel = "search",
        ) { json ->
            objectMapper.readValue(json, flightListType)
        }

    fun putSearch(
        query: SearchFlightsQuery,
        flights: List<FlightRecord>,
    ) {
        commands.setex(searchKey(query), ttl.seconds, objectMapper.writeValueAsString(flights))
    }

    fun invalidateFlightRelated(flightId: UUID) {
        val flightKey = flightKey(flightId)
        commands.del(flightKey)

        val searchKeys = commands.keys("search:*")
        if (searchKeys.isNotEmpty()) {
            commands.del(*searchKeys.toTypedArray())
        }

        logger.info(
            "cache invalidated for flightId={} flightKey={} clearedSearchKeys={}",
            flightId,
            flightKey,
            searchKeys.size,
        )
    }

    override fun close() {
        connection.close()
        redisClient.shutdown()
    }

    private fun <T> readValue(
        key: String,
        cacheLabel: String,
        reader: (String) -> T,
    ): T? {
        val cachedValue = commands.get(key)
        if (cachedValue == null) {
            logger.info("cache miss label={} key={}", cacheLabel, key)
            return null
        }

        return runCatching {
            reader(cachedValue)
        }.onSuccess {
            logger.info("cache hit label={} key={}", cacheLabel, key)
        }.onFailure { exception ->
            logger.warn("cache decode failure label={} key={}: {}", cacheLabel, key, exception.message)
            commands.del(key)
        }.getOrNull()
    }

    private fun flightKey(flightId: UUID): String = "flight:$flightId"

    private fun searchKey(query: SearchFlightsQuery): String =
        "search:${query.origin.uppercase()}:${query.destination.uppercase()}:${query.departureDate ?: "all"}"
}

private fun redisUri(connectionConfig: RedisConnectionConfig): RedisURI =
    when (connectionConfig) {
        is RedisConnectionConfig.Standalone -> RedisURI.create(connectionConfig.uri)
        is RedisConnectionConfig.Sentinel -> {
            val sentinels = connectionConfig.sentinels
            val primarySentinel = sentinels.first()
            val builder = RedisURI.Builder.sentinel(primarySentinel.host, primarySentinel.port, connectionConfig.masterId)

            sentinels.drop(1).forEach { sentinel ->
                builder.withSentinel(sentinel.host, sentinel.port)
            }

            builder.withDatabase(connectionConfig.database).build()
        }
    }
