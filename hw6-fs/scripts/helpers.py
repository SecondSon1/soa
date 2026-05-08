import io
import json
import struct
import subprocess
import time
import sys
import urllib.request

import fastavro
from confluent_kafka import Producer

SR_URL = "http://localhost:8081"
KAFKA_CONTAINER = "hw6-kafka-1"
SR_CONTAINER = "hw6-schema-registry-1"
CASSANDRA_CONTAINER = "hw6-cassandra-1-1"
TOPIC = "warehouse-events"
DLQ_TOPIC = "warehouse-events-dlq"

AVRO_SCHEMA = {
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

_parsed_schema = fastavro.parse_schema(AVRO_SCHEMA)
producer = Producer({"bootstrap.servers": "localhost:9092"})
schema_id_cache: dict[str, int] = {}


def register_schema(schema_dict: dict) -> int:
    key = json.dumps(schema_dict, sort_keys=True)
    if key in schema_id_cache:
        return schema_id_cache[key]
    subject = f"{TOPIC}-value"
    body = json.dumps({"schema": json.dumps(schema_dict)}).encode()
    req = urllib.request.Request(
        f"{SR_URL}/subjects/{subject}/versions",
        data=body,
        headers={"Content-Type": "application/vnd.schemaregistry.v1+json"},
    )
    with urllib.request.urlopen(req) as resp:
        schema_id = json.loads(resp.read())["id"]
    schema_id_cache[key] = schema_id
    return schema_id


def serialize_avro(schema_dict: dict, parsed, record: dict) -> bytes:
    schema_id = register_schema(schema_dict)
    buf = io.BytesIO()
    buf.write(b'\x00')
    buf.write(struct.pack('>I', schema_id))
    fastavro.schemaless_writer(buf, parsed, record)
    return buf.getvalue()


_default_schema_id = register_schema(AVRO_SCHEMA)

PASS = 0
FAIL = 0


def log(msg):
    print(f"\033[1;33m>>> {msg}\033[0m")


def passed(desc):
    global PASS
    PASS += 1
    print(f"\033[0;32m  PASS: {desc}\033[0m")


def failed(desc):
    global FAIL
    FAIL += 1
    print(f"\033[0;31m  FAIL: {desc}\033[0m")


def event(event_id, event_type, ts, product_id=None, zone_id=None, quantity=None,
          from_zone_id=None, to_zone_id=None, order_id=None, order_items=None):
    return {
        "event_id": event_id,
        "event_type": event_type,
        "timestamp": ts,
        "product_id": product_id,
        "zone_id": zone_id,
        "quantity": quantity,
        "from_zone_id": from_zone_id,
        "to_zone_id": to_zone_id,
        "order_id": order_id,
        "order_items": order_items,
    }


def send(evt):
    payload = serialize_avro(AVRO_SCHEMA, _parsed_schema, evt)
    producer.produce(TOPIC, value=payload)
    producer.flush()


def cql(query):
    r = subprocess.run(
        ["docker", "exec", CASSANDRA_CONTAINER, "cqlsh", "-e", query],
        capture_output=True, text=True,
    )
    return r.stdout


def cql_value(query):
    lines = cql(query).splitlines()
    return lines[3].strip() if len(lines) > 3 else ""


def wait_for(query, expected, timeout=10.0, interval=0.2):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        val = cql_value(query)
        if str(expected) == str(val):
            return val
        time.sleep(interval)
    return cql_value(query)


def wait():
    time.sleep(0.5)


def assert_eq(desc, expected, actual):
    if str(expected) == str(actual):
        passed(f"{desc} (expected={expected}, got={actual})")
    else:
        failed(f"{desc} (expected={expected}, got={actual})")


def available(product, zone):
    return cql_value(
        f"SELECT available_quantity FROM warehouse.inventory_by_product_zone "
        f"WHERE product_id='{product}' AND zone_id='{zone}';")


def reserved(product, zone):
    return cql_value(
        f"SELECT reserved_quantity FROM warehouse.inventory_by_product_zone "
        f"WHERE product_id='{product}' AND zone_id='{zone}';")


def total_available(product):
    return cql_value(
        f"SELECT total_available FROM warehouse.inventory_by_product "
        f"WHERE product_id='{product}';")


def zone_available(zone, product):
    return cql_value(
        f"SELECT available_quantity FROM warehouse.inventory_by_zone "
        f"WHERE zone_id='{zone}' AND product_id='{product}';")


def results():
    log(f"Results: {PASS} passed, {FAIL} failed")
    return FAIL == 0
