"""Integration tests: verify producer -> Kafka -> consumer -> Cassandra pipeline."""

import uuid
import time
import requests
import pytest
from conftest import send_event, wait_for_cassandra, PRODUCER_URL, CONSUMER_URL


def test_producer_health():
    r = requests.get(f"{PRODUCER_URL}/health")
    assert r.status_code == 200
    assert r.json()["status"] == "UP"


def test_consumer_health():
    r = requests.get(f"{CONSUMER_URL}/health")
    assert r.status_code == 200
    assert r.json()["status"] == "UP"


def test_producer_metrics_endpoint():
    r = requests.get(f"{PRODUCER_URL}/metrics")
    assert r.status_code == 200
    assert "http_requests_total" in r.text


def test_consumer_metrics_endpoint():
    r = requests.get(f"{CONSUMER_URL}/metrics")
    assert r.status_code == 200
    assert "http_requests_total" in r.text
    assert "events_processed_total" in r.text


def test_product_received_flows_through_pipeline(cassandra_session):
    event_id = f"int-test-{uuid.uuid4()}"
    product_id = f"INT-SKU-{uuid.uuid4().hex[:8]}"

    resp = send_event({
        "event_id": event_id,
        "event_type": "PRODUCT_RECEIVED",
        "timestamp": int(time.time() * 1000),
        "product_id": product_id,
        "zone_id": "ZONE-A",
        "quantity": 42,
    })
    assert resp.status_code == 200
    assert resp.json()["event_id"] == event_id

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'",
        42,
    )
    assert result == 42


def test_event_idempotency(cassandra_session):
    event_id = f"idem-test-{uuid.uuid4()}"
    product_id = f"IDEM-SKU-{uuid.uuid4().hex[:8]}"
    ts = int(time.time() * 1000)

    for _ in range(3):
        send_event({
            "event_id": event_id,
            "event_type": "PRODUCT_RECEIVED",
            "timestamp": ts,
            "product_id": product_id,
            "zone_id": "ZONE-A",
            "quantity": 10,
        })

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'",
        10,
    )
    assert result == 10


def test_product_received_updates_all_views(cassandra_session):
    product_id = f"VIEW-SKU-{uuid.uuid4().hex[:8]}"

    send_event({
        "event_id": str(uuid.uuid4()),
        "event_type": "PRODUCT_RECEIVED",
        "timestamp": int(time.time() * 1000),
        "product_id": product_id,
        "zone_id": "ZONE-B",
        "quantity": 77,
    })

    wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-B'",
        77,
    )

    rows = list(cassandra_session.execute(
        f"SELECT total_available FROM inventory_by_product WHERE product_id='{product_id}'"
    ))
    assert rows[0][0] == 77

    rows = list(cassandra_session.execute(
        f"SELECT available_quantity FROM inventory_by_zone WHERE zone_id='ZONE-B' AND product_id='{product_id}'"
    ))
    assert rows[0][0] == 77
