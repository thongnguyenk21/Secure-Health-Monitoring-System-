# shared/protocols — Machine-readable Contracts

- **Purpose:** Source of truth for all cross-language interfaces: Protobuf (gRPC and BLE link control), OpenAPI (REST) and JSON Schema. Bindings under `shared/{rust,python,typescript}` are generated from here. AsyncAPI is `[LATER]` (no MQTT in the MVP).
- **Tier:** `[ARCH]`
- **Scope:** `[MVP]` (except `model/v1`, `alert/v1` and `federation/v1`, `[LATER]`)
- **Owner:** RW-01 (MVP-01)
- **Tech:** Protobuf + `buf`, OpenAPI 3.1 + Spectral, JSON Schema 2020-12 + ajv.
- **Version:** see `VERSION` and `CHANGELOG.md`; each freeze is tagged `v<VERSION>`.

## Layout

| Path | Contract |
|---|---|
| `proto/iomt/telemetry/v1/` | edge-sync → cloud gRPC uplink |
| `proto/iomt/firmware/v1/` | wearable ↔ Pi BLE link control, firmware OTA manifest |
| `openapi/cloud-api.openapi.yaml` | cloud REST API for the app, admins and gateway enrolment |
| `jsonschema/` | JSON payloads, one example each in `examples/` |
| `examples/` | `<full.proto.Message>[--label].txtpb` and `<schema>.example.json` |

## Checks

```bash
tools/scripts/gen-contracts.sh --check          # Rust bindings up to date
tools/scripts/lint-contracts.sh --against main  # buf lint + breaking, Spectral, ajv
```

Both run in CI. A contract change goes in its own pull request, regenerates the
bindings and adds a `CHANGELOG.md` entry.
