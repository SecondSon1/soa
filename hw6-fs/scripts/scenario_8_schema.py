#!/usr/bin/env python3
"""Scenario 8: Schema evolution (point 10)"""
import json
import sys
import urllib.request
from helpers import *

AVRO_SCHEMA_V1 = {
    "type": "record",
    "name": "WarehouseEvent",
    "namespace": "warehouse.avro",
    "fields": [
        {"name": "event_id", "type": "string"},
        {"name": "event_type", "type": "string"},
        {"name": "timestamp", "type": "long"},
        {"name": "product_id", "type": ["null", "string"], "default": None},
        {"name": "zone_id", "type": ["null", "string"], "default": None},
        {"name": "quantity", "type": ["null", "int"], "default": None},
        {"name": "from_zone_id", "type": ["null", "string"], "default": None},
        {"name": "to_zone_id", "type": ["null", "string"], "default": None},
        {"name": "order_id", "type": ["null", "string"], "default": None},
        {"name": "order_items", "type": ["null", {
            "type": "array",
            "items": {
                "type": "record", "name": "OrderItem",
                "fields": [
                    {"name": "product_id", "type": "string"},
                    {"name": "zone_id", "type": "string"},
                    {"name": "quantity", "type": "int"},
                ],
            },
        }], "default": None},
    ],
}

AVRO_SCHEMA_V2 = {
    "type": "record",
    "name": "WarehouseEvent",
    "namespace": "warehouse.avro",
    "fields": [
        {"name": "event_id", "type": "string"},
        {"name": "event_type", "type": "string"},
        {"name": "timestamp", "type": "long"},
        {"name": "product_id", "type": ["null", "string"], "default": None},
        {"name": "zone_id", "type": ["null", "string"], "default": None},
        {"name": "quantity", "type": ["null", "int"], "default": None},
        {"name": "from_zone_id", "type": ["null", "string"], "default": None},
        {"name": "to_zone_id", "type": ["null", "string"], "default": None},
        {"name": "order_id", "type": ["null", "string"], "default": None},
        {"name": "order_items", "type": ["null", {
            "type": "array",
            "items": {
                "type": "record", "name": "OrderItem",
                "fields": [
                    {"name": "product_id", "type": "string"},
                    {"name": "zone_id", "type": "string"},
                    {"name": "quantity", "type": "int"},
                ],
            },
        }], "default": None},
        {"name": "supplier_id", "type": ["null", "string"], "default": None},
    ],
}

_parsed_v1 = fastavro.parse_schema(AVRO_SCHEMA_V1)
_parsed_v2 = fastavro.parse_schema(AVRO_SCHEMA_V2)

log("Scenario 8: Schema evolution")

log("  Sending V1 event (no supplier_id): SKU-080 ZONE-A qty=100")
payload_v1 = serialize_avro(AVRO_SCHEMA_V1, _parsed_v1,
                            event("evt-080", "PRODUCT_RECEIVED", 1700008000000,
                                  product_id="SKU-080", zone_id="ZONE-A", quantity=100))
producer.produce(TOPIC, value=payload_v1)
producer.flush()
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-080' AND zone_id='ZONE-A';", 100)
assert_eq("V1 event processed", 100, available("SKU-080", "ZONE-A"))

v1_supplier = cql_value(
    "SELECT supplier_id FROM warehouse.inventory_by_product_zone "
    "WHERE product_id='SKU-080' AND zone_id='ZONE-A';")
if v1_supplier == "" or v1_supplier == "null":
    passed("V1 supplier_id is null")
else:
    failed(f"V1 supplier_id expected null, got: {v1_supplier}")

log("  Sending V2 event (with supplier_id=SUP-001): SKU-081 ZONE-A qty=200")
evt_v2 = event("evt-081", "PRODUCT_RECEIVED", 1700008001000,
               product_id="SKU-081", zone_id="ZONE-A", quantity=200)
evt_v2["supplier_id"] = "SUP-001"
payload_v2 = serialize_avro(AVRO_SCHEMA_V2, _parsed_v2, evt_v2)
producer.produce(TOPIC, value=payload_v2)
producer.flush()
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-081' AND zone_id='ZONE-A';", 200)
assert_eq("V2 event processed", 200, available("SKU-081", "ZONE-A"))

v2_supplier = cql_value(
    "SELECT supplier_id FROM warehouse.inventory_by_product_zone "
    "WHERE product_id='SKU-081' AND zone_id='ZONE-A';")
assert_eq("V2 supplier_id stored", "SUP-001", v2_supplier)

log("  Checking Schema Registry for multiple versions")
try:
    with urllib.request.urlopen(f"{SR_URL}/subjects/warehouse-events-value/versions") as resp:
        versions = json.loads(resp.read())
        if len(versions) >= 2:
            passed(f"Schema Registry has {len(versions)} versions")
        else:
            failed(f"Schema Registry has only {len(versions)} version(s)")
except Exception as e:
    failed(f"Schema Registry check failed: {e}")

sys.exit(0 if results() else 1)
