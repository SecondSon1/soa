package com.example.booking.web

import com.example.booking.grpc.resilience.CIRCUIT_BREAKER_OPEN_DESCRIPTION
import com.fasterxml.jackson.databind.PropertyNamingStrategies
import com.fasterxml.jackson.databind.SerializationFeature
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule
import com.fasterxml.jackson.module.kotlin.kotlinModule
import io.grpc.Status
import io.grpc.StatusRuntimeException
import io.ktor.http.HttpStatusCode
import io.ktor.serialization.jackson.jackson
import io.ktor.server.application.Application
import io.ktor.server.application.install
import io.ktor.server.plugins.BadRequestException
import io.ktor.server.plugins.ContentTransformationException
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.plugins.statuspages.StatusPages
import io.ktor.server.response.respond

class ApiException(
    val statusCode: HttpStatusCode,
    override val message: String,
) : RuntimeException(message)

fun Application.configureHttp() {
    install(ContentNegotiation) {
        jackson {
            registerModule(kotlinModule())
            registerModule(JavaTimeModule())
            propertyNamingStrategy = PropertyNamingStrategies.SNAKE_CASE
            disable(SerializationFeature.WRITE_DATES_AS_TIMESTAMPS)
            findAndRegisterModules()
        }
    }

    install(StatusPages) {
        exception<ApiException> { call, cause ->
            call.respond(cause.statusCode, ErrorResponse(cause.message))
        }
        exception<StatusRuntimeException> { call, cause ->
            val (statusCode, message) = cause.toHttpError()
            call.respond(statusCode, ErrorResponse(message))
        }
        exception<BadRequestException> { call, cause ->
            val message =
                if (cause.message?.startsWith("Failed to convert request body to class") == true) {
                    "Request body is malformed or does not match the expected JSON schema"
                } else {
                    cause.message ?: "Bad request"
                }
            call.respond(HttpStatusCode.BadRequest, ErrorResponse(message))
        }
        exception<ContentTransformationException> { call, _ ->
            call.respond(
                HttpStatusCode.BadRequest,
                ErrorResponse("Request body is malformed or does not match the expected JSON schema"),
            )
        }
        exception<IllegalArgumentException> { call, cause ->
            call.respond(HttpStatusCode.BadRequest, ErrorResponse(cause.message ?: "Invalid request"))
        }
        exception<Throwable> { call, cause ->
            call.respond(
                HttpStatusCode.InternalServerError,
                ErrorResponse(cause.message ?: "Unexpected server error"),
            )
        }
    }
}

private fun StatusRuntimeException.toHttpError(): Pair<HttpStatusCode, String> =
    when (status.code) {
        Status.Code.NOT_FOUND -> HttpStatusCode.NotFound to (status.description ?: "Resource not found")
        Status.Code.INVALID_ARGUMENT -> HttpStatusCode.BadRequest to (status.description ?: "Invalid request")
        Status.Code.RESOURCE_EXHAUSTED -> HttpStatusCode.Conflict to (status.description ?: "Resource exhausted")
        Status.Code.FAILED_PRECONDITION ->
            if (status.description == CIRCUIT_BREAKER_OPEN_DESCRIPTION) {
                HttpStatusCode.ServiceUnavailable to "Flight service is temporarily unavailable"
            } else {
                HttpStatusCode.Conflict to (status.description ?: "Request precondition failed")
            }
        Status.Code.UNAUTHENTICATED -> HttpStatusCode.BadGateway to "Upstream service authentication failed"
        Status.Code.UNAVAILABLE, Status.Code.DEADLINE_EXCEEDED ->
            HttpStatusCode.ServiceUnavailable to (status.description ?: "Upstream service is unavailable")
        else -> HttpStatusCode.BadGateway to (status.description ?: "Upstream service call failed")
    }
