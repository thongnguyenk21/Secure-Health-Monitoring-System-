# IoMT 3-Tier Edge AI + Federated Learning

Đề tài: Kiến trúc phân tán 3 tầng tích hợp Edge AI & Federated Learning cho IoMT.

## Tổng quan

Dự án phát triển hệ thống theo dõi sức khỏe phân tán với kiến trúc 3 tầng:
- **IoT Node** (ESP32-C3 + PPG/ECG/IMU + SNN)
- **Edge Gateway** (K3s + Microservices + Time-Series Transformers + FL Client)
- **Cloud** (Spring Boot + TimescaleDB + Flower FL Server + Model Registry)
- **Mobile App** (React Native) hiển thị realtime + cảnh báo

Đặc trưng chính: suy luận cục bộ, huấn luyện liên kết (Federated Learning), cập nhật OTA, ưu tiên bảo vệ quyền riêng tư.

## Cấu trúc dự án

Xem chi tiết tại [docs/architecture/FOLDER_STRUCTURE.md](docs/architecture/FOLDER_STRUCTURE.md)

## Tài liệu

- [JD viết lại](docs/specs/JD_REWRITTEN.md)
- [Kế hoạch triển khai](docs/plan/IMPLEMENTATION_PLAN.md)
- [Đặc tả Work Packages (WP1–WP10)](docs/specs/WP_SPECS.md)
- [Kiến trúc hệ thống](docs/architecture/SYSTEM_ARCHITECTURE.md)
- [Cấu trúc thư mục](docs/architecture/FOLDER_STRUCTURE.md)

## Giai đoạn thực hiện

- **Giai đoạn 1 (Đồ án CN):** WP1–WP5 – Xây dựng hạ tầng & POC (IoT–Edge–Cloud–Mobile + Demo E2E)
- **Giai đoạn 2 (Luận văn):** WP6–WP10 – Tối ưu (Transformers, FL+SSL, SNN nâng cấp), đánh giá SOTA, thử nghiệm lâm sàng

## Tech Stack

| Tầng | Công nghệ chính |
|---|---|
| IoT | ESP32-C3, TFLite Micro, snnTorch/Norse, FreeRTOS |
| Edge | K3s/Docker, PyTorch + ONNX/TensorRT, Flower Client |
| Cloud | Spring Boot 3.x, PostgreSQL+TimescaleDB, Flower Server |
| Mobile | React Native (TypeScript) |

## Bắt đầu nhanh (Dev)

```bash
# 1) Clone repo (nếu cần)
git clone <repo-url> && cd project-personal

# 2) Xem cấu trúc
tree -L 2 || find . -maxdepth 2 -type d

# 3) Dev local (gợi ý)
# - iot-node: cd iot-node/firmware && idf.py build flash monitor
# - edge-gateway: cd edge-gateway && docker compose up -d (dev)
# - cloud: cd cloud/backend && ./mvnw spring-boot:run
# - mobile: cd mobile-app && npx react-native run-android
```

## Lưu ý

- Các giá trị mục tiêu (accuracy, latency, EDP...) là dự kiến – cần thống nhất với giảng viên hướng dẫn.
- Thử nghiệm lâm sàng (WP10) yêu cầu tuân thủ đạo đức nghiên cứu, IRB, Informed Consent, ẩn danh hóa PII.
- Ưu tiên **Privacy-by-Design**: chỉ chia sẻ trọng số mô hình, không truyền dữ liệu thô về Cloud.

## License

TBD (theo quy định đề tài/nền tảng)
