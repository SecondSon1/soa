#!/usr/bin/env python3
"""Scenario 5: Dead Letter Queue (point 7)"""
import subprocess
import sys
from helpers import *

log("Scenario 5: Dead Letter Queue")

log("  Sending invalid event: PRODUCT_SHIPPED with quantity=-5")
send(event("evt-040", "PRODUCT_SHIPPED", 1700004000000,
           product_id="SKU-005", zone_id="ZONE-A", quantity=-5))

log("  Checking consumer is still alive")
send(event("evt-041", "PRODUCT_RECEIVED", 1700004001000,
           product_id="SKU-005", zone_id="ZONE-A", quantity=10))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-005' AND zone_id='ZONE-A';", 10)
assert_eq("consumer still works after invalid event", 10, available("SKU-005", "ZONE-A"))

log("  Checking DLQ topic")
r = subprocess.run(
    ["docker", "exec", KAFKA_CONTAINER,
     "kafka-console-consumer",
     "--bootstrap-server", "localhost:9092",
     "--topic", DLQ_TOPIC,
     "--from-beginning",
     "--timeout-ms", "5000"],
    capture_output=True, text=True,
)
messages = [line for line in r.stdout.strip().splitlines() if line.strip()]

if len(messages) >= 1:
    passed(f"DLQ has messages (count={len(messages)})")
else:
    failed(f"DLQ should have messages (count={len(messages)})")

last = messages[-1] if messages else ""
if "Invalid quantity: -5" in last:
    passed("DLQ message contains error reason")
else:
    failed(f"DLQ message should contain error reason, got: {last[:120]}")

sys.exit(0 if results() else 1)
