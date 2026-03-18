package com.example.booking.grpc.resilience

import com.example.booking.config.RetryConfig
import io.grpc.Status
import java.time.Duration

private val retryableStatusCodes = setOf(Status.Code.UNAVAILABLE, Status.Code.DEADLINE_EXCEEDED)

fun shouldRetry(statusCode: Status.Code): Boolean = statusCode in retryableStatusCodes

fun nextBackoff(
    currentBackoff: Duration,
    config: RetryConfig,
): Duration {
    val multipliedBackoff = (currentBackoff.toMillis() * config.backoffMultiplier).toLong()
    val clampedBackoff = multipliedBackoff.coerceAtMost(config.maxBackoff.toMillis())
    return Duration.ofMillis(clampedBackoff)
}
