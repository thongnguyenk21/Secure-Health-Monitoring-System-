# Proposed Project Structure

## Root Structure

```text
/project-personal/
│
├── docs/
│   ├── architecture/        # System architecture, diagrams
│   ├── specs/               # Detailed specifications (WP specs, JD)
│   ├── plan/                # Implementation plan
│   ├── diagrams/            # Diagrams (draw.io, mermaid)
│   └── reports/             # Reports, demo documents
│
├── iot-node/                # IoT tier (ESP32-C3)
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
├── edge-gateway/            # Edge tier (Raspberry Pi / Jetson)
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
├── cloud/                   # Cloud tier
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

## Organization Rules

| Directory | Principle |
|---|---|
| `iot-node/firmware` | Clearly separate src/include, distinct models, independent tests |
| `edge-gateway/services` | Each microservice is an independent directory with its own Dockerfile |
| `edge-gateway/ml` | Clearly separate Transformers and SSL |
| `cloud` | Separate `backend` (API/Core) and `fl-server` (FL logic) |
| `shared/protocols` | Define shared schema/message (Protobuf/JSON Schema) |
| `docs/diagrams` | Store architecture diagrams (.drawio, .mmd mermaid) |
| `benchmarks` | Reproducible measurement scripts, store timestamped results |
| `experiments` | Research experiments, do NOT commit large checkpoints except configs |
