"""E2E test: full warehouse scenario from product receipt through order completion."""

import uuid
import time
import pytest
from conftest import send_event, wait_for_cassandra


def test_full_warehouse_scenario(cassandra_session):
    """
    Full scenario:
    1. Receive product SKU into ZONE-A (qty=100)
    2. Reserve some stock (qty=30)
    3. Move product from ZONE-A to ZONE-B (qty=20)
    4. Create order from ZONE-A stock
    5. Complete the order
    6. Verify final state across all Cassandra tables
    """
    product_id = f"E2E-SKU-{uuid.uuid4().hex[:8]}"
    order_id = f"E2E-ORD-{uuid.uuid4().hex[:8]}"
    base_ts = int(time.time() * 1000)

    # Step 1: receive product
    resp = send_event({
        "event_id": str(uuid.uuid4()),
        "event_type": "PRODUCT_RECEIVED",
        "timestamp": base_ts,
        "product_id": product_id,
        "zone_id": "ZONE-A",
        "quantity": 100,
        "supplier_id": "SUP-001",
    })
    assert resp.status_code == 200

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'",
        100,
    )
    assert result == 100

    # Step 2: reserve stock
    resp = send_event({
        "event_id": str(uuid.uuid4()),
        "event_type": "PRODUCT_RESERVED",
        "timestamp": base_ts + 1000,
        "product_id": product_id,
        "zone_id": "ZONE-A",
        "quantity": 30,
    })
    assert resp.status_code == 200

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'",
        70,
    )
    assert result == 70

    rows = list(cassandra_session.execute(
        f"SELECT reserved_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'"
    ))
    assert rows[0][0] == 30

    # Step 3: move product
    resp = send_event({
        "event_id": str(uuid.uuid4()),
        "event_type": "PRODUCT_MOVED",
        "timestamp": base_ts + 2000,
        "product_id": product_id,
        "quantity": 20,
        "from_zone_id": "ZONE-A",
        "to_zone_id": "ZONE-B",
    })
    assert resp.status_code == 200

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'",
        50,
    )
    assert result == 50

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-B'",
        20,
    )
    assert result == 20

    # Step 4: create order from ZONE-A
    resp = send_event({
        "event_id": str(uuid.uuid4()),
        "event_type": "ORDER_CREATED",
        "timestamp": base_ts + 3000,
        "order_id": order_id,
        "order_items": [
            {"product_id": product_id, "zone_id": "ZONE-A", "quantity": 15},
        ],
    })
    assert resp.status_code == 200

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT available_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'",
        35,
    )
    assert result == 35

    rows = list(cassandra_session.execute(
        f"SELECT reserved_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'"
    ))
    assert rows[0][0] == 45  # 30 + 15

    rows = list(cassandra_session.execute(
        f"SELECT status FROM orders WHERE order_id='{order_id}'"
    ))
    assert rows[0][0] == "CREATED"

    # Step 5: complete order
    resp = send_event({
        "event_id": str(uuid.uuid4()),
        "event_type": "ORDER_COMPLETED",
        "timestamp": base_ts + 4000,
        "order_id": order_id,
    })
    assert resp.status_code == 200

    result = wait_for_cassandra(
        cassandra_session,
        f"SELECT reserved_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'",
        30,  # 45 - 15
    )
    assert result == 30

    # Step 6: verify final state
    # ZONE-A: available=35, reserved=30
    rows = list(cassandra_session.execute(
        f"SELECT available_quantity, reserved_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-A'"
    ))
    assert rows[0][0] == 35
    assert rows[0][1] == 30

    # ZONE-B: available=20, reserved=0
    rows = list(cassandra_session.execute(
        f"SELECT available_quantity, reserved_quantity FROM inventory_by_product_zone "
        f"WHERE product_id='{product_id}' AND zone_id='ZONE-B'"
    ))
    assert rows[0][0] == 20
    assert rows[0][1] == 0

    # Total: available=55 (35+20), reserved=30
    rows = list(cassandra_session.execute(
        f"SELECT total_available, total_reserved FROM inventory_by_product "
        f"WHERE product_id='{product_id}'"
    ))
    assert rows[0][0] == 55
    assert rows[0][1] == 30

    # Order completed
    rows = list(cassandra_session.execute(
        f"SELECT status FROM orders WHERE order_id='{order_id}'"
    ))
    assert rows[0][0] == "COMPLETED"

    # Verify event processing was recorded
    rows = list(cassandra_session.execute(
        "SELECT count(*) FROM processed_events"
    ))
    assert rows[0][0] >= 5
