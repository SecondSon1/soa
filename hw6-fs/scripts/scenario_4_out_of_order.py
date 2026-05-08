#!/usr/bin/env python3
"""Scenario 4: Out-of-order event handling (point 6)"""
import sys
from helpers import *

log("Scenario 4: Out-of-order events")

log("  PRODUCT_RECEIVED: SKU-004 ZONE-A qty=100 (ts=2000)")
send(event("evt-040", "PRODUCT_RECEIVED", 1700004002000,
           product_id="SKU-004", zone_id="ZONE-A", quantity=100))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-004' AND zone_id='ZONE-A';", 100)
assert_eq("available after first RECEIVED", 100, available("SKU-004", "ZONE-A"))

log("  PRODUCT_RECEIVED: SKU-004 ZONE-A qty=50 (ts=1000, older — should be ignored)")
send(event("evt-041", "PRODUCT_RECEIVED", 1700004001000,
           product_id="SKU-004", zone_id="ZONE-A", quantity=50))
wait()
assert_eq("available unchanged (old event ignored)", 100, available("SKU-004", "ZONE-A"))

log("  PRODUCT_RECEIVED: SKU-004 ZONE-A qty=25 (ts=2000, equal — should be ignored)")
send(event("evt-042", "PRODUCT_RECEIVED", 1700004002000,
           product_id="SKU-004", zone_id="ZONE-A", quantity=25))
wait()
assert_eq("available unchanged (same-ts event ignored)", 100, available("SKU-004", "ZONE-A"))

log("  PRODUCT_RECEIVED: SKU-004 ZONE-A qty=30 (ts=3000, newer — should apply)")
send(event("evt-043", "PRODUCT_RECEIVED", 1700004003000,
           product_id="SKU-004", zone_id="ZONE-A", quantity=30))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-004' AND zone_id='ZONE-A';", 130)
assert_eq("available updated with newer event", 130, available("SKU-004", "ZONE-A"))

sys.exit(0 if results() else 1)
