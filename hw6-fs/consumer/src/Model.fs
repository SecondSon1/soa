module Warehouse.Model

type OrderItem =
    { ProductId: string
      ZoneId: string
      Quantity: int }

type EventPayload =
    | ProductReceived of productId: string * zoneId: string * quantity: int
    | ProductShipped of productId: string * zoneId: string * quantity: int
    | ProductMoved of productId: string * fromZoneId: string * toZoneId: string * quantity: int
    | ProductReserved of productId: string * zoneId: string * quantity: int
    | ProductReleased of productId: string * zoneId: string * quantity: int
    | InventoryCounted of productId: string * zoneId: string * quantity: int
    | OrderCreated of orderId: string * items: OrderItem list
    | OrderCompleted of orderId: string

type WarehouseEvent =
    { EventId: string
      Timestamp: int64
      Payload: EventPayload
      SupplierId: string option }

let payloadName =
    function
    | ProductReceived _ -> "PRODUCT_RECEIVED"
    | ProductShipped _ -> "PRODUCT_SHIPPED"
    | ProductMoved _ -> "PRODUCT_MOVED"
    | ProductReserved _ -> "PRODUCT_RESERVED"
    | ProductReleased _ -> "PRODUCT_RELEASED"
    | InventoryCounted _ -> "INVENTORY_COUNTED"
    | OrderCreated _ -> "ORDER_CREATED"
    | OrderCompleted _ -> "ORDER_COMPLETED"
