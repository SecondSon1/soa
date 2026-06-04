"""Unit tests for the producer's request-normalization and partitioning logic.

These cover the pure logic the HTTP handler relies on, with no Kafka or
network: how an incoming body is turned into a complete event, and how the
Kafka partition key is chosen (which is what guarantees per-product ordering
and idempotency downstream).
"""

import time
import uuid

import pytest

from main import build_event, partition_key


def test_build_event_preserves_provided_fields():
    body = {
        "event_id": "evt-1",
        "event_type": "PRODUCT_RECEIVED",
        "timestamp": 1700000000000,
        "product_id": "SKU-1",
        "zone_id": "ZONE-A",
        "quantity": 42,
        "supplier_id": "SUP-1",
    }

    event = build_event(body)

    assert event["event_id"] == "evt-1"
    assert event["event_type"] == "PRODUCT_RECEIVED"
    assert event["timestamp"] == 1700000000000
    assert event["product_id"] == "SKU-1"
    assert event["zone_id"] == "ZONE-A"
    assert event["quantity"] == 42
    assert event["supplier_id"] == "SUP-1"


def test_build_event_fills_optional_fields_with_none():
    event = build_event({"event_type": "INVENTORY_COUNTED", "product_id": "SKU-2"})

    for field in (
        "zone_id",
        "quantity",
        "from_zone_id",
        "to_zone_id",
        "order_id",
        "order_items",
        "supplier_id",
    ):
        assert event[field] is None, f"{field} should default to None"


def test_build_event_generates_unique_event_id_when_missing():
    a = build_event({"event_type": "PRODUCT_RECEIVED"})
    b = build_event({"event_type": "PRODUCT_RECEIVED"})

    # Valid UUIDs and distinct between calls.
    uuid.UUID(a["event_id"])
    uuid.UUID(b["event_id"])
    assert a["event_id"] != b["event_id"]


def test_build_event_defaults_timestamp_to_now():
    before = int(time.time() * 1000)
    event = build_event({"event_type": "PRODUCT_RECEIVED"})
    after = int(time.time() * 1000)

    assert before <= event["timestamp"] <= after


def test_build_event_requires_event_type():
    with pytest.raises(KeyError):
        build_event({"product_id": "SKU-3"})


def test_partition_key_prefers_product_id():
    event = build_event({
        "event_type": "ORDER_CREATED",
        "event_id": "evt-1",
        "product_id": "SKU-1",
        "order_id": "ORD-1",
    })
    assert partition_key(event) == "SKU-1"


def test_partition_key_falls_back_to_order_id():
    # Order-level events (e.g. ORDER_COMPLETED) carry no product_id.
    event = build_event({
        "event_type": "ORDER_COMPLETED",
        "event_id": "evt-2",
        "order_id": "ORD-1",
    })
    assert partition_key(event) == "ORD-1"


def test_partition_key_falls_back_to_event_id():
    event = build_event({"event_type": "PRODUCT_RECEIVED", "event_id": "evt-3"})
    assert partition_key(event) == "evt-3"


def test_partition_key_is_stable_per_product():
    # Two distinct events for the same product must share a partition key so
    # Kafka keeps them ordered on the same partition.
    first = build_event({"event_type": "PRODUCT_RECEIVED", "product_id": "SKU-9"})
    second = build_event({"event_type": "PRODUCT_SHIPPED", "product_id": "SKU-9"})
    assert partition_key(first) == partition_key(second) == "SKU-9"
