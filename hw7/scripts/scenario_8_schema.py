#!/usr/bin/env python3
"""Scenario 8: Schema evolution — V1 and V2 events (point 10)"""
import json
import subprocess
import sys

import fastavro
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


def send_v1(evt):
    payload = serialize_avro(AVRO_SCHEMA_V1, _parsed_v1, evt)
    producer.produce(TOPIC, value=payload)
    producer.flush()


def send_v2(evt):
    payload = serialize_avro(AVRO_SCHEMA_V2, _parsed_v2, evt)
    producer.produce(TOPIC, value=payload)
    producer.flush()


def supplier_id_for(product, zone):
    return cql_value(
        f"SELECT supplier_id FROM warehouse.inventory_by_product_zone "
        f"WHERE product_id='{product}' AND zone_id='{zone}';")


log("Scenario 8: Schema evolution")

log("  Sending V1 event (no supplier_id): SKU-080 ZONE-A qty=100")
send_v1({
    "event_id": "evt-080",
    "event_type": "PRODUCT_RECEIVED",
    "timestamp": 1700008000000,
    "product_id": "SKU-080",
    "zone_id": "ZONE-A",
    "quantity": 100,
    "from_zone_id": None,
    "to_zone_id": None,
    "order_id": None,
    "order_items": None,
})
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-080' AND zone_id='ZONE-A';", 100)
assert_eq("V1 available=100", 100, available("SKU-080", "ZONE-A"))
v1_supplier = supplier_id_for("SKU-080", "ZONE-A")
assert_eq("V1 supplier_id is null", "null", v1_supplier)

log("  Sending V2 event (with supplier_id=SUP-001): SKU-081 ZONE-A qty=200")
send_v2({
    "event_id": "evt-081",
    "event_type": "PRODUCT_RECEIVED",
    "timestamp": 1700008001000,
    "product_id": "SKU-081",
    "zone_id": "ZONE-A",
    "quantity": 200,
    "from_zone_id": None,
    "to_zone_id": None,
    "order_id": None,
    "order_items": None,
    "supplier_id": "SUP-001",
})
wait_for("SELECT available_quantity FROM warehouse.inventory_by_product_zone "
         "WHERE product_id='SKU-081' AND zone_id='ZONE-A';", 200)
assert_eq("V2 available=200", 200, available("SKU-081", "ZONE-A"))
assert_eq("V2 supplier_id=SUP-001", "SUP-001", supplier_id_for("SKU-081", "ZONE-A"))

log("  Checking Schema Registry has both versions")
r = subprocess.run(
    ["docker", "exec", SR_CONTAINER, "curl", "-s",
     "http://localhost:8081/subjects/warehouse-events-value/versions"],
    capture_output=True, text=True,
)
versions = json.loads(r.stdout) if r.stdout.strip() else []
if len(versions) >= 2:
    passed(f"Schema Registry has {len(versions)} versions")
else:
    failed(f"Schema Registry should have >=2 versions, got {versions}")

sys.exit(0 if results() else 1)
