# shared/protocols — Machine-readable Contracts

- **Purpose:** Source of truth for all cross-language interfaces: Protobuf (gRPC), OpenAPI (REST), JSON Schema and AsyncAPI (MQTT). Bindings under `shared/{rust,python,typescript}` are generated from here.
- **Tier:** `[ARCH]`
- **Scope:** `[MVP]` (except `federation/v1` marked `[LATER]`)
- **Owner:** RW-01 (MVP-01)
- **Tech:** Protobuf + `buf`, OpenAPI 3, JSON Schema, AsyncAPI.
