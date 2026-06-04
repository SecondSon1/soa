import json
import time
import uuid
from http.server import HTTPServer, BaseHTTPRequestHandler
from confluent_kafka import SerializingProducer
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.avro import AvroSerializer
from confluent_kafka.serialization import StringSerializer
from prometheus_client import Counter, Histogram, generate_latest, CONTENT_TYPE_LATEST

SCHEMA_REGISTRY_URL = "http://schema-registry:8081"
KAFKA_BOOTSTRAP = "kafka:29092"
TOPIC = "warehouse-events"

SCHEMA_STR = """{
  "type": "record",
  "name": "WarehouseEvent",
  "namespace": "warehouse.avro",
  "fields": [
    {"name": "event_id", "type": "string"},
    {"name": "event_type", "type": "string"},
    {"name": "timestamp", "type": "long"},
    {"name": "product_id", "type": ["null", "string"], "default": null},
    {"name": "zone_id", "type": ["null", "string"], "default": null},
    {"name": "quantity", "type": ["null", "int"], "default": null},
    {"name": "from_zone_id", "type": ["null", "string"], "default": null},
    {"name": "to_zone_id", "type": ["null", "string"], "default": null},
    {"name": "order_id", "type": ["null", "string"], "default": null},
    {"name": "order_items", "type": ["null", {"type": "array", "items": {"type": "record", "name": "OrderItem", "fields": [{"name": "product_id", "type": "string"}, {"name": "zone_id", "type": "string"}, {"name": "quantity", "type": "int"}]}}], "default": null},
    {"name": "supplier_id", "type": ["null", "string"], "default": null}
  ]
}"""

REQUEST_COUNT = Counter(
    "http_requests_total",
    "Total HTTP requests",
    ["method", "endpoint", "status"],
)
REQUEST_ERRORS = Counter(
    "http_request_errors_total",
    "Total HTTP request errors",
    ["method", "endpoint", "error_type"],
)
REQUEST_DURATION = Histogram(
    "http_request_duration_seconds",
    "HTTP request duration in seconds",
    ["method", "endpoint"],
    buckets=(0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0),
)


def make_producer():
    sr = SchemaRegistryClient({"url": SCHEMA_REGISTRY_URL})
    avro_serializer = AvroSerializer(sr, SCHEMA_STR)
    return SerializingProducer({
        "bootstrap.servers": KAFKA_BOOTSTRAP,
        "key.serializer": StringSerializer("utf_8"),
        "value.serializer": avro_serializer,
    })


def build_event(body):
    """Normalize an incoming request body into a full warehouse event.

    Generates an event_id / timestamp when absent and fills every optional
    field with None so the Avro serializer always sees the complete schema.
    """
    return {
        "event_id": body.get("event_id") or str(uuid.uuid4()),
        "event_type": body["event_type"],
        "timestamp": body.get("timestamp") or int(time.time() * 1000),
        "product_id": body.get("product_id"),
        "zone_id": body.get("zone_id"),
        "quantity": body.get("quantity"),
        "from_zone_id": body.get("from_zone_id"),
        "to_zone_id": body.get("to_zone_id"),
        "order_id": body.get("order_id"),
        "order_items": body.get("order_items"),
        "supplier_id": body.get("supplier_id"),
    }


def partition_key(event):
    """Kafka key: prefer product_id so all events for a product land on the
    same partition (preserves per-product ordering); fall back to order_id,
    then event_id."""
    return event["product_id"] or event.get("order_id") or event["event_id"]


producer = None


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/metrics":
            self._handle_metrics()
        elif self.path == "/health":
            self._handle_health()
        else:
            self._send(404, {"error": "not found"})

    def do_POST(self):
        endpoint = self.path
        start = time.monotonic()
        try:
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            event = build_event(body)
            producer.produce(
                TOPIC,
                key=partition_key(event),
                value=event,
            )
            producer.flush()

            self._send(200, {"event_id": event["event_id"]})
            REQUEST_COUNT.labels("POST", endpoint, "200").inc()
        except Exception as e:
            self._send(500, {"error": str(e)})
            REQUEST_COUNT.labels("POST", endpoint, "500").inc()
            REQUEST_ERRORS.labels("POST", endpoint, type(e).__name__).inc()
        finally:
            REQUEST_DURATION.labels("POST", endpoint).observe(time.monotonic() - start)

    def _handle_metrics(self):
        output = generate_latest()
        self.send_response(200)
        self.send_header("Content-Type", CONTENT_TYPE_LATEST)
        self.end_headers()
        self.wfile.write(output)

    def _handle_health(self):
        start = time.monotonic()
        self._send(200, {"status": "UP"})
        REQUEST_COUNT.labels("GET", "/health", "200").inc()
        REQUEST_DURATION.labels("GET", "/health").observe(time.monotonic() - start)

    def _send(self, status, body):
        resp = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(resp)

    def log_message(self, format, *args):
        pass


if __name__ == "__main__":
    producer = make_producer()
    HTTPServer(("0.0.0.0", 8081), Handler).serve_forever()
