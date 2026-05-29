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

## Графана

- Prometheus: poll'ит /metrics раз в 5 сек
- Grafana: `http://localhost:3000`

Метрики:
- `consumer_lag`
- `events_processed_total`
- `event_processing_duration_seconds`
- `cassandra_write_errors_total`
