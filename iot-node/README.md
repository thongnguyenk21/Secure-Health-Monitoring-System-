# IoT Node (ESP32-C3)

IoT tier that collects PPG/ECG/IMU and runs SNN inference < 5ms.

## Structure
```
iot-node/
├── hardware/
│   ├── pcb/        # KiCad (.kicad_sch, .kicad_pcb)
│   ├── bom/        # BOM.csv
│   └── enclosure/  # .stl/.step
└── firmware/
    ├── firmware-esp32c3/  # ESP-IDF project
    ├── models/            # tflite, snn
    ├── src/
    ├── include/
    └── test/
```

## Build (ESP-IDF)
```bash
cd iot-node/firmware/firmware-esp32c3
idf.py set-target esp32c3
idf.py build
idf.py flash monitor
```

## Sensors
- MAX30102 (PPG, HR, SpO2)
- MPU6050 (IMU 6-DOF)
- AD8232 (ECG - optional)

## AI Model
SNN (LIF/RSNN) trained with snnTorch/Norse → exported to TensorFlow Lite Micro (int8). Target: < 5ms, RAM ≤ 200KB.
