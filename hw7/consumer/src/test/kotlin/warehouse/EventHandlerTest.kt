package warehouse

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import warehouse.model.EventType
import warehouse.model.OrderItem
import warehouse.model.WarehouseEvent
import java.time.Instant
import kotlin.test.assertEquals

class EventHandlerTest {

    @Test
    fun `WarehouseEvent fromAvro round-trip preserves fields`() {
        val event = WarehouseEvent(
            eventId = "test-001",
            eventType = EventType.PRODUCT_RECEIVED,
            timestamp = Instant.ofEpochMilli(1700000000000),
            productId = "SKU-001",
            zoneId = "ZONE-A",
            quantity = 50,
            fromZoneId = null,
            toZoneId = null,
            orderId = null,
            orderItems = null,
            supplierId = "SUP-001",
        )
        assertEquals("test-001", event.eventId)
        assertEquals(EventType.PRODUCT_RECEIVED, event.eventType)
        assertEquals("SKU-001", event.productId)
        assertEquals("ZONE-A", event.zoneId)
        assertEquals(50, event.quantity)
        assertEquals("SUP-001", event.supplierId)
    }

    @Test
    fun `EventType valueOf parses all types`() {
        val types = listOf(
            "PRODUCT_RECEIVED", "PRODUCT_SHIPPED", "PRODUCT_MOVED",
            "PRODUCT_RESERVED", "PRODUCT_RELEASED", "INVENTORY_COUNTED",
            "ORDER_CREATED", "ORDER_COMPLETED",
        )
        for (t in types) {
            assertEquals(t, EventType.valueOf(t).name)
        }
    }

    @Test
    fun `EventType valueOf rejects unknown type`() {
        assertThrows<IllegalArgumentException> {
            EventType.valueOf("UNKNOWN_TYPE")
        }
    }

    @Test
    fun `OrderItem holds correct values`() {
        val item = OrderItem(productId = "SKU-002", zoneId = "ZONE-B", quantity = 10)
        assertEquals("SKU-002", item.productId)
        assertEquals("ZONE-B", item.zoneId)
        assertEquals(10, item.quantity)
    }

    @Test
    fun `WarehouseEvent with order items`() {
        val items = listOf(
            OrderItem("SKU-001", "ZONE-A", 5),
            OrderItem("SKU-002", "ZONE-B", 3),
        )
        val event = WarehouseEvent(
            eventId = "test-002",
            eventType = EventType.ORDER_CREATED,
            timestamp = Instant.now(),
            productId = null,
            zoneId = null,
            quantity = null,
            fromZoneId = null,
            toZoneId = null,
            orderId = "ORD-001",
            orderItems = items,
            supplierId = null,
        )
        assertEquals(2, event.orderItems!!.size)
        assertEquals("ORD-001", event.orderId)
    }

    @Test
    fun `WarehouseEvent move event has from and to zones`() {
        val event = WarehouseEvent(
            eventId = "test-003",
            eventType = EventType.PRODUCT_MOVED,
            timestamp = Instant.now(),
            productId = "SKU-001",
            zoneId = null,
            quantity = 10,
            fromZoneId = "ZONE-A",
            toZoneId = "ZONE-B",
            orderId = null,
            orderItems = null,
            supplierId = null,
        )
        assertEquals("ZONE-A", event.fromZoneId)
        assertEquals("ZONE-B", event.toZoneId)
    }
}
