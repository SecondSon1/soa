import http from "k6/http";
import { check, sleep } from "k6";
import { randomString, randomIntBetween } from "https://jslib.k6.io/k6-utils/1.4.0/index.js";

export const options = {
  stages: [
    { duration: "10s", target: 10 },
    { duration: "30s", target: 10 },
    { duration: "5s", target: 0 },
  ],
  thresholds: {
    http_req_duration: ["p(95)<2000"],
    http_req_failed: ["rate<0.05"],
  },
};

const PRODUCER_URL = __ENV.PRODUCER_URL || "http://localhost:8082";

const EVENT_TYPES = [
  "PRODUCT_RECEIVED",
  "PRODUCT_SHIPPED",
  "PRODUCT_RESERVED",
  "PRODUCT_RELEASED",
  "INVENTORY_COUNTED",
];

export default function () {
  const eventType =
    EVENT_TYPES[randomIntBetween(0, EVENT_TYPES.length - 1)];
  const payload = JSON.stringify({
    event_id: `load-${randomString(16)}`,
    event_type: eventType,
    timestamp: Date.now(),
    product_id: `LOAD-SKU-${randomIntBetween(1, 50)}`,
    zone_id: `ZONE-${String.fromCharCode(65 + randomIntBetween(0, 4))}`,
    quantity: randomIntBetween(1, 100),
  });

  const res = http.post(`${PRODUCER_URL}/`, payload, {
    headers: { "Content-Type": "application/json" },
  });

  check(res, {
    "status is 200": (r) => r.status === 200,
    "has event_id": (r) => r.json("event_id") !== undefined,
  });

  sleep(0.1);
}

export function handleSummary(data) {
  const stdout = JSON.stringify(data, null, 2);
  return {
    stdout: stdout,
    "loadtest-results.json": stdout,
  };
}
