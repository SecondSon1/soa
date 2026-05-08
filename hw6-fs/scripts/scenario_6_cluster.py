#!/usr/bin/env python3
"""Scenario 6: Cassandra cluster fault tolerance (point 8)"""
import subprocess
import sys
import time
from helpers import *

log("Scenario 6: Cassandra cluster fault tolerance")

log("  Checking cluster status (expect 3 UN nodes)")
status = subprocess.run(
    ["docker", "exec", "hw6-cassandra-1-1", "nodetool", "status"],
    capture_output=True, text=True,
)
un_count = status.stdout.count("UN")
assert_eq("cluster has 3 UN nodes", 3, un_count)

log("  PRODUCT_RECEIVED: SKU-006 ZONE-A qty=200")
send(event("evt-060", "PRODUCT_RECEIVED", 1700006000000,
           product_id="SKU-006", zone_id="ZONE-A", quantity=200))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-006' AND zone_id='ZONE-A';", 200)
assert_eq("available after RECEIVED", 200, available("SKU-006", "ZONE-A"))

log("  Stopping cassandra-2")
subprocess.run(["docker", "stop", "hw6-cassandra-2-1"], capture_output=True, timeout=30)
time.sleep(5)

log("  PRODUCT_SHIPPED: SKU-006 ZONE-A qty=50 (with node down)")
send(event("evt-061", "PRODUCT_SHIPPED", 1700006001000,
           product_id="SKU-006", zone_id="ZONE-A", quantity=50))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-006' AND zone_id='ZONE-A';", 150)
assert_eq("available after SHIPPED (node down)", 150, available("SKU-006", "ZONE-A"))

log("  Restarting cassandra-2")
subprocess.run(["docker", "start", "hw6-cassandra-2-1"], capture_output=True, timeout=30)

log("  Waiting for node to rejoin cluster...")
deadline = time.monotonic() + 60
while time.monotonic() < deadline:
    status = subprocess.run(
        ["docker", "exec", "hw6-cassandra-1-1", "nodetool", "status"],
        capture_output=True, text=True,
    )
    if status.stdout.count("UN") == 3:
        break
    time.sleep(5)

un_count = status.stdout.count("UN")
assert_eq("cluster back to 3 UN nodes", 3, un_count)

sys.exit(0 if results() else 1)
