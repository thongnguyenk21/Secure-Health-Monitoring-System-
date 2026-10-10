# edge-sync

Edge-lite service (Rust, Pi 3B+, Docker Compose). Work package: MVP-04. Tag: [MVP].

Replaces the former `buffer-store` + `gateway-api` (decision D14).

- **Purpose:** durable store-and-forward queue in SQLite (WAL), idempotent at-least-once upload to the cloud over gRPC with mTLS, dead-letter table, local internal API.
- **Modules:** `queue` (SQLite outbox, idempotency key `(device_id, window_start_ms, seq)`), `uplink` (gRPC client, backoff, clock-sync gate), `api` (loopback health/queue stats).
- **Rules:** alerts are never dropped; oldest summary vitals are dropped first when the queue is full.
- **Budget:** 32 MB request / 64 MB limit (see `03-spec-standard.md` section 7.1).
- **Endpoints:** `/healthz`, `/readyz`, `/metrics`.

No code yet (decision D10).
