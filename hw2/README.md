# Marketplace

## OpenAPI генерация

```bash
make generate
```

## Запустить локально

```bash
make build
APP_ADDR=0.0.0.0:8080 DATABASE_URL=postgres://postgres:postgres@postgres:5432/marketplace ./target/release/marketplace
```

## Запустить в docker compose

```bash
make compose-up
```