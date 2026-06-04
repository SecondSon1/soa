import time
import pytest
import requests
from cassandra.cluster import Cluster


PRODUCER_URL = "http://localhost:8082"
CONSUMER_URL = "http://localhost:8080"
CASSANDRA_HOST = "localhost"
CASSANDRA_PORT = 9042
KEYSPACE = "warehouse"


@pytest.fixture(scope="session")
def cassandra_session():
    cluster = Cluster([CASSANDRA_HOST], port=CASSANDRA_PORT)
    session = cluster.connect(KEYSPACE)
    yield session
    cluster.shutdown()


@pytest.fixture(autouse=True)
def cleanup_cassandra(cassandra_session):
    yield
    tables = [
        "inventory_by_product_zone",
        "inventory_by_product",
        "inventory_by_zone",
        "processed_events",
        "orders",
        "order_items",
    ]
    for table in tables:
        cassandra_session.execute(f"TRUNCATE {table}")


def send_event(event: dict, timeout: float = 5.0) -> requests.Response:
    return requests.post(f"{PRODUCER_URL}/", json=event, timeout=timeout)


def wait_for_cassandra(session, query: str, expected, timeout: float = 15.0, interval: float = 0.3):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        rows = list(session.execute(query))
        if rows and str(rows[0][0]) == str(expected):
            return rows[0][0]
        time.sleep(interval)
    rows = list(session.execute(query))
    if rows:
        return rows[0][0]
    return None
