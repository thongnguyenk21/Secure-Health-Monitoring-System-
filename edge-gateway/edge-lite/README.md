# edge-lite — runs on Raspberry Pi 3B+ (aarch64)

- **Purpose:** The lightweight edge tier deployed on the Pi 3B+ (BLE ingest, signal processing, buffering, gateway API, model agent). Total RAM budget ≤ ~400 MB.
- **Tier:** `[T1/Edge]`
- **Scope:** `[MVP]`
- **Owner:** RW-06 (MVP-04)
- **Tech:** Rust (`tokio`, `axum`, `tonic`, `btleplug`, `rumqttc`, `rusqlite`/`sled`, `tract`/`ort`), multi-arch images `linux/amd64,linux/arm64`.
