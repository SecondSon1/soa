#!/usr/bin/env python3
"""Scenario 3: Table consistency across denormalized tables (point 5)"""
import sys
from helpers import *

log("Scenario 3: Table consistency across denormalized tables")

log("  PRODUCT_RECEIVED: SKU-003 ZONE-A qty=100")
send(event("evt-020", "PRODUCT_RECEIVED", 1700002000000,
           product_id="SKU-003", zone_id="ZONE-A", quantity=100))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-003' AND zone_id='ZONE-A';", 100)
assert_eq("inventory_by_product_zone", 100, available("SKU-003", "ZONE-A"))
assert_eq("inventory_by_product", 100, total_available("SKU-003"))
assert_eq("inventory_by_zone", 100, zone_available("ZONE-A", "SKU-003"))

sys.exit(0 if results() else 1)
