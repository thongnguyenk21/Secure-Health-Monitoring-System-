# Implementation Plan - IoMT 3-Tier Edge AI + Federated Learning

## 1. Work Package (WP) Breakdown

| No. | WP Code | Work Package Name | Phase | Main Objective |
|---|---|---|---|---|
| 1 | WP1 | IoT Node hardware design | Phase 1 | Fabricate a wearable device that collects PPG/ECG/IMU |
| 2 | WP2 | Firmware + basic SNNs + BLE | Phase 1 | Read sensors, filter noise, SNN inference, BLE transmission |
| 3 | WP3 | Edge Gateway (K3s/Docker + Microservices) | Phase 1 | Build a gateway to receive, process, and locally train |
| 4 | WP4 | Cloud (Spring Boot + TimescaleDB) + Mobile App (React Native) | Phase 1 | Storage backend + mobile app for visualization & alerts |
| 5 | WP5 | E2E integration + Report + Phase 1 Demo | Phase 1 | End-to-end testing & capstone project defense |
| 6 | WP6 | Time-Series Transformers (Informer/PatchTST) at the Edge | Phase 2 | Upgrade time-series analysis capability |
| 7 | WP7 | Federated Learning + SSL + CI/CD OTA | Phase 2 | Secure distributed training + model updates |
| 8 | WP8 | Upgrade SNNs (LIF/Recurrent SNN) at the IoT | Phase 2 | Optimize accuracy & save energy |
| 9 | WP9 | SOTA Metrics Measurement | Phase 2 | Quantitative evaluation of the entire system |
| 10 | WP10 | Clinical trial + Scientific report | Phase 2 | Real-world trial & thesis completion |

## 2. Timeline by Milestone

| Milestone | Content | Estimate | Dependency |
|---|---|---|---|
| M1 | Complete PCB, sensor-reading firmware & signal filters | 4–6 weeks | — |
| M2 | Embed basic SNN + optimized BLE protocol | 4–6 weeks | M1 |
| M3 | Build Edge Gateway (K3s/Docker) + ingest microservices | 4–5 weeks | M2 |
| M4 | Cloud Backend (Spring Boot + TimescaleDB) + API | 4–6 weeks | M3 |
| M5 | Mobile App (React Native) + dashboard + real-time alerts | 4–6 weeks | M4 |
| M6 | E2E integration + demo + complete Capstone Project report | 2–3 weeks | M1–M5 |
| M7 | Deploy Time-Series Transformers (Informer/PatchTST) | 6–8 weeks | M6 |
| M8 | Integrate FL + SSL (Contrastive Learning) + CI/CD OTA | 6–8 weeks | M7 |
| M9 | Upgrade SNN (LIF/Recurrent SNN) | 4–6 weeks | M6 |
| M10 | Measure SOTA metrics | 4–6 weeks | M8, M9 |
| M11 | Clinical trial 10–30 people / 1–2 months | 6–10 weeks | M10 |
| M12 | Complete scientific report + summary | 3–4 weeks | M11 |

**Estimated total duration:** ~53–74 weeks

## 3. Risks & Mitigation Measures

| Risk | Level | Mitigation |
|---|---|---|
| Limited ESP32-C3 resources | High | int8 quantization, pruning, reduce time-steps |
| BLE bandwidth congestion | Medium | Batching, transmit features/events only, optimize MTU |
| Slow FL convergence / Non-IID | High | FedProx, weighted FedAvg, SSL pretraining |
| Lack of labeled data | High | Contrastive Learning, physiological data augmentation |
| Complex OTA integration | Medium | Model versioning, CI/CD canary, rollback |
| Ethical issues during trials | High | Obtain IRB approval, consent form, PII anonymization |
| Unstable Edge–Cloud connection | Medium | Local buffer, retry policy, offline mode |

## 4. Milestones & Deliverables

| Milestone | Estimated Date (reference) | Deliverables |
|---|---|---|
| MS1 - IoT POC | After M1-M2 | PCB, Firmware, working basic SNN |
| MS2 - Edge Ready | After M3 | Gateway operational, realtime ingest |
| MS3 - Phase 1 Complete | After M6 | E2E Demo + Capstone Project report |
| MS4 - Advanced AI | After M7-M9 | Transformers + FL + upgraded SNN + Metrics |
| MS5 - Validation | After M11-M12 | Clinical trial + completed Thesis |
