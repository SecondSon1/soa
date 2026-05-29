#!/bin/bash
set -e

SR=${SCHEMA_REGISTRY_URL:-http://schema-registry:8081}
SUBJECT="warehouse-events-value"

until curl -s "$SR/subjects" > /dev/null 2>&1; do sleep 2; done

curl -s -X PUT "$SR/config/$SUBJECT" \
  -H "Content-Type: application/vnd.schemaregistry.v1+json" \
  -d '{"compatibility": "BACKWARD"}' > /dev/null

SCHEMA_V1=$(cat /schemas/warehouse-event-v1.avsc | sed 's/"/\\"/g' | tr -d '\n')
curl -s -X POST "$SR/subjects/$SUBJECT/versions" \
  -H "Content-Type: application/vnd.schemaregistry.v1+json" \
  -d "{\"schema\": \"$SCHEMA_V1\"}" > /dev/null

SCHEMA_V2=$(cat /schemas/warehouse-event-v2.avsc | sed 's/"/\\"/g' | tr -d '\n')
curl -s -X POST "$SR/subjects/$SUBJECT/versions" \
  -H "Content-Type: application/vnd.schemaregistry.v1+json" \
  -d "{\"schema\": \"$SCHEMA_V2\"}" > /dev/null
