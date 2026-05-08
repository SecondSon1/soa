#!/usr/bin/env python3
"""Corner cases not covered by standard scenarios"""
import sys
from helpers import *

log("Corner 1: Multi-item order with SAME product across different zones")
log("  (Tests product total correctness when OrderCreated touches multiple zones of one product)")

send(event("corner-001", "PRODUCT_RECEIVED", 1800001000000,
           product_id="P-MULTI", zone_id="Z-1", quantity=100))
send(event("corner-002", "PRODUCT_RECEIVED", 1800002000000,
           product_id="P-MULTI", zone_id="Z-2", quantity=100))
wait_for("SELECT total_available FROM warehouse.inventory_by_product "
         "WHERE product_id='P-MULTI';", 200)

assert_eq("setup: total_available", 200, total_available("P-MULTI"))

send(event("corner-003", "ORDER_CREATED", 1800003000000,
           order_id="ORD-MULTI",
           order_items=[
               {"product_id": "P-MULTI", "zone_id": "Z-1", "quantity": 10},
               {"product_id": "P-MULTI", "zone_id": "Z-2", "quantity": 20},
           ]))
wait_for("SELECT reserved_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-MULTI' AND zone_id='Z-2';", 20)

assert_eq("Z-1 available after multi-zone order", 90, available("P-MULTI", "Z-1"))
assert_eq("Z-1 reserved after multi-zone order", 10, reserved("P-MULTI", "Z-1"))
assert_eq("Z-2 available after multi-zone order", 80, available("P-MULTI", "Z-2"))
assert_eq("Z-2 reserved after multi-zone order", 20, reserved("P-MULTI", "Z-2"))
assert_eq("total_available after multi-zone order (90+80)", 170, total_available("P-MULTI"))
total_res = cql_value("SELECT total_reserved FROM warehouse.inventory_by_product "
                      "WHERE product_id='P-MULTI';")
assert_eq("total_reserved after multi-zone order (10+20)", 30, total_res)

log("  Completing multi-zone order")
send(event("corner-004", "ORDER_COMPLETED", 1800004000000, order_id="ORD-MULTI"))
wait_for("SELECT total_reserved FROM warehouse.inventory_by_product "
         "WHERE product_id='P-MULTI';", 0)

assert_eq("Z-1 reserved after complete", 0, reserved("P-MULTI", "Z-1"))
assert_eq("Z-2 reserved after complete", 0, reserved("P-MULTI", "Z-2"))
assert_eq("total_available after complete (unchanged)", 170, total_available("P-MULTI"))

log("  Verifying all 3 tables consistent")
assert_eq("by_zone Z-1 matches by_product_zone",
          available("P-MULTI", "Z-1"), zone_available("Z-1", "P-MULTI"))
assert_eq("by_zone Z-2 matches by_product_zone",
          available("P-MULTI", "Z-2"), zone_available("Z-2", "P-MULTI"))


log("Corner 2: PRODUCT_MOVED consistency across all 3 inventory tables")
log("  (Move between two zones of the same product, verify product totals are correct)")

send(event("corner-010", "PRODUCT_RECEIVED", 1800010000000,
           product_id="P-MOVE", zone_id="Z-A", quantity=200))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-MOVE' AND zone_id='Z-A';", 200)

send(event("corner-011", "PRODUCT_MOVED", 1800011000000,
           product_id="P-MOVE", from_zone_id="Z-A", to_zone_id="Z-B", quantity=75))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-MOVE' AND zone_id='Z-B';", 75)

assert_eq("Z-A after move", 125, available("P-MOVE", "Z-A"))
assert_eq("Z-B after move", 75, available("P-MOVE", "Z-B"))
assert_eq("total_available preserved after move", 200, total_available("P-MOVE"))
assert_eq("by_zone Z-A consistent", available("P-MOVE", "Z-A"), zone_available("Z-A", "P-MOVE"))
assert_eq("by_zone Z-B consistent", available("P-MOVE", "Z-B"), zone_available("Z-B", "P-MOVE"))


log("Corner 3: Order with items from MULTIPLE products")
log("  (Tests that per-product totals are independent)")

send(event("corner-020", "PRODUCT_RECEIVED", 1800020000000,
           product_id="P-X", zone_id="Z-1", quantity=50))
send(event("corner-021", "PRODUCT_RECEIVED", 1800021000000,
           product_id="P-Y", zone_id="Z-1", quantity=80))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-Y' AND zone_id='Z-1';", 80)

send(event("corner-022", "ORDER_CREATED", 1800022000000,
           order_id="ORD-MULTI-PROD",
           order_items=[
               {"product_id": "P-X", "zone_id": "Z-1", "quantity": 5},
               {"product_id": "P-Y", "zone_id": "Z-1", "quantity": 10},
           ]))
wait_for("SELECT reserved_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-Y' AND zone_id='Z-1';", 10)

assert_eq("P-X available after multi-product order", 45, available("P-X", "Z-1"))
assert_eq("P-X reserved after multi-product order", 5, reserved("P-X", "Z-1"))
assert_eq("P-Y available after multi-product order", 70, available("P-Y", "Z-1"))
assert_eq("P-Y reserved after multi-product order", 10, reserved("P-Y", "Z-1"))
assert_eq("P-X total_available", 45, total_available("P-X"))
assert_eq("P-Y total_available", 70, total_available("P-Y"))


log("Corner 4: Rapid successive events to the same product/zone")
log("  (Tests that sequential processing + idempotency hold under fast writes)")

for i in range(10):
    send(event(f"corner-03{i}", "PRODUCT_RECEIVED", 1800030000000 + i * 1000,
               product_id="P-RAPID", zone_id="Z-1", quantity=1))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-RAPID' AND zone_id='Z-1';", 10, timeout=15)

assert_eq("10 rapid events all applied", 10, available("P-RAPID", "Z-1"))
assert_eq("total matches zone", 10, total_available("P-RAPID"))


log("Corner 5: INVENTORY_COUNTED overrides existing quantity")
log("  (Counted should SET, not ADD)")

send(event("corner-040", "PRODUCT_RECEIVED", 1800040000000,
           product_id="P-COUNT", zone_id="Z-1", quantity=999))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-COUNT' AND zone_id='Z-1';", 999)

send(event("corner-041", "INVENTORY_COUNTED", 1800041000000,
           product_id="P-COUNT", zone_id="Z-1", quantity=42))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='P-COUNT' AND zone_id='Z-1';", 42)

assert_eq("counted overrides to 42", 42, available("P-COUNT", "Z-1"))
assert_eq("total reflects counted", 42, total_available("P-COUNT"))


sys.exit(0 if results() else 1)
