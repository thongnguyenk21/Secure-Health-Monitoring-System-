# 01 — Target Tree, Naming Conventions & Dependency Rules

This document defines the **target directory tree** for a production-grade monorepo, naming conventions, and the **dependency rules between modules** (who may import whom). It is the source of truth for `02` (migration) and `04` (work breakdown).

Finalized stack (constraint — see `00 §0.0`):
- **Rust**: Cargo workspace at the repo root covering the ESP32-C3 firmware, **all** edge services, and the cloud backend. Java/Spring/Maven/Flyway **have been removed**.
- **Python** (uv/pyproject): ML only — `edge-gateway/edge-train/ml`, `edge-train/services/local-trainer` (Flower client), `cloud/fl-server` (Flower), `benchmarks`, `experiments`. **Does not run on Pi.**
- **TypeScript**: React Native mobile.
- **`shared/protocols`** (Protobuf + OpenAPI + JSON Schema/AsyncAPI) is the inter-language source of truth; codegen → Rust (prost/tonic), TypeScript, Python. There is also a **shared Rust crate for domain types**.

### 0. Target runtime — Raspberry Pi 3B+ (hard constraint)

| Attribute | Value | Design consequence |
|---|---|---|
| Board | **Raspberry Pi 3B+** | — |
| CPU | Cortex-A53 (ARMv8-A) 4×1.4 GHz | build/run native **aarch64** |
| RAM | **1 GB** (usable ~800–900 MB) | Postgres/JVM forbidden on Pi; edge-lite budget ≤ **~400 MB** |
| OS | Raspberry Pi OS **Lite 64-bit** | `aarch64-unknown-linux-gnu`, headless |
| Dev machine | x86_64 Linux | Docker + Compose for builds; **K3s server (dev only)** validates the K8s/Helm manifests |
| Primary transport Pi↔node | BLE 5.0 GATT via D-Bus/BlueZ | container needs `hostNetwork` + mount `/var/run/dbus` |

- **Edge splits into 2 runtime tiers:** `edge-lite` (runs **on Pi 3B+**: **3 coarse services** — `edge-ingest`, `edge-sync`, `model-agent`) and `edge-train` (local-trainer + heavy ML; runs on a **more powerful machine, does NOT run on Pi**). Mapping (D14): `edge-ingest` = BLE intake + windowing/normalisation + ONNX inference; `edge-sync` = SQLite store-and-forward queue + up/download + REST/gRPC to Cloud; `model-agent` unchanged.
- **`edge-sync` queue uses SQLite (`rusqlite`) or `sled`** — **NOT Postgres** on Pi (Postgres only in Cloud).
- **Inference on Pi** with a small quantized ONNX model via **`tract`** (pure Rust) or **`ort`**.
- **Edge deployment model (DECIDED, D13):** the Pi 3B+ runs **Docker Engine + Docker Compose** (`deploy/compose/compose.pi3.yml`) as the **PRIMARY** edge deployment — **no K3s on the Pi in the MVP**. A **K3s server runs on the dev machine only**, to validate the `deploy/k8s` and `deploy/helm` manifests (WP3 deliverables stay). A **K3s agent on the Pi is `[LATER]`** (only when 3+ Pis share a LAN, or if the supervisor requires K3s on the device).
- **Multi-arch required:** `docker buildx build --platform linux/amd64,linux/arm64`; Rust cross-compile `aarch64-unknown-linux-gnu` with `cross` or `cargo-zigbuild`; CI builds **both arch** for every image.

Owner/tier notation:
- `[T0/IoT]` IoT tier · `[T1/Edge]` Edge tier · `[T2/Cloud]` Cloud tier · `[T3/Mobile]` Mobile tier
- `[ARCH]` architecture/contracts · `[PLAT]` platform/DevOps · `[DOCS]` documentation · `[RESEARCH]` research · `[EVAL]` evaluation

Scope notation (**required on every tree entry**):
- `[MVP]` — Phase 1: needed for the **E2E IoT→Edge→Cloud→Mobile demo** on Pi 3B+.
- `[LATER]` — Phase 2+: advanced research/operations, **does not block the demo**.

> **MVP scope trim (binding — recorded as ADR-0020).** Docs `06`/`07`/`08` describe a production target; this tree
> applies the final **student-MVP trim** and tags everything outside it `[LATER]`.
> - **Security MVP:** BLE LE Secure Connections + bonding; Pi↔cloud mTLS with certificates from **step-ca** run on the
>   cloud VPS (dev uses the `step` CLI) — the custom Rust `tools/x509-ca` tool is **`[LATER]`**; secrets with sops+age;
>   user auth JWT + argon2id; RBAC with **3 MVP roles** (patient, clinician, admin; caregiver/researcher `[LATER]`);
>   append-only audit log table; API rate limiting; consent record table + **manual** export/deletion runbook
>   (self-service endpoints `[LATER]`); Ed25519-signed models; cargo-deny + cargo-audit + trivy + gitleaks in CI;
>   Pi hardening checklist (ssh keys, ufw, unattended upgrades). `[LATER]`: secure boot/flash encryption, cosign, SBOM,
>   secure aggregation/DP, read-only rootfs, hardware-backed keys.
> - **Observability MVP:** Prometheus **scrapes the Pi over the LAN/VPN** (default); `vmagent`/remote_write on the Pi is
>   **`[LATER]`** (NAT'd Pis only) ⇒ the Pi runs **only `node_exporter` (≤ 20 MB)**. Grafana MVP = **4** dashboards
>   (Pi health, edge pipeline, cloud API RED, TimescaleDB/E2E latency); the rest `[LATER]`. Alertmanager with **≤ 8**
>   operational rules. `/healthz` + `/readyz` + `/metrics` on every service, Docker `json-file` log rotation, Caddy
>   ingress with automatic TLS. Loki and OpenTelemetry `[LATER]`. **Clinical alerts never go through Alertmanager.**
> - **Reliability MVP:** chrony on the Pi, the doc-08 time model, idempotent at-least-once edge-sync queue with a
>   dead-letter table, Timescale hypertable + compression + retention + continuous aggregates (D18), pgBackRest nightly
>   + one documented restore drill, model OTA with sha256+Ed25519 and rollback, FCM clinical push with dedup/hysteresis.
>   `[LATER]`: **ESP32 firmware OTA delivery over BLE** (MVP flashes by USB, but the **partition table is A/B-ready**),
>   canary rollout, quiet-hours rules, SMS/e-mail fallback, RTC module.
> - **Pi RAM:** the 3 edge-lite services (104/240 MB request/limit, services-only) **+ `node_exporter` (8/20 MB)** give a
>   combined edge-lite total of **112/260 MB** (no vmagent) under the ~400 MB operating cap and the 1 GB device total;
>   **canonical budget table in `03 §7.1`** (do not re-derive the arithmetic elsewhere).
> - Decisions **D15–D19 are resolved**; no open-decision tags remain below. Metric ranges MET-20..29 (ops), MET-30..39
>   (security), MET-40..49 (reliability); ADRs ADR-0011..0019 + ADR-0020 (this block).

## 1. Design principles (design rules)

1. **1 project = 1 build root.** Each buildable crate/service has exactly one manifest (`Cargo.toml`, `pyproject.toml`, `package.json`) at the **top of the module directory**. Do not nest a same-named subproject (drop `firmware/firmware-esp32c3/`). Firmware has its own **nested Cargo workspace** because the RISC-V/no_std target cannot build on the host. Edge splits into 2 subprojects `edge-lite/` (Pi) and `edge-train/` (powerful machine).
2. **Contracts-first.** Every interface between 2 processes/tiers lives in `shared/protocols/` in a machine-readable form; `shared/{rust,python,typescript}` contains only **auto-generated code** from `shared/protocols/`.
3. **Tier isolation.** No tier imports the source of another tier. Communication only via `shared/protocols` (network/message), not via source-level import.
4. **Service = independent crate.** Each microservice has its own `Cargo.toml`, `src/`, `tests/`, `Dockerfile`; communication only via message queue/contracts. Exception: `local-trainer` is Python (Flower) and belongs to `edge-train`.
5. **Deploy separated from code — ONE single home.** **All** deployment artifacts live in the root `deploy/` (compose/k8s/helm/observability/security), split into **per-tier subfolders** (`deploy/k8s/edge|cloud`, `deploy/helm/edge|cloud`). **Forbidden** `edge-gateway/deploy/**` or any nested `deploy/` inside a tier; each service keeps only its own `Dockerfile`.
6. **Docs are code.** `docs/` is flat by domain, has an index, ADR mandatory for every irreversible decision.
7. **Reproducibility required.** Pin the toolchain (`rust-toolchain.toml`, `.python-version`, `.nvmrc`, `sdkconfig.defaults`), pin images by digest; commit `Cargo.lock`/`uv.lock`; `cargo-deny` for license/advisory. Multi-arch: pin target `aarch64-unknown-linux-gnu` + `linux/amd64,linux/arm64`.
8. **Pi-first budget.** Every `edge-lite` service declares Compose `mem_limit`/`cpus` (and K8s `resources.requests/limits` for the dev-validation manifests); total `edge-lite` budget ≤ ~400 MB (canonical table in `03 §7.1`, mirrored in §2.1). The heavy parts (training, PyTorch) do **not** live on the Pi.
10. **Service granularity is provisional (D14).** `edge-lite` starts with **3 coarse services** (`edge-ingest`, `edge-sync`, `model-agent`) instead of 5. Inside each service keep clear Rust modules (`edge-ingest`: `ble`, `window`, `inference`; `edge-sync`: `queue`, `uplink`, `api`) so a later split into separate crates/containers is cheap — **"split when needed, not before"**. The final number of services is intentionally open.

> **Workspace policy:** The Cargo workspace at the repo root contains the host-buildable crates (shared, edge-lite, edge-train*, cloud). `iot-node/firmware` is `exclude`d from the root workspace and has its own workspace; therefore `cargo check --workspace` at the root is a **host-buildable** command and must always be green. In addition, every `edge-lite` crate must be **`cargo check --target aarch64-unknown-linux-gnu`**-able (Pi cross-compile).
> (*) `edge-train` has a Python part; the Rust part (if any) is still a host member.

## 2. Target directory tree (full)

```text
project-personal/                                  # repo root
├── Cargo.toml                                     # [MVP] [ARCH] Rust workspace (host crates) + [workspace.dependencies]
├── Cargo.lock                                     # [MVP] committed (application workspace)
├── rust-toolchain.toml                            # [MVP] pin channel + components + targets (riscv32imc-*, aarch64-unknown-linux-gnu)
├── rustfmt.toml                                    # [MVP]
├── clippy.toml                                     # [MVP]
├── deny.toml                                      # [MVP] cargo-deny: licenses, advisories, bans, sources
├── .gitleaks.toml                                 # [MVP] secret scan; allowlist = dev .env.example pattern only
├── pyproject.toml                                 # [MVP] uv workspace root (ML — does not run on Pi); MVP-02 root deliverable
├── uv.lock                                        # [MVP] committed
├── .python-version                                # [MVP] pin Python for the uv workspace root
├── .nvmrc / .tool-versions                        # [MVP] pin Node (mobile)
├── .env.example                                   # [MVP] repo-level environment variable template
├── Makefile                                       # [MVP] entrypoint that calls tools/Makefile
├── README.md, LICENSE, CHANGELOG.md, CODEOWNERS, CONTRIBUTING.md, SECURITY.md   # [MVP]
│
├── .github/                                       # [MVP] [PLAT] CI/CD & community
│   ├── workflows/
│   │   ├── ci-contracts.yml                        # [MVP] buf lint/breaking + codegen drift (Rust/TS/Python)
│   │   ├── ci-rust.yml                             # [MVP] fmt/clippy/test host + `cargo check --target aarch64-unknown-linux-gnu`
│   │   ├── ci-firmware.yml                         # [MVP] cargo check/build target riscv32imc-esp-espidf (+ host tests)
│   │   ├── ci-images.yml                           # [MVP] docker buildx linux/amd64 + linux/arm64 (edge-lite)
│   │   ├── ci-ml.yml                               # [LATER] ruff + pytest (edge-train ml, local-trainer, fl-server, benchmarks)
│   │   ├── ci-mobile.yml                           # [MVP] eslint + tsc --noEmit + jest
│   │   ├── security-scan.yml                       # [MVP] gitleaks + trivy + cargo-audit  ·  [LATER] SBOM (syft)
│   │   └── ota-release.yml                         # [MVP] build+sign+push model/firmware/image (multi-arch)
│   ├── ISSUE_TEMPLATE/{bug_report.md,feature_request.md,wp_task.md}   # [MVP]
│   ├── PULL_REQUEST_TEMPLATE.md                    # [MVP] requires REQ/WP link + DoD checklist
│   └── dependabot.yml                              # [MVP]
│
├── shared/                                        # [MVP] [ARCH] contracts + bindings
│   ├── protocols/                                  # machine-readable source of truth (leaf, imports nothing)
│   │   ├── README.md                               # [MVP] versioning & codegen conventions (see 03)
│   │   ├── buf.yaml                                # [MVP] buf lint/breaking
│   │   ├── buf.gen.yaml                            # [MVP] generates Rust (prost/tonic)/Python/TS
│   │   ├── VERSION                                 # [MVP] major interface version of the contract bundle
│   │   ├── proto/iomt/
│   │   │   ├── telemetry/v1/telemetry.proto         # [MVP] IoT->Edge, Edge->Cloud samples/windows; key (device_id, window_start_ms, seq)
│   │   │   ├── model/v1/model.proto                 # [MVP] model metadata, weights, OTA
│   │   │   ├── firmware/v1/firmware.proto           # [MVP] OTA manifest (sha256, version, board, Ed25519 signature)
│   │   │   ├── federation/v1/federation.proto       # [LATER] FL round, client update, aggregation
│   │   │   └── alert/v1/alert.proto                 # [MVP] alert event Cloud->Mobile
│   │   ├── openapi/
│   │   │   ├── cloud-api.openapi.yaml               # [MVP] REST Cloud backend (canonical)
│   │   │   └── gateway-api.openapi.yaml             # [MVP] Edge gateway REST
│   │   ├── jsonschema/
│   │   │   ├── ble-frame.v1.schema.json             # [MVP]
│   │   │   ├── telemetry.sample.v1.schema.json      # [MVP]
│   │   │   ├── model-card.v1.schema.json            # [MVP]
│   │   │   ├── alert-event.v1.schema.json           # [MVP]
│   │   │   └── push-registration.v1.schema.json     # [MVP] FCM token registration body
│   │   ├── asyncapi/mqtt-topics.asyncapi.yaml       # [LATER] MQTT topics (optional) — no MVP consumer after D14; edge-sync uses REST/gRPC to Cloud
│   │   └── examples/                                # [MVP] valid fixtures for CI contract test
│   ├── rust/                                       # [MVP] workspace member; crate `iomt-protocol`
│   │   ├── Cargo.toml
│   │   ├── build.rs                                # prost-build + tonic-build from ../protocols/proto
│   │   └── src/{lib.rs, generated.rs, domain.rs}   # generated (DO NOT EDIT) + domain types (hand-written)
│   ├── python/iomt_protocols/{pyproject.toml, src/iomt_protocols/}   # [MVP] generated (DO NOT EDIT)
│   └── typescript/protocols/{package.json, src/}                     # [MVP] generated (DO NOT EDIT)
│
├── iot-node/                                      # [MVP] [T0/IoT] — Rust (nested Cargo project)
│   ├── README.md
│   ├── firmware/                                   # ESP32-C3 project root (do NOT nest another level)
│   │   ├── Cargo.toml                              # [workspace] members = ["crates/*"]
│   │   ├── rust-toolchain.toml                     # pin + target riscv32imc-esp-espidf
│   │   ├── .cargo/config.toml                      # target, linker, runner (espflash)
│   │   ├── sdkconfig.defaults                      # esp-idf-svc configuration, committed
│   │   ├── partitions.csv                          # [MVP] A/B-ready: nvs/otadata/phy/ota_0/ota_1/model_store (§08 §9.2); USB flash in MVP
│   │   ├── src/{main.rs, lib.rs}
│   │   ├── crates/                                 # 1 crate = 1 bounded unit
│   │   │   ├── snn-lif/                             # no_std LIF neuron core (host-testable)
│   │   │   ├── iomt-frames/                         # no_std encode/decode BLE frame from protocols
│   │   │   ├── sensor-max30102/                     # PPG driver
│   │   │   ├── sensor-mpu6050/                      # IMU driver
│   │   │   ├── signal-proc/                         # filter, peak detect
│   │   │   ├── ble-transport/                        # GATT server, framing, LESC bonding
│   │   │   ├── ota-client/                          # [LATER] DFU receive/verify/mark-valid/rollback (MVP flashes over USB)
│   │   │   └── power-mgmt/                          # [LATER] power optimisation
│   │   ├── tests/host/                              # [MVP] `cargo test` runs on PC (CI)
│   │   ├── test/fixtures/                           # [MVP] recorded iotm-frames captures; feeds `edge-ingest --replay` (REQ-DEMO-001, F-08)
│   │   └── models/{snn_motion_v1.onnx, snn_motion_v1.bin, MODEL_CARD.md}
│   └── hardware/                                   # [LATER] breadboard MVP; PCB/enclosure phase 2
│       ├── README.md
│       ├── pcb/                                     # KiCad project
│       ├── bom/BOM.csv
│       ├── enclosure/                               # .step/.stl + design source
│       └── datasheets/
│
├── edge-gateway/                                  # [T1/Edge] — split into 2 runtime tiers
│   ├── README.md
│   ├── edge-lite/                                  # [MVP] runs ON Pi 3B+ (aarch64, budget ≤ ~400MB)
│   │   ├── config/edge.default.yaml                # default configuration (no secrets)
│   │   └── services/                               # 3 COARSE services — each = 1 independent crate; split when needed, not before (D14)
│   │       ├── edge-ingest/{Cargo.toml, src/{ble,window,inference}/, tests/, Dockerfile}  # ble-ingest + signal-processor: btleplug/BlueZ intake, windowing/normalisation, ONNX int8 (tract|ort); `--replay <fixture>` runs the pipeline with no BLE hardware (REQ-DEMO-001, F-08)
│   │       │   └── src/ble/{handshake.rs (MVP time anchor + proto/fw negotiation), dfu.rs ([LATER] firmware DFU GATT server)}
│   │       ├── edge-sync/{Cargo.toml, src/{queue,uplink,api,tls,enrol}/, tests/, Dockerfile}  # buffer-store + gateway-api: SQLite store-and-forward queue, mTLS Cloud upload, axum + tonic REST/gRPC
│   │       │   ├── src/queue/{schema.sql, dlq.rs, maintenance.rs}   # [MVP] SQLite schema/PRAGMAs, dead-letter + replay CLI, caps/purge/clock gate
│   │       │   └── src/{tls,enrol}/                                 # [MVP] rustls mTLS client, clock gate, CSR/enrol/renew (step-ca)
│   │       └── model-agent/{Cargo.toml, src/{ota}/, tests/, Dockerfile}                   # tokio, tract|ort, ed25519-dalek (verify/swap/rollback); src/ota = [MVP] model OTA; firmware OTA = [LATER]
│   └── edge-train/                                 # [LATER] powerful machine, does NOT run on Pi
│       ├── ml/                                     # Python library edge_ml (transformers/ssl/datasets/export)
│       │   ├── pyproject.toml
│       │   ├── src/edge_ml/{transformers/, ssl/, datasets/, export/}
│       │   └── tests/
│       └── services/local-trainer/{pyproject.toml, src/local_trainer/, tests/, Dockerfile}  # Python Flower client
│
│   # NOTE: there is NO edge-gateway/deploy/** — all manifests live in root deploy/ (see below)
│
├── cloud/                                          # [T2/Cloud] — runs on powerful machine/Cloud, NOT on Pi
│   ├── README.md
│   ├── backend/                                     # [MVP] Rust (axum + sqlx + tonic) — 1 crate
│   │   ├── Cargo.toml
│   │   ├── migrations/                              # sqlx migrate: <UTC timestamp>_<desc>.sql
│   │   │   ├── *_auth.sql                           # [MVP] users, argon2id, refresh tokens, RBAC (3 MVP roles)
│   │   │   ├── *_audit_log.sql, *_device_registry.sql, *_consent.sql   # [MVP] append-only audit, devices, consent
│   │   │   ├── *_lifecycle.sql                      # [MVP] hypertables + CAGGs + compression(7 d)+retention(180 d) + quarantine
│   │   │   └── *_push.sql                           # [MVP] push_subscriptions, push_delivery, quiet-hours settings
│   │   ├── sqlx-data.json                           # (optional) metadata for compile-time query check
│   │   ├── src/{main.rs, lib.rs, api/, domain/, infra/, config.rs}
│   │   ├── src/alerts/                              # [MVP] dedup/hysteresis/escalation engine + FCM HTTP v1 sender
│   │   ├── tests/                                   # sqlx::test / Testcontainers (Postgres+Timescale)
│   │   └── Dockerfile
│   └── fl-server/                                   # [LATER] Flower server (Python, 1 package)
│       ├── pyproject.toml
│       ├── src/fl_server/{app.py, strategies/, registry/, storage/, security/}
│       ├── tests/
│       └── Dockerfile
│
├── mobile-app/                                     # [MVP] [T3/Mobile] — React Native (TypeScript)
│   ├── README.md
│   ├── package.json, package-lock.json, tsconfig.json, babel.config.js, metro.config.js
│   ├── app.json                                     # [MVP] FCM: use a dedicated Firebase **dev** project; google-services.json/APNs kept out of git or injected per env; emulator fallback (REQ-DEMO-004, F-09)
│   ├── src/
│   │   ├── app/                                      # navigation, providers
│   │   ├── features/{auth,devices,measurements,alerts}/   # [MVP]
│   │   │   └── <feature>/{screens/,components/,api/,state/}
│   │   ├── features/models/                          # [LATER]
│   │   ├── components/
│   │   ├── services/                                 # client generated from OpenAPI (generated)
│   │   ├── state/, theme/, utils/
│   ├── assets/, __tests__/, android/, ios/
│
├── deploy/                                         # [MVP] [PLAT] ONE single deploy home (no nested deploy inside a tier)
│   ├── README.md
│   ├── compose/
│   │   ├── compose.dev.yml                            # [MVP] infra + edge + cloud (profiles) + image digest; dev TLS via internal `step` CA / `step-ca` profile or a documented dev domain (REQ-DEMO-003, F-09)
│   │   ├── compose.pi3.yml                            # [MVP] PRIMARY edge deployment: Docker Compose on Pi 3B+ (edge-lite + node_exporter; healthchecks, mem/cpu limits, json-file rotation, restart: unless-stopped)
│   │   ├── compose.cloud.yml                          # [MVP] VPS stack: pg(+Timescale), backend, Caddy, Prometheus, Alertmanager, Grafana, postgres_exporter, node_exporter, blackbox
│   │   ├── .env.example                              # environment variable template (NO real secrets)
│   │   └── mosquitto/config/mosquitto.conf           # [LATER] broker config + auth (kept; no MVP consumer after D14)
│   ├── k8s/                                           # per-tier subfolders
│   │   ├── edge/{base/, overlays/{dev,pi3}/}         # [MVP] edge-lite Deployments + resources/limits
│   │   └── cloud/{base/, overlays/{dev,prod}/}       # [MVP] (clinical overlay [LATER])
│   ├── helm/                                          # per-tier subfolders
│   │   ├── edge/iomt-edge/{Chart.yaml, values-dev.yaml, values-pi3.yaml}   # [MVP] values-pi3 = aarch64 + limits
│   │   └── cloud/iomt-cloud/{Chart.yaml, values-dev.yaml, values-prod.yaml} # [MVP]
│   ├── ingress/Caddyfile                              # [MVP] api + grafana + push (automatic TLS; push = mTLS)
│   ├── observability/
│   │   ├── README.md                                  # [MVP] collection topology (Prometheus scrapes the Pi; push [LATER])
│   │   ├── prometheus/{prometheus.yml, rules/{edge,cloud,ops,backup}.rules.yml, tests/{edge,cloud,ops,backup}.test.yml}  # [MVP] ≤ 8 ops rules; 1 test per rule (MET-25)
│   │   ├── prometheus/rules/slo.rules.yml, tests/slo.test.yml   # [LATER] SLO recording rules + burn-rate alert (MET-28 deferred, ADR-0020; CR-15)
│   │   ├── alertmanager/{alertmanager.yml, templates/telegram.tmpl}   # [MVP] ops-only, ≤ 8 MVP rules (ADR-0013)
│   │   ├── grafana/provisioning/{datasources/prometheus.yml, dashboards/dashboards.yml}   # [MVP]
│   │   ├── grafana/dashboards/{pi-health.json, edge-pipeline.json, cloud-api-red.json, timescaledb-e2e-latency.json}   # [MVP] the canonical 4 dashboards-as-code (matches 04 §7.3); timescaledb-e2e-latency covers TimescaleDB health + cagg-lag/E2E latency
│   │   ├── grafana/dashboards/{device-fleet-ble.json, fl-training.json}   # [LATER]
│   │   ├── node-exporter/{textfile/.gitkeep, collect-container-stats.sh}  # [MVP] Pi scrape target + container RSS/health textfile collector
│   │   ├── blackbox/blackbox.yml                      # [MVP] public TLS expiry + availability
│   │   ├── vmagent/{scrape.yml, remote-write-notes.md}                    # [LATER] only for NAT'd Pis; default is Prometheus pull over LAN/VPN (ADR-0012)
│   │   ├── loki/                                      # [LATER]
│   │   └── otel/                                      # [LATER]
│   ├── security/
│   │   ├── step-ca/{ca.json, config/}                 # [MVP] step-ca on the VPS; dev uses the `step` CLI (private keys never in git)
│   │   ├── ca/{README.md, root.pem}                  # [MVP] public trust anchor (step-ca root) + key-ceremony runbook
│   │   ├── certs/.gitkeep                             # [MVP] dev certs (gitignored)
│   │   ├── policies/kyverno/                          # [LATER]
│   │   └── policies/opa/                              # [LATER]
│   ├── secrets/{.sops.yaml, *_ENC.yaml}               # [MVP] sops+age ciphertext (plaintext never committed)
│   ├── secrets/pi/                                    # [MVP] root-owned 0400 Compose secret files (gitignored)
│   ├── secrets/templates/sealed-secret.example.yaml   # [LATER]
│   ├── pi/chrony.conf                                 # [MVP] NTP sources, makestep, drift file
│   ├── backup/{pgbackrest.conf, backup.sh, restore-drill.sh}   # [MVP] nightly backup + documented restore drill
│   └── systemd/iomt-container-stats.{service,timer}   # [MVP] textfile collector for node_exporter
│
├── benchmarks/                                     # [LATER] [EVAL] SOTA measurement (WP9) — Python (uv)
│   ├── README.md, pyproject.toml
│   ├── protocols/, configs/, scripts/, fixtures/
│   └── results/.gitkeep                              # timestamped results (contents ignored)
│
├── experiments/                                    # [LATER] [RESEARCH] training/experiments (WP6-8) — Python (uv)
│   ├── README.md, pyproject.toml
│   ├── configs/, notebooks/, scripts/
│   └── logs/.gitkeep
│
├── docs/                                           # [MVP] [DOCS]
│   ├── README.md                                     # index of all documentation
│   ├── architecture/{SYSTEM_ARCHITECTURE.md, FOLDER_STRUCTURE.md, DATA_FLOW.md, DEPLOYMENT.md, MERMAID_DIAGRAMS.md}
│   ├── engineering/CODING_CONVENTIONS.md            # [MVP] code/doc standard (D12): rustdoc / Google docstrings / TSDoc, CI-enforced
│   ├── adr/                                          # ADR-0001..0020 — full backfill list in §2.8
│   │   ├── 0000-template.md
│   │   ├── 0001-three-tier-architecture.md ... 0010-edge-service-granularity.md
│   │   ├── 0004-flower-fl-server.md                  # [LATER]
│   │   ├── 0011-central-observability-pi.md, 0012-scrape-first-metrics.md, 0013-ops-vs-clinical-alerts.md   # [MVP]
│   │   ├── 0014-stepca-mtls-certificates.md, 0015-self-issued-jwt-rbac.md, 0016-secrets-custody.md          # [MVP]
│   │   └── 0017-single-time-model.md, 0018-at-least-once-outbox.md, 0019-pgbackrest-stateless-pi.md, 0020-mvp-scope-trim.md   # [MVP]
│   ├── specs/{JD.md, NFR.md, glossary.md, traceability.csv, wp/WP-01..WP-10.md}
│   ├── plans/{IMPLEMENTATION_PLAN.md, TASK_BREAKDOWN.md, ROADMAP.md}
│   ├── diagrams/
│   ├── governance/{PRIVACY.md}                       # [MVP] promoted; DATA_GOVERNANCE.md + CONSENT_TEMPLATE.md + clinical overlay = [LATER]
│   ├── governance/{DATA_GOVERNANCE.md, CONSENT_TEMPLATE.md}   # [LATER] IRB package
│   ├── security/{SECURITY_MODEL.md, THREAT_MODEL.md, KEY_MANAGEMENT.md}   # [MVP] promoted (condensed 07 §2/§3; Ed25519 key mgmt)
│   ├── runbooks/{deploy-edge.md, ota-rollback.md, observability.md, secret-rotation.md, pi-hardening.md, security-check.md, backup-restore.md, queue-replay.md, pi-disaster-recovery.md}   # [MVP] (ota-rollback.md extended to cover firmware OTA sign-off; one runbook list shared with 04 §7.3 / 08 §4.3)
│   ├── runbooks/{incident.md, firmware-ota-rollback.md}   # [LATER] (same list and tags as 04 §7.3 / 08 §4.3)
│   ├── reports/.gitkeep
│   └── history/{Jd.md, WP_SPECS.md}                  # archived original, read-only
│
└── tools/                                          # [MVP] [PLAT] dev tooling
    ├── Makefile                                      # included from root Makefile
    ├── x509-ca/                                       # [LATER] custom Rust rcgen CA (MVP uses step-ca on the VPS + step CLI)
    ├── scripts/{setup-dev.sh, build-all.sh, deploy-dev.sh, clean.sh, gen-contracts.sh, check-traceability.sh, lint-all.sh, test-all.sh, check-deps.sh, build-images.sh, pi-bootstrap.sh, seed-dev.sh}   # seed-dev.sh = demo seed data (REQ-DEMO-002, F-09)
    └── templ/{wp-spec.md, adr.md, model-card.md}
```

### 2.1 Resource budget — `edge-lite` on Pi 3B+ (1 GB RAM)

The **canonical `edge-lite` budget is defined once in `03 §7.1`** (per-service request/limit, the `node_exporter` row, the combined total, the Pi OS/Docker baseline and the ≤ ~400 MB operating cap). The table below **mirrors that budget** for the tree and adds the per-service CPU columns and notes; do **not** re-derive these arithmetic values anywhere else. Rows are labelled **services-only vs services+exporter** so the two totals are never confused. Every Compose service **MUST** declare `mem_limit` + `cpus` (and K8s `resources.requests` + `resources.limits` for the dev-validation manifests):

| Service (edge-lite) | RAM request | RAM limit | CPU request | CPU limit | Notes |
|---|---|---|---|---|---|
| `edge-ingest` | 40 MB | 112 MB | 150 m | 600 m | ble-ingest + signal-processor merged; btleplug/BlueZ via D-Bus + ONNX int8 via `tract`/`ort` (model ≤ 5 MB); ~32 MB duplicated runtime removed |
| `edge-sync` | 32 MB | 64 MB | 100 m | 300 m | buffer-store + gateway-api merged; SQLite (`rusqlite`)/`sled` store-and-forward queue + axum + tonic; **NOT Postgres**; **storage = high-endurance microSD (D19)** |
| `model-agent` | 32 MB | 64 MB | 50 m | 250 m | `tract`/`ort` + ed25519-dalek verify/swap/rollback |
| **Subtotal — 3 edge-lite services (services-only)** | **104 MB** | **240 MB** | **300 m** | **1150 m** | matches the `03 §7.1` "Total" row; 3 coarse services, split when needed (D14) |
| `node_exporter` | 8 MB | 20 MB | 10 m | 50 m | [MVP] curated collectors (~300 series); **scraped over LAN/VPN, no vmagent** (trim, ADR-0012) |
| **Total — edge-lite = 3 services + node_exporter (combined)** | **112 MB** | **260 MB** | **310 m** | **1200 m** | the one combined figure; operating cap **≤ ~400 MB** per `03 §7.1` (MET-11) |

- Forbidden on Pi: Postgres, Redis server (use SQLite/sled + in-process queue), PyTorch, `edge-train`, `local-trainer`, `cloud/*`.
- **Observability on the Pi = `node_exporter` only** (≤ 20 MB limit / ~12 MB RSS); Prometheus/Grafana/Alertmanager run outside the Pi (dev machine/VPS) and **scrape** it over the LAN/VPN (06 §5.1, MET-20). `vmagent`/local TSDB are `[LATER]`/NAT-only. Combined edge-lite limits = 3 services + node_exporter = **260 MB ≤ ~400 MB** (only combined figure; `03 §7.1`).
- **Security adds no Pi container:** rustls + client cert live inside `edge-sync`'s 64 MB limit; steady-state +2–4 MB, +0.01 % CPU (07 §5.1).
- **Reliability adds no container:** SQLite page cache + WAL ≈ +8 MB RSS inside `edge-sync`'s 64 MB limit; chrony lives in the OS baseline; SD write volume + SQLite commit rate metered by **MET-13** (MET-42 merged into MET-13, reserved/unused; 08 §5.1).
- **Pi storage medium (RESOLVED, D19):** high-endurance **microSD** with write minimisation (WAL batching, volatile logs, `noatime`, json-file log rotation); the storage path stays configurable and **SSD/USB is the `[LATER]` lever with no code change**. Measured on-disk volume ≈ 1 GB/day at D18 1 Hz vitals (**MET-13**; MET-42 merged into MET-13) ⇒ decades of TBW on a 10–30 TBW card. Whichever medium: `edge-sync` MUST enable `journal_mode=WAL` + batched commit (batch ≤500 rows or fsync ≤500 ms).

#### 2.1.1 Cache policy — NO shared cache (ADR-0008) [MVP]

- **No shared cache at any tier in the MVP**: no Redis/Memcached, no shared read cache, no CDN caching for the medical API; **do not add any cache component to the directory tree** (per `05 §8.7`).
- **Redis is forbidden on Pi** (1 GB budget). Cloud also does **not** use Redis: write load ~1 row/s and reads <10M rows with ≤30 devices ⇒ no problem that needs caching.
- **Only in-process / in-RAM** at the exact places listed: warm ONNX session + reused tensor buffer (`edge-ingest`); RAM ring buffer + WAL/batched commit in `edge-sync` (ring ≤10 MB, within the 64 MB limit); client `staleTime` cache for chart/list (mobile).
- **Absolutely do NOT cache:** raw/near-real-time signal used for inference & alert; alert engine input; JWT revocation / RBAC changes; PHI on shared cache; per-user API responses (auth-scoped); device online/offline status used for safety; model integrity metadata (SHA256/signature — always re-verify on load/apply).
- **Timescale compression + retention** [MVP] (not a cache): compress after 7 days, retention per governance.
- **Continuous aggregates** [MVP] (D18): `vitals_1m/1h/1d` with real-time aggregation (refresh 60 s/15 min/60 min); they back the 7-day chart rollups and the `timescaledb-e2e-latency` dashboard (MET-14/17/45, `03 §7`).

### 2.2 Edge deployment model (MVP — DECIDED, D13)

**Primary edge deployment = Docker Engine + Docker Compose on the Pi 3B+ (see `ADR-0009`). No K3s runs on the Pi in the MVP.**

```text
Dev machine (x86_64 Linux)                       Raspberry Pi 3B+ (aarch64)
┌───────────────────────────────┐                ┌───────────────────────────────────┐
│ Docker (multi-arch builds)     │  arm64 image   │ Docker Engine + Docker Compose     │
│ + Compose                      │───────────────►│ docker compose -f                  │
│ + K3s server (DEV ONLY:        │   (registry)   │   deploy/compose/compose.pi3.yml   │
│   validates deploy/k8s + helm) │                │ edge-lite services + node_exporter │
│ + cloud stack (compose.cloud)  │◄──scrape───────│ BLE via host network + dbus mount  │
│ + Prometheus/Grafana/Caddy     │  LAN/VPN:9100  │ healthchecks + json-file rotation  │
└───────────────────────────────┘                └───────────────────────────────────┘
```
- **Compose on the Pi (primary):** `docker compose -f deploy/compose/compose.pi3.yml up -d`. Per-service `mem_limit`/`cpus` enforce the §2.1 budget (MET-11); every service declares a `healthcheck` (curl `/healthz`), `restart: unless-stopped`, and `logging` with `json-file` rotation (`max-size=5m`, `max-file=3`).
- **Cloud VPS stack (D15):** `docker compose -f deploy/compose/compose.cloud.yml up -d` runs Postgres(+Timescale), backend, **Caddy ingress** (`deploy/ingress/Caddyfile`, automatic TLS: `api`/`grafana`/`push`), Prometheus, Alertmanager, Grafana, **`step-ca`** (the private CA, part of the cloud demo footprint) and the exporters. This is the **only** ingress point.
- **Demo topology sizing:** the VPS target is **2 vCPU / 4 GB / 40 GB SSD**; the demo stack budget **includes `step-ca` (~40 MB)** and **demo backups run sequentially** (not concurrently); production scaling is `[LATER]`. Full arithmetic in `07 §3.5`; see also `04 §7.3`.
- **BLE container:** `edge-ingest` needs `network_mode: host` + mount `/var/run/dbus:/var/run/dbus` (read-only where possible) and the host **BlueZ** service (§2.4).
- **Monitoring:** Prometheus **scrapes** the Pi's `node_exporter` (`:9100`) and the service `/metrics` endpoints over the LAN/VPN; `vmagent`/remote_write is `[LATER]` for NAT'd Pis only (ADR-0012).
- **Dev machine:** Docker for multi-arch (`amd64`+`arm64`) builds and Compose; a **K3s server validates the Helm/K8s manifests** in `deploy/k8s` and `deploy/helm` (these stay WP3 deliverables).
- **K3s agent on the Pi = `[LATER]`:** only when 3+ Pis share a LAN, or if the supervisor requires K3s on the device. The manifests remain ready for that move (`nodeSelector: { kubernetes.io/arch: arm64, node-role/edge: pi3 }` + `resources`).

### 2.3 Multi-arch build (amd64 + arm64)

- **Image:** `docker buildx build --platform linux/amd64,linux/arm64 -t ghcr.io/iomt/<svc>:<semver> --push` for every `edge-lite` service. CI job `ci-images.yml` builds **both** arch for each service.
- **Rust cross-compile aarch64:** use `cross` **or** `cargo-zigbuild`; target `aarch64-unknown-linux-gnu` declared in `rust-toolchain.toml`. Required command: `cargo check --target aarch64-unknown-linux-gnu` (run in PR).
- **Firmware ESP32-C3** still cross-compiles to `riscv32imc-esp-espidf` (unrelated to aarch64).
- **Helm values:** `deploy/helm/edge/iomt-edge/values-dev.yaml` (x86_64 dev, no strict limit) and `values-pi3.yaml` (arm64, Pi `nodeSelector`, resources per §2.1). Cloud: `values-dev.yaml`/`values-prod.yaml`.

### 2.4 BLE container access (Pi)

- `edge-ingest` needs BlueZ on the Pi host:
  - `hostNetwork: true` (or `network_mode: host` in compose) to use native D-Bus/BlueZ.
  - Mount `/var/run/dbus:/var/run/dbus` (ro if possible) and `--device /dev/...` if needed.
  - Capability: `NET_ADMIN`/`NET_RAW`; `securityContext.privileged: false` but appropriate `capabilities.add` is needed.
  - Operational note: enable `bluetooth.service`, `hciconfig`; if scanning finds no devices ⇒ check D-Bus permissions (`docker` user / Compose container user in the `bluetooth` group).

- **BLE 5 connection limit:** Pi 3B+ (onboard adapter) in practice stably holds ~**7–10 connections/adapter** ⇒ **30 nodes need ≥3–4 adapters or batched scanning/batching**. `edge-ingest` MUST batch ~10 samples/frame at high frequency (100 Hz) to avoid approaching the connection interval limit. Measured by MET-16 (`03 §7`).

### 2.5 Security topology (MVP — 07 §4.1)

- **Cloud ingress:** `443/tcp` Caddy (automatic ACME TLS) for `api`, `grafana`, `push`; `9443/tcp` axum **mTLS** (TLS 1.3 only, cert-gated) for the edge uplink. Every other cloud port (Postgres `5432`, backend `8080`, Prometheus `9090`, Alertmanager `9093`, Grafana `3000`) is **compose-internal only** (no host publish).
- **Pi inbound:** `22/tcp` SSH only (key auth, admin LAN). `edge-ingest` uses BlueZ over `hostNetwork` + D-Bus; no MQTT in the MVP (D14).
- **Identity:** Pi↔cloud mTLS certificates come from **step-ca run on the cloud VPS** (dev uses the `step` CLI); leaf lifetime ≤ 90 d, auto-renew at day 60, revocation by registry status check (no CRL/OCSP). The custom `tools/x509-ca` is `[LATER]`. User auth = self-issued **JWT** + **argon2id** with **3 MVP roles** (patient, clinician, admin; caregiver/researcher `[LATER]`).
- **Data protection:** append-only audit log table (hash chain + daily anchor), API rate limiting, consent record table + **manual** export/deletion runbook (self-service endpoints `[LATER]`); secrets via **sops+age** (cloud) and root-owned 0400 Compose files (Pi). CI: cargo-deny + cargo-audit + trivy + gitleaks.
- **Artifacts & hardening:** models signed with **Ed25519** (verify-before-apply); Pi hardening checklist = ssh keys, `ufw`, unattended upgrades. `[LATER]`: secure boot/flash encryption, cosign, SBOM, secure aggregation/DP, read-only rootfs, hardware-backed keys.

### 2.6 Observability / metrics topology (MVP — 06 §3.1, ADR-0011/0012/0013)

- **Collection model:** **Prometheus scrapes** every `/metrics` endpoint (Pi over the LAN/VPN at 60 s, cloud at 15 s; 15 d retention). `vmagent` + Prometheus `remote_write` over mTLS is **`[LATER]`**, only for Pis behind NAT (ADR-0012). The Pi runs **only `node_exporter`**; no TSDB/cAdvisor on the device (ADR-0011).
- **Endpoints:** every application service exposes `GET /healthz`, `GET /readyz`, `GET /metrics`; Compose declares a `healthcheck` for each; Docker logs use bounded `json-file` rotation.
- **Dashboards-as-code:** exactly **4** provisioned Grafana dashboards in the MVP (`pi-health`, `edge-pipeline`, `cloud-api-red`, `timescaledb-e2e-latency` incl. the TimescaleDB cagg-lag view; matches `04 §7.3`); device-fleet/FL/SLO dashboards are `[LATER]`.
- **Alerting separation (ADR-0013):** Alertmanager carries **≤ 8 operational rules** to Telegram/email/webhook; **clinical alerts never go through Alertmanager** — they use the doc-08 FCM push path (CI-enforced).
- **Ingress:** Caddy terminates TLS for `api`/`grafana` and the mTLS `push` endpoint (automatic certificates).

### 2.7 Reliability & data lifecycle (MVP — 08)

- **Time:** single UTC epoch-milliseconds model everywhere; the ESP32 anchors time at the **BLE handshake** (re-anchor ≤ every 10 min); **chrony** disciplines the Pi; uplink is gated until NTP is within ± 120 s (no row is rewritten).
- **Edge queue:** `edge-sync` durably writes an **at-least-once SQLite outbox** (WAL) keyed by `(device_id, window_start_ms, seq)`; idempotent Cloud ingest; tiered drops (oldest vitals → windows, **never alerts**) with a **dead-letter table + replay CLI**.
- **Cloud data:** Timescale **hypertables** (1 d/7 d chunks) + **compression (7 d)** + **retention (180 d)** + **continuous aggregates** `vitals_1m/1h/1d` real-time (D18); server-side timestamp sanity + quarantine.
- **Backup:** **pgBackRest** nightly (repo1 local + encrypted S3 `repo2`) + continuous WAL + one documented restore drill (RPO ≤ 10 min / RTO ≤ 60 min); the Pi is stateless (reflash + `pi-bootstrap.sh` + enrol).
- **Updates & alerts:** **model** OTA with sha256+Ed25519 and rollback is `[MVP]`; **firmware** OTA over BLE is `[LATER]` (USB flash in the MVP, partition table A/B-ready); FCM clinical push with dedup/hysteresis; canary rollout, quiet hours, SMS/e-mail fallback and RTC are `[LATER]`.

### 2.8 ADR backfill list (ADR-0001..ADR-0020)

| ADR | Title | Scope | Source |
|---|---|---|---|
| ADR-0001 | Three-tier architecture | `[MVP]` | `00`/`01` |
| ADR-0002 | BLE vs MQTT transport | `[MVP]` | `00`/D17 |
| ADR-0003 | Rust workspace | `[MVP]` | `00`/D1 |
| ADR-0004 | Flower FL server | `[LATER]` | `00`/D2 |
| ADR-0005 | TimescaleDB + sqlx | `[MVP]` | `00` |
| ADR-0006 | Model signing (Ed25519) | `[MVP]` | `00` |
| ADR-0007 | Pi3 edge split | `[MVP]` | `00` |
| ADR-0008 | Cache policy (no shared cache) | `[MVP]` | `05 §8` |
| ADR-0009 | Edge runtime = Compose on Pi | `[MVP]` | D13 |
| ADR-0010 | Edge service granularity (3 coarse services) | `[MVP]` | D14 |
| ADR-0011 | Central observability; Pi runs only `node_exporter` (no cAdvisor/TSDB) | `[MVP]` | `06 §9` |
| ADR-0012 | Scrape-first metrics over LAN/VPN; `vmagent` remote-write `[LATER]` (NAT only) | `[MVP]` | `06 §9` + trim |
| ADR-0013 | Operational vs clinical alert separation (Alertmanager ops-only) | `[MVP]` | `06 §9` |
| ADR-0014 | step-ca private CA + 90-day mTLS gateway certificates (no daemon on the Pi) | `[MVP]` | `07 §9` + trim |
| ADR-0015 | Self-issued JWT + argon2id + RBAC (3 MVP roles) | `[MVP]` | `07 §9` + trim |
| ADR-0016 | Secrets custody split (sops+age / 0400 Compose / offline Ed25519 root) | `[MVP]` | `07 §9` |
| ADR-0017 | Single UTC-ms time model + handshake anchor + chrony | `[MVP]` | `08 §9` |
| ADR-0018 | At-least-once SQLite outbox + idempotency key + tiered drops + DLQ | `[MVP]` | `08 §9` |
| ADR-0019 | pgBackRest + stateless Pi | `[MVP]` | `08 §9` |
| ADR-0020 | MVP scope trim rules (this block, §0) | `[MVP]` | this document |

## 3. Naming conventions

### 3.1 Directories & files
- Directories: **lowercase kebab-case**, domain names in the **singular** (`edge-ingest`, not `ingests`). Rust crate directories use kebab-case; the **package name in `Cargo.toml`** is also kebab-case (`edge-ingest`), imports in code use `_` (`edge_ingest`).
- Python package: `snake_case` (`edge_ml`, `iomt_protocols`, `local_trainer`, `fl_server`).
- Rust module/file: `snake_case`; type `PascalCase`; traits by behavior; shared crates use the `iomt-` prefix (`iomt-protocol`).
- TypeScript module: `camelCase` for files, `PascalCase` for React components.
- Docs/config: `kebab-case.md`/`.yaml`, except standard templates (`CONTRIBUTING.md`, `SECURITY.md`).

### 3.2 Contracts (`shared/protocols`)
- Proto package: `iomt.<domain>.v<major>` → e.g. `iomt.telemetry.v1`; file `<domain>.proto`.
- OpenAPI: `<service>.openapi.yaml`; `info.version` = SemVer; base path contains the major (`/api/v1`).
- JSON Schema: `<name>.v<major>.schema.json`; `$id` = `https://schemas.iomt.local/<domain>/<name>.v<major>.json`.
- AsyncAPI: `<channel>.asyncapi.yaml`; topic pattern `iomt/<tier>/<device>/<domain>`.
- Any breaking change must bump the **major** and record an ADR.

### 3.3 Document identifiers
| Type | Format | Example |
|---|---|---|
| Work Package | `WP-NN` | `WP-04` |
| Requirement | `REQ-WP-NN-NNN` | `REQ-WP-03-012` |
| Test case | `T-REQ-WP-NN-NNN[-NN]` | `T-REQ-WP-03-012-01` |
| Metric | `MET-NN` | `MET-01` (IoT inference latency) |
| Risk | `RISK-NN` | `RISK-04` |
| ADR | `ADR-NNNN` | `ADR-0002` |
| Date | ISO-8601 `YYYY-MM-DD` | — |

### 3.4 Artifact & release
- Docker image: `ghcr.io/iomt/<module>:<semver>`; dev uses `<semver>-dev.<gitsha>`.
- Model version: `<name>-<semver>+<sha256-prefix>` (e.g. `snn-motion-1.2.0+ab12cd34`), signed with Ed25519.
- Firmware/OTA release tag: `fw-<target>-<semver>` (e.g. `fw-esp32c3-1.3.0`).
- Crate publish (if any): `iomt-<name>`; internally use `path` + `version.workspace = true`.
- Git: branch `wp-NN/<slug>`, commits per Conventional Commits (`feat(edge): ...`).

### 3.5 Versioning
- **Module/artifact:** SemVer (`MAJOR.MINOR.PATCH`).
- **Interfaces:** versioned in the name (proto package / URL path / schema filename) + compatibility policy (see `03 §5`).
- **Rust workspace:** synchronized version via `[workspace.package] version`; only `iomt-protocol` is published.
- **Contracts bundle:** `shared/protocols/VERSION` is the current major; `buf breaking` blocks breaking changes in CI.
- **DB schema:** `sqlx migrate` (see `03 §9`).

## 4. Dependency rules between modules

Compile-time layering (may only point down, must not point up and must not be lateral between tiers):

```text
L0  shared/protocols/                        (leaf — imports nothing)
L1  shared/rust, shared/python, shared/typescript   (depend on L0, generated only)
L2  tier modules:
      iot-node/                    [T0]  (nested workspace, not part of the root workspace)
      edge-gateway/edge-lite/      [T1]  runs on Pi 3B+ (aarch64)
      edge-gateway/edge-train/     [T1]  powerful machine (x86_64), NOT on Pi
      cloud/                       [T2]
      mobile-app/                  [T3]
L3  cross-cutting: deploy/, tools/, docs/, benchmarks/, experiments/  (not imported by L0-L2)
```

### 4.1 "Allowed imports" matrix

| Source module ↓ \ imported by → | shared/protocols | shared/rust | shared/python | shared/typescript | iot-node | edge-lite | edge-train | cloud | mobile-app |
|---|---|---|---|---|---|---|---|---|---|
| `iot-node/firmware` | ✅ (codegen `iomt-frames`, no_std) | ⚠️ only the no_std domain part if split into a crate | ❌ | ❌ | ✅ internal | ❌ | ❌ | ❌ | ❌ |
| `edge-gateway/edge-lite` | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ internal | ❌ | ❌ | ❌ |
| `edge-gateway/edge-train` | ✅ | ✅ (only `iomt-protocol`) | ✅ | ❌ | ❌ | ❌ | ✅ internal | ❌ | ❌ |
| `cloud/backend` | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ internal | ❌ |
| `cloud/fl-server` | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ only `shared/protocols` (does not import backend) | ❌ |
| `mobile-app` | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ via OpenAPI-generated client | ✅ internal |

### 4.2 Additional rules
1. **No cross-tier source import.** T0/T1/T2/T3 communicate only over the network + `shared/protocols`.
2. **Service isolation (Edge).** `edge-lite/services/<a>` does **not** import `<b>`; communication via gRPC/REST per `shared/protocols/proto` & `openapi` (MQTT only if another consumer needs it — `[LATER]`/optional). Within a coarse service (D14), the Rust modules (`ble`/`window`/`inference`, `queue`/`uplink`/`api`) may call each other in-process. May only import `iomt-protocol` and internal lib crates approved by the owner. **`edge-lite` must not import `edge-train`** (and vice versa) — only via network/contracts.
2b. **Pi budget enforcement.** `check-deps.sh`/CI verifies that `edge-lite` crates do not pull heavy dependencies (PyTorch, Postgres client, large web server) and that the total limit ≤ ~400 MB.
3. **Cloud separation.** `fl-server` (Python) does not import the `backend` crate; data sharing only via DB schema (view) + `shared/protocols`. `backend` does not call `fl-server` code.
4. **`shared/{rust,python,typescript}` is generated-only.** Manual edits forbidden; PRs must be generated from `shared/protocols/` (CI drift check). Exception: `shared/rust/src/domain.rs` is hand-written domain types (not part of the generated area).
5. **Firmware is a nested workspace.** `iot-node/firmware` is not a member of the root workspace (different target); `cargo check --workspace` at the root does **not** build firmware. Only `snn-lif` and `iomt-frames` are `no_std` and host-testable.
6. **Mobile only consumes the Cloud API** via a client generated from `shared/protocols/openapi/cloud-api.openapi.yaml`; do not hardcode DTOs.
7. **Deploy/tools/docs are not imported by runtime**; conversely they are allowed to "know" every module.
8. **Observability dependency rules (06/ADR-0011·0013).** No runtime crate depends on `deploy/**`; the Pi may not depend on a TSDB/`vmagent`/cAdvisor (only `node_exporter` runs there). Metrics, logs and traces are **PHI-free** (CI scan, MET-26); Alertmanager rules carry `domain="ops"` only — no clinical alert may be routed through Alertmanager.
9. **CA tooling is external to the workspace (trim).** `tools/x509-ca` is `[LATER]`; the MVP trust hierarchy is provided by **step-ca** (VPS) + the `step` CLI, so no runtime crate links a custom CA.

### 4.3 Architecture fitness functions (required to run in CI)
- `cargo metadata --no-deps --format-version 1` + `tools/scripts/check-deps.sh`: blocks violations of table 4.1 (crate A does not depend on forbidden crate B).
- `buf breaking --against .git#branch=main` for `shared/protocols/proto/`.
- `openapi-diff` for `shared/protocols/openapi/`.
- `gen-contracts.sh --check`: `shared/` (except `domain.rs`) has no diff versus codegen.
- `cargo deny check` (license/advisory/ban/source) + `cargo audit`.
- `gitleaks detect` (MET-30) + `trivy image` (MET-31) in `security-scan.yml`.
- **Observability fitness:** `promtool check rules` + `promtool test rules` on `deploy/observability/prometheus/**`; `docker compose config` on `compose.pi3.yml` + `compose.cloud.yml` (healthchecks + limits present); Grafana dashboards provisioned-from-Git diff (MET-22/23/25).
- **Cross-check Pi:** `cargo check --target aarch64-unknown-linux-gnu -p edge-ingest -p edge-sync -p model-agent` (blocks dependencies that cannot build on aarch64). **Forbidden** for `edge-lite` crates to depend on Postgres (`sqlx/postgres`) — only `cloud/backend` may use it.

## 5. Rust workspace configuration (canonical example)

### 5.1 `Cargo.toml` (repo root)
```toml
[workspace]
resolver = "2"
members = [
  "shared/rust",
  "edge-gateway/edge-lite/services/edge-ingest",
  "edge-gateway/edge-lite/services/edge-sync",
  "edge-gateway/edge-lite/services/model-agent",
  "cloud/backend",
]
exclude = ["iot-node/firmware"]        # nested workspace, target riscv32imc-*

[workspace.package]
edition = "2021"
rust-version = "1.78"
license = "Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
tokio       = { version = "1", features = ["full"] }
axum        = "0.7"
tonic       = "0.12"
prost       = "0.13"
rumqttc     = "0.24"                   # [LATER] MQTT (optional; no MVP consumer after D14)
btleplug    = "0.11"
tract-onnx  = "0.21"                   # pure-Rust ONNX inference (preferred on Pi)
ort         = { version = "2", default-features = false }  # alternative; choose 1 of 2
rusqlite    = { version = "0.31", features = ["bundled"] } # edge-sync on Pi (SQLite)
sled        = "0.34"                   # (alternative to rusqlite; choose 1)
ed25519-dalek = "2"
sqlx        = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "migrate", "chrono", "uuid"] }  # ONLY cloud/backend
serde       = { version = "1", features = ["derive"] }
tracing     = "0.1"
iomt-protocol = { path = "shared/rust" }
```

> `ort`/`tract-onnx` and `rusqlite`/`sled` are **mutually exclusive choices**; each `edge-lite` crate declares one variant locally, without pulling both into the Pi image.

### 5.2 `rust-toolchain.toml` (root)
```toml
[toolchain]
channel = "1.78.0"
components = ["rustfmt", "clippy", "rust-src"]
targets = [
  "riscv32imc-esp-espidf",        # firmware ESP32-C3
  "riscv32imc-unknown-none-elf",
  "aarch64-unknown-linux-gnu",    # Pi 3B+ (edge-lite)
]
profile = "minimal"
```

### 5.3 `deny.toml` (excerpt)
```toml
[advisories]
db-urls = ["https://github.com/rustsec/advisory-db"]
[licenses]
allow = ["Apache-2.0", "MIT", "BSD-3-Clause", "ISC", "Unicode-3.0"]
[bans]
multiple-versions = "warn"
[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

### 5.4 `pyproject.toml` (uv workspace, root)
```toml
[project]
name = "iomt-ml-workspace"
version = "0.1.0"
requires-python = ">=3.11"

[tool.uv.workspace]
members = [
  "edge-gateway/edge-train/ml",                    # [LATER] does not run on Pi
  "edge-gateway/edge-train/services/local-trainer",# [LATER]
  "cloud/fl-server",                               # [LATER]
  "benchmarks",                                    # [LATER]
  "experiments",                                   # [LATER]
]
```

### 5.5 `iot-node/firmware/Cargo.toml` (nested workspace, excerpt)
```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
esp-idf-svc = "0.49"
esp-idf-hal = "0.44"
esp-hal     = "0.20"          # for pure no_std crates
heapless    = "0.8"
```

### 5.6 `shared/rust/Cargo.toml` (crate `iomt-protocol`, excerpt)
```toml
[package]
name = "iomt-protocol"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
prost.workspace = true
tonic = { workspace = true, optional = true }
serde.workspace = true
thiserror = "1"

[build-dependencies]
prost-build = "0.13"
tonic-build = "0.12"
```
