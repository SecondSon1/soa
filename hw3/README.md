# HW3: Booking API + Flight API

## Запуск в docker compose

```bash
make docker-build
make docker-up
```

Что поднимается:

- booking rest api: `localhost:8080`
- flight gRPC: `localhost:9090`
- booking DB: `localhost:5433`
- flight DB: `localhost:5434`
- Redis master: `localhost:6379`
- Redis Sentinel: `localhost:26379`

## Лоакльный запуск

Ужасно долгий билд gradle внутри образа docker-compose, так что можно поднять все кроме сервисов в докере,
а сервисы поднимать локально.

### Запуск инфры

```bash
make local-up
```

### Запуск сервисов

```bash
make run-flight-local
```

```bash
make run-booking-local
```

Я добавил env-переменные в Run target в Intellij Idea, удобно.

Локальные подняты на:
- booking DB: `localhost:15433`
- flight DB: `localhost:15434`
- Redis Sentinel: `localhost:16379`

## Для сдачи

Вызов flight api без кредов:

```bash
grpcurl -plaintext \
  -import-path flight-contract/src/main/proto \
  -proto flight_service.proto \
  -d '{"flight_id":"11111111-1111-1111-1111-111111111111"}' \
  localhost:9090 \
  flight.v1.FlightService/GetFlight
```

С кредами:

```bash
grpcurl -plaintext \
  -import-path flight-contract/src/main/proto \
  -proto flight_service.proto \
  -H 'x-api-key: docker-compose-api-key' \
  -d '{"flight_id":"11111111-1111-1111-1111-111111111111"}' \
  localhost:9090 \
  flight.v1.FlightService/GetFlight
```