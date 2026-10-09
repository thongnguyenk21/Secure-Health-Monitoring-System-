# edge-lite/service `ble-ingest`

- **Purpose:** Ingest BLE telemetry from IoT nodes; publish to the internal bus.
- **Tier:** `[T1/Edge]`
- **Scope:** `[MVP]`
- **Owner:** RW-06 (MVP-04)
- **Tech:** Rust `tokio`, `btleplug`, `rumqttc`, `iomt-protocol`. Needs BlueZ/D-Bus + `hostNetwork`.
