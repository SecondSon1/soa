#!/usr/bin/env python3
"""Scenario 2: Idempotency (point 4)"""
import sys
from helpers import *

log("Scenario 2: Idempotency")

log("  PRODUCT_RECEIVED: SKU-002 ZONE-A qty=50")
send(event("evt-010", "PRODUCT_RECEIVED", 1700001000000,
           product_id="SKU-002", zone_id="ZONE-A", quantity=50))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-002' AND zone_id='ZONE-A';", 50)
assert_eq("SKU-002 available after RECEIVED", 50, available("SKU-002", "ZONE-A"))

log("  Re-sending same event (evt-010) — should be skipped")
send(event("evt-010", "PRODUCT_RECEIVED", 1700001000000,
           product_id="SKU-002", zone_id="ZONE-A", quantity=50))
wait()
assert_eq("SKU-002 still 50 (idempotent)", 50, available("SKU-002", "ZONE-A"))

sys.exit(0 if results() else 1)
