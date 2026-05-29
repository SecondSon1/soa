#!/usr/bin/env python3
"""Validate SLI metrics from Prometheus after load test.

Queries Prometheus for each defined SLI, compares values against
SLO targets and failure thresholds, outputs a report, and exits
with code 1 if any SLI exceeds its failure threshold.
"""

import json
import sys
import time
import urllib.parse
import urllib.request

PROMETHEUS_URL = "http://localhost:9090"
MAX_RETRIES = 5
RETRY_DELAY = 5

SLIS = [
    {
        "name": "API Availability",
        "query": (
            'sum(rate(http_requests_total{job="warehouse-producer",status!~"5.."}[5m]))'
            " / "
            'sum(rate(http_requests_total{job="warehouse-producer"}[5m]))'
        ),
        "slo": 0.995,
        "failure_threshold": 0.95,
        "comparison": "gte",
        "unit": "ratio",
        "description": "Fraction of non-5xx producer responses",
    },
    {
        "name": "Producer Latency p95",
        "query": (
            "histogram_quantile(0.95, "
            'sum(rate(http_request_duration_seconds_bucket{job="warehouse-producer"}[5m])) by (le)'
            ")"
        ),
        "slo": 0.5,
        "failure_threshold": 1.0,
        "comparison": "lte",
        "unit": "seconds",
        "description": "95th percentile of producer HTTP request duration",
    },
    {
        "name": "Event Processing Latency p95",
        "query": (
            "histogram_quantile(0.95, "
            "sum(rate(event_processing_duration_seconds_bucket[5m])) by (le)"
            ")"
        ),
        "slo": 0.2,
        "failure_threshold": 0.5,
        "comparison": "lte",
        "unit": "seconds",
        "description": "95th percentile of consumer event processing time",
    },
]


def query_prometheus(query):
    url = f"{PROMETHEUS_URL}/api/v1/query?query={urllib.parse.quote(query)}"
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=10) as resp:
        data = json.loads(resp.read())
    if data["status"] != "success":
        return None
    results = data["data"]["result"]
    if not results:
        return None
    value = float(results[0]["value"][1])
    if value != value:  # NaN
        return None
    return value


def format_value(value, unit):
    if value is None:
        return "N/A"
    if unit == "ratio":
        return f"{value * 100:.2f}%"
    return f"{value * 1000:.1f}ms"


def format_threshold(value, unit):
    if unit == "ratio":
        return f"{value * 100:.1f}%"
    return f"{value * 1000:.0f}ms"


def check_sli(value, sli):
    if value is None:
        return None
    if sli["comparison"] == "gte":
        return value >= sli["failure_threshold"]
    return value <= sli["failure_threshold"]


def meets_slo(value, sli):
    if value is None:
        return None
    if sli["comparison"] == "gte":
        return value >= sli["slo"]
    return value <= sli["slo"]


def main():
    print("=" * 60)
    print("SLI Validation Report")
    print("=" * 60)

    results = []
    all_passed = True

    for sli in SLIS:
        value = None
        for attempt in range(MAX_RETRIES):
            value = query_prometheus(sli["query"])
            if value is not None:
                break
            if attempt < MAX_RETRIES - 1:
                print(f"  [{sli['name']}] No data yet, retrying in {RETRY_DELAY}s...")
                time.sleep(RETRY_DELAY)

        passed = check_sli(value, sli)
        slo_met = meets_slo(value, sli)

        if passed is None:
            status = "NO_DATA"
        elif passed:
            status = "PASS"
        else:
            status = "FAIL"
            all_passed = False

        result = {
            "name": sli["name"],
            "description": sli["description"],
            "query": sli["query"],
            "value": value,
            "value_display": format_value(value, sli["unit"]),
            "slo": format_threshold(sli["slo"], sli["unit"]),
            "failure_threshold": format_threshold(sli["failure_threshold"], sli["unit"]),
            "status": status,
            "meets_slo": slo_met,
        }
        results.append(result)

        icon = {"PASS": "+", "FAIL": "!", "NO_DATA": "?"}[status]
        print(f"\n[{icon}] {sli['name']}")
        print(f"    {sli['description']}")
        print(f"    Current value:    {result['value_display']}")
        print(f"    SLO target:       {result['slo']}")
        print(f"    Failure threshold: {result['failure_threshold']}")
        print(f"    Status:           {status}")
        if slo_met is not None:
            print(f"    Meets SLO:        {'yes' if slo_met else 'no'}")

    print("\n" + "=" * 60)

    report = {
        "slis": results,
        "all_passed": all_passed,
        "timestamp": time.time(),
    }
    with open("sli-report.json", "w") as f:
        json.dump(report, f, indent=2)
    print("\nReport saved to sli-report.json")

    if not all_passed:
        print("\nFAILED: One or more SLIs exceeded failure threshold")
        sys.exit(1)
    else:
        print("\nPASSED: All SLIs within acceptable range")


if __name__ == "__main__":
    main()
