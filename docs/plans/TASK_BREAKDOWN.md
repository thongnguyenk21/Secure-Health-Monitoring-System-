# Detailed Task Breakdown

## Phase 1 - Capstone Project

### WP1: IoT Hardware
- [ ] T1.1: Research specs ESP32-C3, MAX30102, MPU6050, AD8232
- [ ] T1.2: Draw KiCad schematic (PPG+IMU power, filtering)
- [ ] T1.3: PCB layout 2-layer (analog/digital separation)
- [ ] T1.4: Generate Gerber, BOM, DRC/ERC
- [ ] T1.5: 3D enclosure (Fusion 360), export STL/STEP
- [ ] T1.6: Fabrication + assembly + bring-up test

### WP2: Firmware + basic SNN
- [ ] T2.1: ESP-IDF project scaffolding
- [ ] T2.2: Sensor drivers (MAX30102, MPU6050)
- [ ] T2.3: Signal processing (filters, peak detection)
- [ ] T2.4: Train SNN baseline (snnTorch) on small dataset
- [ ] T2.5: Convert to TFLite Micro, quantize int8
- [ ] T2.6: Integrate inference (target < 5ms)
- [ ] T2.7: BLE GATT service + protocol design
- [ ] T2.8: Firmware profiling (RAM/Flash/latency)

### WP3: Edge Gateway
- [ ] T3.1: K3s setup (Raspberry Pi/Jetson)
- [ ] T3.2: Base images + Dockerfiles
- [ ] T3.3: ble-ingest service
- [ ] T3.4: signal-processor + windowing
- [ ] T3.5: buffer-store + local persistence
- [ ] T3.6: local-trainer skeleton (FL client)
- [ ] T3.7: model-agent (load/verify/swap)
- [ ] T3.8: gateway-api (REST/gRPC)
- [ ] T3.9: MQTT/Redis Streams + Helm charts

### WP4: Cloud + Mobile
- [ ] T4.1: Spring Boot init + DB schema + TimescaleDB
- [ ] T4.2: Auth/JWT + RBAC
- [ ] T4.3: Devices/Measurements/Alerts/Models APIs
- [ ] T4.4: Model Registry endpoints
- [ ] T4.5: Flyway migrations + OpenAPI
- [ ] T4.6: React Native scaffold + navigation
- [ ] T4.7: API client + auth flow
- [ ] T4.8: Realtime charts + alerts + FCM

### WP5: E2E Integration
- [ ] T5.1: End-to-end wiring (IoT→Edge→Cloud→App)
- [ ] T5.2: Reconnect/offline recovery test
- [ ] T5.3: Data integrity, timestamps
- [ ] T5.4: Demo script + video
- [ ] T5.5: Capstone Project report + slide

## Phase 2 - Thesis

### WP6: Time-Series Transformers
- [ ] T6.1: Data prep + windowing + splits
- [ ] T6.2: Baseline (LSTM/BiLSTM/TCN)
- [ ] T6.3: Informer/PatchTST implementation (PyTorch)
- [ ] T6.4: Training + validation + metrics
- [ ] T6.5: ONNX export + TensorRT (Jetson)
- [ ] T6.6: Benchmark Edge latency/size

### WP7: FL + SSL + OTA
- [ ] T7.1: Flower FL Server (FedAvg strategy)
- [ ] T7.2: FL Client integrate local-trainer
- [ ] T7.3: SSL pretraining (SimCLR-style)
- [ ] T7.4: Supervised fine-tune + Fed rounds
- [ ] T7.5: Non-IID simulation + FedProx eval
- [ ] T7.6: Model Registry + versioning
- [ ] T7.7: CI/CD pipeline (build/test/package)
- [ ] T7.8: OTA update flow (verify/SHA256/rollback)

### WP8: Upgraded SNN
- [ ] T8.1: LIF/RSNN design + surrogate gradient
- [ ] T8.2: Train + hyperparams
- [ ] T8.3: Quantize + prune + time-steps tuning
- [ ] T8.4: Convert TFLite Micro
- [ ] T8.5: Energy/latency benchmark (INA219)
- [ ] T8.6: Compare vs WP2 (EDP)

### WP9: SOTA Metrics
- [ ] T9.1: E2E latency (p50/p95/p99)
- [ ] T9.2: EDP measurement (IoT)
- [ ] T9.3: Comm Cost per FL round
- [ ] T9.4: Model Drift (time-split)
- [ ] T9.5: Robustness (noise/non-IID/dropout)
- [ ] T9.6: Scalability (5–30 nodes)
- [ ] T9.7: Sens/Spec (if labeled)
- [ ] T9.8: Aggregate results + plots

### WP10: Clinical Trial + Thesis
- [ ] T10.1: IRB prep + Informed Consent (VN)
- [ ] T10.2: Ethics approval + recruitment (10–30)
- [ ] T10.3: Data collection (1–2 months), compliance tracking
- [ ] T10.4: SUS/UEQ questionnaires
- [ ] T10.5: Data cleaning + anonymization
- [ ] T10.6: Statistical analysis (Bland–Altman, CI)
- [ ] T10.7: Thesis writing + review
- [ ] T10.8: Final polish + submission
