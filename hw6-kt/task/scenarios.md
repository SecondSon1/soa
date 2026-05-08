## E2E-сценарии для проверки

Ниже — цельные сценарии, которые ассистент может воспроизвести на защите. Студент должен уметь продемонстрировать каждый сценарий, соответствующий набранным баллам.

### Сценарий 1: Базовый цикл склада (пункты 1-3)

1. docker-compose up — система поднимается, consumer подключается к Kafka
2. Отправить PRODUCT_RECEIVED: product=SKU-001, zone=ZONE-A, quantity=100
3. Проверить в Cassandra: inventory_by_product_zone → available=100 в ZONE-A
4. Проверить в Cassandra: inventory_by_product → total_available=100
5. Отправить PRODUCT_RESERVED: product=SKU-001, zone=ZONE-A, quantity=30
6. Проверить: available=70, reserved=30
7. Отправить PRODUCT_MOVED: product=SKU-001, from=ZONE-A, to=ZONE-B, quantity=20
8. Проверить: ZONE-A available=50, ZONE-B available=20
9. Отправить PRODUCT_SHIPPED: product=SKU-001, zone=ZONE-A, quantity=10
10. Проверить: ZONE-A available=40
11. Отправить ORDER_CREATED с позицией SKU-001, quantity=15
12. Проверить: reserved увеличился на 15
13. Отправить ORDER_COMPLETED для этого заказа
14. Проверить: reserved уменьшился на 15

### Сценарий 2: Идемпотентность (пункт 4)

1. Отправить PRODUCT_RECEIVED: product=SKU-002, zone=ZONE-A, quantity=50
2. Проверить: available=50
3. Повторно отправить то же самое событие (тот же event_id)
4. Проверить: available по-прежнему 50 (не 100)

### Сценарий 3: Консистентность таблиц (пункт 5)

1. Отправить PRODUCT_RECEIVED: product=SKU-003, zone=ZONE-A, quantity=100
2. Проверить три таблицы:
   - inventory_by_product_zone: product=SKU-003, zone=ZONE-A → available=100
   - inventory_by_product: product=SKU-003 → total_available=100
   - inventory_by_zone: zone=ZONE-A → содержит SKU-003, available=100
3. Все три таблицы содержат согласованные данные

### Сценарий 4: События вне порядка (пункт 6)

1. Отправить PRODUCT_RECEIVED: product=SKU-004, zone=ZONE-A, quantity=100, timestamp=12:00
2. Отправить PRODUCT_SHIPPED: product=SKU-004, zone=ZONE-A, quantity=20, timestamp=12:05
3. Проверить: available=80
4. Отправить PRODUCT_RECEIVED: product=SKU-004, zone=ZONE-A, quantity=50, timestamp=12:02
   (событие старше, чем последнее обработанное)
5. Проверить: available по-прежнему 80 (событие проигнорировано)

### Сценарий 5: Dead Letter Queue (пункт 7)

1. Отправить невалидное событие: PRODUCT_SHIPPED с quantity=-5
2. Проверить: consumer не упал, продолжает работать
3. Проверить topic warehouse-events-dlq: содержит событие с причиной ошибки
4. Отправить валидное событие после невалидного
5. Проверить: валидное событие обработано корректно

### Сценарий 6: Cassandra cluster и отказоустойчивость (пункт 8)

1. docker-compose up — поднимается кластер из 3 нод Cassandra
2. Выполнить: docker exec cassandra-1 nodetool status → видны 3 ноды в статусе UN
3. Отправить PRODUCT_RECEIVED: product=SKU-006, zone=ZONE-A, quantity=200
4. Проверить: данные записаны корректно
5. Остановить одну ноду: docker stop cassandra-2
6. Отправить PRODUCT_SHIPPED: product=SKU-006, zone=ZONE-A, quantity=50
7. Проверить: событие обработано, available=150 (система работает без ноды)
8. Запустить ноду обратно: docker start cassandra-2
9. Проверить: нода присоединилась к кластеру (nodetool status → 3 ноды UN)
10. Студент демонстрирует разницу CL=ONE vs CL=QUORUM vs CL=ALL при убитой ноде

### Сценарий 7: Мониторинг и consumer lag (пункт 9)

1. Открыть http://localhost:<port>/health → 200 OK
2. Открыть http://localhost:<port>/metrics → видны метрики в Prometheus-формате
3. Отправить 10 событий разных типов
4. Проверить /metrics: events_processed_total увеличился, consumer_lag отображается
5. Открыть Grafana (http://localhost:3000) → dashboard с панелями (lag, throughput, errors)
6. Остановить consumer → consumer_lag растёт
7. Проверить: алерт на consumer lag срабатывает (lag > порога)
8. Запустить consumer обратно → lag уменьшается

### Сценарий 8: Schema Evolution (пункт 10)

1. Отправить событие V1: PRODUCT_RECEIVED (product_id, quantity, zone_id)
2. Проверить: событие обработано, данные в Cassandra корректны
3. Отправить событие V2: PRODUCT_RECEIVED (product_id, quantity, zone_id, supplier_id="SUP-001")
4. Проверить: событие обработано, supplier_id записан в Cassandra
5. Проверить V1-запись: supplier_id = null (значение по умолчанию)
6. Проверить V2-запись: supplier_id = "SUP-001"
7. Студент показывает в Schema Registry обе версии схемы
