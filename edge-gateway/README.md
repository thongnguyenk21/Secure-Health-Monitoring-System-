# Edge Gateway

K3s/Docker + Microservices + Transformers + FL Client (Flower). Receives BLE, processes realtime, performs local training.

## Services
- ble-ingest: Receives data from the IoT Node
- signal-processor: Preprocessing, sliding window
- buffer-store: Stores temporary buffer
- local-trainer: FL Client + local training
- model-agent: Model management + OTA
- gateway-api: Internal REST/gRPC

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
