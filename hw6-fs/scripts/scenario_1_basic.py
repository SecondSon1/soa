#!/usr/bin/env python3
"""Scenario 1: Basic warehouse cycle (points 1-3)"""
import sys
from helpers import *

log("Scenario 1: Basic warehouse cycle")

log("  PRODUCT_RECEIVED: SKU-001 ZONE-A qty=100")
send(event("evt-001", "PRODUCT_RECEIVED", 1700000000000,
           product_id="SKU-001", zone_id="ZONE-A", quantity=100))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-001' AND zone_id='ZONE-A';", 100)
assert_eq("ZONE-A available after RECEIVED", 100, available("SKU-001", "ZONE-A"))
assert_eq("total_available after RECEIVED", 100, total_available("SKU-001"))

log("  PRODUCT_RESERVED: SKU-001 ZONE-A qty=30")
send(event("evt-002", "PRODUCT_RESERVED", 1700000001000,
           product_id="SKU-001", zone_id="ZONE-A", quantity=30))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-001' AND zone_id='ZONE-A';", 70)
assert_eq("ZONE-A available after RESERVED", 70, available("SKU-001", "ZONE-A"))
assert_eq("ZONE-A reserved after RESERVED", 30, reserved("SKU-001", "ZONE-A"))

log("  PRODUCT_MOVED: SKU-001 ZONE-A -> ZONE-B qty=20")
send(event("evt-003", "PRODUCT_MOVED", 1700000002000,
           product_id="SKU-001", quantity=20,
           from_zone_id="ZONE-A", to_zone_id="ZONE-B"))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-001' AND zone_id='ZONE-A';", 50)
assert_eq("ZONE-A available after MOVED", 50, available("SKU-001", "ZONE-A"))
assert_eq("ZONE-B available after MOVED", 20, available("SKU-001", "ZONE-B"))

log("  PRODUCT_SHIPPED: SKU-001 ZONE-A qty=10")
send(event("evt-004", "PRODUCT_SHIPPED", 1700000003000,
           product_id="SKU-001", zone_id="ZONE-A", quantity=10))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-001' AND zone_id='ZONE-A';", 40)
assert_eq("ZONE-A available after SHIPPED", 40, available("SKU-001", "ZONE-A"))

log("  ORDER_CREATED: ORD-001 with SKU-001 qty=15 from ZONE-A")
send(event("evt-005", "ORDER_CREATED", 1700000004000,
           order_id="ORD-001",
           order_items=[{"product_id": "SKU-001", "zone_id": "ZONE-A", "quantity": 15}]))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-001' AND zone_id='ZONE-A';", 25)
assert_eq("ZONE-A available after ORDER_CREATED", 25, available("SKU-001", "ZONE-A"))
assert_eq("ZONE-A reserved after ORDER_CREATED (30+15)", 45, reserved("SKU-001", "ZONE-A"))

log("  ORDER_COMPLETED: ORD-001")
send(event("evt-006", "ORDER_COMPLETED", 1700000005000, order_id="ORD-001"))
wait_for("SELECT reserved_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-001' AND zone_id='ZONE-A';", 30)
assert_eq("ZONE-A reserved after ORDER_COMPLETED (45-15)", 30, reserved("SKU-001", "ZONE-A"))
assert_eq("ZONE-A available unchanged after ORDER_COMPLETED", 25, available("SKU-001", "ZONE-A"))

sys.exit(0 if results() else 1)
