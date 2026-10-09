# iot-node/firmware — ESP32-C3 (Cargo project)

- **Purpose:** Firmware for the IoT node: BLE GATT telemetry, sensor drivers, on-device SNN (LIF) inference. Nested Cargo workspace, excluded from the root workspace.
- **Tier:** `[T0/IoT]`
- **Scope:** `[MVP]` (`power-mgmt` is `[LATER]`)
- **Owner:** RW-05 (MVP-03)
- **Tech:** Rust `no_std`, ESP-IDF (`esp-idf-svc`/`esp-idf-hal`), target `riscv32imc-esp-espidf`.
