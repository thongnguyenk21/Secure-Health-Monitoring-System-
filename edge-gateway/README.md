# Edge Gateway

K3s/Docker + Microservices + Transformers + FL Client (Flower). Nhận BLE, xử lý realtime, huấn luyện cục bộ.

## Services
- ble-ingest: Nhận dữ liệu từ IoT Node
- signal-processor: Tiền xử lý, sliding window
- buffer-store: Lưu buffer tạm thời
- local-trainer: FL Client + local training
- model-agent: Quản lý model + OTA
- gateway-api: REST/gRPC nội bộ

## ML
- transformers/: Informer/PatchTST (PyTorch → ONNX/TensorRT)
- ssl/: Contrastive Learning (SimCLR-style)

## K8s
- k8s/manifests/: raw manifests
- k8s/helm/: Helm charts

## Dev (Docker Compose)
```bash
cd edge-gateway/docker
docker compose up -d
```
