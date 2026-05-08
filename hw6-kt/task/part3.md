## 8-10 баллов

---

8. Cassandra Multi-Node Cluster (1 балл)

Необходимо развернуть кластер Cassandra из 3 нод и продемонстрировать отказоустойчивость и понимание consistency levels.

Требования:

- В docker-compose.yml описаны 3 ноды Cassandra (cassandra-1, cassandra-2, cassandra-3), объединённые в один кластер.
- Keyspace создаётся с NetworkTopologyStrategy и replication_factor = 3.
- Consumer использует осознанные consistency levels:
    - Для записей — QUORUM (гарантия, что данные записаны на большинство нод).
    - Для чтений — студент выбирает ONE или QUORUM и обосновывает выбор в README (trade-off между скоростью и консистентностью).
- Продемонстрирована отказоустойчивость: при остановке одной ноды (docker stop cassandra-2) система продолжает принимать и обрабатывать события без ошибок.

---

9. Monitoring + Consumer Lag (1 балл)

Необходимо реализовать мониторинг consumer-сервиса: метрики, health-проверки, визуализация в Grafana.

Требования:

Prometheus-совместимый endpoint /metrics с метриками:

- consumer_lag — отставание consumer от HEAD топика (разница между latest offset и committed offset). Gauge, по партициям.
- events_processed_total — счётчик обработанных событий (label: event_type). Counter.
- event_processing_duration_seconds — время обработки одного события. Histogram.
- cassandra_write_errors_total — количество ошибок при записи в Cassandra. Counter.

Health endpoint /health для liveness/readiness проб:

- Возвращает 200 OK если consumer подключён к Kafka и Cassandra доступна.
- Возвращает 503 Service Unavailable если одно из подключений потеряно.

Grafana dashboard:

- В docker-compose.yml поднимается Prometheus (scrape consumer /metrics) и Grafana.
- Создан dashboard с минимум 3 панелями:
    - Consumer lag по партициям
    - Throughput — events processed per second
    - Ошибки записи в Cassandra

---

10. Schema Evolution (1 балл)

Необходимо реализовать поддержку двух версий одного типа события и продемонстрировать, что consumer обрабатывает обе версии одновременно.

Задание (по шагам):

1. Зарегистрировать в Schema Registry исходную Avro-схему для одного из событий (например, ProductReceived).
2. Создать вторую версию схемы с дополнительным полем (например, supplier_id). Новое поле должно иметь значение по умолчанию (null) для backward compatibility.
3. Зарегистрировать V2-схему в Schema Registry с проверкой совместимости (backward).
4. Реализовать в consumer обработку обеих версий: V1-события (без нового поля) и V2-события (с новым полем) приходят в одном топике и обрабатываются без ошибок.
5. Для V2 — новое поле записывается в Cassandra (добавить колонку). Для V1 — колонка получает значение по умолчанию (null).
6. Документировать в README: какая стратегия совместимости используется и пошаговая инструкция по добавлению новой версии события.

Требования:

- Версионирование реализовано через Schema Registry (Avro + backward compatibility).
- Consumer содержит явную логику обработки разных версий.
- Старые события (V1) продолжают обрабатываться без ошибок после добавления V2.

Пример: Avro schema evolution (backward compatible)

```
{
  "type": "record",
  "name": "ProductReceived",
  "fields": [
    {"name": "product_id", "type": "string"},
    {"name": "quantity", "type": "int"},
    {"name": "zone_id", "type": "string"},
    {"name": "supplier_id", "type": ["null", "string"], "default": null}
  ]
}
```
