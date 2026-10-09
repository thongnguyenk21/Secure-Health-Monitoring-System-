#!/usr/bin/env bash
set -euo pipefail

echo "=== Building all components ==="
echo "[1/4] IoT firmware (idf.py build)"
if command -v idf.py >/dev/null 2>&1; then
  pushd iot-node/firmware >/dev/null
  idf.py build || echo "⚠ IoT build skipped/failed"
  popd >/dev/null
else
  echo "⚠ idf.py not found, skip IoT"
fi

echo "[2/4] Edge services (Docker build)"
pushd edge-gateway >/dev/null
docker compose build || echo "⚠ Edge build skipped/failed"
popd >/dev/null

echo "[3/4] Cloud backend (Maven)"
pushd cloud/backend >/dev/null
./mvnw clean package -DskipTests || echo "⚠ Cloud build skipped/failed"
popd >/dev/null

echo "[4/4] Mobile app (npm)"
pushd mobile-app >/dev/null
npm ci || npm install || echo "⚠ Mobile npm install skipped/failed"
popd >/dev/null

echo "=== Done ==="
