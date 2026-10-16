# Cloud

Rust backend + TimescaleDB, Flower FL server and Model Registry (D1, D15, D22–D33).

## Components
- `backend/`: Rust (axum, sqlx, tonic). REST API for the apps, gRPC telemetry ingest, alert engine. See `backend/README.md`.
- `fl-server/`: Flower server (FedAvg/FedProx); not started yet.

The clinician and admin web app is in `../web-app/`.
