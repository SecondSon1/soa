import json
import time
import uuid
from http.server import HTTPServer, BaseHTTPRequestHandler
from confluent_kafka import SerializingProducer
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.avro import AvroSerializer
from confluent_kafka.serialization import StringSerializer

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


def make_producer():
    sr = SchemaRegistryClient({"url": SCHEMA_REGISTRY_URL})
    avro_serializer = AvroSerializer(sr, SCHEMA_STR)
    return SerializingProducer({
        "bootstrap.servers": KAFKA_BOOTSTRAP,
        "key.serializer": StringSerializer("utf_8"),
        "value.serializer": avro_serializer,
    })


producer = make_producer()


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))

        event = {
            "event_id": body.get("event_id", str(uuid.uuid4())),
            "event_type": body["event_type"],
            "timestamp": body.get("timestamp", int(time.time() * 1000)),
            "product_id": body.get("product_id"),
            "zone_id": body.get("zone_id"),
            "quantity": body.get("quantity"),
            "from_zone_id": body.get("from_zone_id"),
            "to_zone_id": body.get("to_zone_id"),
            "order_id": body.get("order_id"),
            "order_items": body.get("order_items"),
            "supplier_id": body.get("supplier_id"),
        }

        producer.produce(TOPIC, key=event["product_id"] or event.get("order_id") or event["event_id"], value=event)
        producer.flush()

        resp = json.dumps({"event_id": event["event_id"]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(resp)

    def log_message(self, format, *args):
        pass


HTTPServer(("0.0.0.0", 8081), Handler).serve_forever()
