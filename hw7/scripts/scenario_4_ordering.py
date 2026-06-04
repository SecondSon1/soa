#!/usr/bin/env python3
"""Scenario 4: Out-of-order event rejection (point 6)"""
import sys
from helpers import *

log("Scenario 4: Out-of-order event rejection")

log("  PRODUCT_RECEIVED: SKU-004 ZONE-A qty=100 ts=12:00")
send(event("evt-030", "PRODUCT_RECEIVED", 1700003600000,
           product_id="SKU-004", zone_id="ZONE-A", quantity=100))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-004' AND zone_id='ZONE-A';", 100)
assert_eq("available=100 after RECEIVED", 100, available("SKU-004", "ZONE-A"))

log("  PRODUCT_SHIPPED: SKU-004 ZONE-A qty=20 ts=12:05")
send(event("evt-031", "PRODUCT_SHIPPED", 1700003900000,
           product_id="SKU-004", zone_id="ZONE-A", quantity=20))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-004' AND zone_id='ZONE-A';", 80)
assert_eq("available=80 after SHIPPED", 80, available("SKU-004", "ZONE-A"))

log("  PRODUCT_RECEIVED: SKU-004 ZONE-A qty=50 ts=12:02 (stale — should be ignored)")
send(event("evt-032", "PRODUCT_RECEIVED", 1700003720000,
           product_id="SKU-004", zone_id="ZONE-A", quantity=50))
wait()
assert_eq("available still 80 (out-of-order ignored)", 80, available("SKU-004", "ZONE-A"))

sys.exit(0 if results() else 1)
