# Склад

## Модель

| Table                       | Partition Key | Clustering Key        |
|-----------------------------|--------------|-----------------------|
| `inventory_by_product_zone` | `product_id` | `zone_id`             |
| `inventory_by_product`      | `product_id` | -                     |
| `inventory_by_zone`         | `zone_id` | `product_id`          |
| `processed_events`          | `event_id` | -                     |
| `orders`                    | `order_id` | -                     |
| `order_items`               | `order_id` | `product_id, zone_id` |

## Consistency

- Записи: `QUORUM` - с RF=3, так переживет отказ одной ноды
- Чтения: `ONE` - здесь неактуальное не страшно

## Порядок

Смотрим на timestamp: храним `last_updated`, если пришедшее событие раньше - метод скипа.

## DLQ

Топик: `warehouse-events-dlq`

## Эволюция схемы

Есть обратная совместимость.
Пруф: у нас уже 2 версии схемы:

- V1: `WarehouseEvent` без supplier_id
- V2: `supplier_id` который nullable string, по умолчанию null

## Observability

- Prometheus: poll'ит /metrics раз в 5 сек
- Grafana: `http://localhost:3000`
- Alertmanager: `http://localhost:9093`

Метрики producer:
- `http_requests_total{method, endpoint, status}`
- `http_request_errors_total{method, endpoint, error_type}`
- `http_request_duration_seconds{method, endpoint}`

Метрики consumer:
- `http_requests_total{method, endpoint, status}`
- `http_request_errors_total{method, endpoint, error_type}`
- `http_request_duration_seconds{method, endpoint}`
- `events_processed_total{event_type}`
- `event_processing_duration_seconds`
- `consumer_lag{partition}`
- `cassandra_write_errors_total`

## Alert Rules

Определены в `prometheus/alerts.yml`, Alertmanager поднимается в docker-compose.

| Alert              | Условие                                     | `for` | Severity |
|--------------------|----------------------------------------------|-------|----------|
| `HighConsumerLag`  | `consumer_lag > 100`                         | 1m    | warning  |
| `HighErrorRate`    | error rate > 5% (по `http_request_errors_total / http_requests_total`) | 1m | critical |
| `HighLatency`      | producer p95 > 1s                            | 1m    | warning  |
| `ServiceDown`      | `up == 0`                                    | 30s   | critical |

Демонстрация срабатывания: `docker compose stop consumer` -> через ~30 сек `ServiceDown` переходит в `firing` в Alertmanager UI (`http://localhost:9093`).

## SLI / SLO

Три SLI, считаются из реальных метрик Prometheus. Проверяются скриптом `scripts/validate_sli.py` в CI (job `sli-validation`) — CI падает если любой SLI превышает порог отказа.

### 1. API Availability

| | |
|---|---|
| **Что измеряется** | Доля успешных (не-5xx) ответов producer |
| **PromQL** | `sum(rate(http_requests_total{job="warehouse-producer",status!~"5.."}[5m])) / sum(rate(http_requests_total{job="warehouse-producer"}[5m]))` |
| **SLO** | >= 99.5% |
| **Порог отказа** | < 95% |

**Обоснование:** Producer - точка входа; если больше 5% запросов фейлят, события не доходят до Kafka и данные теряются. SLO 99.5% допускает редкие сбои (рестарт schema-registry, кратковременная недоступность Kafka). Порог 95% - при таком error rate система фактически не работает.

### 2. Producer Latency p95

| | |
|---|---|
| **Что измеряется** | 95-й перцентиль латентности HTTP-запросов к producer |
| **PromQL** | `histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket{job="warehouse-producer"}[5m])) by (le))` |
| **SLO** | <= 500ms |
| **Порог отказа** | > 1000ms |

**Обоснование:** Producer делает produce+flush в Kafka синхронно. Нормальная латентность ~5-50ms. 500ms означает нагрузку на Kafka или сеть. 1000ms - критическая деградация, клиенты API начнут таймаутить.

### 3. Event Processing Latency p95

| | |
|---|---|
| **Что измеряется** | 95-й перцентиль времени обработки события consumer'ом (десериализация + запись в Cassandra) |
| **PromQL** | `histogram_quantile(0.95, sum(rate(event_processing_duration_seconds_bucket[5m])) by (le))` |
| **SLO** | <= 200ms |
| **Порог отказа** | > 500ms |

**Обоснование:** Consumer выполняет batch write в Cassandra с QUORUM. Нормальная латентность ~1-20ms. 200ms указывает на нагрузку Cassandra-кластера или проблемы с координатором. 500ms - consumer не успевает за потоком, lag начнет расти.
