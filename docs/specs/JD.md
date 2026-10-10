# Project: A 3-tier distributed architecture integrating Edge AI & Federated Learning for IoMT

## 1. Abstract

The explosive growth of **Internet of Medical Things (IoMT)** devices offers tremendous potential for continuous health monitoring. However, the current centralized (Client–Cloud) processing architecture exposes serious limitations in terms of **network latency**, **bandwidth**, and especially the **risk of privacy breaches** for sensitive medical data.

This project proposes and develops a distributed **3-tier (IoT Node – Edge Gateway – Cloud)** architecture integrating **Federated Learning (FL)**. Instead of transmitting raw data, the system performs local inference at the edge nodes using ultra-lightweight **Spiking Neural Networks (SNNs)** and **Time-Series Transformers**, then shares only model weights to the Cloud for aggregation (**Federated Averaging**). This solution aims to:

- Optimize computation (**Energy-Delay Product**)
- Minimize communication costs (**Communication Cost**)
- Absolutely protect personal data via the **Privacy-by-Design** mechanism

## 2. Research Objectives

- **Architecture design:** Fully build the data pipeline across the 3 computing tiers, ensuring seamlessness and scalability.
- **Edge AI optimization:** Embed and optimize deep learning models onto resource-constrained hardware. Specifically, deploy SNNs on the microcontroller (IoT Node) and Time-Series Transformers (such as Informer/PatchTST) on the Edge Gateway to analyze medical time series.
- **Data security:** Integrate the Federated Learning protocol combined with Self-Supervised Learning (SSL) to continuously update the machine learning model without collecting raw data to a central server.

## 3. Methodology & Implementation

### 3.1. IoT Tier (Sensing & Acquisition)
- Develop hardware based on **ESP32-C3** combined with **PPG/ECG** and **IMU** sensors.
- Use **TensorFlow Lite Micro** to run inference for basic state classification tasks directly on the device (**< 5 ms**).

### 3.2. Edge Tier (Stream Processing & Local Training)
- Virtualize the environment with **K3s/Docker** on **Raspberry Pi** or **Jetson Nano**.
- Build **Microservices** to receive real-time data and perform **Local Training**.

### 3.3. Cloud Tier (Aggregation & Management)
- Build a distributed time-series data store (**PostgreSQL + TimescaleDB**).
- Deploy the **Federated Averaging (FedAvg)** algorithm.
- Set up the **CI/CD** pipeline for **OTA (Over-The-Air)** model updates.

## 4. System Evaluation

The project conducts rigorous quantitative evaluation based on **SOTA (State-of-the-Art Metrics)** parameters:

- **Computational performance:** End-to-end latency and the **Energy-Delay Product (EDP)** at the edge nodes.
- **Network performance:** Communication cost during the convergence of the Federated Learning model.
- **Model reliability:** Performance degradation (**Model Drift**), robustness (**Robustness**), and scalability (**Scalability**) as physiological data changes over time.

## 5. Expected Contributions

- A complete system prototype (**Hardware & Software**) following the 3-tier model.
- A source codebase and design documentation demonstrating the feasibility of combining **SNNs, Time-Series Transformers, and Federated Learning** on resource-constrained devices.
- An experimental basis and performance evaluation data of reference value for research on **Edge AI in digital healthcare**.

## 6. Implementation Requirements

### Phase 1: Capstone Project (Build infrastructure & POC)
**Objective:** Establish the hardware infrastructure, build a data pipeline across the 3 tiers (IoT–Edge–Cloud), and integrate a basic AI inference model.

**Specific tasks:**
1. **IoT Node hardware design:** Assemble a wearable device based on the ESP32-C3 with integrated biomedical sensors (MAX30102, MPU6050). Complete the schematic, 2-layer PCB layout, BOM, and 3D-printed enclosure design.
2. **Develop Firmware & basic SNNs:** Program sensor reading and signal noise filters. Embed basic Spiking Neural Networks (SNNs) into the ESP32-C3 to classify motion states and set up a bandwidth-optimized BLE data transmission protocol.
3. **Build a basic Edge Gateway:** Install the K3s/Docker virtualization environment on Raspberry Pi/Jetson Nano. Build microservices that receive and process real-time data streams from the IoT Node.
4. **Develop the Cloud & Mobile App system:** Deploy the Spring Boot + TimescaleDB backend. Develop a cross-platform mobile app (React Native) that connects to the Cloud/Gateway to visualize data and receive real-time alerts.
5. **Report compilation & Demo:** Package the design, deploy the infrastructure, and perform an end-to-end system demo.

### Phase 2: Graduation Thesis (Algorithm optimization & System evaluation)
**Objective:** Upgrade the deep learning models (Transformers, Federated Learning, SSL) and perform comprehensive performance evaluation against SOTA metrics.

**Specific tasks:**
1. **Deploy Time-Series Transformers (Edge tier):** Replace the basic model with advanced architectures (Informer/PatchTST) for in-depth analysis and forecasting of health time series.
2. **Integrate Federated Learning & Self-Supervised Learning:** Build a local training mechanism at the Edge Gateway and deploy FedAvg on the Cloud. Apply Contrastive Learning to leverage unlabeled data and complete the CI/CD pipeline for OTA updates.
3. **Upgrade the SNN model (IoT tier):** Apply the LIF (Leaky Integrate-and-Fire) or Recurrent SNNs architecture to improve accuracy and optimize power consumption.
4. **System performance evaluation (SOTA Metrics):** Measure Communication Cost, End-to-end Latency, Energy-Delay Product (EDP), Model Drift, Robustness, and Scalability.
5. **Clinical trial & Scientific report:** Conduct a real-world trial with **10–30 users over 1–2 months**. Evaluate **Sensitivity**, **Specificity**, and user satisfaction (UX) to complete the thesis report.
