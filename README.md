# IoMT 3-Tier Edge AI + Federated Learning

Project: A 3-tier distributed architecture integrating Edge AI & Federated Learning for IoMT.

## Overview

The project develops a distributed health monitoring system with a 3-tier architecture:
- **IoT Node** (ESP32-C3 + PPG/ECG/IMU + SNN)
- **Edge Gateway** (K3s + Microservices + Time-Series Transformers + FL Client)
- **Cloud** (Spring Boot + TimescaleDB + Flower FL Server + Model Registry)
- **Mobile App** (React Native) displaying realtime data + alerts

Key features: local inference, federated learning, OTA updates, privacy-first design.

## Project Structure

See details at [docs/architecture/FOLDER_STRUCTURE.md](docs/architecture/FOLDER_STRUCTURE.md)

## Documentation

- [Rewritten JD](docs/specs/JD_REWRITTEN.md)
- [Implementation Plan](docs/plan/IMPLEMENTATION_PLAN.md)
- [Work Package Specifications (WP1–WP10)](docs/specs/WP_SPECS.md)
- [System Architecture](docs/architecture/SYSTEM_ARCHITECTURE.md)
- [Folder Structure](docs/architecture/FOLDER_STRUCTURE.md)

## Implementation Phases

- **Phase 1 (Capstone Project):** WP1–WP5 – Build infrastructure & POC (IoT–Edge–Cloud–Mobile + E2E Demo)
- **Phase 2 (Thesis):** WP6–WP10 – Optimization (Transformers, FL+SSL, upgraded SNN), SOTA evaluation, clinical trial

## Tech Stack

| Layer | Primary Technologies |
|---|---|
| IoT | ESP32-C3, TFLite Micro, snnTorch/Norse, FreeRTOS |
| Edge | K3s/Docker, PyTorch + ONNX/TensorRT, Flower Client |
| Cloud | Spring Boot 3.x, PostgreSQL+TimescaleDB, Flower Server |
| Mobile | React Native (TypeScript) |

## Quick Start (Dev)

```bash
# 1) Clone repo (if needed)
git clone <repo-url> && cd project-personal

# 2) Inspect structure
tree -L 2 || find . -maxdepth 2 -type d

# 3) Local dev (suggested)
# - iot-node: cd iot-node/firmware && idf.py build flash monitor
# - edge-gateway: cd edge-gateway && docker compose up -d (dev)
# - cloud: cd cloud/backend && ./mvnw spring-boot:run
# - mobile: cd mobile-app && npx react-native run-android
```

## Notes

- Target values (accuracy, latency, EDP...) are preliminary – to be agreed upon with the academic advisor.
- The clinical trial (WP10) requires compliance with research ethics, IRB, Informed Consent, and PII anonymization.
- **Privacy-by-Design** first: only model weights are shared; raw data is never transmitted to the Cloud.

## License

TBD (per project/platform regulations)
