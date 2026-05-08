#!/usr/bin/env python3
"""Scenario 5: Dead Letter Queue (point 7)"""
import json
import subprocess
import sys
import time
from helpers import *

log("Scenario 5: Dead Letter Queue")

log("  Sending invalid event: PRODUCT_SHIPPED with quantity=-5")
send(event("evt-050", "PRODUCT_SHIPPED", 1700005000000,
           product_id="SKU-005", zone_id="ZONE-A", quantity=-5))

log("  Sending valid event to verify consumer is alive")
send(event("evt-051", "PRODUCT_RECEIVED", 1700005001000,
           product_id="SKU-005", zone_id="ZONE-A", quantity=75))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-005' AND zone_id='ZONE-A';", 75)
assert_eq("consumer still alive after bad event", 75, available("SKU-005", "ZONE-A"))

log("  Checking DLQ topic for the invalid event")
r = subprocess.run(
    ["docker", "exec", KAFKA_CONTAINER,
     "kafka-console-consumer",
     "--bootstrap-server", "localhost:9092",
     "--topic", DLQ_TOPIC,
     "--from-beginning",
     "--timeout-ms", "5000"],
    capture_output=True, text=True, timeout=15,
)
dlq_messages = [l for l in r.stdout.strip().splitlines() if l.strip()]

if dlq_messages:
    passed(f"DLQ has {len(dlq_messages)} message(s)")
    msg = json.loads(dlq_messages[0])
    if "error_reason" in msg and "quantity" in msg["error_reason"].lower():
        passed(f"DLQ error_reason mentions quantity: {msg['error_reason']}")
    else:
        failed(f"DLQ message missing expected error_reason: {msg}")
else:
    failed("DLQ topic is empty — invalid event was not routed")

sys.exit(0 if results() else 1)
