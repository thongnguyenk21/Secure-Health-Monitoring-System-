#!/usr/bin/env bash
set -euo pipefail

echo "=== IoMT Dev Setup ==="
echo "Checking prerequisites..."

command -v git >/dev/null 2>&1 && echo "✓ git" || echo "✗ git missing"
command -v docker >/dev/null 2>&1 && echo "✓ docker" || echo "✗ docker missing"
command -v python3 >/dev/null 2>&1 && echo "✓ python3" || echo "✗ python3 missing"
command -v node >/dev/null 2>&1 && echo "✓ node" || echo "✗ node missing"
command -v java -version >/dev/null 2>&1 && echo "✓ java" || echo "✗ java missing"
command -v idf.py >/dev/null 2>&1 && echo "✓ esp-idf" || echo "✗ esp-idf missing (optional for IoT)"

echo "
Next steps:
1. IoT: cd iot-node/firmware && idf.py build
2. Edge: cd edge-gateway && docker compose -f docker/docker-compose.dev.yml up -d
3. Cloud: cd cloud/backend && ./mvnw spring-boot:run
4. Mobile: cd mobile-app && npm install && npx react-native run-android
"
