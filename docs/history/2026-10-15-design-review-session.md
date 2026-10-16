# Design review session, 2026-10-14 to 2026-10-15

This file records what was decided while reworking the report slides with the
project owner. Read it before you change the slides, the C4 page or the design
documents. The full decisions are in `docs/design/DECISIONS.md` (D22–D34). This
file gives the context, the open items and the working rules that the decision
log does not hold.

## Where things are

| Artifact | Path | State |
|---|---|---|
| Report slides (HTML, 39 slides, Vietnamese) | `docs/reports/system-design-slides.html` | Reworked; under review |
| Slide exports (PDF, per-slide PNG) | `docs/reports/system-design-slides.pdf`, `docs/reports/system-design-slides/` | **Stale** (old 11-slide deck). Regenerate only after the owner approves the HTML |
| C4 overview (HTML) | `docs/design/c4/index.html` | Updated to D22–D34 and to the cloud backend and web app code (2026-10-15) |
| Decision log | `docs/design/DECISIONS.md` | D22–D34 added |
| Requirement (brief) | `docs/specs/JD.md` (Vietnamese original: `docs/history/Jd.md`) | Unchanged |
| Branch / PR | `feature/docs-report-redesign` (from `main`) | Pushed; PR to `main` |
| Cloud backend, web app and mobile app | `cloud/backend` (crate `iomt-backend`), `web-app/`, `mobile-app/`, contracts 0.2.0 | Backend and web slice S1–S2, mobile slice M1 (Expo, 2026-10-16), written on `feature/cloud-backend-web`; not committed yet (the owner commits) |

View the slides locally with `cd docs/reports && python3 -m http.server 8080`,
then open http://localhost:8080/system-design-slides.html (arrow keys, `F` for
full screen, `?all` for every slide on one page).

## Decisions taken in this session

| ID | Decision |
|---|---|
| D22 | Wearable firmware is **C on ESP-IDF**. One C++ file wraps TensorFlow Lite Micro behind `extern "C"`. The Rust crates `iomt-frames`, `offline-buffer` and `snn-lif` stay as reference implementations, and the 16-byte record format is kept bit-for-bit |
| D23 | The on-device model is an **SNN (LIF)** trained with snnTorch, unrolled over T = 16 steps and run as int8 on TFLM. Fallback: a LIF core written in C |
| D24 | **No MVP/LATER split.** Federated learning, firmware OTA over BLE (A/B), Secure Boot v2 and flash encryption are in scope. Out of scope: clinical trials (phone-as-gateway was moved into scope by D32) |
| D25 | **No Prometheus, Alertmanager or node_exporter.** Grafana is only an evaluation tool for evidence and is not deployed with the system |
| D26 | Three roles only: patient, clinician, admin. The researcher role is dropped |
| D27 | **Multi-task SNN**: an activity head (rest/walk/run/unknown) and a health-screening head (normal, tachycardia at rest, bradycardia, SpO₂ drop, fall). The record's activity byte is split 4+4 bits, which needs a `firmware.v1` bump. The output is screening, not diagnosis |
| D28 | **Patients use the mobile app; clinicians and admins use the web app** |
| D29 | Learning is split by tier. The wearable only keeps bounded per-user calibration in NVS (resting HR by EWMA, a resting threshold clamped between clinician limits, a step-length factor). Training happens in local-trainer, aggregation in the FL server. "Tôi ổn" and clinician acknowledgement become labels, and the wearable sends feature vectors on those events |
| D30 | One protocol stack per link (BLE GATT + LESC; gRPC/HTTPS over TLS 1.3 + mTLS; REST/HTTPS + WebSocket; FCM HTTP v1) |
| D31 | **The edge is a Raspberry Pi 4 (4 GB), and local-trainer runs on it** (≤ 1.5 GB RAM, 2 cores, low priority, pauses if alerts slow down). This supersedes the separate edge training machine and the Pi 3B+ |
| D32 | **The patient's phone is the outdoor gateway.** Out of the Pi's range the wearable sends over BLE to the mobile app, which runs the same link session and uploads summaries and alerts to the cloud over HTTPS. Three layers hold data: the wearable ring buffer (kept for when neither Pi nor phone is reachable), the gateway's durable queue, and cloud dedupe on `RecordKey`. A gateway acks only after durable storage. Training option 1: the cloud returns a patient's outdoor summaries to that patient's Pi; feature vectors and labels never go through the cloud. New consent scope `model_training`. Android first. Effort 33–49 person-weeks |
| D33 | **Deployment scope is one real wearable**, one Pi 4 and one cloud VPS. Load tests use 30 simulated devices; federated learning uses the real Pi plus simulated Flower clients. The design capacity (5–30 wearables) is unchanged |
| D34 | **The mobile app targets Android only**; iOS is dropped from scope and from future work |

## Current slide structure

1. Cover, contents.
2. Introduction:
   - health burden with cited statistics (WHO, Lancet, NEJM, Vietnamese sources);
   - wearable market (prices with sources);
   - objectives and scope.
3. Requirements: Use Case Diagram (3 actors, 9 use cases); Non-functional Requirements table.
4. Design:
   - Layered Architecture: 5 layers (wearable, edge, network, cloud, apps). It shows coarse entities only, with one network band and protocol boxes.
   - Layer 1: pipeline and record, sensor formulas, step/LIF formulas, multi-task SNN, SNN training pipeline, training formulas, ring buffer, device state machine, personalisation.
   - Layer 2: Pi 4 services and memory budget; window/HRV/quantisation formulas.
   - Layer 4: cloud services diagram; storage and alerts; federated learning (two slides).
   - Security across layers.
5. UI:
   - mobile app for patients: home, fall alert, history, device;
   - web app for clinicians;
   - web app for admins.
6. Cost of one wearable (about 300,000 VND, exploded view), Gantt plan.
7. Results, the evaluation dashboard (Grafana as a measuring tool), performance (wearable; Pi, cloud and end-to-end; summary against the requirement table, with figure slots for measured results), conclusion, references (two slides), Q&A.

## Working rules the owner asked for

- **Do not export the PDF or PNGs after each edit.** Edit the HTML only, check renders in a scratch folder, and export once the owner says the slides are OK.
- **Wording:**
  - Use standard English names for technical terms, with a plain Vietnamese explanation (Use Case Diagram, Layered Architecture, ring buffer, surrogate gradient, QAT, audit log, pseudonymisation and similar).
  - Avoid literal translations such as "băm mã thiết bị" or "nhật ký kiểm toán chỉ-thêm".
  - On patient and clinician screens, use everyday words: no SNN, model scores, Pi, firmware or buffer.
- **Citations:**
  - Cite as author and year in the text, with a full APA-style footnote at the bottom of the slide.
  - Do not use bracket numbers.
  - Every formula card carries a source tag; the project's own rules are labelled "Quy tắc thiết kế của đề tài" or "Ước lượng của đề tài".
- **Formulas** carry a bulleted "Trong đó" variable legend.
- **Figure and table captions** are centred; figures are numbered in order.
- **Diagrams** must have no crossing arrows and no overlapping labels. Run an overflow check over all slides after every change.
- **Commits:**
  - Use `type(scope): summary`, a body that explains why, and `Refs: WP-01`.
  - Add no tool attribution trailers (repo rule in `docs/engineering/CODING_CONVENTIONS.md`).
- ~~**The cloud backend code is on hold.** Do not start it until the owner asks.~~ The owner started it on 2026-10-15 and chose to have Claude write `cloud/backend`, `web-app/` and `mobile-app/` directly. Firmware, edge and the other learning code are still typed by the owner.

## Open items

- `firmware.v1` contract change: split the activity byte (D27) and add the feature-vector message (D29). Do both in one contracts PR with a version bump.
- D32 contracts bump: the app uplink route in `cloud-api.openapi.yaml`, the `model_training` consent scope, and the cloud → Pi download of outdoor summaries. Can go in the same contracts PR as the `firmware.v1` bump.
- D32 in `device-sim`: a second gateway stub (the phone) to test hand-over between Pi and phone (08 T-REL-047-01).
- Port `iomt-frames` and `offline-buffer` to C with byte-for-byte tests against the Rust versions. Then run the SNN-on-TFLM spike (D23).
- Bring the design docs 01, 03, 04, 06, 07 and 08 and the READMEs in line with D22–D31. They still describe the Rust firmware, Prometheus, the MVP split and the Pi 3B+.
- Slide placeholders: university, faculty, course, supervisor, team and student IDs, and the logo.
- Before submission:
  - verify the weaker sources: Vietnamese news figures and the step-length coefficient;
  - add author names for the arXiv:2407.21676 reference;
  - consider renumbering references in citation order.
- Not yet costed: the Pi 4 (about 0.5–1 million VND more than the Pi 3B+ per home) and the extra web app work in MVP-06.
- Alert thresholds (Δ, θ_min, θ_max, α, τ) are placeholders that clinicians must set. The cloud alert engine's defaults (hr_high > 140 for 10 s, hr_low < 40, spo2_low < 90 %, escalation after 10 min) are placeholders too.
- Web app deviations from the slide 26/27 wireframes, for the owner to accept or fix:
  - a clinician sees every patient of the tenant, not only assigned patients (07's permission matrix already says "all in tenant");
  - sex, age, battery and diagnosis are not in the API (the seed writes "Nam, 54" into `display_name` as demo text only);
  - the federated-learning and OTA rollout panels are empty states.
- Mobile app M1 gaps against the slide 25 wireframes and the API: no fall alert type; no speed, battery, firmware-update status or unsent-queue size; no caregiver API ("Báo người thân" is local only); alerts carry no activity; clinical notes are not visible to patients. Next: M2 Android dev build + FCM, M3 BLE and the outdoor gateway (D32).
- Cloud backend gaps against 07/08, recorded as open points in those documents:
  - quarantine window `[now − 30 d, now + 5 min]` versus `[enrolled − 5 min, now + 60 s]` in REQ-REL-007;
  - a resend with a different payload is ignored, not quarantined as `idempotency_conflict` (REQ-REL-008);
  - continuous aggregates are not built (rollups use `time_bucket` at query time);
  - the login limiter counts every attempt and keys on the TCP peer IP, which becomes Caddy's IP behind the proxy;
  - no `tenant_id` columns;
  - the audit log lacks some columns and the daily anchor and chain verifier;
  - ~~keeping the WebSocket `access_token` query string out of logs is not enforced~~ resolved 2026-10-15: backend request spans log the method and path only.
