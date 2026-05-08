package warehouse.metrics

import io.prometheus.client.Counter
import io.prometheus.client.Gauge
import io.prometheus.client.Histogram

object Metrics {
    val consumerLag: Gauge = Gauge.build()
        .name("consumer_lag")
        .help("Consumer lag per partition")
        .labelNames("partition")
        .register()

    val eventsProcessed: Counter = Counter.build()
        .name("events_processed_total")
        .help("Total events processed")
        .labelNames("event_type")
        .register()

    val processingDuration: Histogram = Histogram.build()
        .name("event_processing_duration_seconds")
        .help("Event processing duration in seconds")
        .buckets(0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0)
        .register()

    val cassandraWriteErrors: Counter = Counter.build()
        .name("cassandra_write_errors_total")
        .help("Total Cassandra write errors")
        .register()
}
