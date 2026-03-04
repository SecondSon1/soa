#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC_PATH="${ROOT_DIR}/resources/openapi.yaml"
OUT_DIR="${ROOT_DIR}/src/openapi_generated"
IMAGE="${OPENAPI_GENERATOR_IMAGE:-openapitools/openapi-generator-cli:latest}"

if [[ ! -f "${SPEC_PATH}" ]]; then
  echo "OpenAPI spec not found: ${SPEC_PATH}" >&2
  exit 1
fi

rm -rf "${OUT_DIR}"
mkdir -p "${OUT_DIR}"

if [[ -n "${OPENAPI_GENERATOR_CLI_JAR:-}" && -f "${OPENAPI_GENERATOR_CLI_JAR}" ]]; then
  java -jar "${OPENAPI_GENERATOR_CLI_JAR}" generate \
    -g rust-axum \
    -i "${SPEC_PATH}" \
    -o "${OUT_DIR}" \
    --additional-properties packageName=marketplace_api,packageVersion=0.1.0,hideGenerationTimestamp=true,generateAliasAsModel=true,useSingleRequestParameter=false
else
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    -v "${ROOT_DIR}:/local" \
    "${IMAGE}" generate \
    -g rust-axum \
    -i /local/resources/openapi.yaml \
    -o /local/src/openapi_generated \
    --additional-properties packageName=marketplace_api,packageVersion=0.1.0,hideGenerationTimestamp=true,generateAliasAsModel=true,useSingleRequestParameter=false
fi

echo "OpenAPI rust-axum server code generated in ${OUT_DIR}"
