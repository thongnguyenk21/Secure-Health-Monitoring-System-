# Contracts changelog

Versions of the machine-readable contracts in `shared/protocols`. The git tag
`v<VERSION>` marks each freeze. Within a major version only additive changes are
allowed (03 §5.2 rule 4); `buf breaking` and the reviews enforce it.

## 0.1.0 — first freeze

- `iomt.telemetry.v1` (gRPC, edge-sync → cloud): `TelemetryIngestService.UploadBatch`
  with `SummaryVitals` (~1 Hz) and `WindowFeatures` (per 30–120 s window, optional
  `Inference`); per-record `accepted` / `rejected` (`RejectReason`). `RecordKey`
  = (device_id, window_start_unix_ms, seq, boot_id): the boot id keeps records from
  colliding when a wearable reboots inside one window.
- `iomt.firmware.v1` (BLE `link_control`, wearable ↔ Pi): `LinkHello`, `LinkReport`
  (with `boot_id`), `LinkSelect` / `LinkRefusal` (incl. `MISSING_BOOT_ID`),
  cumulative `LinkAck`; firmware OTA manifest `FirmwareRelease` +
  `SignedFirmwareManifest`. Sample records stay the 16-byte `iomt-frames` format.
- `openapi/cloud-api.openapi.yaml` (REST, app/admin/gateway enrolment): auth
  (login, refresh, logout, password, register), devices (list, get, label,
  assign, revoke, gateway enrolment, node activation), measurements (range with
  raw or 1m/1h/1d rollups, latest), alerts (list, acknowledge), push
  registrations, consents, models (list, install, rollback), audit, health.
  Security schemes: bearer JWT, one-time enrolment token, gateway mutual TLS.
- `jsonschema/push-registration.v1.schema.json`: FCM token registration, upsert
  per app install.

Not in 0.1.0: `model`, `alert` and `federation` protos (edge alerts travel as
`Inference` flags inside `WindowFeatures`), AsyncAPI (no MQTT in the MVP), a live
push channel for the app's real-time screen.
