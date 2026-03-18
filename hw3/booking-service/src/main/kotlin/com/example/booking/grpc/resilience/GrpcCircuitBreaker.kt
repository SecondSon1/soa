package com.example.booking.grpc.resilience

import com.example.booking.config.CircuitBreakerConfig
import io.grpc.Status
import org.slf4j.LoggerFactory
import java.time.Clock
import java.time.Instant
import java.util.ArrayDeque

const val CIRCUIT_BREAKER_OPEN_DESCRIPTION = "Flight service circuit breaker is open"

private enum class CircuitBreakerState {
    CLOSED,
    OPEN,
    HALF_OPEN,
}

class GrpcCircuitBreaker(
    private val config: CircuitBreakerConfig,
    private val clock: Clock = Clock.systemUTC(),
) {
    private val logger = LoggerFactory.getLogger(javaClass)
    private val failureInstants = ArrayDeque<Instant>()

    private var state = CircuitBreakerState.CLOSED
    private var openUntil: Instant = Instant.EPOCH
    private var halfOpenProbeInProgress = false

    @Synchronized
    fun tryAcquirePermission(methodName: String): Boolean {
        val now = Instant.now(clock)
        trimExpiredFailures(now)

        return when (state) {
            CircuitBreakerState.CLOSED -> true
            CircuitBreakerState.OPEN ->
                if (now >= openUntil) {
                    transitionTo(
                        CircuitBreakerState.HALF_OPEN,
                        "open timeout elapsed for $methodName; allowing a probe call",
                    )
                    halfOpenProbeInProgress = true
                    true
                } else {
                    false
                }
            CircuitBreakerState.HALF_OPEN ->
                if (!halfOpenProbeInProgress) {
                    halfOpenProbeInProgress = true
                    true
                } else {
                    false
                }
        }
    }

    @Synchronized
    fun onCallFinished(
        methodName: String,
        status: Status,
    ) {
        if (status.description == CIRCUIT_BREAKER_OPEN_DESCRIPTION) {
            return
        }

        val now = Instant.now(clock)
        trimExpiredFailures(now)

        if (status.isOk || !shouldRetry(status.code)) {
            onReachableResult(methodName)
            return
        }

        when (state) {
            CircuitBreakerState.CLOSED -> {
                failureInstants.addLast(now)
                if (failureInstants.size >= config.failureThreshold) {
                    open(now, "failure threshold reached after ${status.code} on $methodName")
                }
            }
            CircuitBreakerState.OPEN -> Unit
            CircuitBreakerState.HALF_OPEN -> {
                halfOpenProbeInProgress = false
                open(now, "probe call failed with ${status.code} on $methodName")
            }
        }
    }

    @Synchronized
    fun rejectionStatus(): Status =
        Status.FAILED_PRECONDITION.withDescription(CIRCUIT_BREAKER_OPEN_DESCRIPTION)

    @Synchronized
    private fun onReachableResult(methodName: String) {
        if (state == CircuitBreakerState.HALF_OPEN) {
            halfOpenProbeInProgress = false
            failureInstants.clear()
            transitionTo(CircuitBreakerState.CLOSED, "probe call succeeded for $methodName")
        }
    }

    private fun open(
        now: Instant,
        reason: String,
    ) {
        openUntil = now.plus(config.openStateTimeout)
        halfOpenProbeInProgress = false
        transitionTo(CircuitBreakerState.OPEN, reason)
    }

    private fun transitionTo(
        nextState: CircuitBreakerState,
        reason: String,
    ) {
        if (state == nextState) {
            return
        }

        logger.info("grpc circuit breaker transition {} -> {} ({})", state, nextState, reason)
        state = nextState
    }

    private fun trimExpiredFailures(now: Instant) {
        val cutoff = now.minus(config.windowSize)
        while (failureInstants.isNotEmpty() && failureInstants.first() < cutoff) {
            failureInstants.removeFirst()
        }
    }
}
