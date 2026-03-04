.PHONY: generate build compose-build

generate:
	./scripts/generate-openapi.sh

build: generate
	cargo build --release

run: generate
	cargo run --release

check: generate
	cargo check

fmt:
	cargo fmt

compose-build:
	docker compose build

compose-up:
	docker compose up --build

compose-down:
	docker compose down

compose-down-clean:
	docker compose down -v
