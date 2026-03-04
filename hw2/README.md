# Marketplace

## OpenAPI генерация

```bash
make generate
```

## Запустить локально

```bash
make build
APP_ADDR=0.0.0.0:8080 \
DATABASE_URL=postgres://postgres:postgres@postgres:5432/marketplace \
JWT_SECRET=local-dev-secret-change-me \
JWT_ACCESS_TTL_MINUTES=15 \
JWT_REFRESH_TTL_DAYS=7 \
./target/release/marketplace
```

## Запустить в docker compose

```bash
make compose-up
```
