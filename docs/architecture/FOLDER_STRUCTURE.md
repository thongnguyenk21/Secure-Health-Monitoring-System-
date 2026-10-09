# Cấu trúc thư mục đề xuất (Project Structure)

## Cấu trúc gốc

```text
/project-personal/
│
├── docs/
│   ├── architecture/        # Kiến trúc hệ thống, diagrams
│   ├── specs/               # Đặc tả chi tiết (WP specs, JD)
│   ├── plan/                # Kế hoạch triển khai
│   ├── diagrams/            # Sơ đồ (draw.io, mermaid)
│   └── reports/             # Báo cáo, tài liệu demo
│
├── iot-node/                # Tầng IoT (ESP32-C3)
│   ├── hardware/
│   │   ├── pcb/             # KiCad/Altium files (schematic, layout, gerber)
│   │   ├── bom/             # Bill of Materials
│   │   └── enclosure/       # 3D models (.stl, .step)
│   ├── firmware/
│   │   ├── firmware-esp32c3/ # ESP-IDF project
│   │   ├── models/          # TFLite Micro / SNN models
│   │   ├── include/
│   │   ├── src/
│   │   └── test/
│   └── README.md
│
├── edge-gateway/            # Tầng Edge (Raspberry Pi / Jetson)
│   ├── k8s/
│   │   ├── manifests/       # Raw K8s manifests
│   │   └── helm/            # Helm charts
│   ├── services/
│   │   ├── ble-ingest/
│   │   ├── signal-processor/
│   │   ├── buffer-store/
│   │   ├── local-trainer/   # FL Client + Local training
│   │   ├── model-agent/     # Model management + OTA
│   │   └── gateway-api/
│   ├── ml/
│   │   ├── transformers/    # Informer/PatchTST (PyTorch)
│   │   ├── ssl/             # Contrastive Learning
│   │   └── datasets/        # Data loaders/preprocessing
│   ├── docker/               # Dockerfiles, compose
│   └── README.md
│
├── cloud/                   # Tầng Cloud
│   ├── backend/              # Spring Boot
│   │   ├── src/
│   │   ├── db/migration/     # Flyway migrations
│   │   ├── api/              # OpenAPI specs
│   │   └── config/
│   ├── fl-server/            # Federated Learning Server (Flower)
│   │   ├── strategies/       # Custom FedAvg/FedProx
│   │   ├── registry/         # Model registry handlers
│   │   └── app/
│   ├── scripts/              # Deployment scripts
│   └── README.md
│
├── mobile-app/              # React Native App
│   ├── src/
│   │   ├── screens/
│   │   ├── components/
│   │   ├── services/         # API clients
│   │   ├── navigation/
│   │   └── utils/
│   ├── assets/
│   └── README.md
│
├── shared/                  # Shared code/utils
│   ├── protocols/            # BLE/MQTT/gRPC proto, message schemas
│   ├── types/                # TS/Java/Python type definitions
│   └── utils/                # Common utils
│
├── benchmarks/              # SOTA Metrics, benchmarking
│   ├── scripts/
│   ├── results/
│   └── configs/
│
├── experiments/             # Training experiments, notebooks
│   ├── notebooks/
│   ├── configs/
│   └── logs/
│
├── scripts/                 # Root utility scripts
│   ├── setup-dev.sh
│   ├── build-all.sh
│   ├── deploy-dev.sh
│   └── clean.sh
│
├── .github/
│   └── workflows/            # CI/CD (OTA, build, test)
│
├── README.md                # Project overview
├── LICENSE
├── .gitignore
└── Makefile                 # Common tasks
```

## Quy tắc tổ chức

| Thư mục | Nguyên tắc |
|---|---|
| `iot-node/firmware` | Tách rõ src/include, models riêng biệt, test độc lập |
| `edge-gateway/services` | Mỗi microservice là 1 thư mục độc lập, có Dockerfile riêng |
| `edge-gateway/ml` | Tách Transformers và SSL rõ ràng |
| `cloud` | Phân tách `backend` (API/Core) và `fl-server` (FL logic) |
| `shared/protocols` | Định nghĩa schema/message dùng chung (Protobuf/JSON Schema) |
| `docs/diagrams` | Lưu sơ đồ kiến trúc (.drawio, .mmd mermaid) |
| `benchmarks` | Script đo lường tái lập được, lưu kết quả có timestamp |
| `experiments` | Thử nghiệm nghiên cứu, KHÔNG commit large checkpoints trừ cấu hình |
