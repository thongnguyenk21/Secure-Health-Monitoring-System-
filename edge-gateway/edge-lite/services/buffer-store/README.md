# edge-lite/service `buffer-store`

- **Purpose:** Durable local buffer for telemetry. Uses SQLite/embedded store — never Postgres on the Pi.
- **Tier:** `[T1/Edge]`
- **Scope:** `[MVP]`
- **Owner:** RW-06 (MVP-04)
- **Tech:** Rust `tokio`, `rusqlite` (SQLite) or `sled`.
