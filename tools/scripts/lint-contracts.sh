#!/usr/bin/env bash
# Lints every contract in shared/protocols; used locally and by CI.
#   tools/scripts/lint-contracts.sh                  lint only
#   tools/scripts/lint-contracts.sh --against <ref>  also fail on breaking proto changes vs a git ref
# Needs buf on PATH and Node.js (npx) for the OpenAPI and JSON Schema checks.
# Tool versions are pinned here so local runs and CI agree.
set -euo pipefail
cd "$(dirname "$0")/../../shared/protocols"

SPECTRAL="@stoplight/spectral-cli@6.16.3"
AJV="ajv-cli@5.0.0"
AJV_FORMATS="ajv-formats@3.0.1"

echo "== buf lint"
buf lint

if [[ "${1:-}" == "--against" ]]; then
  echo "== buf breaking against ${2:?missing git ref}"
  buf breaking --against "../../.git#ref=${2},subdir=shared/protocols"
fi

echo "== OpenAPI (Spectral)"
for spec in openapi/*.openapi.yaml; do
  npx --yes "$SPECTRAL" lint "$spec" --ruleset .spectral.yaml --fail-severity=warn
done

echo "== JSON Schema examples (ajv, draft 2020-12)"
for schema in jsonschema/*.schema.json; do
  name="$(basename "$schema" .schema.json)"
  example="examples/${name}.example.json"
  if [[ ! -f "$example" ]]; then
    echo "missing example $example for $schema (03 §5.2 rule 3)" >&2
    exit 1
  fi
  npx --yes -p "$AJV" -p "$AJV_FORMATS" -- ajv validate --spec=draft2020 -c ajv-formats -s "$schema" -d "$example"
done
echo "contracts OK"
