module Warehouse.Dlq

open System
open System.Text.Json
open Confluent.Kafka
open Serilog

type DlqProducer(config: Config.Config) =
    let producerConfig = ProducerConfig()
    do producerConfig.BootstrapServers <- config.KafkaBootstrapServers

    let producer =
        ProducerBuilder<Null, string>(producerConfig).Build()

    member _.Send(originalEvent: string, error: string, partition: int, offset: int64) =
        let msg =
            dict [
                "original_event", box originalEvent
                "error_reason", box error
                "error_code", box "VALIDATION_ERROR"
                "failed_at", box (DateTimeOffset.UtcNow.ToString("o"))
                "kafka_metadata", box (dict [ "partition", box partition; "offset", box offset ])
            ]
            |> JsonSerializer.Serialize

        let record = Message<Null, string>(Value = msg)

        try
            producer.ProduceAsync(config.DlqTopic, record).GetAwaiter().GetResult() |> ignore
            Log.Warning("Sent to DLQ: {ErrorReason}", error)
        with ex ->
            Log.Error(ex, "Failed to send to DLQ")

    interface IDisposable with
        member _.Dispose() = producer.Dispose()
