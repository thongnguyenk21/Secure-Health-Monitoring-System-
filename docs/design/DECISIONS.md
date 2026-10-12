# Locked Decisions

| # | Date | Decision | Rationale |
|---|---|---|---|
| D1 | 2026-10-09 | Rust (Cargo workspace) for ESP32-C3 firmware, all edge services, cloud backend (axum + sqlx). Drop Spring Boot/Flyway. | User wants to learn Rust; shared types, lightweight for Pi |
| D2 | 2026-10-09 | Python only for ML: PyTorch, SSL, Flower client/server, benchmarks, experiments (uv) | No practical Rust option for training/Flower |
| D3 | 2026-10-09 | Mobile keeps React Native (TypeScript) | User chose to keep the familiar stack |
| D4 | 2026-10-09 | `shared/protocols` (Protobuf + OpenAPI + JSON Schema) is the cross-language contract | Multiple languages need a single source of truth |
| D5 | 2026-10-09 | Develop on Linux x86_64 (Docker + K3s) first, then move to Raspberry Pi 3B+ (1GB RAM, aarch64) | Real-world deployment target |
| D12 | 2026-10-09 | Code documentation standard: every public item gets a brief summary plus param/return/error sections in the language's native format (rustdoc, Google-style docstrings, TSDoc); enforced in CI. Spec: docs/engineering/CODING_CONVENTIONS.md | User request |
| D21 | 2026-10-12 | Wearable offline mode (option A): the ESP32 always writes every sample into an on-device ring buffer first; while the BLE link to the Pi is up the buffer is drained continuously (near empty), while out of range it accumulates and overwrites the OLDEST entries when full (dropped counter kept); on reconnect the backlog is sent oldest-first and records are removed only after the Pi acknowledges their sequence number (at-least-once; the Pi/cloud de-duplicate by idempotency key). Timestamps use the device monotonic clock plus a time anchor given at reconnect. Phone-as-gateway (option B) is LATER. | User choice 2026-10-12: device must keep working when worn outside Pi range |
