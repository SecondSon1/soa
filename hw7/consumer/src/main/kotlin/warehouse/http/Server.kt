package warehouse.http

import com.sun.net.httpserver.HttpServer
import io.prometheus.client.CollectorRegistry
import io.prometheus.client.exporter.common.TextFormat
import java.io.StringWriter
import java.net.InetSocketAddress

class Server(
    private val port: Int,
    private val healthCheck: () -> Boolean,
) {
    private val server = HttpServer.create(InetSocketAddress(port), 0)

    init {
        server.createContext("/metrics") { exchange ->
            val writer = StringWriter()
            TextFormat.write004(writer, CollectorRegistry.defaultRegistry.metricFamilySamples())
            val body = writer.toString().toByteArray()
            exchange.responseHeaders.add("Content-Type", TextFormat.CONTENT_TYPE_004)
            exchange.sendResponseHeaders(200, body.size.toLong())
            exchange.responseBody.use { it.write(body) }
        }

        server.createContext("/health") { exchange ->
            val healthy = healthCheck()
            val status = if (healthy) 200 else 503
            val body = if (healthy) """{"status":"UP"}""" else """{"status":"DOWN"}"""
            val bytes = body.toByteArray()
            exchange.responseHeaders.add("Content-Type", "application/json")
            exchange.sendResponseHeaders(status, bytes.size.toLong())
            exchange.responseBody.use { it.write(bytes) }
        }
    }

    fun start() {
        server.start()
    }

    fun stop() {
        server.stop(0)
    }
}
