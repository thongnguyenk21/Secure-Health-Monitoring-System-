.PHONY: help setup build up down clean

help:
	@echo "IoMT 3-Tier Edge AI + FL - Common Tasks"
	@echo "  make setup      - Show dev setup info"
	@echo "  make build      - Build all components"
	@echo "  make up         - Start dev stack (docker compose)"
	@echo "  make down       - Stop dev stack"
	@echo "  make clean      - Clean build artifacts"

setup:
	@bash scripts/setup-dev.sh

build:
	@bash scripts/build-all.sh

up:
	@cd edge-gateway/docker && docker compose up -d 2>/dev/null || docker compose up -d

down:
	@cd edge-gateway/docker && docker compose down 2>/dev/null || docker compose down

clean:
	@find . -name "build" -type d -not -path "./iot-node/*" 2>/dev/null | xargs rm -rf 2>/dev/null || true
	@rm -rf cloud/backend/target mobile-app/node_modules 2>/dev/null || true
	@echo "Cleaned build artifacts (where safe)"
