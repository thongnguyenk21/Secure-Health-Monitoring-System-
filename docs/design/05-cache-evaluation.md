# 05 — Cache Evaluation: is a cache needed? (a quantitative assessment)


---

## Final decisions (authoritative — 2026-10-09; supersedes any earlier wording in this file)

1. **No cache component is added to the architecture** (ADR-0008, `03 §4`, policy text `01 §2.1.1`): no Redis/Memcached, no shared read cache, no CDN for the medical API, nothing cache-like added to the Pi (Redis is forbidden on the Pi — `03 §7.1`).
2. **No JWKS cache.** The backend issues and verifies JWT with locally configured keys; there is no external IdP/JWKS in the MVP. `MET-14` (`03 §7`) = chart query p95 — that id must never be reused for caching.
3. **TimescaleDB continuous aggregates are `[MVP]` and always on (D18):** 1 min / 1 h / 1 day rollups over ~1 Hz summary vitals + one feature row per 30–120 s window; compression + retention are `[MVP]` as well (D18, `03 §7.2`).
4. **Pi storage = high-endurance microSD (D19)** — resolved, no longer an open contradiction; SSD/USB is the `[LATER]` clinical-phase lever. Writes are minimised by WAL batching / group commit (MET-13).
5. **Transport Edge→Cloud = gRPC/HTTPS; no MQTT in the MVP.** IoT→Edge stays BLE 5 GATT; MQTT (including retained/last-will) is `[LATER]`.
6. **Implementation defaults, not architecture components:** warm ONNX session with cleared tensor buffers; batched/WAL SQLite group commit in `edge-sync`; HTTP ETag/`sha256` for OTA artifacts; TanStack Query `staleTime` (charts 30 s, alerts 0); alert de-dup/hysteresis state. Only the policy + acceptance criteria get recorded.
7. **Never cached:** the eight items in §4 (raw samples, alert inputs, revocation/RBAC state, PHI, user-scoped responses, safety status, model integrity metadata, Redis on the Pi).
8. **Recorded elsewhere:** ADR-0008 and MET-13..MET-17 live in `03 §4`/`§7` and `01 §2`; the former §6 "proposed edits to 01/03/04" and the former §7 summary of this file were applied on 2026-10-09 and have been removed.

---

## 0. Premises (verified against `docs/design/01..04` + `DECISIONS.md`)

- Assumed workload: **5–30 sensor nodes** (`01`), windows **30–120 s**, PPG/ECG/IMU at **25–100 Hz** (an explicit assumption — no doc states sensor Hz; confirm via `MET-16`/benchmarks), edge = **Pi 3B+ 1 GB** with the edge-lite budget defined once in **`03 §7.1`** (every RAM figure below refers there rather than repeating arithmetic).
- **Upload granularity = D18:** ~1 Hz per-node summary vitals (real-time display) + one feature row per window (analytics/FL); raw samples stay on the edge and are **not durably stored** in the MVP.
- **Storage = D19:** high-endurance microSD — resolved; the old "SD card vs SSD/USB" contradiction is closed (SSD/USB = `[LATER]` clinical phase).
- **Edge→Cloud = gRPC/HTTPS** (no MQTT in the MVP); IoT→Edge = BLE 5 GATT.
- **Archived/legacy documents are not authority:** `docs/history/WP_SPECS.md` and `docs/architecture/SYSTEM_ARCHITECTURE.md` still carry Spring Boot/Redis/MQTT remnants; where they disagree, `docs/design/01..04` + `DECISIONS.md` win (`00 §0.0`, `INDEX.md`).
- What crosses Edge→Cloud (Privacy-by-Design, `docs/specs/JD.md`): ~1 Hz vitals + window/feature rows + alert flags — **never raw samples**.

---

## 1. Workload model (with calculations)

### 1.1 Unit assumptions (stated explicitly for verification)

- Sample payload: `P = PPG(2 float) + IMU(3 float) = 5 × 4 B = 20 B`; + ts 8 B ≈ **28 B/sample**; BLE framing when batching (10 samples/frame within MTU 247 B) ≈ +1 B/sample ⇒ **~29–30 B/sample**.
- Channel count follows `telemetry.proto` (`repeated float ppg`, `repeated float imu_xyz`): PPG 2 channels (IR+Red) + 3 IMU.
- Window record (Edge→Cloud) ≈ 120 B payload ⇒ ≈ **200 B/row** on disk (Timescale index/overhead; compression ~10×).
- f_s = 25 / 50 / 100 Hz; w = 30 / 60 / 120 s; N = 5 / 30 nodes.

### 1.2 BLE IoT→Edge (bytes/s per node)

| f_s | bytes/s per node | notifications/s (1 sample/notif) | notif/s if batch 10 |
|---|---|---|---|
| 25 Hz | ~750 B/s | 25 | 2.5 |
| 50 Hz | ~1.5 KB/s | 50 | 5 |
| 100 Hz | ~3.0 KB/s | 100 | 10 |

- Bandwidth is **not** the problem (3 KB/s × 8 = 24 kbps, BLE 5 has ample capacity). The problem is **connections**: BLE 5 on a Pi 3B+ gives ~7–10 stable connections/adapter ⇒ 30 nodes need ≥3–4 adapters or batch scanning. Not cache-related — a **RISK** for `edge-ingest`, measured by `MET-16`.
- 1 sample/notification at 100 Hz = 100 notif/s/node, close to the connection-interval limit ⇒ **firmware MUST batch ~10 samples/frame**.

### 1.3 Rows/s and rows/day into Timescale (D18 upload model)

| | 5 nodes | 30 nodes |
|---|---|---|
| ~1 Hz summary vitals, rows/s | 5 | **30** |
| rows/day from 1 Hz vitals | 432,000 | **2.59M** |
| + feature rows per window (30–120 s), rows/day | 3,600–14,400 | 21,600–86,400 |
| **total rows/day** | ~0.44–0.48M | **≈ 2.7M** |
| **7-day chart, all devices** | — | **≈ 18.7M rows** |
| 7-day chart, one device (1 Hz) | — | 604,800 rows |

Raw-sample upload (**rejected** — Privacy-by-Design): 100 Hz × 30 nodes = 3,000 rows/s = 259M rows/day ⇒ a 7-day chart = 1.8B rows, infeasible for interactive queries without mandatory rollups; raw stays on the edge.

**Write-path conclusion:** even at 30 rows/s the write load is trivial for PostgreSQL/TimescaleDB (10⁴–10⁵ insert/s) ⇒ **no cache/staging on the write path.** The read path at 18.7M rows/7d is why **CAGGs are `[MVP]` (D18)** — precomputation, not a cache (§2c).

### 1.4 SQLite writes/s on the Pi and SD wear (D19 microSD)

- **MVP (D18 rate):** ~30 rows/s @30 nodes + WAL ⇒ ≈ **1.0 GB/day** on disk (SQLite page/commit floor; the canonical figure is the MET-13 note in `03 §7`). With `journal_mode=WAL` + group commit the **fsync rate is ~2/min** ⇒ wear on a high-endurance card is **negligible**. (The pre-D18 window-only figure was ~1 row/s ≈ 17.3 MB/day; superseded by D18.)
- **`[LATER]` raw retention (FL only)** @3000 samples/s: 28 B × 3000 = **84 KB/s** payload; page/WAL amplification ~2–3× ⇒ ~200 KB/s ≈ 17.3 GB/day ≈ **519 GB/month**. At 10–30 TB TBW that is ≈ **19–58 months** of continuous logging (the earlier "dies in weeks" claim was wrong — see §8 note 3), but fsync without batching tops out at ~50–200 commit/s ≪ 3000 insert/s ⇒ batching is mandatory: batch 500 row/txn ⇒ 6 commit/s, or fsync every 500 ms ⇒ 2 commit/s.
- **RAM ring buffer** holds 60–120 s of raw for windowing/alert: 3000 × 30 B × 60 s ≈ **5.4 MB** — inside `edge-sync`'s share of the budget in `03 §7.1`.
- Raw samples are **not durably stored** in the MVP (inference + windowing read the ring buffer; only vitals/window rows reach SQLite).

### 1.5 Dashboard query shapes on a small VPS (no cache)

Assumes a 2 vCPU / 4 GB cloud VPS (same class as `07 §5.2`) with PG+Timescale on the same machine; a simple index/range scan yields ~1–6M rows/s.

| Query | Rows scanned (D18 model) | Est. **without** CAGG | MVP handling |
|---|---|---|---|
| Latest value per device | 30 index probes `(device_id, ts DESC)` | **<2 ms** | composite index / `DISTINCT ON` |
| Chart 1h, all devices | ~108k | **20–50 ms** | time index |
| Chart 24h, all devices | ~2.6M | ~0.4–1 s | **CAGG 1-min (MVP, D18)** |
| Chart 7d, one device | 604,800 | ~100–300 ms | **CAGG (MVP, D18)** |
| Chart 7d, all devices | **≈18.7M** | **seconds** | **CAGG (MVP, D18)** |
| Alert list (limit 50) | index `(user_id, ts DESC)` | **<5 ms** | B-tree |

**Conclusion:** **no application cache is needed** — every query is an index/range scan, and with the **MVP continuous aggregates (D18)** serving the 24h/7d ranges every chart stays well inside `MET-07` p95 <200 ms. `MET-14` (chart p95 per range) is the gate: >150 ms ⇒ extend rollup coverage. In raw-upload mode a 7d chart would be 1.8B rows — another reason raw stays local.

---

## 2. Candidate-by-candidate evaluation

Legend: **NO-CACHE** = do not do it; **MVP** = required in Phase 1; **LATER** = Phase 2+.

### (a) Warm ONNX session + reused tensor buffers (edge) — **MVP** (implementation requirement, not a cache)

- Cold-loading an int8 model ≤5 MB from storage takes 10–100 ms ⇒ keep the session for the process lifetime. This is an implementation approach, **not** an architecture decision and **not** tied to `MET-01` (which is the ESP32-C3 SNN metric; see §8 note 2) — verify with a host/on-device bench.
- Mandatory test: reused buffers are cleared/zero-filled between inferences (a stale frame must never leak into a medical result).
- RAM: model ≤5 MB + arena inside `edge-ingest`'s share of the budget in `03 §7.1`.

### (b) In-RAM ring buffer + batched/WAL group commit before SQLite — **MVP** (durable queue, not a read cache)

- Needed for store-and-forward during cloud outages (`MET-08` ≥99%, no loss for outages <30 s) and to bound SD wear: fsync without batching tops out at ~50–200 commit/s; batch ≤500 row or fsync ≤500 ms ⇒ 2–6 commit/s (**MET-13**).
- At the D18 write rate (~30 rows/s) plain autocommit would suffice, but the same WAL/group-commit path also covers the `[LATER]` raw-retention case ⇒ keep it as the `edge-sync` queue mechanism (ADR-0018).
- Consistency: a crash loses at most one fsync interval — acceptable for historical telemetry; **the alert path reads `edge-ingest`'s live ring buffer, never SQLite.**

### (c) TimescaleDB continuous aggregates / rollups — **MVP, always on (D18)**

- Supersedes the earlier `LATER` verdict (§8 note 4): upload is ~1 Hz + per-window rows ⇒ 7d all-devices ≈ 18.7M rows (§1.3/§1.5) ⇒ rollups are required to hold chart p95 <200 ms.
- This is **precomputation** (a pre-built rollup table), not a cache: no invalidation; refresh on schedule ⇒ maximum staleness = refresh period (1–5 min). **Alerts never read CAGGs.**
- Measured by `MET-14` (chart p95 per range), `MET-17` (freshness lag), `MET-07`.

### (d) In-process JWT / RBAC / device-metadata cache — **NO-CACHE**

- There is **no JWKS** in the MVP: the backend issues and verifies JWT with locally configured keys ⇒ nothing to fetch, nothing to cache (never reuse `MET-14` for this; see §8 note 1).
- RBAC role→permission and device metadata are PK lookups <1 ms with ≤30 users/devices ⇒ caching only adds invalidation bugs and a security risk (a revoked permission surviving a TTL). Revocation is checked on every request — §4.3.

### (e) Redis / shared cache (Cloud & Pi) — **NO-CACHE**

- **Pi:** forbidden by the 1 GB budget (`03 §7.1`, "Forbidden on Pi: … Redis"). **Cloud:** 30 rows/s writes and index-covered reads (§1.3/§1.5) ⇒ Redis adds a service + invalidation synchronisation + privacy risk without solving any measured problem.

### (f) HTTP ETag/conditional GET + artifact reuse for model OTA — **MVP** (immutable content)

- Model ≤5 MB, updated ~1 time/week ⇒ without reuse every retry/boot re-downloads 5 MB (30 nodes ≈ 150 MB/week wasted + 5 MB SD write each time). ETag = `sha256` of the immutable `name-semver+build` artifact; `model-agent` **still verifies Ed25519 + SHA256** before apply (ADR-0006, `04 RW-06`) ⇒ no stale semantics.
- Measured by `MET-04` (OTA bytes) + a 304 Not Modified test. `[LATER]`: content-addressed object storage/CDN for the model registry (§2i).

### (g) React Native client cache (TanStack Query) + push vs poll — **MVP** (implementation default)

- Trial ≤30 users; `staleTime = 30 s` for charts/latest cuts polling from ~3 to ~1 req/s; history refetches on focus. **Alerts must never read cache** (`staleTime = 0`) and arrive by FCM push (`MET-09` <5 s); history/latest may poll 30–60 s.
- On the phone — not counted against the Pi budget. Measured by `MET-15` (req/user + alert freshness).

### (h) Alert de-duplication/debounce state — **MVP** (domain logic, not a performance cache)

- Grouping by `(device_id, alert_type, severity)` within an N-second window prevents alert storms from a noisy PPG/IMU. Consistency (life-critical): hysteresis + re-arm when the condition ends, escalation always passes through, and **every suppression is logged** for traceability.
- A few KB/device of state. Measured by `MET-09` + a "no lost alerts" sequence test.

### (i) CDN / edge caching — **NO-CACHE**

- The API is authenticated, per-user medical data ⇒ never cached in common (PHI leakage / cross-patient mixing risk); ≤30 devices are too small to need a CDN. `[LATER]`: CDN **only** for immutable assets (model artifact, static images) with `Cache-Control: immutable`, never for `/measurements`, `/alerts`, `/auth`.

### (j) Other candidates

1. **(j1) TimescaleDB compression + retention** — a storage optimisation, not a cache. **MVP (D18):** `add_compression_policy` after 7 days, retention per governance; ~1.0 GB/day at the D18 rate vs 7.3 GB/day if raw were uploaded, columnar compression ~10×. Measured by `MET-17`.
2. **(j2)** Durable store-and-forward queue on the edge — merged into (b); needed for `MET-08`, not a cache.
3. **(j3)** Cloud in-memory "last value per device" map — **NO-CACHE**: the index probe is already <2 ms (§1.5); adds divergence risk from the DB.
4. **(j4)** MQTT retained/last-will for last-known-state — **not in the MVP**: Edge→Cloud transport is gRPC/HTTPS and MQTT is `[LATER]` (§0); the latest-value query is <2 ms anyway.
5. **(j5)** sqlx connection pool / prepared statements — mandatory pooling (lifecycle), not a cache.
6. **(j6)** TLS session resumption — enabled by default at the TLS layer for OTA connections; **NO-CACHE** (no added layer).
7. **(j7)** Edge cache of device-registry/config from Cloud — **NO-CACHE**: local SQLite lookup <1 ms; small config loaded at start.

---

## 3. Verdict table (specification summary)

| # | Component (path `01`) | Verdict | Key point | Metric |
|---|---|---|---|---|
| a | `edge-gateway/edge-lite/services/edge-ingest` | **MVP** (impl. req.) | warm session, process lifetime; buffers cleared between inferences | host/on-device bench + no-buffer-leak test |
| b | `…/edge-sync` | **MVP** (durable queue) | ring buffer ≤10 MB; WAL + batch ≤500 row / fsync ≤500 ms; delete after Cloud ack; alerts do not read it | `MET-13`, `MET-08` |
| c | `cloud/backend/migrations` (Timescale CAGG) | **MVP, always on (D18)** | 1 min/1 h/1 d rollups; refresh 1–5 min; alerts do not use it | `MET-14`, `MET-17`, `MET-07` |
| d | `cloud/backend/src/infra` (auth) | **NO-CACHE** | local JWT keys, no JWKS; revocation checked per request (§4.3) | — (never reuse `MET-14`) |
| e | Redis / shared cache (Pi + cloud) | **NO-CACHE** | forbidden on Pi (`03 §7.1`); unjustified on cloud | — |
| f | `…/model-agent` (OTA) | **MVP** (immutable) | ETag = `sha256`; keeps active + previous slot; Ed25519/SHA256 verified before apply | `MET-04` + 304 test |
| g | `mobile-app/src/features/*/{api,state}` | **MVP** (impl. default) | `staleTime` 30 s charts/latest; **alerts `staleTime=0`** + FCM push | `MET-15`, `MET-09` |
| h | `cloud/backend/src/domain` (alert engine) | **MVP** (domain logic) | de-dup key includes severity; hysteresis + re-arm; escalation passes through | `MET-09` + no-loss test |
| i | CDN / edge caching | **NO-CACHE** | per-user PHI; `[LATER]` only for immutable assets | — |
| j1 | `cloud/backend/migrations` (compression/retention) | **MVP (D18)** | compress after 7 d; retention per governance | `MET-17` |
| j3–j7 | last-value map / MQTT retained / registry / TLS resumption | **NO-CACHE** | built-in feature or <2 ms query; no measurable benefit | — |

---

## 4. Things that must ABSOLUTELY not be cached (and why)

Cross-references from other docs cite these as `05 §4.1` … `05 §4.8`.

| §4.n | Must not be cached | Why |
|---|---|---|
| 4.1 | Raw / near-real-time physiological samples (`edge-ingest` input) | Stale = wrong diagnosis, life-threatening; raw lives only in the live ring buffer and is not durably stored in the MVP (§1.4) |
| 4.2 | Alert decisions / alert-engine inputs | Must reflect current state, nothing seconds stale; alerts also bypass CAGGs and the `edge-sync` queue (§2b, §2c) |
| 4.3 | JWT revocation / RBAC changes | Checked on **every** authenticated request via in-process registry lookup (<1 ms); a TTL-cached permission would keep a revoked user effective until expiry — forbidden |
| 4.4 | PHI / raw signal on the Cloud or a shared cache | Violates Privacy-by-Design (`docs/specs/JD.md`): raw never leaves the edge; only ~1 Hz vitals + window/feature rows + alert flags are uploaded (§0) |
| 4.5 | User-scoped (auth-scoped) API responses in a shared cache | Risk of data leakage between patients/tenants |
| 4.6 | Device online/offline & connection status when used for safety | Must be fresh |
| 4.7 | Model integrity metadata (SHA256 / signature) | Always re-verify on load/apply (Ed25519 + SHA256, ADR-0006); never trust it from a cache |
| 4.8 | Redis / shared cache on the Pi | Forbidden by the 1 GB budget (`03 §7.1`) |

---

## 8. Verification and adjustments (2026-10-09) — section id kept (other docs cite `05 §8`); supersedes §0–§4 in case of conflict

Recalculated independently: the arithmetic in §1 is correct and the "no shared cache / no read-cache layer" conclusion stands. Short form: *Final decisions* at the top.

1. **(d) JWKS cache → NO-CACHE.** The backend issues and verifies JWT itself (locally configured keys); there is no external IdP/JWKS. Only reconsider if an external IdP is integrated later. The old JWKS metric is dropped; `MET-14` in `03 §7` means chart query p95 and must not be reused for caching.
2. **(a) Warm ONNX session** is the default implementation approach (never reload the model per inference), not an architecture decision, and not related to `MET-01` (MET-01 = SNN <5 ms on ESP32-C3). Record it only as an implementation requirement + a no-buffer-leak test.
3. **SD wear (D19 microSD):** at 519 GB/month in the `[LATER]` raw mode and 10–30 TB TBW the card lasts ≈19–58 months, not "a few weeks". Batching is still worth doing; the D18 MVP rate (~1.0 GB/day, ~2 fsync/min with WAL) is negligible.
4. **Upload granularity — RESOLVED by D18:** ~1 Hz summary vitals + one feature row per 30–120 s window ⇒ 30 rows/s @30 nodes ≈ 2.6M rows/day; a 7-day all-devices chart ≈ 18.7M rows and a single-device 604k-row scan is already ~100–300 ms ⇒ **continuous aggregates are `[MVP]`, always on** (§2c). `MET-14` >150 ms triggers extending rollup coverage. Raw upload stays rejected (Privacy-by-Design).
5. **Reclassification:** (b) batch/WAL, (f) ETag/sha256, (g) TanStack Query and (h) alert de-dup are all implementation defaults / domain logic, not added architecture components — only the policy + acceptance criteria need to be recorded.
6. **Metrics:** the metric set first proposed by this file was recorded in the canonical registry `03 §7` as MET-13 (SD write volume + SQLite commit rate, recomputed from D18), MET-14 (p95 chart query per range 1h/24h/7d), MET-15 (mobile req/user + alert freshness), MET-16 (BLE connection capacity), MET-17 (disk/day + CAGG freshness lag). This file only cites `03 §7`; it does not maintain its own metric registry.
7. **Decision conclusion:** **no cache component** is added to the architecture — no shared cache, Redis forbidden on the Pi, no CDN for the medical API, no JWKS cache (ADR-0008, policy text `01 §2.1.1`). CAGGs are MVP (D18), Pi storage is microSD (D19) and Edge→Cloud is gRPC/HTTPS: both formerly open points (upload frequency, storage medium) are closed.
