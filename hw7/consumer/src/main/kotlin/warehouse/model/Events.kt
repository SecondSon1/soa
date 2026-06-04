package warehouse.model

import java.time.Instant

enum class EventType {
    PRODUCT_RECEIVED,
    PRODUCT_SHIPPED,
    PRODUCT_MOVED,
    PRODUCT_RESERVED,
    PRODUCT_RELEASED,
    INVENTORY_COUNTED,
    ORDER_CREATED,
    ORDER_COMPLETED,
}

data class OrderItem(
    val productId: String,
    val zoneId: String,
    val quantity: Int,
)

data class WarehouseEvent(
    val eventId: String,
    val eventType: EventType,
    val timestamp: Instant,
    val productId: String?,
    val zoneId: String?,
    val quantity: Int?,
    val fromZoneId: String?,
    val toZoneId: String?,
    val orderId: String?,
    val orderItems: List<OrderItem>?,
    val supplierId: String?,
) {
    companion object {
        fun fromAvro(record: warehouse.avro.WarehouseEvent): WarehouseEvent {
            return WarehouseEvent(
                eventId = record.getEventId().toString(),
                eventType = EventType.valueOf(record.getEventType().toString()),
                timestamp = Instant.ofEpochMilli(record.getTimestamp()),
                productId = record.getProductId()?.toString(),
                zoneId = record.getZoneId()?.toString(),
                quantity = record.getQuantity(),
                fromZoneId = record.getFromZoneId()?.toString(),
                toZoneId = record.getToZoneId()?.toString(),
                orderId = record.getOrderId()?.toString(),
                orderItems = record.getOrderItems()?.map { item ->
                    OrderItem(
                        productId = item.getProductId().toString(),
                        zoneId = item.getZoneId().toString(),
                        quantity = item.getQuantity(),
                    )
                },
                supplierId = record.getSupplierId()?.toString(),
            )
        }
    }
}
