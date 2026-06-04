package warehouse.http

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import io.prometheus.client.CollectorRegistry
import io.prometheus.client.exporter.common.TextFormat
import warehouse.metrics.Metrics
import java.io.StringWriter
import java.net.InetSocketAddress

class Server(
    private val port: Int,
    private val healthCheck: () -> Boolean,
) {
    private val server = HttpServer.create(InetSocketAddress(port), 0)

    init {
        server.createContext("/metrics") { exchange ->
            instrumented(exchange, "/metrics") {
                val writer = StringWriter()
                TextFormat.write004(writer, CollectorRegistry.defaultRegistry.metricFamilySamples())
                val body = writer.toString().toByteArray()
                exchange.responseHeaders.add("Content-Type", TextFormat.CONTENT_TYPE_004)
                exchange.sendResponseHeaders(200, body.size.toLong())
                exchange.responseBody.use { it.write(body) }
                200
            }
        }

        server.createContext("/health") { exchange ->
            instrumented(exchange, "/health") {
                val healthy = healthCheck()
                val status = if (healthy) 200 else 503
                val body = if (healthy) """{"status":"UP"}""" else """{"status":"DOWN"}"""
                val bytes = body.toByteArray()
                exchange.responseHeaders.add("Content-Type", "application/json")
                exchange.sendResponseHeaders(status, bytes.size.toLong())
                exchange.responseBody.use { it.write(bytes) }
                status
            }
        }
    }

    private fun instrumented(exchange: HttpExchange, endpoint: String, handler: () -> Int) {
        val method = exchange.requestMethod
        val timer = Metrics.httpRequestDuration.labels(method, endpoint).startTimer()
        try {
            val status = handler()
            Metrics.httpRequestsTotal.labels(method, endpoint, status.toString()).inc()
        } catch (e: Exception) {
            Metrics.httpRequestsTotal.labels(method, endpoint, "500").inc()
            Metrics.httpRequestErrorsTotal.labels(method, endpoint, e.javaClass.simpleName).inc()
            val body = """{"error":"internal server error"}""".toByteArray()
            exchange.sendResponseHeaders(500, body.size.toLong())
            exchange.responseBody.use { it.write(body) }
        } finally {
            timer.observeDuration()
        }
    }

    fun start() {
        server.start()
    }

    fun stop() {
        server.stop(0)
    }
}
