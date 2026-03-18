COMPOSE ?= docker compose
COMPOSE_FILE ?= docker-compose.yml
LOCAL_COMPOSE_FILE ?= docker-compose.local.yml
LOCAL_GRPC_API_KEY ?= local-dev-api-key

.PHONY: docker-build docker-up docker-down local-up local-down run-flight-local run-booking-local

docker-build:
	$(COMPOSE) -f $(COMPOSE_FILE) build

docker-up:
	$(COMPOSE) -f $(COMPOSE_FILE) up -d

docker-down:
	$(COMPOSE) -f $(COMPOSE_FILE) down

local-up:
	$(COMPOSE) -f $(LOCAL_COMPOSE_FILE) up -d

local-down:
	$(COMPOSE) -f $(LOCAL_COMPOSE_FILE) down

run-flight-local:
	FLIGHT_GRPC_PORT=9091 \
	FLIGHT_DB_URL=jdbc:postgresql://localhost:15434/flight \
	FLIGHT_DB_USER=flight \
	FLIGHT_DB_PASSWORD=flight \
	FLIGHT_REDIS_SENTINELS=localhost:16379 \
	FLIGHT_REDIS_MASTER_ID=flight-cache-master \
	FLIGHT_REDIS_DATABASE=0 \
	FLIGHT_GRPC_API_KEY=$(LOCAL_GRPC_API_KEY) \
	./gradlew :flight-service:run

run-booking-local:
	BOOKING_HTTP_PORT=8081 \
	BOOKING_DB_URL=jdbc:postgresql://localhost:15433/booking \
	BOOKING_DB_USER=booking \
	BOOKING_DB_PASSWORD=booking \
	FLIGHT_GRPC_HOST=localhost \
	FLIGHT_GRPC_PORT=9091 \
	FLIGHT_GRPC_API_KEY=$(LOCAL_GRPC_API_KEY) \
	FLIGHT_GRPC_RETRY_MAX_ATTEMPTS=3 \
	FLIGHT_GRPC_RETRY_INITIAL_BACKOFF_MS=100 \
	FLIGHT_GRPC_RETRY_MAX_BACKOFF_MS=400 \
	FLIGHT_GRPC_RETRY_BACKOFF_MULTIPLIER=2.0 \
	FLIGHT_GRPC_CIRCUIT_BREAKER_FAILURE_THRESHOLD=5 \
	FLIGHT_GRPC_CIRCUIT_BREAKER_WINDOW_SECONDS=30 \
	FLIGHT_GRPC_CIRCUIT_BREAKER_OPEN_SECONDS=15 \
	./gradlew :booking-service:run
