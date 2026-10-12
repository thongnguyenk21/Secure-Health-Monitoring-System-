# IoT Node (ESP32-C3)

Wearable tier: reads PPG/IMU sensors, runs SNN inference on the chip (< 5 ms) and
sends 16-byte records to the Pi over BLE GATT, buffering them while out of range.

Firmware is **Rust** on ESP-IDF via `esp-idf-svc` (target `riscv32imc-esp-espidf`).
Hardware-independent logic lives in `no_std` crates that are tested on the host.
If BLE through Rust proves unstable during the spike week, only the BLE part may
move to an ESP-IDF C component behind the same interface; everything else stays Rust.

## Structure
```
iot-node/
├── hardware/
│   ├── pcb/          # [LATER] KiCad (.kicad_sch, .kicad_pcb)
│   ├── bom/          # BOM.csv
│   ├── enclosure/    # .stl/.step
│   └── datasheets/
└── firmware/         # nested Cargo workspace (excluded from the root workspace)
    ├── crates/
    │   ├── iomt-frames/      # 16-byte record + CRC-8 (no_std, done)
    │   ├── offline-buffer/   # ring buffer + ack (no_std, done)
    │   ├── signal-proc/      # filtering, peak detection (no_std)
    │   ├── snn-lif/          # LIF neurons (no_std, host-testable)
    │   ├── sensor-max30102/  # PPG driver (I2C)
    │   ├── sensor-mpu6050/   # IMU driver (I2C)
    │   ├── ble-transport/    # GATT server, notify, ack
    │   ├── ota-client/       # [LATER] firmware OTA over BLE
    │   └── power-mgmt/       # [LATER] sleep, duty cycle
    ├── models/       # SNN weights
    └── src/          # main (esp-idf-svc): task orchestration, periodic sensor reads
```

## Build and test
```bash
cd iot-node/firmware
cargo test            # host tests for the no_std crates
```
Building and flashing the device binary (`cargo build --release` for
`riscv32imc-esp-espidf`, then `espflash flash --monitor`) applies once `src/` has a
binary crate.

## Sensors
- MAX30102 (PPG: heart rate, SpO2), I2C address 0x57
- MPU6050 / GY-521 (6-axis IMU: steps, activity), I2C address 0x68
- AD8232 (ECG) — optional, no sensor chosen yet

Wiring and pin map: C4 map, tab "Phần cứng" (`docs/design/c4/index.html`).

## AI model
SNN (LIF) core hand-written in Rust `no_std` (decision D16), target < 5 ms per inference.
