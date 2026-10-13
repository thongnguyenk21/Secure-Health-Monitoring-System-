# 04 — Work Split: parallel redesign execution (non-overlapping ownership)

Decompose the work of **executing the redesign** (not the project's WP-01..10) into Redesign Work Packages `RW-xx`, each package **uniquely owning one set of paths**, non-overlapping, so multiple people can work in parallel. Prerequisite: `02-migration-plan.md` has been executed.

Stack (`00 §0.0`): **Rust** for the ESP32-C3 firmware + all edge services + cloud backend (`axum`/`sqlx`); **Python** (uv) only for ML (`edge_ml`, `local-trainer`, `cloud/fl-server`, benchmarks, experiments); **TypeScript** for mobile; `shared/protocols` is the contract (Rust/TS/Python codegen).

## 1. Ownership table (absolutely non-overlapping)

| RW | Name | Owns (globs) | Does NOT own | Depends on |
|---|---|---|---|---|
| RW-00 | Execute migration | entire repo (temporarily, sequential) | — | — |
| RW-01 | Contracts & generated bindings | `shared/**` (including `shared/protocols/**`, `shared/{rust,python,typescript}`) | all layers | RW-00 |
| RW-02 | Workspace build config & root governance | `/Cargo.toml`, `/Cargo.lock`, `/rust-toolchain.toml`, `/rustfmt.toml`, `/clippy.toml`, `/deny.toml`, `/pyproject.toml`, `/uv.lock`, `/.python-version`, `/.github/**`, `/README.md`, `/LICENSE`, `/CONTRIBUTING.md`, `/SECURITY.md`, `/CODEOWNERS`, `/CHANGELOG.md`, `/Makefile`, `/.gitignore`, `/.editorconfig`, `/.gitattributes`, `/.env.example`, `/.nvmrc`, `/.tool-versions` | `docs/**`, `tools/**`, every service's `**/Cargo.toml` | RW-00 |
| RW-03 | Docs, ADR, spec restructure | `docs/**` | root `README.md` | RW-00, RW-01 (ref) |
| RW-04 | Dev tooling & scripts | `tools/**` | `/.github/**`, root `Makefile`, root `Cargo.toml` | RW-00 |
| RW-05 | IoT node (firmware Rust + hardware) | `iot-node/**` | everything else | RW-01 (freeze) |
| RW-06 | Edge-lite — **3 coarse** Rust services (`edge-ingest`, `edge-sync`, `model-agent`; D14) running **on the Pi 3B+** (aarch64, budget ≤ ~400 MB) | `edge-gateway/edge-lite/**` | `edge-gateway/edge-train/**`, `deploy/**`, all other layers | RW-01 (freeze) |
| RW-07 | Cloud backend (Rust axum+sqlx) | `cloud/backend/**`, `cloud/README.md` | `cloud/fl-server/**` | RW-01 (freeze) |
| RW-08 | FL server (Python Flower) | `cloud/fl-server/**` | `cloud/README.md`, `cloud/backend/**` | RW-01 (freeze) |
| RW-09 | Mobile app (React Native TS) | `mobile-app/**` | everything else | RW-01 (freeze) |
| RW-10 | Platform: deploy/observability/security (**ONE single deployer**) | `deploy/**` | all layers (no `deploy/` nested in a layer) | RW-00 |
| RW-11 | Benchmarks & experiments (Python) | `benchmarks/**`, `experiments/**` | everything else | RW-01, RW-05..09 (interfaces) |
| RW-12 | Edge-train — Python `edge_ml` + `local-trainer` (powerful machine, **does NOT run on Pi**) | `edge-gateway/edge-train/**` | `edge-gateway/edge-lite/**`, all other layers | RW-01 (freeze) |

> **Deploy boundary:** ONLY ONE deployer at the root `deploy/**` owned by **RW-10** (compose/k8s/helm/observability/security/secrets). **There is no** `edge-gateway/deploy/**`; each service keeps only its own `Dockerfile` (`01 §1.5`).

> **MVP trim supersedes tags (§7).** The full-scope `RW-xx` globs below stay the redesign scope, so every new path introduced by `06`/`07`/`08` is still covered here (the leading dirs already own them). For the student MVP the `[MVP]`/`[LATER]` tags of `06 §6`/`07 §6`/`08 §6` are **trimmed** by the binding rules in §7.1: Pi↔cloud mTLS uses **`step-ca`** (cloud VPS; `step` CLI in dev), so the custom Rust CA `tools/x509-ca/` is **`[LATER]`**; the Pi runs **`node_exporter` only** (`vmagent` `[LATER]`); Grafana has **4** dashboards; Alertmanager has **≤ 8** operational rules. All new paths map to exactly one MVP-WP in §7.3, and ownership globs stay non-overlapping (§7.5).

### Coordination rules
1. **Only modify within the owned globs.** If a file outside the scope needs changing ⇒ open a PR/issue for the corresponding owner; **do not modify it yourself**.
2. **Freezing contracts is a prerequisite:** RW-01 releases `shared/protocols/VERSION = 0.1.0` + tag `contracts-v0.1.0`; after that RW-05..09, RW-11, RW-12 start. Before the freeze, layers may only write skeletons that do not depend on contract content.
3. **Shared workspace dependencies:** `[workspace.dependencies]`, `Cargo.lock`, `uv.lock` are owned by RW-02. To add a shared dependency ⇒ open a request for RW-02; a service adds its own local dependency in its own `Cargo.toml` (RW-06/07).
4. **Packages communicate through declared interfaces**, not through source imports (per `01 §4`).
5. **Every package must pass green** `tools/scripts/lint-all.sh`, `check-deps.sh`, and the acceptance criteria in §3 before a PR.

## 2. Execution diagram (waves)

```text
Wave 0 (sequential):  RW-00 Migration
Wave 1 (parallel): RW-01 Contracts | RW-02 Workspace/Root | RW-03 Docs | RW-04 Tooling | RW-10 Platform
                    └─ RW-01 freeze v0.1.0 is the synchronization milestone
Wave 2 (parallel): RW-05 IoT (Rust) | RW-06 Edge-lite (Rust, Pi 3B+) | RW-07 Cloud backend (Rust) | RW-08 FL server (Py) | RW-09 Mobile (TS)
Wave 3 (parallel): RW-11 Benchmarks/Experiments (Py) | RW-12 Edge-train (Py, LATER)  (+ RW-03 docs cross-check metrics)
```

## 3. Per-RW details

### RW-00 — Execute migration (must run first, single owner)
- **Goal:** fully run `02-migration-plan.md`, create the Git baseline + target tree + stubs, commit the restructure.
- **Deliverable:** commit `refactor(repo): migrate scaffold...`; clean `git status`.
- **Reviewer acceptance:**
  - `git log --oneline | head` has baseline + migration commit.
  - `test ! -d iot-node/firmware/firmware-esp32c3`; `test ! -d scripts`; `test ! -d shared/types`; `test -d shared/protocols`.
  - `test ! -e cloud/backend/pom.xml && test -z "$(rg -l --glob '*.java' . || true)"`.
  - `git ls-files | wc -l` ≥ 18; `git log --follow iot-node/README.md` OK.

### RW-01 — Contracts & generated bindings
- **Goal:** create the machine-readable source of truth (`shared/protocols`) + codegen for Rust/TS/Python, along with a Rust domain types crate.
- **Owned:** `shared/**`.
- **Deliverables:** proto `telemetry/model/federation/alert v1`; OpenAPI cloud/gateway; JSON Schema (ble-frame, telemetry, model-card, alert); AsyncAPI MQTT `[LATER]`/optional; `buf.yaml`, `buf.gen.yaml`, `VERSION=0.1.0`; valid examples; crate `shared/rust` (`iomt-protocol`: `build.rs` prost/tonic + `generated.rs` DO NOT EDIT + `domain.rs` hand-written); generated bindings Python (`shared/python/iomt_protocols`) and TypeScript (`shared/typescript/protocols`) with a "DO NOT EDIT" note.
- **Reviewer acceptance:**
  - `buf lint shared/protocols/proto` pass; `buf breaking --against .git#branch=main` pass.
  - `spectral lint shared/protocols/openapi/*.yaml` pass; `ajv validate` for every file in `shared/protocols/examples/`.
  - `test -f shared/protocols/VERSION && cat shared/protocols/VERSION` = `0.1.0`.
  - **Rust:** `cargo check -p iomt-protocol` pass; `cargo test -p iomt-protocol` pass.
  - `gen-contracts.sh --check` produces no diff; `rg -l "DO NOT EDIT" shared/{python,typescript}` ⊇ every language directory (except `shared/rust/src/domain.rs`).

### RW-02 — Workspace build config & root governance
- **Goal:** standardize the Rust/uv workspace + repo profile.
- **Owned:** root files listed in §1 (including `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `pyproject.toml`, locks).
- **Reviewer acceptance:**
  - `cargo metadata --no-deps --format-version 1 >/dev/null` pass (workspace parse, after the skeleton crates exist).
  - `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass on the skeleton.
  - `test -s LICENSE && test -s CONTRIBUTING.md && test -s SECURITY.md && test -s CODEOWNERS && test -s rust-toolchain.toml && test -s deny.toml`.
  - `CODEOWNERS` matches every top-level directory: `for d in shared iot-node edge-gateway cloud mobile-app deploy tools docs benchmarks experiments; do rg -q "$d" CODEOWNERS; done`.
  - `markdownlint README.md CONTRIBUTING.md SECURITY.md` pass; `gitleaks detect` clean.
  - `actionlint .github/workflows/*.yml` pass; `make help` prints all targets.

### RW-03 — Docs, ADR & spec restructure
- **Goal:** turn docs into the source of truth, free of contradictions; reflect the Rust/Python/TS stack.
- **Owned:** `docs/**`.
- **Deliverables:** `docs/README.md` (full index); `docs/specs/JD.md` canonical; `docs/specs/NFR.md` (all `MET-NN`); `docs/specs/glossary.md`; `docs/specs/wp/WP-01..WP-10.md` following the `03 §3` template; `docs/specs/traceability.csv`; `docs/adr/0000-template.md` + backfill ADR-0001..0006 (including ADR-0003 Rust workspace, ADR-0005 TimescaleDB+sqlx); plans in `docs/plans/`; diagrams in `docs/diagrams/`; `docs/governance/**`, `docs/security/**`, `docs/runbooks/**`.
- **Reviewer acceptance:**
  - Each `docs/specs/wp/WP-*.md` file has all required headings: `for f in docs/specs/wp/WP-*.md; do for h in Metadata Interfaces "Functional Requirements" "Acceptance Criteria" "Definition of Done" Traceability; do rg -q "$h" "$f" || echo "MISS $f $h"; done; done`.
  - `markdown-link-check docs/README.md` pass (all internal links exist).
  - `tools/scripts/check-traceability.sh` pass; no quantitative targets remain outside `NFR.md` (grep `WP_SPECS` only remains in `docs/history/`).
  - Each ADR has `Status`, `Context`, `Decision`, `Consequences`; no Java/Spring/Flyway/Maven keywords remain outside the ADR "Alternatives considered".

### RW-04 — Dev tooling & scripts
- **Goal:** automate build/lint/test/contracts/trace across multiple languages.
- **Owned:** `tools/**`.
- **Deliverables:** `tools/Makefile` (targets: `setup build test lint fmt up down contracts trace check-deps clean`, where `build`/`test` call `cargo ... --workspace` + `uv run pytest` + `npm test`); `tools/scripts/{setup-dev.sh,build-all.sh,deploy-dev.sh,clean.sh,gen-contracts.sh,check-traceability.sh,lint-all.sh,test-all.sh,check-deps.sh}`; `tools/templ/{wp-spec.md,adr.md,model-card.md}`. **`tools/scripts/pi-bootstrap.sh`** (`[MVP]`, installs Docker + applies `compose.pi3.yml` + joins LAN NTP; owned by RW-04 full-scope, MVP-02 in §7). **`tools/x509-ca/`** (Rust rcgen CA bin) is **`[LATER]`** — superseded by `step-ca` for the MVP (§7.1); if built later it stays under `tools/**` (RW-04).
- **Reviewer acceptance:**
  - `shellcheck tools/scripts/*.sh` pass; `setup-dev.sh` checks `command -v cargo && command -v rustc && command -v uv` (fixes F-023, removes the Java check).
  - `make -C tools lint` / `make -C tools contracts` run without syntax errors.
  - `tools/scripts/check-deps.sh` detects violations of the `01 §4` matrix (has a negative test: service A imports service B).
  - `tools/scripts/build-all.sh` calls correctly: `cargo build --workspace`, firmware build with `--target`, `uv sync`, `npm ci`.
  - Every script has `set -euo pipefail` and `--help`.

### RW-05 — IoT node (Rust/ESP32-C3)
- **Goal:** ESP32-C3 Cargo project builds + host tests; pure-Rust SNN LIF.
- **Owned:** `iot-node/**`.
- **Deliverables:** `iot-node/firmware/{Cargo.toml (nested workspace), rust-toolchain.toml, .cargo/config.toml, sdkconfig.defaults, partitions.csv}`; `src/{main.rs,lib.rs}`; crates `snn-lif` (no_std LIF, host-testable), `iomt-frames` (no_std), `sensor-max30102`, `sensor-mpu6050`, `signal-proc`, `ble-transport`, `power-mgmt`; `tests/host/`; `models/{snn_motion_v1.onnx, snn_motion_v1.bin, MODEL_CARD.md}`; `hardware/` complete pcb/bom/enclosure.
- **Interfaces:** `shared/protocols/jsonschema/ble-frame.v1.schema.json` → generates `iomt-frames`.
- **Reviewer acceptance:**
  - `test -f iot-node/firmware/Cargo.toml && test -f iot-node/firmware/rust-toolchain.toml && test -f iot-node/firmware/sdkconfig.defaults`.
  - `test ! -d iot-node/firmware/firmware-esp32c3`; no `CMakeLists.txt`/`idf_component.yml` remain.
  - `cargo check --manifest-path iot-node/firmware/Cargo.toml --target riscv32imc-esp-espidf` pass.
  - Host tests pass: `cargo test --manifest-path iot-node/firmware/Cargo.toml -p snn-lif -p iomt-frames`.
  - BLE frame test matches the sample `shared/protocols/examples/ble-frame.sample.json`.

### RW-06 — Edge-lite (Rust, running ON the Pi 3B+)
- **Goal:** **3 coarse** `edge-lite` service crates (`edge-ingest`, `edge-sync`, `model-agent`; D14) build/test independently, **cross-compile aarch64** and package a **multi-arch** image; total budget ≤ ~400 MB.
- **Owned:** `edge-gateway/edge-lite/**`.
- **Deliverables:**
  - `edge-lite/config/edge.default.yaml` (no secrets).
  - Rust crates (D14; split when needed, not before): `edge-lite/services/edge-ingest` (ble-ingest + signal-processor: tokio, btleplug; internal modules `ble`, `window`, `inference` with `tract`/`ort` ONNX int8), `edge-lite/services/edge-sync` (buffer-store + gateway-api: tokio, **`rusqlite` (SQLite)/`sled` — NOT Postgres**, axum + tonic; internal modules `queue`, `uplink`, `api`), `edge-lite/services/model-agent` (tokio, `tract`/`ort`, ed25519-dalek; verify/swap/rollback). Each crate has `Cargo.toml`, `src/`, `tests/`, `Dockerfile`.
  - Compose `mem_limit`/`cpus` per `01 §2.1` (K8s `resources.requests`/`resources.limits` in the dev-validation manifests); `restart: unless-stopped`; the BLE container (`edge-ingest`) needs `network_mode: host` + mount `/var/run/dbus` + host BlueZ (`01 §2.4`).
- **Interfaces:** `shared/protocols/proto/iomt/telemetry/v1`, `shared/protocols/jsonschema`; implement `gateway-api.openapi.yaml`; MQTT/AsyncAPI only if a later consumer needs it (`[LATER]`).
- **Reviewer acceptance:**
  - `for s in edge-ingest edge-sync model-agent; do test -f edge-gateway/edge-lite/services/$s/Cargo.toml && test -f edge-gateway/edge-lite/services/$s/Dockerfile; done`.
  - **Rust modules present (cheap-later-split rule, D14):** `for d in edge-gateway/edge-lite/services/edge-ingest/src/ble edge-gateway/edge-lite/services/edge-ingest/src/window edge-gateway/edge-lite/services/edge-ingest/src/inference edge-gateway/edge-lite/services/edge-sync/src/queue edge-gateway/edge-lite/services/edge-sync/src/uplink edge-gateway/edge-lite/services/edge-sync/src/api; do test -d "$d"; done`.
  - **Rust (host):** `cargo check -p edge-ingest -p edge-sync -p model-agent` pass; `cargo test -p ...` pass; `cargo clippy -p ... -- -D warnings` pass.
  - **Cross-compile Pi:** `cargo check --workspace --target aarch64-unknown-linux-gnu` pass (firmware is `exclude`d); every `edge-lite` crate builds for `aarch64-unknown-linux-gnu`.
  - **Image multi-arch:** `docker buildx build --platform linux/amd64,linux/arm64` passes for each `edge-lite` service.
  - `check-deps.sh` proves service A does not import service B's crate (only `iomt-protocol` is shared); no `edge-lite` crate pulls in Postgres/`sqlx-postgres` or a large web server; total limit ≤ ~400 MB.
  - model-agent test verifies the Ed25519 signature + SHA256 + rollback; **skips download when `sha256` is unchanged** (ETag/304) but **always** verifies Ed25519+SHA256 before applying.
  - `edge-ingest` (`inference` module): test **warm ONNX session** + **reuses tensor buffers without leaking the previous frame's data**.
  - `edge-sync` (`queue` module): defaults to **WAL + batched/group-commit** (batch ≤500 rows or fsync ≤500 ms); measures **MET-13** (SD write bytes/day + commit/s).
  - `edge-sync` (`queue/schema.sql`, `queue/maintenance.rs`): SQLite schema + PRAGMAs per `08 §3.4`; caps + tiered drop policy (**0 alert drops**); **`queue/dlq.rs`** `edge-sync dlq list|replay` subcommand; **idempotent replay test** — replaying the same `(device_id, window_start_ms, seq)` produces duplicate-safe rows; drain test **MET-41**; SD write volume + commit rate **MET-13** (MET-42 merged into MET-13; reserved/unused).
  - `edge-sync` (`src/tls/`, `src/enrol/`): presents the **client cert from `step-ca`** (cloud VPS), enrols/activates against the cloud CA, refuses to uplink until **chrony is within ± 120 s**, renews at day 60; **acceptance: `step-ca` issues a Pi client cert and the mTLS handshake to the cloud succeeds** (MET-36).
  - `edge-ingest` (`ble/handshake.rs`): time anchor + proto/fw negotiation; reports `mono0_us` + versions; measures **MET-40** (with MVP-03). `ble/dfu.rs` (BLE DFU) is **`[LATER]`** (MVP flashes by USB).
  - Each service exposes `/metrics`, `/healthz`, `/readyz` (loopback binding) and self-RSS `iomt_<component>_process_resident_memory_bytes`; `model-agent` (`ota/firmware.rs`) verifies `sha256`+Ed25519 and rolls back.
  - **Does NOT own:** `edge-gateway/edge-train/**` (RW-12), `deploy/**` (RW-10).

### RW-07 — Cloud backend (Rust axum + sqlx)
- **Goal:** backend crate builds/verifies + `sqlx migrate` on PostgreSQL/TimescaleDB + OpenAPI.
- **Owned:** `cloud/backend/**`, `cloud/README.md`.
- **Deliverables:** `Cargo.toml` (axum, sqlx, tonic, tokio); module `src/{api,domain,infra,config.rs}`; `migrations/<UTC timestamp>_<desc>.sql` (hypertable Timescale); `sqlx-data.json` (optional) for compile-time query check; integration tests (`sqlx::test`/Testcontainers Postgres+Timescale); Dockerfile.
- **Interfaces:** implement `shared/protocols/openapi/cloud-api.openapi.yaml`; uses `iomt-protocol`.
- **Reviewer acceptance:**
  - `cargo check -p cloud-backend` pass; `cargo clippy -p cloud-backend -- -D warnings` pass; `cargo test -p cloud-backend` pass.
  - `sqlx migrate run` initializes a clean hypertable on the Postgres container; `sqlx migrate info` has no pending/dirty migrations.
  - The OpenAPI exposed by the server (`/api-docs/openapi.json`, utoipa) matches `shared/protocols/openapi/cloud-api.openapi.yaml` (openapi-diff non-breaking).
  - `rg "password|secret" cloud/backend/src` contains no real values.
  - Migration enables **Timescale compression + retention**; load test **MET-14** p95 chart per 1h/24h/7d range; **do NOT** add a read cache; **CAGG (1 m/1 h/1 d, real-time) is MVP** per D18.
  - Migrations `*_{auth,audit_log,device_registry,consent,lifecycle,push}.sql` present and **apply cleanly** (`sqlx migrate run`); audit log append-only + hash chain (`07 §3.7`), consent records (`07 §3.8`); **acceptance: audit-log and consent migrations apply on a clean DB**.
  - `argon2id` login + rotating refresh (MET-32/33); **RBAC matrix test for the 3 MVP roles** `patient`/`clinician`/`admin` (caregiver/researcher `[LATER]`); audit completeness (MET-34); rate/body/header limits + security headers (MET-35/37); device registry enrol/revoke ≤ 60 s (MET-36); consent gate + **manual export/deletion runbook** (self-service endpoints `[LATER]`).
  - Server-side timestamp sanity + quarantine; **FCM clinical push with dedup/hysteresis** implemented (`cloud/backend/src/alerts/`), **FCM push dry-run** renders the alert; quiet-hours `[LATER]`.
  - `/healthz`+`/readyz` green; backend exports **`iomt_api_*` RED + `iomt_api_uplink_lag_seconds`**; postgres_exporter custom query exports `iomt_pg_cagg_lag_seconds` (MET-17).
  - **No** `pom.xml`/`mvnw`/`src/main/java/**`/`db/migration/**` exist.

### RW-08 — FL server (Python Flower)
- **Goal:** Flower server + strategies + registry/model signing.
- **Owned:** `cloud/fl-server/**`.
- **Deliverables:** `pyproject.toml` (uv); `src/fl_server/{app.py,strategies,registry,storage,security}`; tests (FedAvg/FedProx, simulated round loop); Dockerfile.
- **Interfaces:** `shared/protocols/proto/iomt/federation/v1`, `shared/protocols/proto/iomt/model/v1`; may use the Python bindings generated from the contract.
- **Reviewer acceptance:**
  - `uv run ruff check cloud/fl-server` + `uv run pytest cloud/fl-server` pass; the round-loop test meets MET-06 with mock clients.
  - `rg "cloud.backend|vn\.iomt\.backend" cloud/fl-server` is empty (no backend dependency).
  - Model publish has `sha256` + Ed25519 signature; test rejects a model with the wrong hash.

### RW-09 — Mobile app (React Native/TS)
- **Goal:** RN app scaffold + feature-based + test.
- **Owned:** `mobile-app/**`.
- **Deliverables:** `package.json`/`package-lock.json`/`tsconfig.json`; `src/app` navigation; `src/features/{auth,devices,measurements,alerts,models}`; `src/services` uses the OpenAPI-generated client; components/state/theme/utils; `__tests__`.
- **Interfaces:** `shared/protocols/openapi/cloud-api.openapi.yaml`, `shared/protocols/jsonschema/alert-event.v1.schema.json`.
- **Reviewer acceptance:**
  - `npx tsc --noEmit` pass; `npx eslint .` pass; `jest` pass.
  - No hand-written DTOs: `rg "interface .*Dto" mobile-app/src/services` (generated from the client).
  - TanStack Query: `staleTime` chart/latest = 30 s, alert `staleTime=0` + push; measures **MET-15**.
  - `npm ci` reproducible (has `package-lock.json`).

### RW-10 — Platform (ONE deployer `deploy/**`)
- **Goal:** runnable infrastructure + observability + security; **no** `deploy/` nested in a layer.
- **Owned:** `deploy/**` (only; `edge-gateway/deploy/**` does not exist).
- **Deliverables:** `compose/compose.dev.yml` (profiles: infra/edge/cloud/fl; image pin digest; Rust build context + Python fl-server) + `compose/compose.pi3.yml` (**PRIMARY** edge deployment on the Pi 3B+, `01 §2.2`); `compose/.env.example`; `compose/mosquitto/config/mosquitto.conf` with auth; `k8s/{edge,cloud}/{base,overlays}` + `helm/{edge,cloud}`; `observability/prometheus`+`grafana` [MVP], `observability/{loki,otel}` [LATER]; `security/certs` (dev) [MVP], `security/policies/{kyverno,opa}` [LATER]; `secrets/.sops.yaml` + sealed-secret template [LATER].
- **Reviewer acceptance:**
  - `docker compose -f deploy/compose/compose.dev.yml config` valid; `docker compose -f deploy/compose/compose.pi3.yml config` valid and the `up -d` smoke test passes (on the Pi or arm64 emulation); `docker compose ... up` starts the infrastructure.
  - `kubeconform deploy/k8s/**` pass; `helm lint deploy/helm/edge/* deploy/helm/cloud/*`; Helm/K8s manifests apply on a **dev K3s** (`helm template ... | kubectl apply --dry-run=server -f -`). **K3s-on-Pi = `[LATER]`.**
  - `test ! -d edge-gateway/deploy` (no second deployer).
  - **Observability (MVP):** `compose.cloud.yml` + `compose.pi3.yml` valid with healthchecks; Caddy serves `api`/`grafana` with automatic TLS; **Prometheus scrapes the Pi over the LAN/VPN (default; `vmagent` `[LATER]`)**; **all Prometheus targets up** (`up==1`); **4 Grafana dashboards provisioned from Git** (`{pi-health,edge-pipeline,cloud-api-red,timescaledb-e2e-latency}`); **≤ 8 operational alert rules** pass `promtool check rules` + `promtool test rules`; Pi footprint within limits (MET-20), observability footprint (MET-29); clinical alerts never enter Alertmanager (ADR-0013).
  - **Security (MVP):** `deploy/security/ca/` (public `root.pem` from **`step-ca`** + ceremony README); `deploy/secrets/.sops.yaml` encrypt/decrypts a sample; Pi Compose secret files land 0400 root; hardening runbook passes (MET-39); ports published: only `443` on the VPS, none on the Pi except `22`; **`step-ca` runs on the cloud VPS** (dev = `step` CLI).
  - **Reliability (MVP):** `deploy/pi/chrony.conf` deployed and **chrony sync check** passes (`chronyc tracking` within ± 120 s); `deploy/backup/{pgbackrest.conf,backup.sh,restore-drill.sh}` run nightly; **one documented pgBackRest backup+restore drill executed** (MET-46); digest-pinned update/rollback smoke; Prometheus rules for backup/queue/clock/CAGG/push.
  - `gitleaks detect` clean; no hardcoded credentials (`rg "iomt" deploy/compose/compose.dev.yml` no longer has a literal password).
  - Images have fixed tags/digests (no `latest`).

### RW-11 — Benchmarks & experiments (Python)
- **Goal:** reproducible MET measurement + training (PyTorch/Flower/ONNX).
- **Owned:** `benchmarks/**`, `experiments/**`.
- **Deliverables:** `benchmarks/{protocols,configs,scripts,fixtures,results}`; scripts measuring MET-02..MET-09 and **MET-13..MET-17** (`benchmarks/scripts/{sd_write,api,mobile,ble,tsdb}`: `iostat`/`fatrace` + SQLite `PRAGMA` for MET-13; k6 chart by range for MET-14; gateway req/user log for MET-15; btleplug stress for MET-16; `pg` size + CAGG refresh ts for MET-17) + synthetic fixtures; `experiments/{configs,notebooks,scripts,logs}`; results with `git_sha` + timestamp; uv `pyproject.toml` for each package.
- **Interfaces:** uses the contracts + CLI/API of RW-05..09, RW-12.
- **Reviewer acceptance:**
  - `uv run pytest benchmarks` (or `uv run python benchmarks/scripts/... --smoke`) pass with fixtures.
  - Scripts exist that measure all of **MET-13..MET-17** (`sd_write`, `api`, `mobile`, `ble`, `tsdb`).
  - Each results file contains metadata `git_sha`, `datetime`, `config_hash`.
  - `docs/specs/traceability.csv` points to the corresponding benchmark script for each MET.

### RW-12 — Edge-train (Python, powerful machine — does NOT run on Pi)
- **Goal:** Python package `edge_ml` + `local-trainer` (Flower client) build/test independently on a powerful `x86_64` machine; **not** part of the Pi image/workload.
- **Owned:** `edge-gateway/edge-train/**`.
- **Deliverables:** `edge-train/ml/` package `edge_ml` (transformers/ssl/datasets/export) + `edge-train/services/local-trainer` (Flower client) with `pyproject.toml`/`tests/`/`Dockerfile`.
- **Interfaces:** `shared/protocols/proto/iomt/model/v1`, `shared/protocols/proto/iomt/federation/v1` (Python bindings); `edge_ml` exports ONNX int8 for `edge-ingest`/`model-agent`.
- **Reviewer acceptance:**
  - `for p in edge-gateway/edge-train/ml edge-gateway/edge-train/services/local-trainer; do test -f "$p/pyproject.toml"; done`.
  - `uv run ruff check edge-gateway/edge-train/ml edge-gateway/edge-train/services/local-trainer` + `uv run pytest edge-gateway/edge-train/ml edge-gateway/edge-train/services/local-trainer` pass.
  - `rg "edge-train" deploy/k8s/edge/overlays/pi3 deploy/helm/edge/iomt-edge/values-pi3.yaml` is empty (edge-train does not go on the Pi).
  - **Does NOT own:** `edge-gateway/edge-lite/**` (RW-06), `deploy/**` (RW-10).

## 4. Interface dependency matrix between RWs (to avoid blocking)

| Producer | Consumer | Handoff interface |
|---|---|---|
| RW-01 | RW-05..09, RW-11, RW-12 | `shared/protocols/**` v0.1.0 + `shared/{rust,python,typescript}` bindings |
| RW-02 | all | root Cargo workspace + `rust-toolchain.toml` + `deny.toml` + uv workspace + CI |
| RW-04 | all | `tools/Makefile` targets + `tools/scripts/*` |
| RW-05 | RW-11 | telemetry frames conforming to `ble-frame.v1.schema.json` |
| RW-06 | RW-11 | `gateway-api.openapi.yaml` REST/gRPC (AsyncAPI/MQTT topics only `[LATER]`) |
| RW-07 | RW-09, RW-11 | `cloud-api.openapi.yaml` implementation + TS client generated |
| RW-08 | RW-06, RW-11 | `federation.proto`/`model.proto` + registry API |
| RW-12 | RW-06, RW-08 | `edge_ml` exports ONNX int8 + `local-trainer` (Flower client) ↔ `fl-server` |
| RW-10 | all | compose/k8s/observability/security config |
| RW-03 | all | spec/ADR template + `NFR.md` + `traceability.csv` |
| MVP-07 | MVP-04/05 | scrape contract: `<svc>:<port>/metrics`, `/healthz`, `/readyz`; **`step-ca` root + leaf issuance runbook** (client cert for edge, server cert for cloud) |
| MVP-02/07 | all | `step-ca` trust artifacts (`root.pem` + runbook, `07 §4.3`) and `sops`+`age` encrypted secrets |
| MVP-04 | MVP-05 | telemetry batch keyed `(device_id, window_start_ms, seq)` (at-least-once, idempotent) |
| MVP-03 | MVP-04 | BLE handshake anchor (`mono0_us`, fw/proto versions) as `firmware.v1` `LinkControl`; 16-byte records + cumulative `LinkAck{through_seq}` freeing the ring (08 §3.2b, REQ-REL-039..046); DFU frames `[LATER]` |
| MVP-07 | all | `pgbackrest` repo1/repo2 + Prometheus backup/queue/clock/CAGG rules |

## 5. Language ↔ verify command matrix (mandatory in every PR)

| Language | Format | Lint/Typecheck | Test | Security |
|---|---|---|---|---|
| Rust (host workspace) | `cargo fmt --all -- --check` | `cargo clippy --workspace --all-targets -- -D warnings` | `cargo test --workspace` | `cargo deny check`, `cargo audit` |
| Rust (firmware) | `cargo fmt --manifest-path iot-node/firmware/Cargo.toml -- --check` | `cargo clippy --target riscv32imc-esp-espidf` | `cargo test -p snn-lif -p iomt-frames` (host) | `cargo deny check` |
| Python | `ruff format --check` | `ruff check .` | `pytest` | `pip-audit` |
| TypeScript | `prettier --check` | `eslint .` + `tsc --noEmit` | `jest` | `npm audit` |
| Contracts | `buf format --diff` | `buf lint`, `spectral lint`, `ajv validate` | `buf breaking`, `openapi-diff`, codegen drift | — |
| Observability/ops | `promtool` (rules format) | `promtool check rules` + `promtool test rules`; `docker compose config`; Grafana provisioning diff | log PII scan, healthcheck smoke | MET-20..MET-29 |
| Security | `gitleaks` format | `gitleaks detect`, `cargo deny check`, `cargo audit`, `trivy` | `T-REQ-SEC-*` suite, RBAC table test, audit chain verify, `step-ca` issue + mTLS handshake | MET-30..MET-39 |
| Reliability | — | schema/PRAGMA assertions | outage (`iptables DROP`), `kill -9`, idempotent queue replay, restore drill, chrony sync, FCM push dry-run | MET-40..MET-49 |

## 6. Common definition of done (applies to every RW)
- [ ] Only touch files within the owned globs.
- [ ] `tools/scripts/lint-all.sh`, `check-deps.sh`, `check-traceability.sh` green.
- [ ] `cargo check --workspace` (host) green; does not break `cargo fmt`/`clippy -D warnings`/`test --workspace`.
- [ ] **Edge-lite (RW-06):** `cargo check --workspace --target aarch64-unknown-linux-gnu` green + `docker buildx build --platform linux/amd64,linux/arm64` green.
- [ ] `deploy/` is the only deployer; there is no `edge-gateway/deploy/**`.
- [ ] Two-person review: 1 owner + 1 reviewer from a different layer.
- [ ] No secret/PII in the diff (`gitleaks`).
- [ ] Contract change ⇒ update `shared/protocols/` + bump `shared/protocols/VERSION` + ADR if breaking.
- [ ] Every service exposes `/healthz`+`/readyz`+`/metrics` and declares a Compose healthcheck; new alert rules carry `domain="ops"`, a `for:`, and a `promtool test rules` case; dashboards change only via Git provisioning (MET-22/23/25).
- [ ] Gate G0 (`07 §3.12`) satisfied when touching auth/audit/device/privacy code: RBAC table test, audit-completeness test, revoke ≤ 60 s test, `gitleaks` green; Pi-affecting changes re-run the hardening checklist (MET-39).
- [ ] Any uplink/hypertable change re-runs the outage test (MET-41), the **idempotent queue replay** test, and updates the restore drill (MET-46); any migration is reversible and covered by a backup.

## 7. MVP wave plan (Phase 1 — demo E2E IoT→Edge→Cloud→Mobile on the Pi 3B+)

Phase 1 includes only the `[MVP]` items of `01`, **trimmed by §7.1** (final scopes from `06 §6`/`07 §6`/`08 §6` + the MVP trim). Merge the small packages (docs/tooling/root config) so that **exactly 7 work packages** remain, with non-overlapping ownership.

### 7.1 MVP scope trim rules (ADR-0020 "MVP scope trim rules")

Binding final scopes for the student MVP; anything outside is `[LATER]` (§7.6). No open decisions remain: **D15** cloud VPS Compose, **D16** own Rust `no_std` SNN, **D17** BLE, **D18** ~1 Hz summary + window rows (CAGGs are MVP), **D19** microSD with write minimisation.

**Security MVP**
- BLE LE Secure Connections + bonding; **Pi↔cloud mTLS with certificates issued by `step-ca` running on the cloud VPS** (dev uses the `step` CLI). The custom Rust CA **`tools/x509-ca/` is `[LATER]` and MUST NOT be built for the MVP**.
- Secrets with `sops`+`age`; user auth JWT + argon2id; RBAC with **3 MVP roles** (`patient`, `clinician`, `admin`); `caregiver`/`researcher` `[LATER]`.
- Append-only audit-log table; API rate limiting; consent-record table + **manual runbook** for export/deletion (self-service endpoints `[LATER]`); signed models (Ed25519); `cargo-deny`+`cargo-audit`+`trivy`+`gitleaks` in CI; Pi hardening checklist (SSH keys, ufw, unattended upgrades).
- `[LATER]`: secure boot/flash encryption, cosign, SBOM, secure aggregation/DP, read-only rootfs, hardware-backed keys.

**Observability MVP**
- Prometheus **scrapes** the Pi over the LAN/VPN (**default**); `vmagent`/`remote_write` on the Pi is `[LATER]` (only for Pis behind NAT) ⇒ the Pi runs **only `node_exporter` (≤ 20 MB)**.
- Grafana MVP **4** dashboards (`pi-health`, `edge-pipeline`, `cloud-api-red`, `timescaledb-e2e-latency`); the rest `[LATER]`. Alertmanager MVP **≤ 8 operational rules**; `/healthz`+`/readyz` on every service; `/metrics` on every service; Docker `json-file` log rotation; Caddy ingress with automatic TLS. Loki and OpenTelemetry `[LATER]`. **Clinical alerts never go through Alertmanager** (ADR-0013).

**Reliability MVP**
- `chrony` on the Pi; single UTC-ms timestamp model per `08`; idempotent at-least-once `edge-sync` queue + **dead-letter table**; Timescale hypertable + compression + retention + **continuous aggregates (D18)**; pgBackRest nightly + **one documented restore drill**; model OTA `sha256`+Ed25519 + rollback; FCM clinical push with dedup/hysteresis.
- `[LATER]`: ESP32 firmware OTA over BLE (MVP flashes by **USB**; `partitions.csv` stays **A/B-ready**), canary rollout, quiet-hours rules, SMS/e-mail fallback, RTC module.

**Resources / IDs**
- Pi RAM: the canonical budget lives in **`03 §7.1`** (3 services + `node_exporter` + Docker daemon; edge-lite total request 112 MB / limit 260 MB, operating cap ≤ ~400 MB, within the 1 GB device) — all other sections reference it instead of repeating the arithmetic.
- ID ranges: **MET-20..29** ops, **MET-30..39** security, **MET-40..49** reliability; **ADR-0011..0019** (docs 06/07/08) + **ADR-0020** (this trim block).

### 7.2 MVP work packages (7 total, non-overlapping globs)

Estimates (`Est. (PW)`) come from `10 §2`; 1 PW ≈ 20 h part-time student effort, tests + docs included; the high end is the honest one.

| MVP-WP | Name | Owns (globs) | Depends on | Est. (PW) | Minimum acceptance |
|---|---|---|---|---|---|
| MVP-01 | Contracts & generated bindings | `shared/**` | — | 2–3 | `buf lint` + `buf breaking` pass; `gen-contracts.sh --check` clean; `cargo check -p iomt-protocol` pass; OpenAPI declares `securitySchemes` (bearer JWT + client-cert) and all `07 §4.2` endpoints, `device_id` documented hashed; `firmware/v1/firmware.proto` + `push-registration.v1.schema.json` exist; telemetry documents `(device_id, window_start_ms, seq)` idempotency + UTC-ms semantics <br>**Status 2026-10-12:** `telemetry/v1` + `firmware/v1` protos, `buf.yaml` (lint STANDARD / breaking FILE), `.txtpb` examples, committed Rust bindings via `shared/rust/codegen` + CI "Contracts up to date" done; **not yet frozen/tagged `v0.1.0`**. `firmware.v1` `LinkReport.boot_id` added 2026-10-13 (dedupe on `(device_id, boot_id, seq)`). **Blocking v0.1.0:** decide whether `telemetry.v1` `RecordKey` gets `boot_id` or `edge-sync` assigns its own `seq` (08 §3.2b.9 #8). Open: OpenAPI, JSON Schemas (`ble-frame.v1`, `push-registration.v1`), `model`/`alert` protos. Deviation: Rust codegen uses the in-repo `iomt-codegen` crate, not `buf.gen.yaml` (see `01 §4.3`). |
| MVP-03 | IoT node (firmware ESP32-C3 + host tests) | `iot-node/**` | MVP-01 freeze | 6–9 | `cargo check --manifest-path iot-node/firmware/Cargo.toml --target riscv32imc-esp-espidf` pass; host tests `snn-lif`/`iomt-frames` pass; BLE SC-only bonding (REQ-SEC-007), no long-term secret in NVS; `partitions.csv` A/B-ready (`ota_0`/`ota_1`/`otadata`), USB flashing; handshake reports `mono0_us` + fw/proto versions; **recorded frame capture checked in under `iot-node/firmware/test/fixtures/`** (feeds the `edge-ingest --replay` path, F-08); **HIL milestone: one node flashed over USB, BLE-bonded, streaming real frames to a Pi for ≥ 10 min, MET-40 numbers recorded**; BLE DFU + secure boot/flash encryption `[LATER]` <br>**Status 2026-10-12:** `iomt-frames` (16-byte record + CRC-8) done with host tests. `offline-buffer` (D21, 08 §3.2b): `RingBuffer` + `Courier` done by the user, 11 unit tests. **Open decision:** the firmware cannot use `iomt-protocol` as is (pulls `tonic`) → prost-only `firmware.v1` crate or a feature flag (08 §3.2b.9 #7). `tools/device-sim` in progress (physio, TCP framing and gateway stub done, 17 tests; device loop, CLI next). **Not implemented:** the device link loop, sensor drivers, `ble-transport`, ESP-IDF build, HIL milestone, and the **recorded fixtures** under `test/fixtures/`. |
| MVP-04 | Edge-lite (**3 coarse** Rust services `edge-ingest`/`edge-sync`/`model-agent`, Pi 3B+ aarch64, ≤ ~400 MB, SQLite; D14) | `edge-gateway/edge-lite/**` | MVP-01 freeze (+ dev `step` CLI) | 6–9 | `cargo check --workspace --target aarch64-unknown-linux-gnu` pass; **arm64 image build** (`docker buildx build --platform linux/amd64,linux/arm64`); **compose smoke test** (`compose.pi3.yml config` valid + `up -d` on Pi/arm64); `edge-sync` uses SQLite not Postgres; modules `ble`/`window`/`inference` + `queue`/`uplink`/`api`; **`edge-ingest --replay <fixture>` mode drives window→inference→queue→uplink from recorded fixtures without BLE hardware**; `queue/schema.sql`+`maintenance.rs` per `08 §3.4` with caps + **0 alert drops**; **`edge-sync dlq replay` idempotent replay test** (same `(device_id, window_start_ms, seq)` ⇒ no duplicate); `tls/`+`enrol/`: **`step-ca` issues a Pi client cert and the mTLS handshake succeeds**, uplink refused until chrony within ± 120 s; drain MET-41 / SD **MET-13** (MET-42 merged into MET-13, reserved/unused); `/metrics`+`/healthz`+`/readyz` loopback; `model-agent` **always** verifies Ed25519+SHA256; measures MET-13/36/40 |
| MVP-05 | Cloud backend (Rust axum + sqlx + TimescaleDB) | `cloud/backend/**`, `cloud/README.md`, `docs/runbooks/security-check.md` | MVP-01 freeze (+ `step-ca` root from MVP-07, out-of-band) | 6–9 | `cargo test -p cloud-backend` pass; `sqlx migrate run` clean; **audit-log and consent migrations apply** (**milestone M-a: audit-log hash-chain + completeness verified separately**, MET-34, F-18); hypertable + CAGG(1 m/1 h/1 d, real-time) + compression(7 d) + retention(180 d) (D18, MET-44/45); JWT + argon2id (MET-32/33); **RBAC matrix test for the 3 MVP roles**; rate/body/header + security headers (MET-35/37); device registry revoke ≤ 60 s (MET-36); consent gate + **manual export/delete runbook** (self-service `[LATER]`); timestamp sanity + quarantine; **FCM dedup/hysteresis + dry-run** (MET-48), quiet-hours `[LATER]`; server mTLS on 9443; `/healthz`+`/readyz` + `iomt_api_*` RED |
| MVP-06 | Mobile app (React Native TS) | `mobile-app/**` | MVP-05 (openapi client + push) | 3–4 | `tsc --noEmit` + `eslint` + `jest` pass; no hand-written DTOs; TanStack Query `staleTime` chart/latest = 30 s, alert `staleTime=0`; **FCM token registered/unique on app start + alert ack UI**; **Firebase dev project + FCM push dry-run renders the clinical alert** (dev project note, §7.7); no PHI in push payloads; measures MET-15/48; quiet-hours settings `[LATER]` |
| MVP-07 | Platform deploy (ONE `deploy/` owner; Pi compose + 4 dashboards + ops alerts; step-ca; backups) | `deploy/**`, `docs/runbooks/**` (excluding `docs/runbooks/security-check.md`) | MVP-01, MVP-02, MVP-05 (step-ca + Postgres) | 5–7 | `compose.cloud.yml`+`compose.pi3.yml` valid + `up -d` smoke; Caddy auto-TLS (**dev TLS path via internal `step` CA or documented dev domain, §7.7**); **Prometheus scrapes the Pi over LAN/VPN (`vmagent` `[LATER]`)**; **all targets up** (`up==1`); **4 Grafana dashboards provisioned** (MET-23); **≤ 8 ops rules lint + `promtool test rules`** (MET-25); Pi only `node_exporter` ≤ 20 MB (footprint MET-20/29, per `03 §7.1`); **`step-ca` on the VPS issues Pi certs** (**milestone M-b: mTLS enrol + day-60 renewal verified end-to-end**, MET-36, F-18); sops+age sample + 0400 Pi secrets; hardening runbook MET-39; `chrony` + **chrony sync check** ± 120 s; **pgBackRest backup+restore drill documented/executed** (MET-46); `test ! -d edge-gateway/deploy`; **K3s-on-Pi `[LATER]`** |

**Effort model.** Total **31–45 PW** across the 7 packages; security work inside MVP-04/05/07 accounts for **≈ 4–6 PW** of that total (the earlier "security gate ≈ 10 working days" figure is withdrawn as understated). The **audit-log hash-chain** (MVP-05, milestone M-a) and **mTLS enrol/renewal** (MVP-04 ↔ MVP-07 `step-ca`, milestone M-b) are separate acceptance milestones, not lumped into the package sign-off. With **1 person** the package is ~31–45 weeks (≈ 8–11 months) — it does not fit a 6–9-month part-time capstone; plan for **≥ 2 people** or enforce the minimum slice in §7.8.

### 7.3 New path → single MVP owner (from `06 §4.3`, `07 §4.3`, `08 §4.3`; trim applied)

Every new/changed path below belongs to **exactly one** MVP-WP; no glob overlaps.

| New/changed path | Tag | Owner |
|---|---|---|
| `shared/protocols/openapi/cloud-api.openapi.yaml` (`securitySchemes` + `07 §4.2` endpoints) | `[MVP]` | MVP-01 |
| `shared/protocols/proto/iomt/firmware/v1/firmware.proto` | `[MVP]` | MVP-01 |
| `shared/protocols/jsonschema/push-registration.v1.schema.json` | `[MVP]` | MVP-01 |
| `.gitleaks.toml`; `.github/workflows/security-scan.yml` (gitleaks+trivy+cargo-audit+cargo-deny) | `[MVP]` | MVP-02 |
| `tools/scripts/pi-bootstrap.sh` | `[MVP]` | MVP-02 |
| `tools/device-sim/` (in progress: simulated wearables + dev gateway stub over TCP; 08 §3.2b.8) | `[MVP]` | MVP-02 |
| `docs/security/{SECURITY_MODEL,THREAT_MODEL,KEY_MANAGEMENT}.md`; `docs/governance/PRIVACY.md` | `[MVP]` | MVP-02 |
| `tools/x509-ca/` | `[LATER]` | MVP-02 (full-scope RW-04, `tools/**`) |
| `iot-node/firmware/partitions.csv` (A/B-ready) | `[MVP]` | MVP-03 |
| `iot-node/firmware/crates/offline-buffer/` (ring buffer + courier, done; D21) | `[MVP]` | MVP-03 |
| `iot-node/firmware/test/fixtures/` (recorded frame captures for hardware-free pipeline runs; F-08) | `[MVP]` | MVP-03 |
| `iot-node/firmware/crates/ota-client/` (BLE DFU) | `[LATER]` | MVP-03 |
| `edge-gateway/edge-lite/services/edge-sync/src/queue/{schema.sql,dlq.rs,maintenance.rs}` | `[MVP]` | MVP-04 |
| `edge-gateway/edge-lite/services/edge-sync/src/{tls,enrol}/` | `[MVP]` | MVP-04 |
| `edge-gateway/edge-lite/services/edge-ingest/src/ble/handshake.rs` | `[MVP]` | MVP-04 |
| `edge-gateway/edge-lite/services/edge-ingest` replay mode (`--replay <fixture>`, reads `iot-node/firmware/test/fixtures/`; F-08) | `[MVP]` | MVP-04 |
| `edge-gateway/edge-lite/services/edge-ingest/src/ble/dfu.rs` | `[LATER]` | MVP-04 |
| `edge-gateway/edge-lite/services/model-agent/src/ota/firmware.rs` | `[MVP]` | MVP-04 |
| `cloud/backend/migrations/*_{auth,audit_log,device_registry,consent,lifecycle,push}.sql` | `[MVP]` | MVP-05 |
| `cloud/backend/src/alerts/` (dedup/hysteresis/FCM sender) | `[MVP]` | MVP-05 |
| `docs/runbooks/security-check.md` | `[MVP]` | MVP-05 |
| `mobile-app` FCM register/ack + alert ack UI | `[MVP]` | MVP-06 |
| `mobile-app` quiet-hours settings | `[LATER]` | MVP-06 |
| `deploy/compose/compose.cloud.yml`; `deploy/compose/compose.pi3.yml` (edit) | `[MVP]` | MVP-07 |
| `deploy/observability/prometheus/{prometheus.yml,rules/{edge,cloud,ops,backup}.rules.yml,tests/{edge,cloud,ops,backup}.test.yml}` | `[MVP]` | MVP-07 |
| `deploy/observability/prometheus/rules/slo.rules.yml` + `tests/slo.test.yml` | `[LATER]` | MVP-07 (SLO recording rules/dashboard deferred with MET-28; CR-15) |
| `deploy/observability/grafana/provisioning/**` + `dashboards/{pi-health,edge-pipeline,cloud-api-red,timescaledb-e2e-latency}.json` | `[MVP]` | MVP-07 |
| `deploy/observability/alertmanager/{alertmanager.yml,templates/telegram.tmpl}` | `[MVP]` | MVP-07 |
| `deploy/observability/node-exporter/{textfile/.gitkeep,collect-container-stats.sh}`; `deploy/observability/blackbox/blackbox.yml` | `[MVP]` | MVP-07 |
| `deploy/ingress/Caddyfile`; `deploy/systemd/iomt-container-stats.{service,timer}` | `[MVP]` | MVP-07 |
| `deploy/pi/chrony.conf`; `deploy/backup/{pgbackrest.conf,backup.sh,restore-drill.sh}` | `[MVP]` | MVP-07 |
| `deploy/security/ca/{README.md,root.pem}` (public cert from `step-ca`) | `[MVP]` | MVP-07 |
| `deploy/secrets/{.sops.yaml,*_ENC.yaml,pi/}` | `[MVP]` | MVP-07 |
| `docs/runbooks/{deploy-edge,ota-rollback,observability,secret-rotation,pi-hardening,backup-restore,queue-replay,pi-disaster-recovery}.md` (`ota-rollback.md` extended to cover firmware OTA sign-off) | `[MVP]` | MVP-07 |
| `docs/runbooks/{incident,firmware-ota-rollback}.md` | `[LATER]` | MVP-07 |
| `deploy/observability/vmagent/**` | `[LATER]` | MVP-07 |
| `deploy/observability/{loki,otel}/`; `dashboards/{device-fleet-ble,fl-training}.json`; `deploy/secrets/templates/sealed-secret.example.yaml` | `[LATER]` | MVP-07 |

### 7.4 MVP wave plan and dependency order (minimal serialization)

```text
Wave M0 (parallel): MVP-01 Contracts ──freeze v0.1.0──┐  |  MVP-02 Foundation
Wave M0.5 (time-boxed spikes; F-11): (a) BLE-in-container smoke (BlueZ/D-Bus via host network), (b) rustls + step-ca handshake, (c) sqlx + Timescale migration, (d) int8 ONNX inference on the Pi — each ≤ 3 days, each with a documented go/no-go fallback
Wave M1 (parallel): MVP-03 IoT | MVP-04 Edge-lite (Pi) | MVP-05 Cloud | MVP-06 Mobile | MVP-07 Platform
Wave M2 (synchronization): demo E2E IoT→Edge→Cloud→Mobile on the Pi 3B+ (Docker Compose via compose.pi3.yml; K3s on the Pi = [LATER])
```

**Critical path:** MVP-01 freeze → MVP-02 foundation → MVP-05 cloud (enrol API + storage) → MVP-04 mTLS end-to-end → MVP-07 `step-ca` + deploy → Wave M2 E2E. Serialized length ≈ **22–30 PW**. After MVP-01/MVP-02, `{MVP-03, MVP-04, MVP-05, MVP-06}` can run concurrently; MVP-07 needs MVP-05. With 1 person the full package is ~31–45 weeks (≈ 8–11 months) — plan for **≥ 2 people** or enforce the minimum slice (§7.8).

Dependency order (soft edges; start against a dev CA / local stack, harden once upstream lands):
1. **MVP-01 freeze v0.1.0** — the synchronization milestone; unblocks MVP-03..07.
2. **MVP-02 foundation** — root config, CI (`gitleaks`/`cargo-deny`/`cargo-audit`/`trivy`), `pi-bootstrap.sh`.
3. **MVP-05 cloud + MVP-07 platform** in parallel; field the `step-ca` root (out-of-band) so MVP-04 can enrol.
4. **MVP-04 edge-lite** — dev `step` CLI first, then enrol against the cloud `step-ca`; depends on MVP-05 enrol API for the E2E mTLS handshake.
5. **MVP-03 IoT** — depends on MVP-01 freeze; handshake anchor feeds MVP-04 MET-40.
6. **MVP-06 mobile** — depends on MVP-05 OpenAPI client + FCM push endpoints.
7. **Wave M2** — E2E demo; `step-ca` cert issuance, mTLS handshake, 4 dashboards, ≤ 8 ops alerts, backup/restore drill, chrony sync, idempotent queue replay, and FCM dry-run are the sign-off checks.

### 7.5 MVP boundary (ensuring non-overlapping)
- **No** MVP-WP touches `edge-gateway/edge-train/**`, `cloud/fl-server/**`, `benchmarks/**`, `experiments/**` — these paths are `[LATER]` (RW-08/RW-11/RW-12).
- `deploy/**` is owned only by **MVP-07**; layers keep only their own `Dockerfile`.
- `shared/**` is owned only by **MVP-01**; MVP-02 only **reads** contracts to configure CI.
- `docs/**` is owned by **MVP-02** **except** `docs/runbooks/**` (MVP-07) and `docs/runbooks/security-check.md` (MVP-05).
- Gaps check (must be empty): `comm -12 <(list MVP-02..07 globs) <(list MVP-01..07 globs)`; every path in §7.3 resolves to exactly one of MVP-01..MVP-07.

### 7.6 LATER backlog (everything tagged `[LATER]`, does not block the demo)

- **Whole packages:** edge-train/RW-12 (`edge-gateway/edge-train/**`), FL server/RW-08 (`cloud/fl-server/**`), benchmarks/RW-11, experiments/RW-11.
- **Security:** `tools/x509-ca/` custom Rust CA (superseded by `step-ca`); secure boot/flash encryption/anti-rollback; cosign; SBOM; secure aggregation/DP; read-only OS rootfs; hardware-backed keys; `caregiver`/`researcher` roles; self-service DSAR export/delete endpoints; CRL/OCSP; certificate pinning (mobile); per-tenant DB roles; at-rest encryption; external notarisation; IRB package; pseudonymised research export; sealed-secrets template; WireGuard/fail2ban/auditd; firmware boot-time signature verification; `docs/runbooks/incident.md`.
- **Observability:** `vmagent`/`remote_write` on the Pi (NAT case); Loki; OpenTelemetry; pull scraping over a VPN mesh; the extra dashboards (`device-fleet-ble`, `fl-training`) and any beyond the 4; alert rules beyond the 8-rule MVP cap (incl. SLI/SLO burn-rate alert + SLO dashboard + `slo.rules.yml`/`slo.test.yml`); Thanos/VictoriaMetrics long-term retention; Grafana alerting/on-call beyond Telegram/email; cAdvisor (rejected).
- **Reliability:** ESP32 firmware OTA delivery over BLE / `ble/dfu.rs` (MVP flashes by USB; `partitions.csv` A/B-ready; firmware OTA sign-off runbook is the `[MVP]` `ota-rollback.md`, the standalone `firmware-ota-rollback.md` is `[LATER]`); canary/staged rollout; quiet-hours rules; SMS/e-mail fallback; info-severity daily digest; OneSignal; DS3231 RTC; BLE Current Time Service; CAGG retention policies; golden Pi image / automated enrolment; automatic rollout analysis/abort; 4 USB BLE adapters; offsite restore ≤ 4 h; SSD/USB storage; K3s-on-Pi; MQTT/AsyncAPI.

### 7.7 Demo prerequisites (F-09 — environment gates before Wave M2)

Wave M2 stalls on environment setup unless these exist; each has exactly one owner:
1. **Seed data (MVP-02):** `tools/scripts/seed-dev.sh` seeds demo users (patient/clinician/admin), a device registry entry, and sample measurements into the cloud DB so login + charts work on a fresh stack.
2. **Dev TLS (MVP-07):** a documented dev path for Caddy automatic TLS — either the internal `step` CA / `step-ca` (same trust chain as production, dev hostname) or a documented public dev domain (e.g. `sslip.io`-style) with reachable `:80/:443`; production ACME stays the default.
3. **Firebase dev project (MVP-06):** a dedicated Firebase **dev project** + FCM server key (sops-encrypted via MVP-07) + a device/emulator with `google-services.json`; the FCM dry-run must fire against this dev project before Wave M2 sign-off.
4. **Hardware (MVP-03/04):** one Pi 3B+ + ≥ 1 ESP32-C3 node for the HIL milestone; arm64 images buildable via `docker buildx` without a Pi.

### 7.8 Minimum viable E2E slice and sprint plan (F-07/F-08, from `10 §3`)

Goal: **one ESP32-C3 node → Pi edge-lite → cloud → mobile alert** first, then harden to the full ADR-0020 MVP. The slice is a **build order, not a scope cut**: items sequenced late below (CAGGs, FCM, the remaining dashboards, backups) stay `[MVP]` and must land before Wave M2 sign-off; whole `[LATER]` packages (FL, edge-train, `tools/x509-ca`, `vmagent`/Loki/OTel, extra roles, MQTT) stay out per §7.6.

**Slice path (first demonstrable E2E):** MVP-01 (contracts reduced to telemetry + firmware + auth) → MVP-02 (workspace + CI) → MVP-03 (one node, vitals only) → MVP-04 (`edge-ingest` BLE → `edge-sync` SQLite outbox → uplink; plain HTTP first) → MVP-05 (ingest + latest + auth + one alert rule) → MVP-06 (latest + one alert screen; polling before FCM) → MVP-07 (`compose.pi3`/`compose.cloud` + Caddy + 1 dashboard + email alert). Hardware-free iteration uses `edge-ingest --replay` fixtures.

**Sprint plan (indicative, 2 people, ~2–3 weeks/sprint; 1 PW ≈ 20 h):**

| Sprint | Weeks | Milestone |
|---|---|---|
| S1 | 1–3 | MVP-01 freeze + MVP-02 CI green; dev stack boots locally (`compose.dev`), `sqlx migrate` runs, auth login returns JWT. |
| S2 | 4–6 | MVP-05 ingest + latest + one alert rule; MVP-04 `edge-sync` uplinks **over plain HTTP first** to the cloud; `--replay` fixture proves ingest without hardware. |
| S3 | 7–9 | MVP-03 node streams over BLE to `edge-ingest` (**HIL milestone**, F-19); MVP-06 shows latest vitals; **1-node E2E on the Pi**. |
| S4 | 10–12 | Swap HTTP→mTLS (`step-ca`, milestone M-b); add Grafana dashboards toward the 4-dashboard set; email alert fires; **E2E demo recorded**. |
| S5 (buffer) | 13–15 | Backup+restore drill, runbooks, hardening, 5-node soak, CAGGs (D18), FCM push (after demo basics, per MVP-06) — full ADR-0020 MVP acceptance. |

## 8. 5-line summary

1. §1/§3 keep the full-scope `RW-xx` table and note the MVP trim supersedes tags; `tools/x509-ca` is LATER (step-ca wins).
2. §7.1 records the binding MVP trim (step-ca mTLS, 4 dashboards, ≤ 8 ops rules, node_exporter-only Pi, USB flashing, 3 RBAC roles) as **ADR-0020**, IDs MET-20..49 / ADR-0011..0020.
3. §7.2/§7.3 fold every new path from docs 06/07/08 into exactly one of MVP-01..MVP-07 with non-overlapping globs, explicit acceptance checks, and per-package effort estimates; replay fixtures, the HIL milestone, and the runbook/rules reconciliations are included.
4. §4/§5/§6 add the scrape-contract and step-ca interface rows, the ops/security/reliability verify commands, and the health/alert/Gate-G0/outage DoD.
5. §7.4/§7.5/§7.6 give the wave + dependency order (incl. the M0.5 spike wave and critical path), the non-overlap boundary, and a clearly marked LATER backlog; §7.7/§7.8 add the demo prerequisites and the minimum viable E2E slice + sprint plan; §7.1 removes the D15–D19 open-decision tags.
