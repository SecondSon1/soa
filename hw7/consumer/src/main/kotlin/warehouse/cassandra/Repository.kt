package warehouse.cassandra

import com.datastax.oss.driver.api.core.ConsistencyLevel
import com.datastax.oss.driver.api.core.CqlSession
import com.datastax.oss.driver.api.core.cql.BatchStatement
import com.datastax.oss.driver.api.core.cql.BatchType
import com.datastax.oss.driver.api.core.cql.BoundStatement
import com.datastax.oss.driver.api.core.cql.PreparedStatement
import com.datastax.oss.driver.api.core.cql.SimpleStatement
import warehouse.metrics.Metrics
import warehouse.model.OrderItem
import java.time.Instant

class Repository(private val session: CqlSession) {
    private val isEventProcessed: PreparedStatement = session.prepare(
        SimpleStatement.newInstance("SELECT event_id FROM processed_events WHERE event_id = ?")
            .setConsistencyLevel(ConsistencyLevel.ONE)
    )
    private val insertProcessedEvent: PreparedStatement = session.prepare(
        "INSERT INTO processed_events (event_id, event_type, processed_at) VALUES (?, ?, ?)"
    )

    private val getInventoryByProductZone: PreparedStatement = session.prepare(
        SimpleStatement.newInstance("SELECT available_quantity, reserved_quantity, supplier_id, last_updated FROM inventory_by_product_zone WHERE product_id = ? AND zone_id = ?")
            .setConsistencyLevel(ConsistencyLevel.ONE)
    )
    private val upsertInventoryByProductZone: PreparedStatement = session.prepare(
        "UPDATE inventory_by_product_zone SET available_quantity = ?, reserved_quantity = ?, supplier_id = ?, last_updated = ? WHERE product_id = ? AND zone_id = ?"
    )

    private val upsertInventoryByProduct: PreparedStatement = session.prepare(
        "UPDATE inventory_by_product SET total_available = ?, total_reserved = ?, last_updated = ? WHERE product_id = ?"
    )

    private val upsertInventoryByZone: PreparedStatement = session.prepare(
        "UPDATE inventory_by_zone SET available_quantity = ?, reserved_quantity = ?, last_updated = ? WHERE zone_id = ? AND product_id = ?"
    )

    private val insertOrder: PreparedStatement = session.prepare(
        "INSERT INTO orders (order_id, status, created_at) VALUES (?, ?, ?)"
    )
    private val updateOrderCompleted: PreparedStatement = session.prepare(
        "UPDATE orders SET status = ?, completed_at = ? WHERE order_id = ?"
    )
    private val insertOrderItem: PreparedStatement = session.prepare(
        "INSERT INTO order_items (order_id, product_id, zone_id, quantity) VALUES (?, ?, ?, ?)"
    )
    private val getOrderItems: PreparedStatement = session.prepare(
        SimpleStatement.newInstance("SELECT product_id, zone_id, quantity FROM order_items WHERE order_id = ?")
            .setConsistencyLevel(ConsistencyLevel.ONE)
    )

    fun isProcessed(eventId: String): Boolean {
        val rs = session.execute(isEventProcessed.bind(eventId))
        return rs.one() != null
    }

    fun markProcessedStatement(eventId: String, eventType: String, now: Instant): BoundStatement =
        insertProcessedEvent.bind(eventId, eventType, now)

    data class Inventory(val available: Int, val reserved: Int, val supplierId: String?, val lastUpdated: Instant?)

    fun getZoneInventory(productId: String, zoneId: String): Inventory {
        val row = session.execute(getInventoryByProductZone.bind(productId, zoneId)).one()
            ?: return Inventory(0, 0, null, null)
        return Inventory(
            row.getInt("available_quantity"),
            row.getInt("reserved_quantity"),
            row.getString("supplier_id"),
            row.getInstant("last_updated"),
        )
    }

    fun zoneStatements(productId: String, zoneId: String, available: Int, reserved: Int, supplierId: String?, ts: Instant): List<BoundStatement> {
        return listOf(
            upsertInventoryByProductZone.bind(available, reserved, supplierId, ts, productId, zoneId),
            upsertInventoryByZone.bind(available, reserved, ts, zoneId, productId),
        )
    }

    fun productTotalStatement(productId: String, overrides: Map<String, Pair<Int, Int>>, ts: Instant): BoundStatement {
        val totals = recalcProductTotals(productId, overrides)
        return upsertInventoryByProduct.bind(totals.first, totals.second, ts, productId)
    }

    fun inventoryStatements(productId: String, zoneId: String, available: Int, reserved: Int, supplierId: String?, ts: Instant): List<BoundStatement> {
        val stmts = mutableListOf<BoundStatement>()
        stmts.addAll(zoneStatements(productId, zoneId, available, reserved, supplierId, ts))
        stmts.add(productTotalStatement(productId, mapOf(zoneId to (available to reserved)), ts))
        return stmts
    }

    private fun recalcProductTotals(productId: String, overrides: Map<String, Pair<Int, Int>>): Pair<Int, Int> {
        val rows = session.execute(
            SimpleStatement.newInstance(
                "SELECT zone_id, available_quantity, reserved_quantity FROM inventory_by_product_zone WHERE product_id = ?",
                productId,
            ).setConsistencyLevel(ConsistencyLevel.ONE)
        )
        var totalAvailable = 0
        var totalReserved = 0
        val seenOverrides = mutableSetOf<String>()
        for (row in rows) {
            val zoneId = row.getString("zone_id")!!
            val override = overrides[zoneId]
            if (override != null) {
                seenOverrides.add(zoneId)
                totalAvailable += override.first
                totalReserved += override.second
            } else {
                totalAvailable += row.getInt("available_quantity")
                totalReserved += row.getInt("reserved_quantity")
            }
        }
        for ((zoneId, override) in overrides) {
            if (zoneId !in seenOverrides) {
                totalAvailable += override.first
                totalReserved += override.second
            }
        }
        return totalAvailable to totalReserved
    }

    fun createOrderStatements(orderId: String, items: List<OrderItem>, ts: Instant): List<BoundStatement> {
        val stmts = mutableListOf<BoundStatement>()
        stmts.add(insertOrder.bind(orderId, "CREATED", ts))
        for (item in items) {
            stmts.add(insertOrderItem.bind(orderId, item.productId, item.zoneId, item.quantity))
        }
        return stmts
    }

    fun completeOrderStatements(orderId: String, ts: Instant): Pair<List<BoundStatement>, List<OrderItem>> {
        val stmts = mutableListOf<BoundStatement>()
        stmts.add(updateOrderCompleted.bind("COMPLETED", ts, orderId))

        val rows = session.execute(getOrderItems.bind(orderId))
        val items = rows.map { row ->
            OrderItem(
                productId = row.getString("product_id")!!,
                zoneId = row.getString("zone_id")!!,
                quantity = row.getInt("quantity"),
            )
        }.toList()

        return stmts to items
    }

    fun executeBatch(statements: List<BoundStatement>) {
        try {
            val batch = BatchStatement.builder(BatchType.LOGGED)
                .addStatements(statements)
                .setConsistencyLevel(ConsistencyLevel.QUORUM)
                .build()
            session.execute(batch)
        } catch (e: Exception) {
            Metrics.cassandraWriteErrors.inc()
            throw e
        }
    }

    fun isAvailable(): Boolean {
        return try {
            session.execute(
                SimpleStatement.newInstance("SELECT key FROM system.local")
                    .setConsistencyLevel(ConsistencyLevel.ONE)
            )
            true
        } catch (_: Exception) {
            false
        }
    }
}
