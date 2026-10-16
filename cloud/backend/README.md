# cloud/backend — Rust (axum + sqlx + tonic)

Cloud backend of the IoMT system (MVP-05): REST API for the web and mobile apps,
gRPC telemetry ingest for gateways, the alert engine and TimescaleDB storage.

- **Contracts:** `shared/protocols/openapi/cloud-api.openapi.yaml` (0.2.0) and
  `iomt.telemetry.v1` (`shared/protocols/proto`).
- **Owner:** RW-07 (MVP-05). **Scope:** slice S1–S2 of the demo plan.

## Run locally

```bash
# 1. TimescaleDB (pinned image)
docker compose -f deploy/compose/compose.dev.yml up -d timescaledb

# 2. Device ids of the simulated wearables (same key as the gateway below)
KEY=$(printf '5a%.0s' {1..32})
cargo run -p device-sim -- ids --id-key-hex $KEY --count 3

# 3. Database, dev accounts and wearables
export DATABASE_URL=postgres://iomt:iomt@127.0.0.1:5432/iomt
cargo run -p iomt-backend -- seed-dev --device-id <id1> --device-id <id2> --device-id <id3>

# 4. Backend: REST on :8080, gRPC ingest on :9443
CORS_ORIGIN=http://localhost:5173,http://localhost:19006 cargo run -p iomt-backend

# 5. Gateway stub that uploads to the backend, and three wearables
cargo run -p device-sim -- gateway --id-key-hex $KEY --cloud http://127.0.0.1:9443
cargo run -p device-sim -- device --count 3 --scenario high-hr
```

Seeded accounts (development only, password `iomt-dev-password`):
`admin@iomt.local`, `bs.minh@iomt.local` (clinician), `bn017@iomt.local` … (patients BN-017, BN-031, …).

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | required | PostgreSQL with TimescaleDB |
| `HTTP_ADDR` | `0.0.0.0:8080` | REST API and `/api/v1/live` WebSocket |
| `GRPC_ADDR` | `0.0.0.0:9443` | `TelemetryIngestService` |
| `JWT_ED25519_PKCS8_B64` | generated | Access token signing key; without it sessions end on restart |
| `CORS_ORIGIN` | none | Allowed browser origins in development, comma-separated (web app :5173, Expo web :19006) |
| `TENANT_ID` | `00000000-…-0001` | Tenant written into tokens |

## Layout

| Path | Content |
|---|---|
| `migrations/` | Schema: users, refresh tokens, devices, `vitals` and `windows` hypertables (compression 7 d, retention 180 d), quarantine, alerts, clinical notes, consents, models, hash-chained audit log |
| `src/api/` | REST handlers, one file per resource; `extract.rs` has the caller, paging and JSON-error extractors |
| `src/ingest.rs` | gRPC ingest: validation, idempotent inserts on `RecordKey`, quarantine, device gate |
| `src/alerts/` | `rules.rs` (pure hysteresis rules), `engine.rs` (DB transitions, offline detection, escalation, live events) |
| `src/infra/` | JWT (Ed25519), argon2id, rotating refresh tokens, audit chain, rate limiter |
| `src/seed.rs` | Development seed |

## Behaviour worth knowing

- **Alert rules** (placeholders until clinicians set them): `hr_high` > 140 bpm for 10 s, clears < 130 bpm for 30 s, critical ≥ 170; `hr_low` < 40 / > 45, critical ≤ 35; `spo2_low` < 90 % / > 92 %, critical ≤ 85 %. Samples with signal quality < 50 are ignored. One unresolved alert per device and type.
- **Offline and escalation:** every 30 s, an assigned wearable silent for 5 min opens `device_offline`; an unacknowledged warning becomes critical after 10 min; an unacknowledged critical alert is marked escalated after 2 min (FCM paging lands in S5).
- **Privacy:** admins get `403` on measurements, patients and alerts; every health-data read is audited.
- **Not yet:** gateway mTLS on the gRPC port and node activation (S4: a pending wearable becomes active on its first valid upload), FCM push, continuous aggregates (rollups use `time_bucket` on the raw table), battery in telemetry.

## Checks

```bash
cargo fmt --all -- --check
cargo clippy -p iomt-backend --all-targets -- -D warnings
cargo test -p iomt-backend
```
