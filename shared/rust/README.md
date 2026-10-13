# shared/rust — crate `iomt-protocol`

- **Purpose:** Workspace member exposing the shared, validated domain types and the Protobuf/gRPC (prost/tonic) bindings generated from `shared/protocols` for all Rust tiers. `src/generated.rs` is produced by `codegen/` (run `tools/scripts/gen-contracts.sh`, never edit it by hand); `src/convert.rs` is the only path from wire types to domain types.
- **Tier:** `[ARCH]`
- **Scope:** `[MVP]`
- **Owner:** RW-01 (MVP-01)
- **Tech:** Rust, `serde`, `thiserror`, `prost`, `tonic`.
