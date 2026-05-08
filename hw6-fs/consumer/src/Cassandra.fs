module Warehouse.Cassandra

open System
open Cassandra
open Warehouse.Model

type Inventory =
    { Available: int
      Reserved: int
      LastUpdated: DateTimeOffset option }

let emptyInventory =
    { Available = 0
      Reserved = 0
      LastUpdated = None }

let connect (config: Config.Config) =
    let contactPoints =
        config.CassandraContactPoints.Split(',')
        |> Array.map (fun s -> s.Trim())

    let builder = Cluster.Builder()

    for cp in contactPoints do
        builder.AddContactPoint(cp) |> ignore

    builder.WithPort(config.CassandraPort) |> ignore

    let queryOptions = QueryOptions()
    queryOptions.SetConsistencyLevel(ConsistencyLevel.One) |> ignore
    builder.WithQueryOptions(queryOptions) |> ignore

    let cluster = builder.Build()
    cluster.Connect(config.CassandraKeyspace)

type Repository(session: ISession) =
    let isEventProcessed =
        session.Prepare("SELECT event_id FROM processed_events WHERE event_id = ?")

    let insertProcessedEvent =
        session.Prepare("INSERT INTO processed_events (event_id, event_type, processed_at) VALUES (?, ?, ?)")

    let getInventoryByProductZone =
        session.Prepare(
            "SELECT available_quantity, reserved_quantity, last_updated FROM inventory_by_product_zone WHERE product_id = ? AND zone_id = ?"
        )

    let upsertInventoryByProductZone =
        session.Prepare(
            "UPDATE inventory_by_product_zone SET available_quantity = ?, reserved_quantity = ?, last_updated = ? WHERE product_id = ? AND zone_id = ?"
        )

    let upsertInventoryByProduct =
        session.Prepare(
            "UPDATE inventory_by_product SET total_available = ?, total_reserved = ?, last_updated = ? WHERE product_id = ?"
        )

    let upsertInventoryByZone =
        session.Prepare(
            "UPDATE inventory_by_zone SET available_quantity = ?, reserved_quantity = ?, last_updated = ? WHERE zone_id = ? AND product_id = ?"
        )

    let getAllZonesForProduct =
        session.Prepare(
            "SELECT zone_id, available_quantity, reserved_quantity FROM inventory_by_product_zone WHERE product_id = ?"
        )

    let insertOrder =
        session.Prepare("INSERT INTO orders (order_id, status, created_at) VALUES (?, ?, ?)")

    let updateOrderCompleted =
        session.Prepare("UPDATE orders SET status = ?, completed_at = ? WHERE order_id = ?")

    let insertOrderItem =
        session.Prepare("INSERT INTO order_items (order_id, product_id, zone_id, quantity) VALUES (?, ?, ?, ?)")

    let getOrderItems =
        session.Prepare("SELECT product_id, zone_id, quantity FROM order_items WHERE order_id = ?")

    let updateSupplierId =
        session.Prepare("UPDATE inventory_by_product_zone SET supplier_id = ? WHERE product_id = ? AND zone_id = ?")

    member _.IsProcessed(eventId: string) =
        let rs = session.Execute(isEventProcessed.Bind(eventId))
        rs |> Seq.isEmpty |> not

    member _.MarkProcessedStatement(eventId: string, eventType: string, now: DateTimeOffset) : BoundStatement =
        insertProcessedEvent.Bind(eventId, eventType, box now)

    member _.GetZoneInventory(productId: string, zoneId: string) =
        let rs = session.Execute(getInventoryByProductZone.Bind(productId, zoneId))

        match rs |> Seq.tryHead with
        | None -> emptyInventory
        | Some row ->
            { Available = row.GetValue<int>("available_quantity")
              Reserved = row.GetValue<int>("reserved_quantity")
              LastUpdated =
                if row.IsNull("last_updated") then
                    None
                else
                    Some(row.GetValue<DateTimeOffset>("last_updated")) }

    member _.ZoneStatements
        (productId: string, zoneId: string, available: int, reserved: int, ts: DateTimeOffset)
        : BoundStatement list =
        [ upsertInventoryByProductZone.Bind(box available, box reserved, box ts, productId, zoneId)
          upsertInventoryByZone.Bind(box available, box reserved, box ts, zoneId, productId) ]

    member _.ProductStatement
        (productId: string, zoneOverrides: Map<string, int * int>, ts: DateTimeOffset)
        : BoundStatement =
        let rows =
            session.Execute(getAllZonesForProduct.Bind(productId)) |> Seq.toList

        let fromDb, seen =
            rows
            |> List.fold
                (fun (acc, seen) (row: Row) ->
                    let z = row.GetValue<string>("zone_id")

                    match Map.tryFind z zoneOverrides with
                    | Some(a, r) -> ((a, r) :: acc, Set.add z seen)
                    | None ->
                        let a = row.GetValue<int>("available_quantity")
                        let r = row.GetValue<int>("reserved_quantity")
                        ((a, r) :: acc, seen))
                ([], Set.empty)

        let unseen =
            zoneOverrides
            |> Map.toList
            |> List.choose (fun (z, v) -> if Set.contains z seen then None else Some v)

        let totalAvail, totalRes =
            fromDb @ unseen
            |> List.fold (fun (a, r) (da, dr) -> (a + da, r + dr)) (0, 0)

        upsertInventoryByProduct.Bind(box totalAvail, box totalRes, box ts, productId)

    member this.InventoryStatements
        (productId: string, zoneId: string, available: int, reserved: int, ts: DateTimeOffset)
        : BoundStatement list =
        this.ZoneStatements(productId, zoneId, available, reserved, ts)
        @ [ this.ProductStatement(productId, Map.ofList [ zoneId, (available, reserved) ], ts) ]

    member _.CreateOrderStatements(orderId: string, items: OrderItem list, ts: DateTimeOffset) : BoundStatement list =
        let order = insertOrder.Bind(orderId, "CREATED", box ts)

        let itemStmts =
            items
            |> List.map (fun item -> insertOrderItem.Bind(orderId, item.ProductId, item.ZoneId, box item.Quantity))

        order :: itemStmts

    member _.CompleteOrderStatements(orderId: string, ts: DateTimeOffset) : BoundStatement list * OrderItem list =
        let orderStmt = updateOrderCompleted.Bind("COMPLETED", box ts, orderId)
        let rows = session.Execute(getOrderItems.Bind(orderId))

        let items =
            rows
            |> Seq.map (fun (row: Row) ->
                { ProductId = row.GetValue<string>("product_id")
                  ZoneId = row.GetValue<string>("zone_id")
                  Quantity = row.GetValue<int>("quantity") })
            |> Seq.toList

        ([ orderStmt ], items)

    member _.SetSupplierIdStatement(productId: string, zoneId: string, supplierId: string) : BoundStatement =
        updateSupplierId.Bind(supplierId, productId, zoneId)

    member _.ExecuteBatch(statements: BoundStatement list) =
        let batch = BatchStatement()
        batch.SetConsistencyLevel(ConsistencyLevel.Quorum) |> ignore

        for stmt in statements do
            batch.Add(stmt) |> ignore

        try
            session.Execute(batch :> IStatement) |> ignore
        with ex ->
            Metrics.cassandraWriteErrors.Inc()
            raise ex

    member _.IsAvailable() =
        try
            session.Execute("SELECT key FROM system.local") |> ignore
            true
        with _ ->
            false
