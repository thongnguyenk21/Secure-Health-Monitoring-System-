# shared — Contracts & Generated Bindings

- **Purpose:** Cross-language contracts and generated bindings: `shared/protocols/` is the machine-readable source of truth; `shared/{rust,python,typescript}/` are generated from it.
- **Tier:** `[ARCH]` (architecture / contracts)
- **Scope:** `[MVP]` (federation proto and some later items marked `[LATER]`)
- **Owner:** RW-01 (MVP-01)
- **Tech:** Protobuf + `buf`, OpenAPI, JSON Schema, AsyncAPI; codegen to Rust (`prost`/`tonic`), Python, TypeScript. Rust crate `iomt-protocol`.
