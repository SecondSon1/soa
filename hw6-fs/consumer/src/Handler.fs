module Warehouse.Handler

open System
open Serilog
open Warehouse.Model
open Warehouse.Cassandra

let handle (repo: Repository) (event: WarehouseEvent) =
    if repo.IsProcessed(event.EventId) then
        Log.Information("Skipping duplicate event: {EventId}", event.EventId)
    else
        use _ = Metrics.startTimer Metrics.processingDuration
        let ts = DateTimeOffset.FromUnixTimeMilliseconds(event.Timestamp)
        let now = DateTimeOffset.UtcNow

        let isStale (inv: Inventory) =
            match inv.LastUpdated with
            | Some lu -> ts <= lu
            | None -> false

        let stmts =
            match event.Payload with
            | ProductReceived(productId, zoneId, quantity) ->
                let inv = repo.GetZoneInventory(productId, zoneId)

                if isStale inv then
                    None
                else
                    let supplierStmt =
                        event.SupplierId
                        |> Option.map (fun sid -> repo.SetSupplierIdStatement(productId, zoneId, sid))
                        |> Option.toList

                    Some(
                        repo.InventoryStatements(productId, zoneId, inv.Available + quantity, inv.Reserved, ts)
                        @ supplierStmt
                    )

            | ProductShipped(productId, zoneId, quantity) ->
                let inv = repo.GetZoneInventory(productId, zoneId)

                if isStale inv then
                    None
                else
                    Some(repo.InventoryStatements(productId, zoneId, inv.Available - quantity, inv.Reserved, ts))

            | ProductMoved(productId, fromZone, toZone, quantity) ->
                let fromInv = repo.GetZoneInventory(productId, fromZone)
                let toInv = repo.GetZoneInventory(productId, toZone)

                if isStale fromInv || isStale toInv then
                    None
                else
                    let fromAvail = fromInv.Available - quantity
                    let toAvail = toInv.Available + quantity

                    Some(
                        repo.ZoneStatements(productId, fromZone, fromAvail, fromInv.Reserved, ts)
                        @ repo.ZoneStatements(productId, toZone, toAvail, toInv.Reserved, ts)
                        @ [ repo.ProductStatement(
                                productId,
                                Map.ofList [ fromZone, (fromAvail, fromInv.Reserved)
                                             toZone, (toAvail, toInv.Reserved) ],
                                ts) ]
                    )

            | ProductReserved(productId, zoneId, quantity) ->
                let inv = repo.GetZoneInventory(productId, zoneId)

                if isStale inv then
                    None
                else
                    Some(
                        repo.InventoryStatements(productId, zoneId, inv.Available - quantity, inv.Reserved + quantity, ts)
                    )

            | ProductReleased(productId, zoneId, quantity) ->
                let inv = repo.GetZoneInventory(productId, zoneId)

                if isStale inv then
                    None
                else
                    Some(
                        repo.InventoryStatements(productId, zoneId, inv.Available + quantity, inv.Reserved - quantity, ts)
                    )

            | InventoryCounted(productId, zoneId, quantity) ->
                let inv = repo.GetZoneInventory(productId, zoneId)

                if isStale inv then
                    None
                else
                    Some(repo.InventoryStatements(productId, zoneId, quantity, inv.Reserved, ts))

            | OrderCreated(orderId, items) ->
                let inventories =
                    items |> List.map (fun i -> repo.GetZoneInventory(i.ProductId, i.ZoneId))

                if inventories |> List.exists isStale then
                    None
                else
                    let orderStmts = repo.CreateOrderStatements(orderId, items, ts)

                    let zoneChanges =
                        List.zip items inventories
                        |> List.map (fun (item, inv) ->
                            (item.ProductId, item.ZoneId, inv.Available - item.Quantity, inv.Reserved + item.Quantity))

                    let zoneStmts =
                        zoneChanges
                        |> List.collect (fun (pid, zid, avail, res) -> repo.ZoneStatements(pid, zid, avail, res, ts))

                    let productStmts =
                        zoneChanges
                        |> List.groupBy (fun (pid, _, _, _) -> pid)
                        |> List.map (fun (pid, changes) ->
                            let overrides =
                                changes
                                |> List.map (fun (_, zid, avail, res) -> (zid, (avail, res)))
                                |> Map.ofList

                            repo.ProductStatement(pid, overrides, ts))

                    Some(orderStmts @ zoneStmts @ productStmts)

            | OrderCompleted orderId ->
                let orderStmts, items = repo.CompleteOrderStatements(orderId, ts)

                let inventories =
                    items |> List.map (fun i -> repo.GetZoneInventory(i.ProductId, i.ZoneId))

                if inventories |> List.exists isStale then
                    None
                else
                    let zoneChanges =
                        List.zip items inventories
                        |> List.map (fun (item, inv) ->
                            (item.ProductId, item.ZoneId, inv.Available, inv.Reserved - item.Quantity))

                    let zoneStmts =
                        zoneChanges
                        |> List.collect (fun (pid, zid, avail, res) -> repo.ZoneStatements(pid, zid, avail, res, ts))

                    let productStmts =
                        zoneChanges
                        |> List.groupBy (fun (pid, _, _, _) -> pid)
                        |> List.map (fun (pid, changes) ->
                            let overrides =
                                changes
                                |> List.map (fun (_, zid, avail, res) -> (zid, (avail, res)))
                                |> Map.ofList

                            repo.ProductStatement(pid, overrides, ts))

                    Some(orderStmts @ zoneStmts @ productStmts)

        match stmts with
        | None ->
            Log.Warning("Skipping out-of-order event: {EventId}", event.EventId)
        | Some stmts ->
            let allStmts =
                stmts @ [ repo.MarkProcessedStatement(event.EventId, payloadName event.Payload, now) ]

            repo.ExecuteBatch(allStmts)
            Metrics.eventsProcessed.WithLabels(payloadName event.Payload).Inc()
            Log.Information("Processed event: {EventId} type={EventType}", event.EventId, payloadName event.Payload)
