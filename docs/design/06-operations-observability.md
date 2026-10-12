# 06 — Operations & Observability (design)

Scope: the operational monitoring, health, logging and alerting plane for the IoT→Edge→Cloud→Mobile system
on Raspberry Pi 3B+ (D13/D14). Design only — no source code. Config/rule/schema snippets are documentation.
Binding context: `DECISIONS.md` D1–D20 and ADR-0020 (MVP trim), `01 §0/§2.1/§2.2/§2.6`, `03 §4/§7`, `05 §8`, `07` (security/identity).
All repository artifacts are English (D11). The canonical metric registry is `03 §7` (MET-20..MET-29); this document
introduces ADR-0011..ADR-0013 and MUST NOT restate metric targets.

---

## 1. Goals and non-goals

### 1.1 Goals

1. **Answer "is the demo/test-site healthy and where is it slow?"** for every tier with a small, boring stack.
2. **Fit the Pi budget.** All Pi-side monitoring must fit **inside the existing `edge-lite` ≤ 400 MB cap** together
   with the 3 services (MET-11) and add **no extra always-on container beyond `node_exporter`** (`03 §7.1`).
3. **Collect over the LAN/VPN.** Prometheus **scrapes** the Pi's `node_exporter` and each service `/metrics` over the
   LAN/VPN (ADR-0012); the NAT-push path (`vmagent` + `remote_write` over the gateway mTLS cert) is `[LATER]` and
   reserved for Pis that are not reachable over the LAN/VPN.
4. **Protect the SD card.** Logs and metric buffers on the Pi must be rotated/bounded (D19, MET-13).
5. **Never leak PHI.** Metrics and logs are operational data; no PHI, no raw BLE MAC, no free-text labels (07 §3.8).
6. **Separate operational alerting from clinical alerting.** Clinical alerts go to patients/caregivers only through
   the clinical push path (doc 08). Alertmanager is operations-only (hard rule, §8).

### 1.2 Non-goals (this document)

- No tracing (OpenTelemetry = `[LATER]`, §3.9), no log aggregation/Loki in the MVP (§3.8).
- No cAdvisor, no Prometheus/Grafana/Alertmanager on the Pi (§3.1, ADR-0011); `vmagent`/`remote_write` is `[LATER]`.
- No long-term metric retention, no ML anomaly detection, no on-call rotation tooling.
- No clinical alert delivery, no patient-facing notifications (doc 08).

---

## 2. Requirements (numbered, testable)

| id | Requirement (testable) | test |
|---|---|---|
| REQ-OPS-001 | Prometheus, Grafana and Alertmanager MUST run on the cloud VPS (or the dev machine), never on the Pi. | T-REQ-OPS-01-01 |
| REQ-OPS-002 | The only monitoring process added to the Pi MUST be `node_exporter` (RSS ≤ 20 MB); no `vmagent` runs in the MVP (ADR-0011/0012). | T-REQ-OPS-02-01 |
| REQ-OPS-003 | Σ `mem_limit` of the 3 `edge-lite` services + `node_exporter` MUST be ≤ 260 MB, within the ≤ 400 MB cap (canonical budget `03 §7.1`; extends MET-11). | T-REQ-OPS-03-01 |
| REQ-OPS-004 | Every `edge-lite` and cloud service MUST expose `GET /metrics` (Prometheus text), `GET /healthz` (liveness) and `GET /readyz` (readiness); `/healthz` MUST NOT touch dependencies. | T-REQ-OPS-04-01 |
| REQ-OPS-005 | `deploy/compose/compose.pi3.yml` and `deploy/compose/compose.cloud.yml` MUST declare a `healthcheck` for every application container (liveness) and for Postgres (`pg_isready`). | T-REQ-OPS-05-01 |
| REQ-OPS-006 | All application metrics MUST be named `iomt_<component>_<name>_<unit>`; exporter metrics MUST be rewritten into `iomt_host_*`, `iomt_pg_*`, `iomt_caddy_*` at scrape time. Tool self-metrics (`prometheus_*`, `alertmanager_*`) keep native names. | T-REQ-OPS-06-01 |
| REQ-OPS-007 | Label cardinality MUST stay within the §3.2 limits; a scrape MUST set `sample_limit`/`label_limit` and any overflow MUST alert (MET-27). | T-REQ-OPS-07-01 |
| REQ-OPS-008 | Prometheus MUST collect edge metrics by **pull-scrape** over the LAN/VPN (ADR-0012); `node_exporter` listens on `:9100` on the LAN/VPN interface only, and no Pi service opens any other inbound metrics port. | T-REQ-OPS-08-01 |
| REQ-OPS-009 | Application logs MUST be single-line JSON to stdout; Docker `json-file` rotation MUST be configured for every container (`max-size=5m`, `max-file=3` on the Pi; `max-size=20m`, `max-file=5` in the cloud). | T-REQ-OPS-09-01 |
| REQ-OPS-010 | No log line, metric label, or metric value MAY contain PHI, a raw BLE MAC, a password/token, or a free-text error string; a CI scan of an assembled log corpus MUST find 0 matches (MET-26). | T-REQ-OPS-10-01 |
| REQ-OPS-011 | Grafana MUST be provisioned entirely from Git (`deploy/observability/grafana/**`); no dashboard MAY be created by hand in the UI; exactly 4 MVP dashboards MUST exist (§3.6, MET-23). | T-REQ-OPS-11-01 |
| REQ-OPS-012 | Operational alerts MUST be expressed as Prometheus rules with a `for:` duration and tested with `promtool test rules`; MVP enables ≤ 8 rules and 100% of enabled rules MUST have a test (MET-25). | T-REQ-OPS-12-01 |
| REQ-OPS-013 | Clinical alerts MUST never be routed through Alertmanager; a CI test MUST assert no rule/label `domain="clinical"` is routed to any Alertmanager receiver (ADR-0013). | T-REQ-OPS-13-01 |
| REQ-OPS-014 | Public ingress MUST be Caddy with automatic TLS: `api.<domain>` → backend, `grafana.<domain>` → Grafana; Prometheus/Alertmanager MUST NOT be publicly exposed. The mTLS `push.<domain>` remote-write route is `[LATER]` (reserved for `vmagent`, ADR-0012). | T-REQ-OPS-14-01 |
| REQ-OPS-015 `[LATER]` | For NAT'd Pis only, `vmagent` MUST mount a 16 MB tmpfs write buffer so metric buffering writes 0 bytes to the microSD card (deferred with `vmagent`, ADR-0012). | T-REQ-OPS-15-01 |

---

## 3. Design

### 3.1 Component placement and collection topology

```
        Raspberry Pi 3B+ (edge-lite, D13)                      Cloud VPS (D15) / dev machine
  ┌───────────────────────────────────────────┐  scrape 60 s    ┌─────────────────────────────────────────────┐
  │ edge-ingest :9101  edge-sync :9102        │ ◄─────────────  │ Caddy :443                                  │
  │ model-agent :9103                         │  LAN/VPN        │  ├─ api.<domain>     → backend:8080         │
  │ node_exporter :9100                       │  pull-scrape    │  └─ grafana.<domain> → grafana:3000         │
  │ Docker engine :9323                       │  /metrics       │ Prometheus :9090  + Alertmanager :9093      │
  └───────────────────────────────────────────┘                 │ Grafana :3000                               │
                                                               │ postgres_exporter · node_exporter · blackbox│
                                                               └─────────────────────────────────────────────┘
```

- **Default collection model = PULL-SCRAPE (ADR-0012).** Prometheus scrapes the Pi's `node_exporter` (`:9100`) and each
  `/metrics` endpoint over the LAN/VPN at 60 s; the Pi initiates nothing and runs no remote-write receiver.
- **`[LATER]` — NAT'd Pis only:** `vmagent` scrapes loopback targets, buffers in a 16 MB tmpfs and `remote_write`s to
  Prometheus through Caddy `push.<domain>` over mTLS (REQ-OPS-015); this is not part of the MVP topology.
- **VPN mesh (Tailscale/WireGuard)** is `[LATER]`; the MVP scrapes over the lab LAN and the same pull path applies
  unchanged once the mesh exists.

### 3.2 Naming, types, labels, cardinality

**Naming:** `iomt_<component>_<name>_<unit>`.
- `<component>` ∈ `ingest`, `sync`, `model`, `api`, `fl`, `host`, `pg`, `caddy`.
- Counters end in `_total`; histograms in `_seconds`/`_bytes`; gauges carry the physical unit (`_bytes`, `_seconds`,
  `_ratio`, `_rows`, `_celsius`, `_connections`). `_info` gauges have value 1 and carry labels.
- Exporters are rewritten at scrape time (single `metric_relabel_configs` regex each):
  `node_*`→`iomt_host_*`, `pg_*`→`iomt_pg_*`, `caddy_*`→`iomt_caddy_*`. Tool self-metrics keep native names.

**Required labels (low cardinality):**

| label | values | source | applies to |
|---|---|---|---|
| `env` | `dev`, `prod` | Prometheus scrape config external label | all |
| `tier` | `edge`, `cloud` | external label | all |
| `gateway_id` | ≤ 10 (MVP) | per-Pi external label | edge series only |
| `job`, `instance` | automatic | Prometheus | all |
| `route` | ≤ 25 (route *template*, never raw path) | backend | `api` RED metrics |
| `device_id` | ≤ 30 (hashed, 32 hex — 07 §3.2) | edge services | ≤ 3 metrics (fleet/BLE) |
| `model_version` | ≤ 10 | edge/backend | model metrics |
| `outcome` / `reason` / `status_class` / `mode` | ≤ 10 each (enums only) | app | counters |

**Cardinality limits (hard):**

| scope | limit | alert |
|---|---|---|
| series per Pi | ≤ 800 | `IomtSeriesBudgetHigh` at 1 000 |
| active series total | ≤ 8 000 | `IomtSeriesBudgetHigh` at 12 000, absolute ceiling 50 000 |
| distinct values per label | ≤ 100 (allowlist above) | `promtool`/CI check + review |
| scrape guardrails | `sample_limit: 5000`, `label_limit: 30` per target | `scrape_samples_post_metric_relabeling` |

### 3.3 Metrics catalogue

`edge-ingest` (`component = ingest`):

| metric | type | unit | labels | purpose |
|---|---|---|---|---|
| `iomt_ingest_ble_connections` | gauge | connections | — | live GATT connections (MET-16) |
| `iomt_ingest_ble_frames_total` | counter | frames | — | frames decoded |
| `iomt_ingest_ble_disconnects_total` | counter | events | `reason` | disconnect causes |
| `iomt_ingest_ble_rssi_dbm` | gauge | dBm | `device_id` | link quality per node |
| `iomt_ingest_window_duration_seconds` | histogram | s | — | windowing cadence |
| `iomt_ingest_inference_duration_seconds` | histogram | s | `model_version` | ONNX latency on the Pi |
| `iomt_ingest_inference_total` | counter | inferences | `model_version`,`outcome` | ok/error |
| `iomt_ingest_dropped_samples_total` | counter | samples | `reason` | overflow/decode/missing-window |
| `iomt_ingest_ready` | gauge | ratio | — | 1 if `/readyz` passes |

`edge-sync` (`component = sync`):

| metric | type | unit | labels | purpose |
|---|---|---|---|---|
| `iomt_sync_queue_rows` | gauge | rows | — | store-and-forward backlog |
| `iomt_sync_queue_oldest_seconds` | gauge | s | — | queue age (SLO input) |
| `iomt_sync_queue_bytes` | gauge | bytes | — | ring buffer size (≤ 10 MB, 01 §2.1) |
| `iomt_sync_upload_batches_total` | counter | batches | `outcome` | ok/retry/fail |
| `iomt_sync_upload_duration_seconds` | histogram | s | — | uplink latency |
| `iomt_sync_upload_backoff_seconds` | gauge | s | — | current retry backoff |
| `iomt_sync_sqlite_commits_total` | counter | commits | — | commit rate (MET-13) |
| `iomt_sync_sqlite_commit_batch_rows` | histogram | rows | — | batch size (≤ 500, MET-13) |
| `iomt_sync_http_requests_total` | counter | requests | `route`,`status_class` | local gateway RED |
| `iomt_sync_tls_not_after_timestamp_seconds` | gauge | unix s | — | cert expiry (07 auto-renew) |
| `iomt_sync_ready` | gauge | ratio | — | readiness |

`model-agent` (`component = model`):

| metric | type | unit | labels | purpose |
|---|---|---|---|---|
| `iomt_model_version_info` | gauge (1) | — | `model_version`,`installed` | current model |
| `iomt_model_verify_total` | counter | verifications | `outcome` | ok / bad_sig / bad_sha / downgrade_blocked |
| `iomt_model_swap_duration_seconds` | histogram | s | — | apply latency |
| `iomt_model_last_swap_timestamp_seconds` | gauge | unix s | — | last successful swap |
| `iomt_model_ready` | gauge | ratio | — | model loadable/verified |

`cloud backend` (`component = api`):

| metric | type | unit | labels | purpose |
|---|---|---|---|---|
| `iomt_api_http_requests_total` | counter | requests | `route`(≤25),`method`,`status_class` | RED rate/errors |
| `iomt_api_http_request_duration_seconds` | histogram | s | `route`(≤12),`status_class` | RED duration (MET-07) |
| `iomt_api_uplink_batches_total` | counter | batches | `outcome` | edge ingest rate |
| `iomt_api_telemetry_rows_total` | counter | rows | `kind` (vitals/window) | D18 volumes |
| `iomt_api_uplink_lag_seconds` | histogram | s | — | `now − ts_source` (MET-02 server segment) |
| `iomt_api_auth_logins_total` | counter | logins | `outcome` | brute-force signal (MET-33) |
| `iomt_api_db_query_duration_seconds` | histogram | s | `query_kind` (≤10) | sqlx latency |
| `iomt_api_db_pool_active` | gauge | connections | — | pool saturation |
| `iomt_api_ready` | gauge | ratio | — | readiness (DB ping) |

`FL server` (`component = fl`, `[LATER]`): `iomt_fl_round_total`, `iomt_fl_round_duration_seconds`,
`iomt_fl_clients_registered`, `iomt_fl_aggregation_duration_seconds`, `iomt_fl_ready`.

`Postgres/Timescale` (`component = pg`, via `postgres_exporter` + custom `queries.yaml` after rewrite):
`iomt_pg_up`, `iomt_pg_stat_database_numbackends`, `..._xact_commit`, `..._xact_rollback`,
`iomt_pg_database_size_bytes`, `iomt_pg_settings_max_connections`, `iomt_pg_stat_user_tables_n_dead_tup`,
and custom, project-defined series: `iomt_pg_timescale_chunks`, `iomt_pg_timescale_compression_ratio`,
`iomt_pg_cagg_lag_seconds` (MET-17), `iomt_pg_cagg_last_refresh_timestamp_seconds`.

`host` (`component = host`, via `node_exporter`, curated collectors, after rewrite) — the ones we alert/dashboard on:
`iomt_host_cpu_seconds_total`, `iomt_host_memory_MemAvailable_bytes`, `iomt_host_memory_MemTotal_bytes`,
`iomt_host_filesystem_avail_bytes`, `iomt_host_filesystem_size_bytes`, `iomt_host_disk_written_bytes_total`,
`iomt_host_load1`, `iomt_host_thermal_zone_temp_celsius`, `iomt_host_time_seconds`, `iomt_host_timex_offset_seconds`,
`iomt_host_timex_sync_status`, `iomt_host_network_receive_bytes_total`, `iomt_host_network_transmit_bytes_total`,
plus textfile-collected `iomt_host_container_memory_bytes{container}`, `iomt_host_container_memory_limit_bytes{container}`,
`iomt_host_container_health{container}`, `iomt_host_container_restart_count{container}`,
`iomt_host_backup_last_success_timestamp_seconds` (VPS).

`Docker engine` (`component` = docker, endpoint `127.0.0.1:9323`, native `docker_engine_*`/`builder_*`):
daemon-level container/state and engine health (replaces cAdvisor). Per-container RSS comes from the textfile
collector above, because the Docker engine endpoint is daemon-level only.

`Caddy` (`component = caddy`, admin `:2019/metrics` after rewrite): `iomt_caddy_http_requests_total`,
`iomt_caddy_http_request_duration_seconds`.

### 3.4 Liveness / readiness semantics

| service | `/healthz` (liveness — cheap, no I/O) | `/readyz` (readiness — may do bounded I/O) |
|---|---|---|
| edge-ingest | HTTP server + tokio loop responsive | BlueZ/D-Bus reachable, ≥1 BLE adapter, warm ONNX session loaded |
| edge-sync | HTTP server + SQLite handle open (no query) | SQLite writable, disk free ≥ 100 MB, clock within ± 120 s (07 clock gate). **Cloud reachability is NOT readiness** (it is `iomt_sync_upload_backoff_seconds`) |
| model-agent | HTTP server up | model dir present, signature key present, last verify chain OK |
| cloud backend | HTTP server up | `SELECT 1` ≤ 500 ms, expected migration version, JWT key loaded |
| FL server `[LATER]` | HTTP server up | strategy + registry store reachable |
| node_exporter / Prometheus / Grafana / Alertmanager / Caddy / Postgres | vendor endpoint (see §3.5) | n/a (scraper-side) |

**Rule:** Compose `healthcheck` uses **`/healthz` only** (Docker has one healthcheck per container; it does not
auto-restart unhealthy containers — restart happens only on process exit via `restart: unless-stopped`).
Readiness is consumed by (a) K8s `readinessProbe` in `deploy/k8s`, (b) the `iomt_*_ready` gauges and alerts.
Docker health status is exported (`iomt_host_container_health`) and alerted; it never silently restarts a service.

### 3.5 Docker healthchecks (compose)

`deploy/compose/compose.pi3.yml` (excerpt; loopback-only, host network so healthchecks can reach `127.0.0.1`):

```yaml
services:
  edge-ingest:
    network_mode: host
    mem_limit: 112m
    cpus: "0.60"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:9101/healthz"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 20s
  edge-sync:
    network_mode: host
    mem_limit: 64m
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:9102/healthz"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 20s
  model-agent:
    network_mode: host
    mem_limit: 64m
    healthcheck:
      test: ["CMD", "curl", "-fsS", "http://127.0.0.1:9103/healthz"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 20s
  node-exporter:
    image: quay.io/prometheus/node-exporter:v1.8.2
    network_mode: host
    mem_reservation: 8m
    mem_limit: 20m
    # no healthcheck: absence of samples is the failure signal (see alert IomtEdgeDown)
    command:
      - --path.rootfs=/host
      - --web.listen-address=:9100   # LAN/VPN interface only, scraped by Prometheus (REQ-OPS-008)
      - --collector.disable-defaults
      - --collector.cpu
      - --collector.diskstats
      - --collector.filesystem
      - --collector.loadavg
      - --collector.meminfo
      - --collector.netdev
      - --collector.stat
      - --collector.thermal_zone
      - --collector.time
      - --collector.timex
      - --collector.uname
      - --collector.vmstat
      - --collector.textfile
      - --collector.textfile.directory=/host/var/lib/iomt/node-exporter/textfile
    volumes:
      - /:/host:ro,rslave
  # [LATER] NAT'd Pis only (not in the MVP compose file): a `vmagent` service would scrape loopback
  # targets, buffer in a 16 MB tmpfs and `remote_write` to Caddy `push.<domain>` over mTLS (REQ-OPS-015).
```

`deploy/compose/compose.cloud.yml` healthchecks: backend `curl -fsS http://127.0.0.1:8080/healthz`;
postgres `pg_isready -U $POSTGRES_USER`; prometheus `wget -qO- http://127.0.0.1:9090/-/healthy`;
alertmanager `http://127.0.0.1:9093/-/healthy`; grafana `http://127.0.0.1:3000/api/health`;
caddy `wget -qO- http://127.0.0.1:2019/config/` (admin API up).

### 3.6 Collection intervals, retention and dashboards

- **Pi scrape interval = 60 s** (Prometheus pull-scrape). Chosen against SD/RAM: ~720 series/Pi → 12 samples/s.
  All Pi alert rules use `for: ≥ 2m`, so 60 s is ample. Scrape cache/CPU cost ≈ ≤ 1% of one core.
- **Cloud scrape interval = 15 s** (backend, pg, caddy, docker/host of VPS, blackbox). `scrape_timeout` 10 s (5 s Pi).
- **`[LATER]` (NAT-push only):** `remote_write` batch deadline = 10 s (`queue_config.batch_send_deadline`); ≤ 300 samples/s.
- **Retention = 15 days** (VPS disk), `--storage.tsdb.retention.time=15d`,
  `--storage.tsdb.retention.size=1GB` as a hard disk guard; `--web.enable-remote-write-receiver` is `[LATER]` (NAT-push only).

**Dashboards (dashboards-as-code; exactly 4 `[MVP]`, per MET-23).** Provisioning:
`deploy/observability/grafana/provisioning/datasources/prometheus.yml` (datasource `Prometheus → http://prometheus:9090`)
and `.../dashboards/dashboards.yml` (`path: /var/lib/grafana/dashboards`, `foldersFromFilesStructure: true`).

| dashboard (JSON) | panels | example PromQL |
|---|---|---|
| `pi-health.json` | CPU, mem avail, disk %, SD write rate, temp, per-container RSS/limit, restarts | `1 - (iomt_host_memory_MemAvailable_bytes / iomt_host_memory_MemTotal_bytes)`; `rate(iomt_host_disk_written_bytes_total[5m])`; `iomt_host_thermal_zone_temp_celsius`; `iomt_host_container_memory_bytes / iomt_host_container_memory_limit_bytes` |
| `edge-pipeline.json` | BLE conns, frames/s, infer p95, queue rows & age, upload outcome, SQLite batch | `histogram_quantile(0.95, sum(rate(iomt_ingest_inference_duration_seconds_bucket[10m])) by (le))`; `iomt_sync_queue_oldest_seconds` |
| `cloud-api-red.json` | request rate, 5xx ratio, p50/p95/p99 by route, DB pool, uplink lag | `sum(rate(iomt_api_http_requests_total[5m])) by (route)`; `sum(rate(iomt_api_http_requests_total{status_class="5xx"}[5m])) / sum(rate(iomt_api_http_requests_total[5m]))` |
| `e2e-latency.json` | server-side uplink lag (MET-02 segment), TimescaleDB size/chunks/CAGG lag (per `01 §2`), alert delivery | `histogram_quantile(0.95, sum(rate(iomt_api_uplink_lag_seconds_bucket[10m])) by (le))`; `iomt_pg_cagg_lag_seconds`; `rate(iomt_pg_database_size_bytes[1h])`; `histogram_quantile(0.95, sum(rate(alertmanager_notification_latency_seconds_bucket[10m])) by (le,integration))` |

`[LATER]` dashboards (deferred, not MVP): `device-fleet-ble.json` (fleet/BLE), `fl-training.json` (FL).

**Auth/access:** Grafana local users (admin + viewer), password from a sops secret (07 §3.4); `allow_sign_up=false`;
served via Caddy `grafana.<domain>` (TLS). Prometheus/Alertmanager are **not** published (SSH tunnel or VPN).

### 3.7 Alerting

Rules live in `deploy/observability/prometheus/rules/{edge,cloud,ops}.rules.yml`; the SLO recording-rules file
`slo.rules.yml` and the SLO dashboard are `[LATER]` (MET-28). Receivers are in
`deploy/observability/alertmanager/alertmanager.yml`. Severities: `critical` (Telegram + email), `warning`
(Telegram), `info` (email/webhook digest). Routes: match `domain="ops"`; group by `alertname,gateway_id`;
`group_wait: 30s`, `group_interval: 5m`, `repeat_interval: 4h` (critical) / `24h` (warning). A `Watchdog` alert
(always firing) is routed to the `null` receiver to prove the pipeline end-to-end (MET-24).

**Hard rule (ADR-0013): clinical alerts NEVER pass through Alertmanager.** Every operational rule carries
`domain="ops"`; the Alertmanager route matches only `domain="ops"`; a CI test asserts no `domain="clinical"` rule
exists and the backend never calls Alertmanager (clinical path = doc 08).

**MVP rules (at most 8, MET-25)** — every enabled rule has a `promtool` test case:

| alert | expression (abridged) | for | sev |
|---|---|---|---|
| `IomtEdgeDown` | `absent_over_time(iomt_host_time_seconds{gateway_id=~".+"}[5m])` | 5m | critical |
| `IomtEdgeClockDrift` | `abs(time() - iomt_host_time_seconds) > 120` | 10m | warning |
| `IomtEdgeQueueAge` | `iomt_sync_queue_oldest_seconds > 900` / `> 3600` | 10m / 5m | warning / critical |
| `IomtEdgeDiskLow` | `iomt_host_filesystem_avail_bytes / iomt_host_filesystem_size_bytes < 0.10` / `< 0.05` | 10m / 5m | warning / critical |
| `IomtApiHighErrorRate` | `sum(rate(iomt_api_http_requests_total{status_class="5xx"}[5m])) / sum(rate(iomt_api_http_requests_total[5m])) > 0.01` | 10m | critical |
| `IomtServiceUnready` | `up == 0` (cloud jobs) `or iomt_sync_ready == 0 or iomt_ingest_ready == 0 or iomt_api_ready == 0` | 5m | critical |
| `IomtPostgresDown` | `iomt_pg_up == 0` | 2m | critical |
| `IomtBackupFailed` | `time() - iomt_host_backup_last_success_timestamp_seconds > 30*3600` | 10m | critical |

**`[LATER]` rules** (deferred; not enabled in the MVP) — push freshness, SLO burn-rate and the long tail:

| alert | expression (abridged) | for | sev |
|---|---|---|---|
| `IomtEdgePushStale` `[LATER]` | `time() - max by (gateway_id)(iomt_host_time_seconds) > 60` | 5m | warning |
| `IomtEdgeUploadFailing` `[LATER]` | `rate(iomt_sync_upload_batches_total{outcome="fail"}[10m]) > 0` | 10m | warning |
| `IomtEdgeTempHigh` `[LATER]` | `iomt_host_thermal_zone_temp_celsius > 75` / `> 82` | 10m / 5m | warning / critical |
| `IomtEdgeMemoryLow` `[LATER]` | `1 - iomt_host_memory_MemAvailable_bytes / iomt_host_memory_MemTotal_bytes > 0.90` | 10m | warning |
| `IomtContainerNearLimit` `[LATER]` | `iomt_host_container_memory_bytes / iomt_host_container_memory_limit_bytes > 0.90` | 10m | warning |
| `IomtContainerRestarting` `[LATER]` | `changes(iomt_host_container_restart_count[15m]) > 2` | 0m | warning |
| `IomtSeriesBudgetHigh` `[LATER]` | `count by (gateway_id)({__name__=~"iomt_.+"}) > 1000` / `prometheus_tsdb_head_series > 12000` | 30m | warning |
| `IomtRemoteWriteDropped` `[LATER]` | `rate(vmagent_remotewrite_dropped_rows_total[10m]) > 0` | 10m | warning |
| `IomtCertExpirySoon` `[LATER]` | `iomt_sync_tls_not_after_timestamp_seconds - time() < 7*86400` | 1h | warning |
| `IomtTlsProbeExpirySoon` `[LATER]` | `probe_ssl_earliest_cert_expiry - time() < 10*86400` | 1h | warning |
| `IomtApiLatencyHigh` `[LATER]` | `histogram_quantile(0.95, sum(rate(iomt_api_http_request_duration_seconds_bucket[5m])) by (le)) > 0.2` | 10m | warning (MET-07) |
| `IomtPostgresConnHigh` `[LATER]` | `iomt_pg_stat_database_numbackends / iomt_pg_settings_max_connections > 0.8` | 10m | warning |
| `IomtTimescaleCaggLag` `[LATER]` | `iomt_pg_cagg_lag_seconds > 300` | 10m | warning (MET-17) |
| `IomtRuleEvalErrors` `[LATER]` | `rate(prometheus_rule_evaluation_failures_total[10m]) > 0` | 10m | warning |
| `IomtSloFastBurn` `[LATER]` | edge: `(1 - avg_over_time(iomt_edge_reporting_ratio[1h])) > 14.4 * 0.01` | 5m | critical |

**Inhibition:** `IomtEdgeDown` inhibits other alerts for the same `gateway_id`; `IomtPostgresDown` inhibits
`IomtApiHighErrorRate`/`IomtApiLatencyHigh`. Silences via Alertmanager UI/`amtool` (documented in the runbook).

### 3.8 Logging

- Structured **JSON to stdout** using the Rust `tracing` crate + `tracing-subscriber` JSON formatter.
  Schema (allowlist): `ts`, `level`, `service`, `version`, `gateway_id`/`host`, `event`, `span`, `trace_id` (empty
  until OTel `[LATER]`), plus explicit domain fields. Errors are enum kinds (`error_kind`), never formatted strings
  containing user data.
- **No PII / no secrets:** no names, no raw BLE MAC (only `device_id`), no tokens, no raw signal values (07 §3.8).
  Enforcement: `#[instrument(skip(...))]` on sensitive params + a CI regex scan of an assembled log corpus (MET-26).
- **Rotation (SD protection):** Docker `json-file` driver `max-size=5m`, `max-file=3` on the Pi (≤ 15 MB/container,
  ≤ 64 MB total); `max-size=20m`, `max-file=5` in the cloud. Pi OS journald `Storage=volatile`, `RuntimeMaxUse=32M` (D19).
- **Centralised shipping (Loki/Vector) = `[LATER]`** (rationale: +150 MB cloud RAM and a Pi agent for logs that
  already fit in bounded files; retrieval is `docker logs` over SSH for the MVP).
- **Audit log** (07 §3.7) is application data on the cloud, not an observability concern here; its daily anchor line
  flows to the log stream.

### 3.9 Tracing (`[LATER]`)

OpenTelemetry is `[LATER]`: the MVP has one synchronous request path per hop and the dashboards already localise
latency to a hop via `iomt_ingest_inference_*`, `iomt_sync_upload_*`, `iomt_api_uplink_lag_*`. OTel would add a
collector (+200 MB) and code churn for marginal benefit; it is revisited when FL/multi-hop debugging needs it.

### 3.10 Cloud ingress (Caddy)

`deploy/ingress/Caddyfile` (Caddy v2.8+, automatic ACME TLS, 07 §3.1):

```caddyfile
{
  email {$ACME_EMAIL}
}
api.{$DOMAIN} {
  reverse_proxy backend:8080          # JWT-protected API only
}
grafana.{$DOMAIN} {
  reverse_proxy grafana:3000          # Grafana login (allow_sign_up=false)
}
# [LATER] NAT'd Pis only (not in the MVP Caddyfile): remote-write receiver
push.{$DOMAIN} {
  tls {
    client_auth {
      mode require_and_verify
      trust_pool file {
        pem_file /etc/caddy/root.pem  # project CA (07 ADR-0014)
      }
    }
  }
  handle /api/v1/write* {
    reverse_proxy prometheus:9090     # remote-write receiver [LATER]
  }
  handle {
    respond 404
  }
}
```

`push.<domain>` reuses the gateway certificate (mTLS) already issued for `edge-sync`, so no new secret type is
introduced; revocation of push access is via the CA/registry (CRL = `[LATER]`, 07 §6). Prometheus and Alertmanager
are never routed by Caddy.

---

## 4. Interfaces

### 4.1 Ports

| Where | Endpoint | Exposure | Auth |
|---|---|---|---|
| Pi LAN/VPN | `9100` node_exporter (scraped by Prometheus) | LAN/VPN | none |
| Pi loopback | `9323` Docker engine (MVP); `8429` vmagent `[LATER]` (NAT-push only) | loopback only | none |
| Pi loopback | `9101` edge-ingest, `9102` edge-sync (+`8080` gateway-api), `9103` model-agent | loopback only | none |
| Pi | `22/tcp` SSH | admin LAN | keys (07) |
| Cloud public | `443/tcp` Caddy (`api`, `grafana`; `push` `[LATER]`) | public | JWT / login / mTLS |
| Cloud public | `9443/tcp` axum mTLS (07) | cert-gated | client cert |
| Cloud internal | `9090` Prometheus, `9093` Alertmanager, `3000` Grafana, `9187` postgres_exporter, `9115` blackbox | compose-internal | none |
| Topics | **none** — no MQTT in the MVP (D14); metrics use HTTP scrape (remote-write is [LATER]), not a broker | — | — |

### 4.2 Endpoints (every application service)

- `GET /metrics` → Prometheus text (loopback / compose-internal only).
- `GET /healthz` → `200 {"status":"ok"}` when alive (no dependency I/O); `503` when the loop is stuck.
- `GET /readyz` → `200 {"status":"ready"}` / `503 {"status":"not_ready","reason":"<enum>"}` (enum reason only).

### 4.3 Files in the repo tree (owner per `04`)

| Path | Tag | Owner |
|---|---|---|
| `deploy/compose/compose.cloud.yml` (VPS stack: pg, backend, caddy, prometheus, alertmanager, grafana, exporters) | `[MVP]` | MVP-07 |
| `deploy/observability/prometheus/{prometheus.yml,rules/*.rules.yml,tests/*.test.yml}` | `[MVP]` | MVP-07 |
| `deploy/observability/alertmanager/{alertmanager.yml,templates/telegram.tmpl}` | `[MVP]` | MVP-07 |
| `deploy/observability/grafana/provisioning/{datasources/prometheus.yml,dashboards/dashboards.yml}` | `[MVP]` | MVP-07 |
| `deploy/observability/grafana/dashboards/{pi-health,edge-pipeline,cloud-api-red,e2e-latency}.json` | `[MVP]` | MVP-07 |
| `deploy/observability/grafana/dashboards/{device-fleet-ble,fl-training}.json` | `[LATER]` | RW-10 |
| `deploy/observability/vmagent/{scrape.yml,remote-write-notes.md}` | `[LATER]` (NAT'd Pis only) | RW-10 |
| `deploy/observability/node-exporter/{textfile/.gitkeep,collect-container-stats.sh}` | `[MVP]` | MVP-07 |
| `deploy/observability/blackbox/blackbox.yml` | `[MVP]` | MVP-07 |
| `deploy/observability/loki/`, `deploy/observability/otel/` | `[LATER]` | RW-10 |
| `deploy/ingress/Caddyfile` | `[MVP]` | MVP-07 |
| `deploy/systemd/iomt-container-stats.{service,timer}` | `[MVP]` | MVP-07 |
| `docs/runbooks/observability.md` | `[MVP]` | MVP-07 |

---

## 5. Resource impact (with arithmetic)

### 5.1 Pi 3B+ — must fit 400 MB with the 3 services

| item | RAM limit | est. RSS | CPU | disk writes | assumption |
|---|---|---|---|---|---|
| node_exporter | 20 MB (request 8 MB) | ~12 MB | ~0.5% core @60 s | ~0 | Go, curated collectors (~300 series) |
| textfile collector (`systemd` timer, 5 min) | ~3 MB transient | — | <0.1% | ~1 KB/5 min ≈ 0.3 MB/day | emits container RSS/health/restarts |
| healthcheck `curl` processes | ~2 MB transient | — | <0.1% | ~0 | <100 ms each |
| Docker logs (`json-file` 5m×3) | 0 | 0 | 0 | ≤ 15 MB/container bound | ≤ 64 MB total for 4 containers |

**Budget:** canonical in `03 §7.1` (do not re-derive here): edge-lite services request/limit `104/240 MB` +
`node_exporter` `8/20 MB` = **combined limit 112/260 MB ≤ ~400 MB cap** (MET-11/MET-20). No vmagent, no new
daemon, no K3s, no cAdvisor. Total added load ≪ headroom described in `01 §2.1`.

### 5.2 Cloud VPS (2 vCPU / 4 GB / 40 GB — 07 §5.2)

| component | RAM estimate | assumption |
|---|---|---|
| Prometheus | ~200 MB | ≤ 8 000 active series, 15 d retention |
| Grafana | ~150 MB | 4 MVP dashboards, ≤ 10 concurrent users |
| Alertmanager | ~40 MB | ≤ 8 MVP rules, 3 receivers |
| node_exporter (VPS) | ~12 MB | curated |
| postgres_exporter | ~20 MB | no per-table label explosion |
| blackbox_exporter | ~15 MB | 2 probes |
| step-ca | ~30 MB | private CA (ADR-0014), low traffic |
| Caddy | 50 MB | already counted in 07 §5.2 |
| **observability total** | **~470 MB** | within 07's ≈ 400 MB rounding; total stack ≈ 2.0 GB, ≥ 2.0 GB headroom |

**Prometheus disk:** Pi `4 × 720 / 60 = 48 samples/s`; cloud `2 400 / 15 = 160 samples/s`; total ≈ 208 samples/s
× 1.3 B ≈ 270 B/s ≈ **23 MB/day** ⇒ 15 d ≈ 350 MB, with WAL/index ≈ **≤ 700 MB** on a 40 GB SSD (hard guard
`retention.size=1GB`). At 10 Pis: ≈ 32 MB/day ⇒ ≈ 480 MB raw, still ≤ 1 GB.

---

## 6. MVP vs LATER

| # | item | tag | owner |
|---|---|---|---|
| 1 | node_exporter (Pi, curated) + Docker engine metrics endpoint (no cAdvisor) | `[MVP]` | MVP-07 |
| 2 | Prometheus + Alertmanager + Grafana on the VPS/dev machine; 15 d retention | `[MVP]` | MVP-07 |
| 3 | Prometheus pull-scrapes the Pi (`node_exporter` + `/metrics`) over the LAN/VPN at 60 s | `[MVP]` | MVP-07 |
| 4 | `/healthz`, `/readyz`, `/metrics` + Compose healthchecks for all services | `[MVP]` | MVP-04/05/07 |
| 5 | 4 provisioned dashboards + datasource provisioning (MET-23) | `[MVP]` | MVP-07 |
| 6 | ≤ 8 operational alert rules + Alertmanager (Telegram/email/webhook) + promtool tests | `[MVP]` | MVP-07 |
| 7 | JSON structured logs + bounded Docker rotation + no-PII scan | `[MVP]` | MVP-07 |
| 8 | Caddy ingress: `api` + `grafana` | `[MVP]` | MVP-07 |
| 9 | Container RSS/health textfile collector + systemd timer | `[MVP]` | MVP-07 |
| 10 | blackbox_probe (public TLS expiry + availability) | `[MVP]` | MVP-07 |
| 11 | vmagent push + 16 MB tmpfs buffer + remote_write over mTLS + Caddy `push` route | `[LATER]` | RW-10 |
| 12 | SLI/SLO recording rules + burn-rate alert + SLO dashboard | `[LATER]` | RW-10 |
| 13 | Loki central log shipping | `[LATER]` | RW-10 |
| 14 | OpenTelemetry tracing + collector | `[LATER]` | RW-10 |
| 15 | Pull scraping over a VPN mesh (Tailscale/WireGuard) | `[LATER]` | RW-10 |
| 16 | FL training dashboard + `iomt_fl_*` metrics | `[LATER]` | RW-10 |
| 17 | Long-term metric retention / Thanos / VictoriaMetrics | `[LATER]` | RW-10 |
| 18 | Grafana alerting / on-call integrations beyond Telegram/email | `[LATER]` | RW-10 |

---

## 7. Acceptance criteria and new metrics (MET-20..MET-29)

Columns per `03 §7`. Acceptance = every REQ-OPS row of §2 has a green test plus its metric. **The canonical
registry is `03 §7` (MET-20..MET-29); to avoid a duplicated registry this document does not restate the table.**
MVP-reconciled status (identical to `03 §7`):

- **MET-20** — `node_exporter` limit ≤ **20 MB** (request 8 MB); combined limits (3 edge-lite services +
  `node_exporter`) ≤ **260 MB**, within the ≤ **400 MB** cap.
- **MET-21** (edge metrics push freshness) — `[LATER]` (requires `vmagent`/`remote_write`; the MVP is pull-scrape).
- **MET-23** (dashboards-as-code) — exactly **4** MVP dashboards (Pi health, edge pipeline, cloud API RED,
  TimescaleDB/E2E latency) provisioned from Git; the rest are `[LATER]`.
- **MET-28** (SLO reporting) — `[LATER]`.

All other MET-20..29 names, targets, units and measurement methods live only in `03 §7`; any threshold change
goes through a PR + update of that registry.

---

## 8. Failure modes and mitigations

| # | failure mode | impact | likelihood | mitigation / detection |
|---|---|---|---|---|
| F-01 | Pi behind NAT is unreachable by cloud scrape (MVP scrapes over the LAN/VPN only) | blind edge | Med | `[LATER]` NAT-push path (`vmagent`) for such Pis; `IomtEdgeDown` via `absent_over_time` |
| F-02 `[LATER]` (NAT-push) | vmagent/queue exceeds limit, samples dropped | metric gaps | Med | 48 MB limit + 16 MB tmpfs; `IomtRemoteWriteDropped`; gap visible on dashboards |
| F-03 | SD wear from logs/buffer | card failure | Med | json-file 5m×3; volatile journald (no tmpfs buffer in the MVP); MET-13/MET-20 |
| F-04 | Prometheus disk fills | monitoring loss | Med | 15 d + `retention.size=1GB`; `IomtPostgresDiskLow`-style FS alert on the VPS |
| F-05 | Alert storm / flapping | alert fatigue | Med | `for:` durations, grouping, inhibition, repeat_interval |
| F-06 | Alertmanager down | lost alerts | Med | `Watchdog` + blackbox probe of `api`/`grafana`; Telegram heartbeat |
| F-07 | Cardinality explosion from a code change | Prometheus OOM/slow | Low | `sample_limit`/`label_limit`, MET-27 alert, CI review |
| F-08 | Pi clock drift breaks TLS validity | uplink fails | Med | `IomtEdgeClockDrift`; 07 clock gate blocks uplink until ≤ ±120 s |
| F-09 `[LATER]` (NAT-push) | A revoked gateway still pushes metrics | low-value metric injection | Low | Push is metrics-only (no PHI); operator can block the Pi IP; CRL `[LATER]`; labels are PHI-free |
| F-10 | Docker metrics endpoint disabled/removed by upgrade | lose engine metrics | Low | `up{job="docker"}==0` alert; per-container RSS still from textfile collector |
| F-11 | PHI/secrets inadvertently logged | privacy breach | Low | JSON allowlist + `#[instrument(skip)]` + CI corpus scan (MET-26) |
| F-12 | Grafana publicly exposed | info disclosure | Low | Grafana auth + TLS via Caddy; `allow_sign_up=false`; Prometheus/AM not public |
| F-13 | Exponential alert fan-out on edge loss | noise | Med | `IomtEdgeDown` inhibits per-`gateway_id` alerts |
| F-14 | Pi total RSS pushes the device into swap/OOM | edge down | Low | `mem_limit` on all Pi containers (3 services + `node_exporter`) + `IomtEdgeMemoryLow`/`IomtContainerNearLimit` |

---

## 9. Decisions taken (ADR-0011..ADR-0013, no open questions)

**ADR-0011 — Observability is centralised off the Pi; the Pi runs only `node_exporter`; cAdvisor is rejected.**
*Decision:* Prometheus/Grafana/Alertmanager run on the VPS (or dev machine); the Pi adds only `node_exporter`
(limit ≤ 20 MB); Docker engine metrics + a node_exporter textfile collector replace cAdvisor.
*Rationale:* the 400 MB Pi cap and 1 GB RAM cannot host a TSDB or cAdvisor (~50–100 MB); Docker engine metrics are
free on the daemon and the textfile collector covers per-container RSS/health at ~0.3 MB/day of SD writes.

**ADR-0012 — Scrape-first metrics: Prometheus pulls the Pi over the LAN/VPN; `vmagent` remote_write is `[LATER]`.**
*Decision:* the default collection model is pull-scrape; the `vmagent` → Prometheus `remote_write` over mTLS via
Caddy `push.<domain>` path is `[LATER]` and reserved for Pis behind NAT that the cloud cannot reach directly.
*Rationale:* pull needs no Pi-side agent (saving the 400 MB budget) and needs no inbound port beyond `node_exporter`'s
`:9100` on the LAN/VPN; NAT'd Pis are covered later by reusing the existing gateway certificate/CA (no new secret).

**ADR-0013 — Operational and clinical alerting are physically separated; Alertmanager is ops-only.**
*Decision:* Prometheus/Alertmanager carry `domain="ops"` alerts to Telegram/email/webhook; clinical alerts use the
doc 08 push path and MUST NOT appear in any Alertmanager rule or route (CI-enforced, REQ-OPS-013).
*Rationale:* a mis-tuned monitoring stack must never be able to page a patient/caregiver; separating the planes makes
this a testable invariant rather than an operational convention.

**Cross-document reconciliation (note, no edit made here):** `07 §4.1` lists Grafana as compose-internal via SSH
tunnel; this document exposes Grafana through Caddy at `grafana.<domain>` (TLS + Grafana auth, REQ-OPS-014) while
keeping Prometheus/Alertmanager internal. When `01 §2.5` (proposed by 07) is written, it must read:
"cloud 443 public (api/grafana/push) + 9443 mTLS; Prometheus/Alertmanager internal; Pi inbound = 22/tcp only".

---

## 10. Applied edits

Applied to 01/03/04 on 2026-10-09 (see ADR-0020 and 04 section 7).

---

## 5-line summary

1. All TSDB/UI/alerting (Prometheus, Grafana, Alertmanager) live on the VPS or dev machine; the Pi adds only `node_exporter` (8 MB request / 20 MB limit), combined with the 3 services at 112/260 MB within the ≤ 400 MB cap.
2. Metrics are pull-first: Prometheus scrapes the Pi (`node_exporter` + `/metrics`) over the LAN/VPN; `vmagent` remote-write is `[LATER]`, only for Pis behind NAT.
3. Naming is `iomt_<component>_<name>_<unit>` with exporter rewrites, strict label/cardinality limits, four dashboards-as-code, and at most eight operational alert rules with `for:` durations and promtool tests.
4. Health is standardised on `/healthz` (liveness) + `/readyz` (readiness) with Compose healthchecks; logs are JSON with bounded Docker rotation and zero PII, and Loki/OTel are explicitly `[LATER]`.
5. Alertmanager is operations-only — clinical alerts are hard-separated onto the doc 08 push path (ADR-0011/0012/0013) — and section 10 records that the edits were applied to 01/03/04 on 2026-10-09.
