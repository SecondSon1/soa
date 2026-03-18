package com.example.flight

import com.example.flight.cache.FlightCache
import com.example.flight.config.loadFlightServiceConfig
import com.example.flight.db.DatabaseFactory
import com.example.flight.grpc.FlightGrpcService
import com.example.flight.grpc.auth.ApiKeyServerInterceptor
import com.example.flight.repository.FlightRepository
import com.example.flight.service.FlightDomainService
import com.fasterxml.jackson.databind.SerializationFeature
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule
import com.fasterxml.jackson.module.kotlin.kotlinModule
import io.grpc.netty.shaded.io.grpc.netty.NettyServerBuilder
import com.fasterxml.jackson.databind.ObjectMapper
import java.util.concurrent.TimeUnit

fun main() {
    val config = loadFlightServiceConfig()
    val dataSource = DatabaseFactory.createDataSource(config.database)
    val repository = FlightRepository(dataSource)
    val objectMapper =
        ObjectMapper()
            .registerModule(kotlinModule())
            .registerModule(JavaTimeModule())
            .disable(SerializationFeature.WRITE_DATES_AS_TIMESTAMPS)
    val cache = FlightCache(config.cache.connection, config.cache.ttl, objectMapper)
    val flightService = FlightDomainService(repository, cache)
    val server =
        NettyServerBuilder
            .forPort(config.grpcPort)
            .intercept(ApiKeyServerInterceptor(config.auth.apiKey))
            .addService(FlightGrpcService(flightService))
            .build()

    Runtime.getRuntime().addShutdownHook(
        Thread {
            server.shutdown()
            server.awaitTermination(5, TimeUnit.SECONDS)
            cache.close()
            dataSource.close()
        },
    )

    server.start()
    server.awaitTermination()
}
