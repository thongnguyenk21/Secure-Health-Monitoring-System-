# 03 — Spec Standard, ADR, API Contracts, Versioning & Traceability

Mandatory standard for all specs, architecture decisions, and interface contracts in the repo. It fully replaces the current disjointed table format of `docs/specs/WP_SPECS.md`.

> Reference stack (`00 §0.0`): Rust (firmware/edge/cloud backend), Python (ML: `edge_ml`, `local-trainer`, `fl-server`, benchmarks/experiments), TypeScript (mobile). Contracts in `shared/protocols/`; codegen → Rust (prost/tonic), TypeScript, Python. Java/Spring/Maven/Flyway are gone.

## 1. Document classification & lifecycle

| Type | ID prefix | Canonical location | Template |
|---|---|---|---|
| Work Package spec | `WP-NN` | `docs/specs/wp/WP-NN-*.md` | §3 |
| Architecture Decision Record | `ADR-NNNN` | `docs/adr/NNNN-*.md` | §4 |
| Non-Functional Requirement registry | `MET-NN` | `docs/specs/NFR.md` | §7 |
| API contract | — | `shared/protocols/**` | §5 |
| Traceability matrix | — | `docs/specs/traceability.csv` | §6 |

Spec status: `Draft → In Review → Approved → Implemented → Deprecated`. Only `Approved` specs may be coded; `Implemented` when DoD (§8) is met and all tests are green.

## 2. Requirement writing rules (mandatory)

1. **Atomic:** 1 requirement = 1 verifiable behavior.
2. **Identification:** `REQ-WP-NN-NNN` for WP specs (NNN increments, never reused); cross-cutting registries use `REQ-OPS-NN` (observability, `06 §2`), `REQ-SEC-NN` (security/identity, `07 §2`), `REQ-REL-NN` (reliability/data lifecycle, `08 §2`) and `REQ-DEMO-NN` (demo/testability enablement, `03 §10`).
3. **Testable:** must be able to write at least 1 test `T-REQ-WP-NN-NNN-xx` (cross-cutting: `T-REQ-OPS-NN-01`, `T-REQ-SEC-NN-xx`, `T-REQ-REL-NN-xx`, `T-REQ-DEMO-NN-01`). Vague words ("fast", "stable", "friendly") are forbidden unless accompanied by a quantitative threshold.
4. **Clear active/passive:** use MUST/SHOULD/MAY (RFC 2119) in the content.
5. **Has evidence:** tied to a metric (`MET-NN`), test, and generated artifact.
6. **No solution mixing:** the spec states *what & constraints*, not implementation details (implementation belongs to code/PR).

## 3. Template WP Spec

Copy from `tools/templ/wp-spec.md`. All sections are mandatory; write `N/A + reason` if not applicable.

```markdown
# WP-NN — <Work package name>

## 0. Metadata
| Field | Value |
|---|---|
| WP ID | WP-NN |
| Phase | Phase 1 / Phase 2 |
| **Scope** | **MVP (blocks E2E demo on Pi 3B+) / LATER (does not block demo)** |
| Tier | T0 IoT / T1 Edge / T2 Cloud / T3 Mobile / cross |
| Runtime | edge-lite (Pi 3B+, aarch64) / edge-train (powerful machine) / cloud / mobile / N/A |
| Language | Rust / Python / TypeScript |
| Owner (CODEOWNERS) | @<user/team> |
| Status | Draft / In Review / Approved / Implemented / Deprecated |
| Depends on | WP-xx, ADR-xxxx |
| Contract refs | shared/protocols/proto/... , shared/protocols/openapi/... |
| Created / updated | YYYY-MM-DD / YYYY-MM-DD |

## 1. Context & objectives (Context)
<2–5 sentences. Problem, motivation, JD § reference>

## 2. Scope
### 2.1 In scope
- ...
### 2.2 Out of scope
- ... (explicit to prevent scope creep)

## 3. Interfaces (contracts)
| Direction | Contract | Description |
|---|---|---|
| Input  | shared/protocols/jsonschema/xxx.v1.schema.json | ... |
| Output | shared/protocols/proto/iomt/yyy/v1/yyy.proto | ... |
| REST   | shared/protocols/openapi/zzz.openapi.yaml | ... |
| Event  | shared/protocols/asyncapi/mqtt-topics.asyncapi.yaml | ... |

## 4. Functional Requirements
| ID | Requirement (MUST/SHOULD/MAY) | Source |
|---|---|---|
| REQ-WP-NN-001 | The system MUST ... | JD §x |
| REQ-WP-NN-002 | The system SHOULD ... | ADR-xxxx |

## 5. Non-Functional Requirements (tied to MET)
| MET ID | Target | Threshold | Measurement method | Source |
|---|---|---|---|---|
| MET-01 | IoT inference latency | p95 < 5 ms | on-device esp_timer + host bench | NFR.md |
| MET-02 | E2E latency | p95 < 300 ms | timestamp source→sink | NFR.md |

## 6. Acceptance Criteria (testable)
- [ ] AC-01: Given <condition>, When <action>, Then <measurable result>.
      → Test: T-REQ-WP-NN-001-01
- [ ] AC-02: ...

## 7. Risks & Mitigation
| RISK ID | Risk | Probability | Impact | Mitigation |
|---|---|---|---|---|
| RISK-NN-1 | ... | Med | High | ... |

## 8. Definition of Done (see §8)
- [ ] All REQs have corresponding tests and are green in CI.
- [ ] Contracts updated + `buf breaking`/`openapi-diff` pass.
- [ ] Traceability updated (`docs/specs/traceability.csv`).
- [ ] Language-specific lint/format/typecheck pass (cargo fmt/clippy, ruff, eslint/tsc).
- [ ] Every touched service exposes `/healthz`+`/readyz`+`/metrics` and declares a Compose healthcheck (MET-22).
- [ ] No secrets/PII in the diff (`gitleaks`) and no unresolved critical/high advisories (cargo-deny/cargo-audit/trivy) (MET-30/MET-31).
- [ ] Any migration is reversible and covered by a backup; restore drill documented (MET-46).
- [ ] Public items documented per `docs/engineering/CODING_CONVENTIONS.md`.
- [ ] Multi-arch (if touching `edge-lite`): buildx `linux/amd64,linux/arm64` + `cargo check --target aarch64-unknown-linux-gnu` + Pi memory within the canonical budget of `03 §7.1` (combined limit 260 MB, operating cap ≤ ~400 MB; MET-11/MET-12); `docker compose -f deploy/compose/compose.pi3.yml config` is valid and the `up -d` smoke test passes (on the Pi, or on an arm64 emulation if no Pi).
- [ ] Docs/runbook updated; ADR if there is a new decision.
- [ ] 2-person review (1 owner + 1 reviewer from another tier).

## 9. Traceability
| REQ | Code path | Test | MET | Status |
|---|---|---|---|---|
| REQ-WP-NN-001 | edge-gateway/edge-lite/services/.../src/... | tests/...::test_... | MET-01 | Done |

## 10. Changelog
| Date | Version | Change | Author |
|---|---|---|---|
```

## 4. ADR template (abridged MADR)

Copy from `tools/templ/adr.md`. Name it `docs/adr/NNNN-kebab-title.md`, number incrementally, do not edit an `Accepted` ADR (replace it with a new superseding ADR).

```markdown
# ADR-NNNN — <Decision>

## Status
Proposed | Accepted | Deprecated | Superseded by ADR-XXXX
Date: YYYY-MM-DD
Deciders: @..., @...

## Context
<Situation, constraints, forces at play. Reference related WP/REQ.>

## Decision
<Specific decision in active voice: "We will use Rust for the entire edge/cloud and Flower (Python) for FL.">

## Consequences
### Positive
- ...
### Negative / Trade-offs
- ...
### Neutral
- ...

## Alternatives considered
| Alternative | Why rejected |
|---|---|
| Spring Boot for cloud backend | excessive runtime, does not share Rust types with edge/firmware, increases operational cost |

## References
- WP-02, REQ-WP-02-003
- shared/protocols/jsonschema/ble-frame.v1.schema.json
```

The minimum ADRs that must be backfilled (`01 §2`): `ADR-0001` 3-tier architecture `[MVP]`, `ADR-0002` BLE vs MQTT `[MVP]`, `ADR-0003` unified Rust workspace (firmware/edge/cloud) `[MVP]`, `ADR-0005` PostgreSQL/TimescaleDB + `sqlx migrate` (replacing Flyway) `[MVP]`, `ADR-0006` model digital signature (Ed25519) + SHA256 `[MVP]`, `ADR-0007` split of the Pi3 edge (`edge-lite`/`edge-train`) `[MVP]`, `ADR-0008` cache policy "no shared cache" (in-process + content-addressed only) `[MVP]`, `ADR-0009` Edge runtime: Docker Compose on Pi, K3s on dev `[MVP]`, `ADR-0010` Edge service granularity: 3 coarse services (`edge-ingest`/`edge-sync`/`model-agent`, D14) `[MVP]`. `ADR-0004` Flower as the FL framework + Python only for ML = `[LATER]`.

Additional ADRs to backfill (from `06 §9`, `07 §9`, `08 §9`, trimmed per the MVP scopes): `ADR-0011` centralised observability off the Pi (Prometheus/Grafana/Alertmanager on the VPS; Pi runs only `node_exporter` ≤ 20 MB; no cAdvisor; no vmagent in MVP) `[MVP]`; `ADR-0012` **pull-scrape** default (Prometheus scrapes the Pi over the LAN/VPN), `vmagent`/`remote_write` `[LATER]` (only for Pis behind NAT) `[MVP]`; `ADR-0013` operational vs clinical alert separation (Alertmanager ops-only, ≤ 8 rules; clinical alerts never through Alertmanager) `[MVP]`; `ADR-0014` **`step-ca`** private CA run on the cloud VPS (dev uses the `step` CLI) for Pi↔cloud mTLS, 90-day leaf + renewal at day 60; the custom Rust CA tool `tools/x509-ca` is `[LATER]` and removed from the MVP `[MVP]`; `ADR-0015` self-issued JWT (15 min + rotating refresh) + argon2id + **3 MVP roles** (patient, clinician, admin; caregiver/researcher `[LATER]`) `[MVP]`; `ADR-0016` secrets split: sops+age (cloud) + root-owned 0400 Compose secrets (Pi), no plaintext ever in git `[MVP]`; `ADR-0017` single UTC-epoch-ms time model + BLE handshake anchor + chrony gate before uplink `[MVP]`; `ADR-0018` at-least-once SQLite outbox + `(device_id, window_start_ms, seq)` idempotency + tiered drops + dead-letter table `[MVP]`; `ADR-0019` pgBackRest (nightly backup + one documented restore drill) + stateless Pi `[MVP]`; `ADR-0020` MVP scope trim rules (the binding scope block above) `[MVP]`.

## 5. API Contract Conventions

### 5.1 Location & format (single source of truth = `shared/protocols/`)
| Interface | Format | Location | Codegen |
|---|---|---|---|
| Cloud REST | OpenAPI 3.1 | `shared/protocols/openapi/cloud-api.openapi.yaml` | Rust axum handlers (utoipa/openapi-generator) + TS client |
| Gateway REST | OpenAPI 3.1 | `shared/protocols/openapi/gateway-api.openapi.yaml` | Rust axum + tonic + TS |
| IoT↔Edge payload | JSON Schema 2020-12 | `shared/protocols/jsonschema/ble-frame.v1.schema.json` | Rust `no_std` (`iomt-frames`) + Python validators |
| MQTT streams | AsyncAPI 3.0 | `shared/protocols/asyncapi/mqtt-topics.asyncapi.yaml` | topic constants + validators (Rust/Python) — `[LATER]`/optional (no MVP consumer after D14; Edge↔Cloud uses REST/gRPC) |
| Internal/FL gRPC | Protobuf (proto3) | `shared/protocols/proto/iomt/<domain>/v<major>/*.proto` | Rust (prost/tonic) + Python (grpcio) |
| REST DTO | Proto or OpenAPI | as above | Rust/Python/TS |

### 5.2 Rules
1. **Contracts-first:** a PR that changes an interface must modify `shared/protocols/` **first**; code is generated from the contract.
2. **Validation mandatory in CI:** `buf lint` + `buf breaking --against .git#branch=main`; `spectral lint` for OpenAPI; `ajv` for JSON Schema; `asyncapi validate`.
3. **Valid examples:** every schema/proto must have a sample in `shared/protocols/examples/` used for testing.
4. **No breaking within a major:** only add optional fields; rename/delete ⇒ bump major (proto package `v2`, OpenAPI `/api/v2`, schema name `.v2.`).
5. **Deprecation:** mark `deprecated: true` (OpenAPI) or `[deprecated = true]` (proto), keep for at least 1 major cycle, record the milestone in CHANGELOG.
6. **Unified errors:** every REST error returns the common `Error` schema (code, message, traceId, details); every gRPC uses `google.rpc.Status`.
7. **ID/timestamp:** every payload carries `schema_version`, `device_id` (hashed) and `ts_unix_ms` (**UTC epoch milliseconds**, no local time); the idempotency key of every uplink record is `(device_id, window_start_ms, seq)`; contains no PII.
8. **Auth:** REST uses self-issued JWT (bearer, 15 min + rotating refresh, argon2id passwords); Edge→Cloud uses mTLS (TLS 1.3, project CA from `step-ca`); both are described in `securitySchemes`; see `docs/design/07-security-identity.md`.
9. **Generated-only:** `shared/rust/src/generated.rs`, `shared/python/**`, `shared/typescript/**` are auto-generated — manual edits forbidden (CI drift check). `shared/rust/src/domain.rs` is hand-written.

### 5.3 Proto contracts (implemented)

The earlier `TelemetrySample` sketch with raw `ppg`/`imu_xyz` arrays is **withdrawn**: it contradicted D18 (raw PPG/IMU never leaves the Pi). The source of truth is the proto files themselves:

- `shared/protocols/proto/iomt/telemetry/v1/telemetry.proto` — Edge→Cloud gRPC `TelemetryIngestService.UploadBatch`. `UploadBatchRequest{schema_version, sent_at_unix_ms, vitals[], windows[]}` (≤ 500 records, MET-13); `SummaryVitals` ~1 Hz per wearable; `WindowFeatures` per 30–120 s window (HR mean/min/max, SpO₂ min, HRV RMSSD, steps, dominant activity, mean signal quality, optional `Inference{model_version, anomaly_score, anomaly_flag}`). Every record carries `RecordKey(device_id, window_start_unix_ms, seq)`. `UploadBatchResponse` returns per-record `accepted[]` / `rejected[]` with `RejectReason` `INVALID_VALUE` / `TIMESTAMP_QUARANTINED` / `UNKNOWN_DEVICE` (rejected ⇒ gateway DLQ, not retried). The gateway is identified by its mTLS certificate — there is no `gateway_id` field.
- `shared/protocols/proto/iomt/firmware/v1/firmware.proto` — BLE `link_control` envelope `LinkControl{hello|report|select|ack}` (see `08 §3.2`) and the OTA manifest `FirmwareRelease` / `SignedFirmwareManifest` (Ed25519 over the serialized `release` bytes, verified before decoding; `key_id` for rotation).
- Sample records on BLE are **not** protobuf: they stay the fixed 16-byte binary record of `iot-node/firmware/crates/iomt-frames`.
- Fixtures: `shared/protocols/examples/<full.Message>[--label].txtpb`, encoded by `protoc` during `gen-contracts.sh` (rule 3 of §5.2).
- Rust: `iomt-protocol` exposes the generated types plus hand-written `convert.rs` (wire→domain always via `DeviceId::new`, `SignalQuality::new`, `validate()`) and `link.rs` (`negotiate()`, REQ-REL-029).

### 5.4 OTA artifact & model integrity (`model-agent`, `[MVP]`)

- The model artifact is **immutable per version** (`name-semver+build`); use HTTP **ETag = `sha256`** + conditional GET: when `sha256` is unchanged, the server returns **304 Not Modified** and the node **skips the download** (saving ~5 MB/retry, reducing SD writes).
- **MUST** re-verify **Ed25519 + SHA256** before applying, **regardless of** ETag/304 or cache — never trust integrity metadata from cache (ADR-0006, `04` RW-06/MVP-04).

### 5.5 Alert semantics — de-dup/debounce (`[MVP]`)

- The de-dup/debounce key **MUST** include `severity`: `(device_id, alert_type, severity)` — too coarse a key can swallow real medical alerts.
- **MUST** have **hysteresis** (avoid chatter from PPG/IMU noise) + **re-arm** when the alert condition ends.
- **Escalation** (increasing severity) **MUST** pass through de-dup (always emitted immediately).
- Every suppressed alert **MUST** be logged for traceability; the alert path does **NOT** read cache (`staleTime=0`), delivered via push (MET-09, MET-15).

### 5.6 Cross-cutting contract rules (`[MVP]`)

- **Security:** `device_id` is always the hashed form; no PII fields in any schema; every device lifecycle endpoint declares its auth (token / mTLS / JWT+role) in OpenAPI (`07 §4.2`).
- **Reliability:** uplink is at-least-once and **MUST** be idempotent on `(device_id, window_start_ms, seq)`; server-side timestamp sanity rejects out-of-range rows to quarantine (never silently drops); alerts are never dropped by any policy (`08 §4`).

## 6. Traceability scheme (spec ↔ code ↔ test ↔ metric)

### 6.1 Canonical table `docs/specs/traceability.csv`

Required columns, in this order: `req_id,type,source_doc,wp,test_id,metric`, followed by the optional tracking columns `implementation_paths,status`.

| Column | Meaning |
|---|---|
| `req_id` | requirement ID exactly as written in its source doc |
| `type` | ID family: `REQ-WP` / `REQ-OPS` / `REQ-SEC` / `REQ-REL` / `REQ-DEMO` |
| `source_doc` | path of the doc that defines the requirement (`docs/specs/wp/WP-NN-*.md`, or `docs/design/0x-*.md` while the cross-cutting registries live there) |
| `wp` | owning package: `WP-NN` for `REQ-WP` rows; `MVP-01`..`MVP-07` for cross-cutting rows, so **every** `REQ-OPS` / `REQ-SEC` / `REQ-REL` / `REQ-DEMO` maps to exactly one MVP package (`04 §7.2`) |
| `test_id` | `;`-separated test IDs covering the requirement (format per §2.2 rule 3) |
| `metric` | `;`-separated `MET-NN` from `docs/specs/NFR.md` (§7), or `-` if no metric applies |
| `implementation_paths` | optional: `;`-separated repo paths, `-` while `status=Planned` |
| `status` | `Planned` / `In progress` / `Done` |

```csv
req_id,type,source_doc,wp,test_id,metric,implementation_paths,status
REQ-WP-03-012,REQ-WP,docs/specs/wp/WP-03-edge-gateway.md,WP-03,T-REQ-WP-03-012-01,MET-08,edge-gateway/edge-lite/services/edge-ingest/src/ble/frame.rs,Done
REQ-OPS-001,REQ-OPS,docs/design/06-operations-observability.md,MVP-07,T-REQ-OPS-001-01,MET-29,deploy/compose/compose.cloud.yml,Planned
REQ-SEC-029,REQ-SEC,docs/design/07-security-identity.md,MVP-02,T-REQ-SEC-029-01,MET-30;MET-31,.github/workflows/security-scan.yml,Planned
REQ-REL-013,REQ-REL,docs/design/08-reliability-data-lifecycle.md,MVP-04,T-REQ-REL-013-01,MET-43,edge-gateway/edge-lite/services/edge-sync/src/queue,Planned
REQ-DEMO-001,REQ-DEMO,docs/design/03-spec-standard.md,MVP-04,T-REQ-DEMO-001-01,-,edge-gateway/edge-lite/services/edge-ingest/src,Planned
```

The file is created (initially as a minimal stub) by MVP-02 together with `tools/scripts/check-traceability.sh` (`04 §7.2`); it does not exist before that package lands.

### 6.2 Traceability rules
1. Each `REQ` appears on exactly 1 row; multiple tests/paths/metrics are separated by `;`.
2. Each `REQ` MUST have ≥1 `test_id`, and ≥1 `implementation_paths` once `status=Done`; a row's `wp` MUST be a real `WP-NN` or one of `MVP-01`..`MVP-07`.
3. Cross-cutting rows (`REQ-OPS`/`REQ-SEC`/`REQ-REL`/`REQ-DEMO`) MUST carry the owning MVP package in `wp`, so "≥1 test per requirement" is checkable for the whole ops/security/reliability/demo tier and not only for `REQ-WP`.
4. Test identifiers contain the `REQ` (or map 1–1 in the CSV) so CI can cross-check: `T-REQ-WP-NN-NNN-xx`, `T-REQ-OPS-NN-01`, `T-REQ-SEC-NN-xx`, `T-REQ-REL-NN-xx`, `T-REQ-DEMO-NN-01`. Rust test names are wrapped with `#[test]`/`#[tokio::test]`; Python tests use `pytest`.
5. `MET-NN` references `docs/specs/NFR.md`; experimental data is stored in `benchmarks/results/`.

### 6.3 Enforcement (CI)
- `tools/scripts/check-traceability.sh`: parse the CSV, fail if a required column is missing, a path does not exist, a test is missing, a `wp` value is not a `WP-NN`/`MVP-xx`, or a REQ declared in any requirement source (`docs/specs/wp/*`, `06 §2`, `07 §2`, `08 §2`, `03 §10`) has no row.
- The stub created by MVP-02 MUST already fail on missing rows/columns for `REQ-OPS`/`REQ-SEC`/`REQ-REL`/`REQ-DEMO` (no silent skipping, F-10).
- `ci-rust.yml`/`ci-ml.yml` run the same test suite; the coverage gate is tagged `@REQ-WP-NN-NNN`.

## 7. NFR / Metric registry (`docs/specs/NFR.md`)

All quantitative targets are centralized in one place (resolves F-007). The cross-cutting requirement *definitions* live in `06-operations-observability.md` (`REQ-OPS-*`), `07-security-identity.md` (`REQ-SEC-*`) and `08-reliability-data-lifecycle.md` (`REQ-REL-*`) (plus `REQ-DEMO-*` in `03 §10`); their §10.2 lists exactly these registry edits. Requirement IDs follow `REQ-<AREA>-NNN`; `REQ-WP-NN-NNN` stays for `docs/specs/wp/WP-NN-*.md`.

**Canonical-registry rule:** this §7 is the **only** place where a `MET-NN` target may be stated. Any metric table elsewhere in `00`–`08` (notably `06 §7`) MUST be a pointer to this section and MUST NOT restate thresholds; if a copy disagrees with this table, this table wins.

**ID allocation:** `MET-01`–`MET-17` and `MET-20`–`MET-49` are allocated as listed below. `MET-18` and `MET-19` are **intentionally reserved/unused** (a deliberate numbering gap — do not "fill" them without an ADR). `MET-42` is **reserved/unused** after being merged into `MET-13` (2026-10-09). The cloud JWKS cache metric is dropped and has no ID (no JWKS cache in the MVP, `05 §8.1`); it must not be resurrected under a new ID without an ADR.

Format:

```markdown
| MET ID | Name | Threshold | Unit | Measurement method | Measurement location | Test/bench | WP |
|---|---|---|---|---|---|---|---|
| MET-01 | IoT inference latency (p95) | < 5 | ms | on-device esp_timer | ESP32-C3 | firmware/tests/host + trace | WP-02/08 |
| MET-02 | E2E latency (p95) | < 300 | ms | ts_source→ts_sink | Edge+Cloud | benchmarks/scripts/e2e | WP-09 |
| MET-03 | EDP (IoT) | ↓ ≥30% vs baseline | mJ·ms | V·I·Δt (INA219) | IoT node | benchmarks/scripts/power | WP-08/09 |
| MET-04 | Comm cost / FL round | ↓ vs raw upload | KB | bytes up+down | FL server | benchmarks/scripts/fl | WP-07/09 |
| MET-05 | Model accuracy (motion) | ≥ 0.90 | ratio | held-out test set | Edge | experiments/ | WP-08 |
| MET-06 | FL rounds to converge | ≤ 30 | rounds | target acc reached | FL server | experiments/ | WP-07 |
| MET-07 | API latency (p95) | < 200 | ms | k6/Locust | Cloud | benchmarks/scripts/api | WP-04 |
| MET-08 | Edge uptime (lab) | ≥ 99 | % | scrape uptime | Edge (Pi, Docker Compose) | observability | WP-03 |
| MET-09 | Alert latency | < 5 | s | ts_alert→ts_display | Cloud+Mobile | benchmarks/scripts/alert | WP-04 |
| MET-10 | Sensitivity / Specificity | ≥ 0.90 / ≥ 0.90 | ratio | clinical eval | Clinical | reports/ | WP-10 |
| MET-11 | **edge-lite total RAM (Pi 3B+)** | combined (3 services + `node_exporter`) ≤ **260**; operating cap ≤ **400** | MB | Σ `mem_limit` per the canonical budget `§7.1` (services 240 + exporter 20); `docker stats --no-stream` | Pi 3B+ (Docker Compose on the Pi) | `deploy/compose/compose.pi3.yml` + `deploy/k8s/edge` | WP-03/10 |
| MET-12 | **Multi-arch image (amd64+arm64)** | both platforms present | platform | `docker buildx imagetools inspect` | CI (`ci-images.yml`) | `deploy/helm/edge` | WP-03/10 |
| MET-13 | **Pi SD write volume & SQLite commit rate** (absorbs MET-42) | SD writes ≤ **1.0 GB/day** at the D18 load (~1 Hz summary vitals + per-window rows; hard ceiling 1.5 GB/day); commit latency: batch ≤ 500 rows or fsync ≤ 500 ms; sustained fsync ≤ 0.2/s | GB/day; fsync/s; commits/s | `iostat -x` + `fatrace` + `/proc/self/io` + SQLite `PRAGMA` counters over 24 h | Pi 3B+ (`edge-sync`) | benchmarks/scripts/sd_write | WP-03 / RW-06 |
| MET-14 | **Chart query latency (p95) per range** | < 200 (enable CAGG when > 150) | ms | k6 per range 1h/24h/7d | Cloud | benchmarks/scripts/api | WP-04 / RW-07 |
| MET-15 | **Mobile API req/user + alert freshness** | ≤ 1 req/s @30 user (`staleTime` 30 s); alert p95 < 5 s | req/s; s | log gateway + push test | Cloud+Mobile | benchmarks/scripts/mobile | WP-04/09 / RW-09 |
| MET-16 | **BLE connection/notification capacity** | ≥ 30 nodes connected stably (≥3–4 adapters/batching) | connections | stress test btleplug/BlueZ + notif/s | Pi 3B+ (`edge-ingest`) | benchmarks/scripts/ble | WP-03 / RW-06 |
| MET-17 | **Timescale disk/day + CAGG freshness lag** | disk per governance; lag ≤ refresh 1–5 min | MB/day; min | pg size queries + CAGG refresh ts | Cloud | benchmarks/scripts/tsdb | WP-04 / RW-07 |
| MET-20 | **Pi observability footprint** | node_exporter limit ≤ 20 (RSS ~12, vmagent `[LATER]`); combined limits (3 edge-lite services + node_exporter) ≤ 260 within the ≤ 400 cap | MB | textfile `iomt_host_container_memory_bytes` vs `_limit_bytes`; `docker stats --no-stream` in the smoke test | Pi 3B+ | `deploy/compose/compose.pi3.yml` smoke | MVP-07 |
| MET-21 | **Edge metrics push freshness (p95)** — `[LATER]` (requires vmagent/remote_write; MVP uses Prometheus pull-scrape over the LAN/VPN) | ≤ 30 | s | `quantile_over_time(0.95, (time() - iomt_host_time_seconds)[10m:1m])` by `gateway_id` | Cloud | `deploy/observability/vmagent/` `[LATER]` | MVP-07 |
| MET-22 | **Healthcheck coverage** | 100% of services expose `/healthz`+`/readyz`+`/metrics` and declare a Compose healthcheck; smoke test green | % | repo scan + `docker compose config` + `up -d` smoke | Pi + Cloud | compose smoke | MVP-04/05/07 |
| MET-23 | **Dashboards-as-code** | 4 MVP dashboards (Pi health, edge pipeline, cloud API RED, TimescaleDB/E2E latency) provisioned from Git; 0 hand-made; the rest `[LATER]` | count | Grafana `/api/search` vs Git list in CI | Cloud | Grafana provisioning diff | MVP-07 |
| MET-24 | **Operational alert delivery (p95)** | ≤ 15 | s | `histogram_quantile(0.95, sum(rate(alertmanager_notification_latency_seconds_bucket[5m])) by (le,integration))` | Cloud | Alertmanager + `promtool` | MVP-07 |
| MET-25 | **Alert rule test coverage** | 100 (≤ 8 operational rules) | % | `promtool test rules deploy/observability/prometheus/tests/*` in CI; 1 case per rule | CI | `promtool test rules` | MVP-07 |
| MET-26 | **Log hygiene** | 0 PII/secret matches; ≤ 15 MB per container (rotation bound) | findings; MB | regex scan of an assembled `docker logs` corpus + `docker inspect .LogPath` size | Pi + Cloud | log PII scan | MVP-07 |
| MET-27 | **Series cardinality** | ≤ 800 series/Pi; ≤ 8 000 active total; 0 unexpected relabel drops | series | `count by (gateway_id)({__name__=~"iomt_.+"})`, `prometheus_tsdb_head_series` | Cloud | Prometheus queries | MVP-07 |
| MET-28 | **SLO reporting** — `[LATER]` | 30-day availability SLI for edge (≥ 99%, ties MET-08) and cloud API (≥ 99.5%); burn-rate alert tested | % | recording rules + SLO dashboard + `promtool` alert test | Cloud | SLO rules `[LATER]` | MVP-07 |
| MET-29 | **Observability cloud footprint** | Prometheus+Grafana+Alertmanager RSS ≤ 400; Prometheus 15 d disk ≤ 500 | MB | `process_resident_memory_bytes`, `prometheus_tsdb_storage_blocks_bytes` | Cloud VPS | `docker stats` | MVP-07 |
| MET-30 | **Secret exposure** | 0 secrets in git; 0 unallowlisted gitleaks findings | findings | `gitleaks detect` in pre-commit + `security-scan.yml` on every PR | CI + repo | `gitleaks detect` | MVP-02 |
| MET-31 | **Dependency/image advisories** | 0 RustSec critical/high open; 0 CRITICAL trivy findings; HIGH triaged with linked issue ≤ 7 d | findings | `cargo audit`, `cargo deny`, `trivy image` in CI | CI | `security-scan.yml` | MVP-02 |
| MET-32 | **argon2id verify cost (p95)** | ≤ 100 | ms | 100 verifies timed (m=19456 KiB, t=2, p=1) on the 2-vCPU VPS | Cloud | `docs/runbooks/security-check.md` | MVP-05 |
| MET-33 | **Brute-force resistance** | 1 000 login attempts/min from 1 IP → ≤ 5 hash computations, rest 429 | attempts → hashes | scripted burst against `/auth/login`; count hash invocations in logs | Cloud | `docs/runbooks/security-check.md` | MVP-05 |
| MET-34 | **Audit completeness** | 100 | % | 200 scripted mixed requests (reads, denials, auth, admin) → expected vs actual audit rows | Cloud | audit test-suite | MVP-05 |
| MET-35 | **Transport security** | mTLS listener rejects 100% of cert-less connections; 0 TLS < 1.2; 0 weak ciphers | % / findings | `openssl s_client` + `testssl.sh` sweep of :443 and :9443 | Cloud | `docs/runbooks/security-check.md` | MVP-05 |
| MET-36 | **Certificate lifecycle + revocation** | leaf ≤ 90 d, renewal at ≤ 60 d with 0 outages over 90 d; revoke effective ≤ 60 s | d / s | clock-forward renewal test + revoke→next-request timer | Cloud + Pi (`step-ca`) | `step` CLI + renewal test | MVP-04/05 |
| MET-37 | **Auth + rate-limit overhead** | ≤ 10 ms added p95; total API p95 still < 200 ms (MET-07) | ms | k6 scenario with JWT validation + token bucket vs baseline | Cloud | k6 | MVP-05 |
| MET-38 | **Artifact integrity** | 100% of applies verify Ed25519+SHA256; 0 unsigned/tampered/downgraded artifacts accepted | % | negative tests: mutated bytes, absent signature, older version | Pi (`model-agent`) | `model-agent` tests | MVP-04 |
| MET-39 | **Pi hardening baseline** | 100% checklist pass (ssh keys, ufw, unattended-upgrades); LAN scan exposes only 22/tcp | % / open ports | runbook checklist + `nmap -Pn` from the admin host | Pi 3B+ | `docs/runbooks/pi-hardening.md` | MVP-07 |
| MET-40 | **Device→Cloud timestamp accuracy** | p95 ≤ 200 ms, max ≤ 500 ms per 10-min session | ms | injected UTC reference at handshake; compare stored `ts_unix_ms` vs Pi chrony reference over 24 h | Pi + Cloud | chrony + handshake test | MVP-04 |
| MET-41 | **Queue durability & drain** | kill -9 loss ≤ 500 ms of vitals, 0 alerts; drain ≥ 1 000 rec/s after outage | rec/s, ms | 100 randomized `kill -9` + 30-min `iptables DROP` then timed catch-up | Pi (`edge-sync`) | outage test | MVP-04 |
| MET-42 | **Reserved/unused** — merged into **MET-13** (CR-24/F-12, 2026-10-09); do not re-measure SD writes under this ID | — | — | — | — | — | — |
| MET-43 | **Queue cap/age compliance** | caps never exceeded; `iomt_sync_queue_dropped_total` = 0 (vital/window) in 30 d at demo load | rows, MB, h | gauges `iomt_sync_queue_rows/bytes/oldest_seconds` + drop counter | Pi | Prometheus queries | MVP-04 |
| MET-44 | **Cloud storage @30 nodes/180 d** | DB ≤ 18 GB; repo1 ≤ 12 GB; disk used ≤ 34/40 GB | GB | `pg_database_size` + `hypertable_size()` + `df` weekly | Cloud | TimescaleDB queries | MVP-05 |
| MET-45 | **CAGG freshness & maintenance jobs** | lag ≤ 2× refresh interval; compression/retention jobs 100% success/day | min; % | `_timescaledb_*` job stats + `chunk_compression_stats` | Cloud | TimescaleDB queries | MVP-05 |
| MET-46 | **Backup RPO/RTO & drill** | RPO ≤ 10 min; RTO ≤ 60 min; nightly pgBackRest backup; ≥ 1 documented restore drill | min; count | `pgbackrest info` timestamps + `restore-drill.sh` exit code | Cloud | `docs/runbooks/backup-restore.md` | MVP-07 |
| MET-47 | **Firmware OTA success & rollback** — `[LATER]` (MVP flashes by USB; partition table is A/B-ready) | ≥ 95% first-attempt success; 100% bad-image rollback ≤ 1 boot; ≤ 180 s/node | %; s | OTA counters + injected tampered/crash-loop image | Pi + IoT | OTA test `[LATER]` | MVP-03/04 |
| MET-48 | **Push latency & success** | alert→display p95 < 5 s; FCM success ≥ 99%/24 h; quiet-hours rules `[LATER]` | s; % | synthetic alert every 15 min + push log | Cloud + Mobile | push test | MVP-05/06 |
| MET-49 | **Rollout safety** — `[LATER]` | canary ≥ 24 h soak with 0 crashes/OOM before fleet; ≤ 25% fleet per stage | h; % | rollout log + crash counters | Cloud + Pi | rollout log `[LATER]` | MVP-07 |
```

**MVP-trim note (ADR-0020):** rows tagged `[LATER]` (MET-21 push freshness, MET-28 SLO reporting, MET-47 firmware OTA over BLE, MET-49 canary rollout) are explicitly deferred; the MVP runs **pull-scrape**, **4 dashboards**, **≤ 8 Alertmanager ops rules**, **USB firmware flashing** and a **manual rollout**, per the binding MVP scopes in `06 §6`, `07 §6`, `08 §6`. Clinical alerts never traverse Alertmanager (ADR-0013).

- **MET-13 note:** the old `~17 MB/day` figure described the logical payload of the 1-row/window model and is **stale**; at the binding D18 granularity (~1 Hz summary vitals + per-window rows) the on-disk floor is `~12 KB/s ≈ 1.0 GB/day` (SQLite page/commit floor) — see `08 §5.1`. The SD-wear conclusion is unchanged. **MET-42 was merged into MET-13 on 2026-10-09**: MET-13 is now the single SD-write metric (volume + commit/fsync rate), and MET-42 is reserved/unused; anything that previously measured MET-42 measures MET-13 instead.

Any threshold change must go through a PR + be recorded in ADR/CHANGELOG (not edited piecemeal within a WP).

### 7.1 `edge-lite` resource budget on Pi 3B+ (1 GB RAM) — [MVP] — **canonical (single source of truth)**

**This section is THE canonical Pi memory budget.** `01 §2.1`, `04 §7`, `06 §5.1` and every other doc MUST reference `03 §7.1` instead of repeating the arithmetic; where a copy elsewhere disagrees with the table below, this table wins. The `≤ ~400 MB` figure is an operating **cap**, not a total.

Hard constraint: Pi OS Lite 64-bit ~150–200 MB + Docker Engine daemon ~30–50 MB (no K3s agent/containerd on the Pi in the MVP, D13) ⇒ **operating cap ≤ ~400 MB for all containers on the Pi** (MET-11), with the extra headroom kept as safety margin. Every `edge-lite` service **MUST** declare Compose `mem_limit`/`cpus` (and K8s `resources.requests`/`resources.limits` for the dev-validation manifests):

| Service (`edge-gateway/edge-lite/services/…`) | RAM request | RAM limit | CPU request | CPU limit | Notes |
|---|---|---|---|---|---|
| `edge-ingest` | 40 MB | 112 MB | 150 m | 600 m | ble-ingest + signal-processor merged (D14); btleplug/BlueZ via D-Bus + ONNX int8 via `tract`/`ort`; `hostNetwork` |
| `edge-sync` | 32 MB | 64 MB | 100 m | 300 m | buffer-store + gateway-api merged (D14); SQLite (`rusqlite`) / `sled` — **NO Postgres**; axum + tonic uplink |
| `model-agent` | 32 MB | 64 MB | 50 m | 250 m | `tract`/`ort` + ed25519-dalek verify/swap/rollback |
| **Subtotal — 3 `edge-lite` services (services-only)** | **104 MB** | **240 MB** | **300 m** | **1150 m** | 3 coarse services, split when needed (D14) |
| `node_exporter` (Pi observability, ADR-0011/0012) | 8 MB | 20 MB | — | — | the **only** other process on the Pi; RSS ~12 MB |
| **Total — services + `node_exporter` (combined)** | **112 MB** | **260 MB** | — | — | **use these two figures everywhere**; inside the ≤ ~400 MB cap |

- **Canonical combined figures:** request **112 MB** (104 + 8) / limit **260 MB** (240 + 20). Never recompute or relabel them elsewhere: "Total (services-only)" = 104/240, "Total (combined)" = 112/260, "cap" = ~400 MB — three different things.
- **Pi observability (MVP, ADR-0011/0012):** only `node_exporter` runs on the Pi (request 8 MB / limit 20 MB / RSS ~12 MB) and Prometheus **pulls** it over the LAN/VPN; `vmagent`/`remote_write` is `[LATER]` (only for Pis behind NAT), so no second monitoring container exists on the Pi in the MVP. Total device stays under 1 GB.
- **Forbidden on Pi:** Postgres, Redis server (use SQLite/sled + in-process queue), PyTorch, `edge-train`, `local-trainer`, `cloud/*`.
- **In-RAM ring buffer (`edge-sync`):** holds 60–120 s of raw data for windowing/alert, RAM **≤10 MB** — within the `edge-sync` 64 MB limit (`01 §2.1`). The alert path MUST read the `edge-ingest` live ring buffer, **NOT** the `edge-sync` SQLite.
- **WAL / batched commit (default):** `edge-sync` MUST enable `PRAGMA journal_mode=WAL` + group commits (batch ≤500 row or fsync ≤500 ms) to reduce SD wear; measured by MET-13.
- **Redis: forbidden on Pi** — there is no shared cache/Redis at any tier in the MVP (ADR-0008, `01 §2.1.1`).
- **CI enforcement:** `check-deps.sh` confirms `edge-lite` crates do not pull heavy dependencies; `ci-images.yml` buildx both `linux/amd64,linux/arm64`; `ci-rust.yml` `cargo check --target aarch64-unknown-linux-gnu`; `docker compose -f deploy/compose/compose.pi3.yml config` validates and the `up -d` smoke test passes (Pi or arm64 emulation).

### 7.2 Resolved decisions (D18, D19 — no open decisions)

- **Upload granularity = D18:** per-node summary vitals at ~1 Hz (HR, SpO2, activity, signal quality) for real-time display, plus one feature row per 30–120 s window for analytics/FL; raw samples stay local on the edge. Consequence: TimescaleDB hypertables + compression + retention + **continuous aggregates (1 min / 1 h / 1 day) are `[MVP]`**, not `[LATER]`. Design in `08 §3.5`; measured by MET-14 / MET-17 / MET-44 / MET-45. (The old CAGG trigger rule is superseded: CAGG is always on in the MVP.)
- **Pi storage = D19:** high-endurance microSD with write minimisation (WAL batching, volatile logs, `noatime`, log rotation); the storage path is configurable and SSD/USB is the `[LATER]` clinical-phase lever (no code change). Measured by MET-13 (MET-42 is reserved/unused — merged into MET-13).

No open decisions remain for this phase (D20).

### 7.3 Cloud demo footprint (VPS) — canonical

**Demo topology (A1):** one cloud VPS, **2 vCPU / 4 GB / 40 GB SSD**, running the demo stack as containers. The container list and the RSS arithmetic below are canonical for the MVP demo; any VPS sizing statement elsewhere MUST include `step-ca` and MUST match these numbers (the pre-fix `07 §5.2` line "no step-ca" is superseded by this section).

| Container / job (cloud demo) | RSS budget | Notes |
|---|---|---|
| `timescaledb` (Postgres + TimescaleDB) | 1 200 MB | `shared_buffers` 512 MB |
| `cloud-backend` (axum + sqlx) | 150 MB | incl. rate-limiter state |
| `caddy` (ingress, automatic TLS) | 50 MB | `api.` / `grafana.` vhosts |
| `prometheus` + `grafana` + `alertmanager` | ≤ 400 MB | MET-29 |
| **`step-ca`** (private CA, ADR-0014) | 50 MB | **MVP — part of the demo footprint**; issues Pi↔cloud mTLS leaves |
| **Σ container RSS** | **≈ 1.85 GB** | on 4 GB ⇒ **≥ ~2.1 GB headroom** |

- **Hard bound:** Σ container RSS MUST stay ≤ **2.5 GB**, leaving ≥ ~1.5 GB for the OS, Docker daemon and page cache.
- **Backups:** for the demo, the `pgBackRest` job runs **sequentially** (never concurrent with the demo/load profile) and is a periodic job, not a resident container; RPO/RTO per MET-46.
- **Not in the demo footprint (all `[LATER]`):** Redis/IdP, `vmagent`/remote_write, Loki, OpenTelemetry, HA/second node, horizontal scaling. **Production sizing is `[LATER]`** — this budget covers the 5–30 node demo only.

## 8. Definition of Done (applies to every WP)

- [ ] Every `REQ` has Given/When/Then acceptance criteria and a green automated test.
- [ ] `shared/protocols/` updated; lint + breaking-check pass.
- [ ] `docs/specs/traceability.csv` updated and `check-traceability.sh` passes.
- [ ] **Rust:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (host), `cargo deny check` pass.
- [ ] **Rust (Pi/aarch64):** every `edge-lite` crate `cargo check --target aarch64-unknown-linux-gnu -p edge-ingest -p edge-sync -p model-agent` is green.
- [ ] **Firmware (Rust):** `cargo build --manifest-path iot-node/firmware/Cargo.toml --target riscv32imc-esp-espidf` + host tests `cargo test -p snn-lif -p iomt-frames`.
- [ ] **Python:** `ruff check .` + `pytest` (in `edge-gateway/edge-train/ml`, `edge-gateway/edge-train/services/local-trainer`, `cloud/fl-server`, `benchmarks`) pass.
- [ ] **Mobile (TS):** `npx tsc --noEmit` + `eslint` + `jest` pass.
- [ ] **Multi-arch/CI:** `ci-images.yml` buildx `linux/amd64,linux/arm64` for every `edge-lite` service; the image has both platforms (`docker buildx imagetools inspect`, MET-12).
- [ ] **Edge deploy (Pi):** `docker compose -f deploy/compose/compose.pi3.yml config` is valid and the `up -d` smoke test passes on the Pi (or on an arm64 emulation if no Pi is available).
- [ ] **Pi budget:** every `edge-lite` service declares Compose `mem_limit`/`cpus` (and K8s `resources.requests/limits` for the dev-validation manifests); memory within the canonical budget of `§7.1` — 3 services 104/240 MB, `node_exporter` 8/20 MB, combined **112 MB request / 260 MB limit**, operating cap ≤ ~400 MB (MET-11); does not pull Postgres/PyTorch (only `cloud/*` may use Postgres).
- [ ] For `LATER` WPs: not required in the E2E demo; if touching `edge-lite` must still meet the Pi budget.
- [ ] Does not violate the dependency rules of `01 §4` (`check-deps.sh` passes).
- [ ] **Health/metrics:** every touched service exposes `/healthz` (liveness) + `/readyz` (readiness) + `/metrics` and declares a Compose healthcheck (MET-22); new alert rules carry `domain="ops"`, a `for:`, and a `promtool test rules` case (MET-25); clinical alerts never appear in an Alertmanager rule or route (`ADR-0013`); dashboards are changed only via Git provisioning (MET-23).
- [ ] **Secrets/supply chain:** no secrets/PII in code/diff (`gitleaks` passes); `cargo deny check`, `cargo audit` and `trivy` are clean (MET-30 / MET-31); Pi-affecting changes re-run the hardening checklist (MET-39).
- [ ] **Backup/data:** any migration is reversible and covered by a backup; the restore drill (`restore-drill.sh`) is documented and green (MET-46); uplink/hypertable changes re-run the outage test (MET-41); security/privacy changes touching auth/audit/device/consent pass the RBAC table test, the audit-completeness test and revoke ≤ 60 s (MET-34 / MET-36).
- [ ] **Docs:** public items are documented per `docs/engineering/CODING_CONVENTIONS.md`; related docs/runbook/ADR are updated.
- [ ] 2 reviewers (1 owner + 1 from another tier) approve; all comments addressed.
- [ ] Artifact (model/image) has a version + checksum/signature if released.

## 9. Versioning (repo-wide synchronization)

| Object | Scheme | Notes |
|---|---|---|
| Rust workspace | `[workspace.package] version` | common version; bump per Conventional Commits (`feat`→minor, `fix`→patch, breaking→major) |
| Crate publish | SemVer `MAJOR.MINOR.PATCH` | only `iomt-protocol` publishes; services use `path`/`version.workspace` |
| Python package | SemVer in `pyproject.toml` | `edge_ml`, `local_trainer`, `fl_server`, `iomt_protocols` |
| API/proto/schema | Major in the name | `/api/v1`, `iomt.xxx.v1`, `*.v1.schema.json` |
| DB migration | `sqlx migrate` `<UTC timestamp>_<desc>.sql` (for example `20260109120000_create_hypertables.sql`) | one-way, do not edit a released migration |
| Model | `name-semver+build` + SHA256 | `snn-motion-1.2.0+ab12cd34`, signed Ed25519 |
| Firmware/OTA | `fw-<target>-semver` | rollback if health-check fails |
| Contract bundle | `shared/protocols/VERSION` | increased on a breaking batch |

## 10. Cross-cutting MVP requirement addenda (`REQ-DEMO-*`, added 2026-10-09)

Added by the consistency/feasibility reviews (`09`, `10`) inside the ADR-0020 scope. These are **MVP** requirements; they use the new `REQ-DEMO-NN` family (§2.2 rule 2), get `T-REQ-DEMO-NN-01` tests (§2.2 rule 3) and are registered in `docs/specs/traceability.csv` with `type=REQ-DEMO` and the owning MVP package in `wp` (§6.1).

| ID | Requirement (RFC 2119) | Test | MVP package | Source |
|---|---|---|---|---|
| REQ-DEMO-001 | `edge-ingest` MUST provide a replay mode `--replay <fixture>` that reads recorded `iotm-frames` fixtures from `iot-node/firmware/test/fixtures/` and drives the full ingest → window → outbox → uplink pipeline **with no BLE hardware and no live node**, so CI and developers can run the pipeline anywhere. A replay MUST be idempotent (re-running it adds no duplicate `(device_id, window_start_ms, seq)` rows). | T-REQ-DEMO-001-01: `edge-ingest --replay iot-node/firmware/test/fixtures/<name>.jsonl` in `compose.dev`/CI ⇒ rows land in the `edge-sync` outbox and uplink to the cloud succeeds; a second run inserts 0 new rows | MVP-04 (fixture captured with MVP-03) | `10` F-08  Status 2026-10-12: not built yet; a live simulator `tools/device-sim` is planned (08 §3.2b.8) and could record fixtures. |
| REQ-DEMO-002 | The repo MUST ship `tools/scripts/seed-dev.sh`, which seeds a demo dataset (users covering the 3 MVP roles, 1–5 registered devices, ~24 h of vitals + windows, one alert) so the cloud demo always starts non-empty; it MUST be re-runnable and MUST NOT touch production configuration. | T-REQ-DEMO-002-01: empty DB ⇒ script exits 0 ⇒ login as each MVP role, latest-vitals chart and one alert render | MVP-07 (demo prerequisite) | `10` F-09 |
| REQ-DEMO-003 | Local development and the demo MUST obtain TLS from the project private CA (`step` CLI in dev, `step-ca` on the VPS, ADR-0014) **or** from a documented public dev domain served by Caddy with automatic ACME TLS; ad-hoc self-signed certificates MUST NOT be used and MUST NOT be committed. The trust anchor (`deploy/security/ca/root.pem`) and the dev flow MUST be documented. | T-REQ-DEMO-003-01: `curl --cacert deploy/security/ca/root.pem https://<dev-host>/healthz` succeeds and the trust-store import steps are reproducible from the docs | MVP-07 | `10` F-09 |
| REQ-DEMO-004 | Before FCM push is enabled, MVP-06 MUST document a dedicated Firebase **dev** project (project id, per-environment `google-services.json`/APNs config kept out of git or injected per env, emulator fallback) and the FCM push dry-run MUST target that dev project. | T-REQ-DEMO-004-01: runbook exists and the push dry-run receipt/logs show the dev project id | MVP-06 | `10` F-09 |

Scope note: process-level additions from the same reviews — the time-boxed spikes wave **M0.5**, the **hardware-in-the-loop** milestone, the effort model/critical path and the minimum viable E2E slice sprint plan — are planning artefacts and live in `04-work-split.md` §7, not here.
