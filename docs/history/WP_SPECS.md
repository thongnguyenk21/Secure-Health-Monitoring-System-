# Đặc tả chi tiết Work Packages (WP1–WP10)

## WP1: Thiết kế phần cứng IoT Node

| Mục | Đặc tả chi tiết |
|---|---|
| Mục tiêu | Chế tạo thiết bị đeo thu thập tín hiệu sinh lý PPG/ECG/IMU ổn định |
| Vi điều khiển | ESP32-C3 (RISC-V 32-bit, BLE 5.0, Wi-Fi, ~400KB SRAM, 4MB Flash) |
| Cảm biến | MAX30102 (PPG, HR, SpO2), MPU6050 (6-DOF), tùy chọn AD8232 (ECG) |
| Nguồn điện | Pin Li-Po 3.7V 1000–1500mAh, TP4056, LDO 3.3V, bảo vệ quá áp/thiếu áp |
| PCB | 2 lớp, ≤40×30mm, test point, tối ưu tín hiệu analog/digital |
| Vỏ hộp | In 3D (PETG), gọn nhẹ, tiện đeo |
| Công cụ | KiCad, Fusion 360/FreeCAD, Oscilloscope, Multimeter |
| Deliverables | Schematic, PCB Layout, Gerber, BOM, 3D model (.step/.stl), mẫu hoạt động |
| Tiêu chí nghiệm thu | PPG+IMU ổn định; pin ≥8h; sai số HR ≤ ±5 BPM vs tham chiếu |

## WP2: Firmware & SNNs cơ bản + BLE

| Mục | Đặc tả chi tiết |
|---|---|
| Mục tiêu | Thu thập – xử lý – suy luận – truyền dữ liệu |
| Nền tảng | ESP-IDF (ưu tiên), Arduino Core, FreeRTOS |
| Xử lý tín hiệu | Band-pass (0.5–5Hz), Moving Average/Butterworth, Complementary/Kalman, Peak detection |
| Mô hình AI | SNN cơ bản (Spiking FC), train với snnTorch/Norse → TFLite Micro |
| Bài toán | rest/walk/run/fall |
| Chỉ tiêu | Inference < 5ms, RAM ≤150–200KB, model int8 ≤120–150KB |
| Truyền thông BLE | GATT Service/Characteristic, Custom Profile, tối ưu MTU, Connection Interval |
| Deliverables | Firmware src, unit test, tài liệu BLE, log benchmark |
| Tiêu chí nghiệm thu | Inference < 5ms; Accuracy ≥85%; BLE ổn định ≥30 phút |

## WP3: Edge Gateway (K3s/Docker + Microservices)

| Mục | Đặc tả chi tiết |
|---|---|
| Mục tiêu | Gateway xử lý realtime & hỗ trợ huấn luyện cục bộ |
| Phần cứng | Raspberry Pi 4/5 (4–8GB) hoặc Jetson Nano/Orin Nano |
| Kiến trúc | K3s, containerd/Docker, Helm |
| Microservices | ble-ingest, signal-processor, buffer-store, local-trainer, model-agent, gateway-api |
| Messaging | MQTT (Mosquitto/EMQX) hoặc Redis Streams, gRPC nội bộ |
| Lưu trữ cục bộ | SQLite/InfluxDB Lite hoặc filesystem |
| Giám sát | Prometheus + Node Exporter + Grafana |
| Deliverables | K3s manifests/Helm charts, Dockerfiles, sơ đồ kiến trúc, hướng dẫn deploy |
| Tiêu chí nghiệm thu | ≥24h uptime; throughput ≥80–100 mẫu/s; tự phục hồi pod; ổn định CPU/RAM |

## WP4: Cloud (Spring Boot + TimescaleDB) + Mobile App (React Native)

| Mục | Đặc tả chi tiết |
|---|---|
| Backend | Spring Boot 3.x (Java 17+), Spring Security + JWT, Spring Data JPA |
| CSDL | PostgreSQL 14+ + TimescaleDB (hypertable measurements) |
| API | /auth, /devices, /measurements, /alerts, /models, /users |
| Bảo mật | HTTPS/TLS, BCrypt, RBAC, audit log |
| Model Registry | Lưu global model weights + metadata (version, round, hash, ts) |
| Mobile App | React Native (TS), Navigation, Axios, Chart Kit/Victory, Push Notification (FCM) |
| Tính năng | Login, devices, realtime charts, history, alerts, connection status |
| Deliverables | Backend src + migration, OpenAPI, ERD, App src, APK buildable, API docs |
| Tiêu chí nghiệm thu | API p95 < 200ms; app update ≤2s; alert ≤5s |

## WP5: Tích hợp E2E + Báo cáo + Demo GĐ1

| Mục | Đặc tả chi tiết |
|---|---|
| Mục tiêu | Demo IoT→Edge→Cloud→App |
| Kịch bản | Node→Edge ingest→Cloud lưu→App hiển thị+cảnh báo |
| Kiểm thử | E2E, reconnect test, data integrity |
| Tài liệu | Báo cáo Đồ án CN, slide, user manual, sơ đồ hệ thống |
| Demo | Video toàn bộ luồng |
| Deliverables | Test script, báo cáo PDF, slide, video, repo Git (README) |
| Tiêu chí nghiệm thu | Luồng mượt, không mất dữ liệu khi ngắt mạng ngắn (<30s) |

## WP6: Time-Series Transformers (Informer/PatchTST)

| Mục | Đặc tả chi tiết |
|---|---|
| Mục tiêu | Nâng cấp phân tích chuỗi thời gian tại Edge |
| Kiến trúc | Informer (sparse attention) hoặc PatchTST (patching + channel-independence) |
| Framework | PyTorch 2.x, PyTorch Lightning, einops |
| Input | Sliding window 30–120s (PPG-derived + IMU + ts) |
| Bài toán | Forecasting / Anomaly detection / Classification |
| Baseline | LSTM, BiLSTM, TCN |
| Tối ưu Edge | ONNX (opset17+), torch.compile, TensorRT (Jetson) |
| Metrics | MAE, RMSE, MAPE, Accuracy/F1 |
| Deliverables | Model src, training pipeline, config, ONNX/TensorRT, báo cáo so sánh |
| Tiêu chí nghiệm thu | MAE/RMSE ≥10–20% cải thiện vs baseline tốt nhất |

## WP7: Federated Learning + SSL + CI/CD OTA

| Mục | Đặc tả chi tiết |
|---|---|
| Mục tiêu | Huấn luyện phân tán bảo mật + OTA |
| FL Framework | Flower (flwr) |
| Thuật toán | FedAvg, mở rộng FedProx nếu non-IID |
| Kiến trúc | FL Client (Edge) – FL Server (Cloud) |
| SSL | Contrastive Learning (SimCLR-style) + pretraining |
| Augmentation | Time warping, jittering, scaling, permutation (an toàn sinh lý) |
| Bảo mật | Secure Aggregation (khảo sát), DP-SGD (tùy chọn) |
| CI/CD | GitHub Actions/GitLab CI (lint/test/build/package/versioning) |
| OTA | Model Agent pull từ Registry, SHA256 verify, backup+rollback, hot-swap |
| Deliverables | fl-server, fl-client, Model Registry API, workflows, tài liệu FL flow |
| Tiêu chí nghiệm thu | Hội tụ ≤15–30 rounds; accuracy ≥95–98% vs centralized; OTA thành công không downtime |

## WP8: Nâng cấp SNNs (LIF/Recurrent SNN)

| Mục | Đặc tả chi tiết |
|---|---|
| Mục tiêu | Cải thiện accuracy & tiết kiệm năng lượng |
| Kiến trúc | LIF, Recurrent SNN (RSNN) |
| Huấn luyện | snnTorch/Norse + Surrogate Gradient |
| Encoding | Rate coding / Latency coding |
| Tối ưu | Quantization int8, Weight pruning, giảm time_steps, event-driven |
| Benchmark | INA219/Power Profiler Kit (Energy per inference = V*I*Δt) |
| So sánh | vs WP2 (Accuracy, Latency, RAM/Flash, Energy, EDP) |
| Deliverables | Training scripts, .tflite, firmware mới, bảng benchmark |
| Tiêu chí nghiệm thu | Accuracy ≥90% (ưu tiên ≥92%); Energy ↓ ≥30% vs WP2; Inference < 5ms |

## WP9: Đo lường SOTA Metrics

| Chỉ tiêu | Định nghĩa | Phương pháp đo | Công cụ | Mục tiêu |
|---|---|---|---|---|
| End-to-end Latency | T nguồn→hiển thị/cảnh báo | Timestamp source/sink (avg/p95/p99) | Python log | < 150–300ms |
| EDP | Energy × Inference Latency | V,I,t_infer | INA219/Power Profiler | ↓ ≥25–35% vs WP2 |
| Communication Cost | Bytes/FL round (up+down) | Payload+overhead | Flower metrics + wireshark | Giảm so với raw |
| Model Drift | Suy giảm hiệu suất theo thời gian | Train-old/test-new | sklearn | Phát hiện & phục hồi |
| Robustness | Ổn định non-IID+nhiễu | Inject noise/dropout/imbalance | Simulator+real | Accuracy drop ≤5–8% |
| Scalability | Hiệu năng khi tăng nodes | 5→10→20→30 | Locust/k6 + Flower | Round time tuyến tính |
| Sensitivity/Specificity | Chỉ số chẩn đoán | Confusion Matrix, ROC-AUC | sklearn | ≥90%/≥90% |

**Deliverables:** benchmark/, raw logs, biểu đồ, hướng dẫn tái lập

## WP10: Thử nghiệm lâm sàng & Báo cáo khoa học

| Mục | Đặc tả chi tiết |
|---|---|
| Đối tượng | 10–30 tình nguyện viên |
| Thời gian | 1–2 tháng |
| Thiết bị tham chiếu | Thiết bị y tế chuẩn (đối chiếu, không thay thế chẩn đoán) |
| Chỉ tiêu | Sensitivity, Specificity, Precision, Recall, F1, Compliance, SUS, UEQ |
| Đạo đức | IRB, Informed Consent Form (VN), ẩn danh hóa PII |
| Phân tích | Descriptive, Bland–Altman, 95% CI |
| Deliverables | Luận văn, consent form, khảo sát SUS/UEQ, dataset ẩn danh |
| Tiêu chí nghiệm thu | Cỡ mẫu đủ; Compliance ≥70–80%; SUS ≥70; báo cáo đầy đủ, trung thực |

## Tech Stack Tổng hợp

| Tầng | Công nghệ | Ghi chú |
|---|---|---|
| IoT Node | ESP32-C3, MAX30102, MPU6050, AD8232(tùy chọn), TFLite Micro, snnTorch/Norse, FreeRTOS | Ưu tiên TFLite Micro cho nhúng |
| Edge Gateway | Pi4/5/Jetson, K3s, Docker, MQTT/gRPC, PyTorch+Lightning, ONNX, TensorRT | Jetson lợi thế cho Transformer |
| Cloud | Spring Boot 3.x, PostgreSQL+TimescaleDB, Flyway, JWT+RBAC, Flower, Model Registry | Tách FL Server vs Core API |
| Mobile | React Native (TS), Charts, FCM/OneSignal | Push notification đơn giản |
| AI/ML | snnTorch, Norse, PyTorch, Flower, sklearn | Tập trung nhẹ, phù hợp biên |
| DevOps/Tools | Git, GitHub Actions/GitLab CI, Docker Buildx, Prometheus+Grafana | Theo dõi cơ bản |
| Benchmark | Locust/k6, Wireshark, INA219/Power Profiler, pytest/JUnit | Đo lường & tái lập |
