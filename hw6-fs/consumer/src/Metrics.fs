module Warehouse.Metrics

open System
open System.Diagnostics
open Prometheus

let eventsProcessed =
    Metrics.CreateCounter(
        "events_processed_total",
        "Total events processed",
        CounterConfiguration(LabelNames = [| "event_type" |])
    )

let processingDuration =
    Metrics.CreateHistogram(
        "event_processing_duration_seconds",
        "Event processing duration",
        HistogramConfiguration(Buckets = [| 0.001; 0.005; 0.01; 0.05; 0.1; 0.5; 1.0 |])
    )

let consumerLag =
    Metrics.CreateGauge(
        "consumer_lag",
        "Consumer lag per partition",
        GaugeConfiguration(LabelNames = [| "partition" |])
    )

let cassandraWriteErrors =
    Metrics.CreateCounter("cassandra_write_errors_total", "Total Cassandra write errors")

let startTimer (histogram: Histogram) =
    let sw = Stopwatch.StartNew()

    { new IDisposable with
        member _.Dispose() =
            histogram.Observe(sw.Elapsed.TotalSeconds) }
