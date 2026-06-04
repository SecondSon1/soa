package warehouse.handler

import com.datastax.oss.driver.api.core.cql.BoundStatement
import org.slf4j.LoggerFactory
import warehouse.cassandra.Repository
import warehouse.metrics.Metrics
import warehouse.model.EventType
import warehouse.model.WarehouseEvent
import java.time.Instant

class EventHandler(private val repo: Repository) {
    private val log = LoggerFactory.getLogger(EventHandler::class.java)

    fun handle(event: WarehouseEvent) {
        if (repo.isProcessed(event.eventId)) {
            log.info("duplicate {}", event.eventId)
            return
        }

        validate(event)

        val timer = Metrics.processingDuration.startTimer()
        try {
            val now = Instant.now()
            val eventTs = event.timestamp
            val statements = mutableListOf<BoundStatement>()

            when (event.eventType) {
                EventType.PRODUCT_RECEIVED -> handleReceived(event, eventTs, statements)
                EventType.PRODUCT_SHIPPED -> handleShipped(event, eventTs, statements)
                EventType.PRODUCT_MOVED -> handleMoved(event, eventTs, statements)
                EventType.PRODUCT_RESERVED -> handleReserved(event, eventTs, statements)
                EventType.PRODUCT_RELEASED -> handleReleased(event, eventTs, statements)
                EventType.INVENTORY_COUNTED -> handleCounted(event, eventTs, statements)
                EventType.ORDER_CREATED -> handleOrderCreated(event, eventTs, statements)
                EventType.ORDER_COMPLETED -> handleOrderCompleted(event, eventTs, statements)
            }

            statements.add(repo.markProcessedStatement(event.eventId, event.eventType.name, now))
            repo.executeBatch(statements)
            Metrics.eventsProcessed.labels(event.eventType.name).inc()
        } finally {
            timer.observeDuration()
        }
    }

    private fun validate(event: WarehouseEvent) {
        when (event.eventType) {
            EventType.PRODUCT_RECEIVED, EventType.PRODUCT_SHIPPED,
            EventType.PRODUCT_RESERVED, EventType.PRODUCT_RELEASED,
            EventType.INVENTORY_COUNTED -> {
                requireNotNull(event.productId) { "product_id is required" }
                requireNotNull(event.zoneId) { "zone_id is required" }
                val qty = requireNotNull(event.quantity) { "quantity is required" }
                require(qty > 0) { "Invalid quantity: $qty (must be positive)" }
            }
            EventType.PRODUCT_MOVED -> {
                requireNotNull(event.productId) { "product_id is required" }
                requireNotNull(event.fromZoneId) { "from_zone_id is required" }
                requireNotNull(event.toZoneId) { "to_zone_id is required" }
                val qty = requireNotNull(event.quantity) { "quantity is required" }
                require(qty > 0) { "Invalid quantity: $qty (must be positive)" }
            }
            EventType.ORDER_CREATED -> {
                requireNotNull(event.orderId) { "order_id is required" }
                val items = requireNotNull(event.orderItems) { "order_items is required" }
                require(items.isNotEmpty()) { "order_items must not be empty" }
                items.forEach { require(it.quantity > 0) { "Invalid quantity: ${it.quantity} (must be positive)" } }
            }
            EventType.ORDER_COMPLETED -> {
                requireNotNull(event.orderId) { "order_id is required" }
            }
        }
    }

    private fun isOutOfOrder(productId: String, zoneId: String, eventTimestamp: Instant): Boolean {
        val inv = repo.getZoneInventory(productId, zoneId)
        val lastUpdated = inv.lastUpdated ?: return false
        if (eventTimestamp <= lastUpdated) {
            log.warn("out-of-order: event_ts={} <= last={} product={} zone={}",
                eventTimestamp, lastUpdated, productId, zoneId)
            return true
        }
        return false
    }

    private fun handleReceived(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val productId = event.productId!!
        val zoneId = event.zoneId!!
        if (isOutOfOrder(productId, zoneId, event.timestamp)) return

        val inv = repo.getZoneInventory(productId, zoneId)
        val supplierId = event.supplierId ?: inv.supplierId
        stmts.addAll(repo.inventoryStatements(productId, zoneId, inv.available + event.quantity!!, inv.reserved, supplierId, ts))
    }

    private fun handleShipped(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val productId = event.productId!!
        val zoneId = event.zoneId!!
        if (isOutOfOrder(productId, zoneId, event.timestamp)) return

        val inv = repo.getZoneInventory(productId, zoneId)
        stmts.addAll(repo.inventoryStatements(productId, zoneId, inv.available - event.quantity!!, inv.reserved, inv.supplierId, ts))
    }

    private fun handleMoved(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val productId = event.productId!!
        val fromZone = event.fromZoneId!!
        val toZone = event.toZoneId!!
        if (isOutOfOrder(productId, fromZone, event.timestamp)) return

        val fromInv = repo.getZoneInventory(productId, fromZone)
        val newFromAvail = fromInv.available - event.quantity!!
        stmts.addAll(repo.zoneStatements(productId, fromZone, newFromAvail, fromInv.reserved, fromInv.supplierId, ts))

        val toInv = repo.getZoneInventory(productId, toZone)
        val newToAvail = toInv.available + event.quantity
        stmts.addAll(repo.zoneStatements(productId, toZone, newToAvail, toInv.reserved, toInv.supplierId, ts))

        stmts.add(repo.productTotalStatement(productId, mapOf(
            fromZone to (newFromAvail to fromInv.reserved),
            toZone to (newToAvail to toInv.reserved),
        ), ts))
    }

    private fun handleReserved(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val productId = event.productId!!
        val zoneId = event.zoneId!!
        if (isOutOfOrder(productId, zoneId, event.timestamp)) return

        val inv = repo.getZoneInventory(productId, zoneId)
        stmts.addAll(repo.inventoryStatements(productId, zoneId, inv.available - event.quantity!!, inv.reserved + event.quantity, inv.supplierId, ts))
    }

    private fun handleReleased(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val productId = event.productId!!
        val zoneId = event.zoneId!!
        if (isOutOfOrder(productId, zoneId, event.timestamp)) return

        val inv = repo.getZoneInventory(productId, zoneId)
        stmts.addAll(repo.inventoryStatements(productId, zoneId, inv.available + event.quantity!!, inv.reserved - event.quantity, inv.supplierId, ts))
    }

    private fun handleCounted(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val productId = event.productId!!
        val zoneId = event.zoneId!!
        if (isOutOfOrder(productId, zoneId, event.timestamp)) return

        val inv = repo.getZoneInventory(productId, zoneId)
        stmts.addAll(repo.inventoryStatements(productId, zoneId, event.quantity!!, inv.reserved, inv.supplierId, ts))
    }

    private fun handleOrderCreated(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val orderId = event.orderId!!
        val items = event.orderItems!!

        stmts.addAll(repo.createOrderStatements(orderId, items, ts))

        val overridesByProduct = mutableMapOf<String, MutableMap<String, Pair<Int, Int>>>()
        for (item in items) {
            val inv = repo.getZoneInventory(item.productId, item.zoneId)
            val newAvail = inv.available - item.quantity
            val newReserved = inv.reserved + item.quantity
            stmts.addAll(repo.zoneStatements(item.productId, item.zoneId, newAvail, newReserved, inv.supplierId, ts))
            overridesByProduct.getOrPut(item.productId) { mutableMapOf() }[item.zoneId] = newAvail to newReserved
        }
        for ((productId, overrides) in overridesByProduct) {
            stmts.add(repo.productTotalStatement(productId, overrides, ts))
        }
    }

    private fun handleOrderCompleted(event: WarehouseEvent, ts: Instant, stmts: MutableList<BoundStatement>) {
        val orderId = event.orderId!!

        val (orderStmts, items) = repo.completeOrderStatements(orderId, ts)
        stmts.addAll(orderStmts)

        val overridesByProduct = mutableMapOf<String, MutableMap<String, Pair<Int, Int>>>()
        for (item in items) {
            val inv = repo.getZoneInventory(item.productId, item.zoneId)
            val newReserved = inv.reserved - item.quantity
            stmts.addAll(repo.zoneStatements(item.productId, item.zoneId, inv.available, newReserved, inv.supplierId, ts))
            overridesByProduct.getOrPut(item.productId) { mutableMapOf() }[item.zoneId] = inv.available to newReserved
        }
        for ((productId, overrides) in overridesByProduct) {
            stmts.add(repo.productTotalStatement(productId, overrides, ts))
        }
    }
}
