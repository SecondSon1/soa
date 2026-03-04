FROM openapitools/openapi-generator-cli:latest AS generator
WORKDIR /work
COPY resources/openapi.yaml /work/openapi.yaml
RUN java -jar /opt/openapi-generator/modules/openapi-generator-cli/target/openapi-generator-cli.jar generate \
    -g rust-axum \
    -i /work/openapi.yaml \
    -o /work/openapi_generated \
    --additional-properties packageName=marketplace_api,packageVersion=0.1.0,hideGenerationTimestamp=true,generateAliasAsModel=true,useSingleRequestParameter=false

FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml build.rs ./
COPY scripts ./scripts
COPY src ./src
COPY --from=generator /work/openapi_generated /app/src/openapi_generated
ENV SKIP_OPENAPI_GENERATION=1
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/marketplace /usr/local/bin/marketplace
EXPOSE 8080
CMD ["marketplace"]
