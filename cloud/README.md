# Cloud

Spring Boot backend + TimescaleDB + Flower FL Server + Model Registry.

## Components
- backend/: Spring Boot 3.x (REST API, JWT+RBAC, TimescaleDB)
- fl-server/: Flower server (FedAvg/FedProx), aggregation logic
- scripts/: deployment

## Run backend (dev)
```bash
cd cloud/backend
./mvnw spring-boot:run
```

## FL Server
Run separately or with the same stack. See fl-server/README.md when initializing.
