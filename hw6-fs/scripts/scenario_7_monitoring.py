#!/usr/bin/env python3
"""Scenario 7: Monitoring and consumer lag (point 9)"""
import sys
import urllib.request
from helpers import *

CONSUMER_URL = "http://localhost:8080"

log("Scenario 7: Monitoring")

log("  Checking /health endpoint")
try:
    with urllib.request.urlopen(f"{CONSUMER_URL}/health") as resp:
        body = resp.read().decode()
        assert_eq("/health status code", 200, resp.status)
        if "UP" in body:
            passed(f"/health returns UP: {body}")
        else:
            failed(f"/health unexpected body: {body}")
except Exception as e:
    failed(f"/health failed: {e}")

log("  Checking /metrics endpoint")
try:
    with urllib.request.urlopen(f"{CONSUMER_URL}/metrics") as resp:
        metrics = resp.read().decode()
        assert_eq("/metrics status code", 200, resp.status)

        for name in ["consumer_lag", "events_processed_total",
                      "event_processing_duration_seconds", "cassandra_write_errors_total"]:
            if name in metrics:
                passed(f"/metrics contains {name}")
            else:
                failed(f"/metrics missing {name}")
except Exception as e:
    failed(f"/metrics failed: {e}")

log("  Sending 5 events to generate metrics")
for i in range(5):
    send(event(f"evt-070-{i}", "PRODUCT_RECEIVED", 1700007000000 + i * 1000,
               product_id="SKU-007", zone_id="ZONE-A", quantity=10))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-007' AND zone_id='ZONE-A';", 50)

log("  Verifying metrics updated")
try:
    with urllib.request.urlopen(f"{CONSUMER_URL}/metrics") as resp:
        metrics = resp.read().decode()
        if 'events_processed_total{event_type="PRODUCT_RECEIVED"}' in metrics:
            passed("events_processed_total has PRODUCT_RECEIVED label")
        else:
            failed("events_processed_total missing PRODUCT_RECEIVED label")
except Exception as e:
    failed(f"post-event /metrics failed: {e}")

sys.exit(0 if results() else 1)
