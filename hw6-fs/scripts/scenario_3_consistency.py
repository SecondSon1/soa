#!/usr/bin/env python3
"""Scenario 3: Denormalized table consistency (point 5)"""
import sys
from helpers import *

log("Scenario 3: Table consistency")

log("  PRODUCT_RECEIVED: SKU-003 ZONE-C qty=200")
send(event("evt-030", "PRODUCT_RECEIVED", 1700003000000,
           product_id="SKU-003", zone_id="ZONE-C", quantity=200))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-003' AND zone_id='ZONE-C';", 200)

log("  PRODUCT_MOVED: SKU-003 ZONE-C -> ZONE-D qty=80")
send(event("evt-031", "PRODUCT_MOVED", 1700003001000,
           product_id="SKU-003", quantity=80,
           from_zone_id="ZONE-C", to_zone_id="ZONE-D"))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-003' AND zone_id='ZONE-C';", 120)

log("  Checking inventory_by_product_zone")
assert_eq("by_product_zone ZONE-C", 120, available("SKU-003", "ZONE-C"))
assert_eq("by_product_zone ZONE-D", 80, available("SKU-003", "ZONE-D"))

log("  Checking inventory_by_zone")
assert_eq("by_zone ZONE-C", 120, zone_available("ZONE-C", "SKU-003"))
assert_eq("by_zone ZONE-D", 80, zone_available("ZONE-D", "SKU-003"))

log("  Checking inventory_by_product (totals)")
assert_eq("total_available after MOVED", 200, total_available("SKU-003"))

sys.exit(0 if results() else 1)
