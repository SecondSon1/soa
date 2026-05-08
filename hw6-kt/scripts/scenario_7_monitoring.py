#!/usr/bin/env python3
"""Scenario 7: Monitoring and consumer lag (point 9)"""
import subprocess
import sys
from helpers import *

log("Scenario 7: Monitoring")

log("  Checking /health endpoint")
r = subprocess.run(["curl", "-s", "http://localhost:8080/health"], capture_output=True, text=True)
if "UP" in r.stdout:
    passed("/health returns UP")
else:
    failed(f"/health should return UP, got: {r.stdout[:100]}")

log("  Checking /metrics endpoint")
r = subprocess.run(["curl", "-s", "http://localhost:8080/metrics"], capture_output=True, text=True)
metrics_text = r.stdout

has_lag = "consumer_lag" in metrics_text
has_processed = "events_processed_total" in metrics_text
has_duration = "event_processing_duration_seconds" in metrics_text
has_errors = "cassandra_write_errors_total" in metrics_text

if has_lag:
    passed("/metrics has consumer_lag")
else:
    failed("/metrics missing consumer_lag")

if has_processed:
    passed("/metrics has events_processed_total")
else:
    failed("/metrics missing events_processed_total")

if has_duration:
    passed("/metrics has event_processing_duration_seconds")
else:
    failed("/metrics missing event_processing_duration_seconds")

if has_errors:
    passed("/metrics has cassandra_write_errors_total")
else:
    failed("/metrics missing cassandra_write_errors_total")

log("  Sending events to generate metrics")
for i in range(5):
    send(event(f"evt-070-{i}", "PRODUCT_RECEIVED", 1700007000000 + i * 1000,
               product_id=f"SKU-07{i}", zone_id="ZONE-A", quantity=10))
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-074' AND zone_id='ZONE-A';", 10)

r = subprocess.run(["curl", "-s", "http://localhost:8080/metrics"], capture_output=True, text=True)

if 'events_processed_total{event_type="PRODUCT_RECEIVED"' in r.stdout:
    passed("events_processed_total incremented for PRODUCT_RECEIVED")
else:
    failed("events_processed_total should show PRODUCT_RECEIVED counts")

sys.exit(0 if results() else 1)
