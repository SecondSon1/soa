module Warehouse.Kafka

open System
open Chr.Avro.Confluent
open Confluent.Kafka
open Confluent.SchemaRegistry
open Serilog
open Warehouse.Model

[<CLIMutable>]
type AvroOrderItem =
    { product_id: string
      zone_id: string
      quantity: int }

[<CLIMutable>]
type AvroWarehouseEvent =
    { event_id: string
      event_type: string
      timestamp: int64
      product_id: string option
      zone_id: string option
      quantity: int64 option
      from_zone_id: string option
      to_zone_id: string option
      order_id: string option
      order_items: AvroOrderItem array
      supplier_id: string option }

let parseAvro (record: AvroWarehouseEvent) =
    let eventType = record.event_type

    let reqStr field (v: string option) =
        match v with
        | None -> failwithf "%s is required for %s" field eventType
        | Some s -> s

    let reqPosInt field (v: int64 option) =
        match v with
        | None -> failwithf "%s is required for %s" field eventType
        | Some i ->
            if i <= 0L then
                failwithf "Invalid %s: %d (must be positive)" field i

            int i

    let payload =
        match eventType with
        | "PRODUCT_RECEIVED" ->
            ProductReceived(
                reqStr "product_id" record.product_id,
                reqStr "zone_id" record.zone_id,
                reqPosInt "quantity" record.quantity
            )
        | "PRODUCT_SHIPPED" ->
            ProductShipped(
                reqStr "product_id" record.product_id,
                reqStr "zone_id" record.zone_id,
                reqPosInt "quantity" record.quantity
            )
        | "PRODUCT_MOVED" ->
            ProductMoved(
                reqStr "product_id" record.product_id,
                reqStr "from_zone_id" record.from_zone_id,
                reqStr "to_zone_id" record.to_zone_id,
                reqPosInt "quantity" record.quantity
            )
        | "PRODUCT_RESERVED" ->
            ProductReserved(
                reqStr "product_id" record.product_id,
                reqStr "zone_id" record.zone_id,
                reqPosInt "quantity" record.quantity
            )
        | "PRODUCT_RELEASED" ->
            ProductReleased(
                reqStr "product_id" record.product_id,
                reqStr "zone_id" record.zone_id,
                reqPosInt "quantity" record.quantity
            )
        | "INVENTORY_COUNTED" ->
            InventoryCounted(
                reqStr "product_id" record.product_id,
                reqStr "zone_id" record.zone_id,
                reqPosInt "quantity" record.quantity
            )
        | "ORDER_CREATED" ->
            let orderId = reqStr "order_id" record.order_id

            let items =
                let arr = record.order_items

                if isNull arr || Array.isEmpty arr then
                    failwith "order_items is required for ORDER_CREATED"

                arr
                |> Array.map (fun i ->
                    if i.quantity <= 0 then
                        failwithf "Invalid quantity: %d (must be positive)" i.quantity

                    { ProductId = i.product_id
                      ZoneId = i.zone_id
                      Quantity = i.quantity })
                |> Array.toList

            OrderCreated(orderId, items)
        | "ORDER_COMPLETED" -> OrderCompleted(reqStr "order_id" record.order_id)
        | _ -> failwithf "Unknown event type: %s" eventType

    { EventId = record.event_id
      Timestamp = record.timestamp
      Payload = payload
      SupplierId = record.supplier_id }

let run (config: Config.Config) (dlq: Dlq.DlqProducer) (connected: bool ref) (handle: WarehouseEvent -> unit) =
    let srConfig = SchemaRegistryConfig()
    srConfig.Url <- config.SchemaRegistryUrl
    use schemaRegistry = new CachedSchemaRegistryClient(srConfig)

    let consumerConfig = ConsumerConfig()
    consumerConfig.BootstrapServers <- config.KafkaBootstrapServers
    consumerConfig.GroupId <- config.KafkaGroupId
    consumerConfig.EnableAutoCommit <- Nullable<bool>(false)
    consumerConfig.AutoOffsetReset <- Nullable<AutoOffsetReset>(AutoOffsetReset.Earliest)

    use consumer =
        ConsumerBuilder<string, AvroWarehouseEvent>(consumerConfig)
            .SetAvroValueDeserializer(schemaRegistry)
            .Build()

    consumer.Subscribe(config.KafkaTopic)
    Log.Information("Consumer started, subscribed to {Topic}", config.KafkaTopic)

    let mutable running = true

    Console.CancelKeyPress.Add(fun args ->
        args.Cancel <- true
        running <- false
        Log.Information("Shutdown signal received (SIGINT)"))

    AppDomain.CurrentDomain.ProcessExit.Add(fun _ ->
        running <- false
        Log.Information("Shutdown signal received (SIGTERM)"))

    let updateLag () =
        try
            for tp in consumer.Assignment do
                let watermark = consumer.QueryWatermarkOffsets(tp, TimeSpan.FromSeconds(1.0))
                let position = consumer.Position(tp)

                if position.Value >= 0L then
                    let lag = max 0L (watermark.High.Value - position.Value)
                    Metrics.consumerLag.WithLabels(string tp.Partition.Value).Set(float lag)
        with _ ->
            ()

    while running do
        try
            let cr = consumer.Consume(TimeSpan.FromSeconds(1.0))

            connected.Value <- true
            updateLag ()

            if not (isNull cr) then
                let partition = cr.Partition.Value
                let offset = cr.Offset.Value
                Log.Debug("Received: partition={Partition} offset={Offset}", partition, offset)

                try
                    let event = parseAvro cr.Message.Value

                    Log.Information(
                        "Processing: event_id={EventId} type={EventType} offset={Offset} partition={Partition}",
                        event.EventId,
                        payloadName event.Payload,
                        offset,
                        partition
                    )

                    handle event
                    consumer.Commit(cr) |> ignore
                with ex ->
                    Log.Error(ex, "Failed processing partition={Partition} offset={Offset}", partition, offset)
                    dlq.Send(sprintf "%A" cr.Message.Value, ex.Message, partition, offset)
                    consumer.Commit(cr) |> ignore
        with :? ConsumeException as ex ->
            Log.Error(ex, "Consume error")

    consumer.Close()
    Log.Information("Consumer closed")
