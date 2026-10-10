# edge-ingest

Edge-lite service (Rust, Pi 3B+, Docker Compose). Work package: MVP-04. Tag: [MVP].

Replaces the former `ble-ingest` + `signal-processor` (decision D14: three coarse services; split later only when needed).

- **Purpose:** receive BLE GATT frames from the ESP32-C3 node, build 30-120 s windows, run int8 ONNX inference, emit ~1 Hz summary vitals and per-window feature rows to `edge-sync`.
- **Modules:** `ble` (btleplug/BlueZ, bonding), `window` (windowing + normalisation), `inference` (tract / ort, warm session).
- **Budget:** 40 MB request / 112 MB limit (see the Pi memory budget in the spec standard).
- **Testability:** `--replay <fixture>` mode reads recorded frames from `iot-node/firmware/test/fixtures/` so the pipeline runs without hardware.
- **Endpoints:** `/healthz`, `/readyz`, `/metrics`.

No code yet (decision D10).
