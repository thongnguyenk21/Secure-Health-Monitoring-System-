# Đề tài: Kiến trúc phân tán 3 tầng tích hợp Edge AI & Federated Learning cho IoMT

## 1. Tóm tắt đề tài (Abstract)

Sự bùng nổ của các thiết bị **Internet of Medical Things (IoMT)** mang lại tiềm năng to lớn trong việc theo dõi sức khỏe liên tục. Tuy nhiên, kiến trúc xử lý tập trung (Client–Cloud) hiện tại đang bộc lộ những hạn chế nghiêm trọng về **độ trễ mạng**, **băng thông** và đặc biệt là **rủi ro xâm phạm quyền riêng tư** dữ liệu y tế nhạy cảm.

Đề tài này đề xuất và phát triển một kiến trúc phân tán **3 tầng (IoT Node – Edge Gateway – Cloud)** tích hợp **Học máy liên kết (Federated Learning – FL)**. Thay vì truyền dữ liệu thô, hệ thống thực hiện suy luận cục bộ tại các nút biên bằng **Mạng nơ-ron xung (SNNs)** siêu nhẹ và **Time-Series Transformers**, sau đó chỉ chia sẻ trọng số mô hình lên Cloud để tổng hợp (**Federated Averaging**). Giải pháp này nhằm:

- Tối ưu hóa tính toán (**Energy-Delay Product**)
- Giảm thiểu chi phí truyền thông (**Communication Cost**)
- Bảo vệ tuyệt đối dữ liệu cá nhân theo cơ chế **Privacy-by-Design**

## 2. Mục tiêu nghiên cứu (Research Objectives)

- **Thiết kế kiến trúc:** Xây dựng hoàn chỉnh đường ống dữ liệu (data pipeline) xuyên suốt 3 tầng tính toán, đảm bảo tính liền mạch và khả năng mở rộng (scalability).
- **Tối ưu hóa Edge AI:** Nhúng và tối ưu các mô hình học sâu lên phần cứng giới hạn tài nguyên. Cụ thể, triển khai SNNs trên vi điều khiển (IoT Node) và Time-Series Transformers (như Informer/PatchTST) trên Edge Gateway để phân tích chuỗi thời gian y tế.
- **Bảo mật dữ liệu:** Tích hợp giao thức Federated Learning kết hợp Học tự giám sát (Self-Supervised Learning – SSL) nhằm cập nhật mô hình học máy liên tục mà không cần thu thập dữ liệu thô về máy chủ trung tâm.

## 3. Phương pháp và Nội dung thực hiện (Methodology & Implementation)

### 3.1. Tầng IoT (Cảm biến & Thu thập)
- Phát triển phần cứng dựa trên **ESP32-C3** kết hợp cảm biến **PPG/ECG** và **IMU**.
- Ứng dụng **TensorFlow Lite Micro** để chạy suy luận các bài toán phân loại trạng thái cơ bản ngay tại thiết bị (**< 5 ms**).

### 3.2. Tầng Edge (Xử lý luồng & Huấn luyện cục bộ)
- Ảo hóa môi trường bằng **K3s/Docker** trên **Raspberry Pi** hoặc **Jetson Nano**.
- Xây dựng các **Microservices** để tiếp nhận dữ liệu thời gian thực và thực hiện **huấn luyện cục bộ (Local Training)**.

### 3.3. Tầng Cloud (Tổng hợp & Quản lý)
- Xây dựng kho dữ liệu chuỗi thời gian phân tán (**PostgreSQL + TimescaleDB**).
- Triển khai thuật toán **Federated Averaging (FedAvg)**.
- Thiết lập luồng **CI/CD** cho quá trình cập nhật mô hình **OTA (Over-The-Air)**.

## 4. Đánh giá hệ thống (System Evaluation)

Đề tài tiến hành đánh giá định lượng nghiêm ngặt dựa trên các tham số **SOTA (State-of-the-Art Metrics)**:

- **Hiệu suất tính toán:** Độ trễ đầu cuối (**End-to-end Latency**) và chỉ số **Energy-Delay Product (EDP)** tại các nút biên.
- **Hiệu suất mạng:** Chi phí truyền tải (**Communication Cost**) trong quá trình hội tụ của mô hình Federated Learning.
- **Độ tin cậy của mô hình:** Hiện tượng suy giảm hiệu suất (**Model Drift**), độ bền vững (**Robustness**) và khả năng mở rộng (**Scalability**) khi dữ liệu sinh lý thay đổi theo thời gian.

## 5. Kết quả dự kiến và Đóng góp (Expected Contributions)

- Một nguyên mẫu hệ thống (**Hardware & Software**) hoàn chỉnh theo mô hình 3 tầng.
- Bộ mã nguồn và tài liệu thiết kế chứng minh tính khả thi của việc kết hợp **SNNs, Time-Series Transformers và Federated Learning** trên thiết bị tài nguyên hạn chế.
- Cơ sở thực nghiệm và dữ liệu đánh giá hiệu năng có giá trị tham khảo cho nghiên cứu về **Edge AI trong y tế số**.

## 6. Yêu cầu thực hiện

### Giai đoạn 1: Đồ án chuyên ngành (Xây dựng hạ tầng & POC)
**Mục tiêu:** Thiết lập hạ tầng phần cứng, xây dựng data pipeline xuyên suốt 3 tầng (IoT–Edge–Cloud) và tích hợp mô hình AI suy luận cơ bản.

**Nhiệm vụ cụ thể:**
1. **Thiết kế phần cứng IoT Node:** Lắp ráp thiết bị đeo dựa trên ESP32-C3 tích hợp cảm biến y sinh (MAX30102, MPU6050). Hoàn thiện sơ đồ nguyên lý, layout PCB 2 lớp, BOM và thiết kế vỏ hộp in 3D.
2. **Phát triển Firmware & SNNs cơ bản:** Lập trình đọc cảm biến, bộ lọc nhiễu tín hiệu. Nhúng mạng nơ-ron xung (SNNs) cơ bản vào ESP32-C3 để phân loại trạng thái vận động và thiết lập giao thức truyền dữ liệu BLE tối ưu băng thông.
3. **Xây dựng Edge Gateway cơ bản:** Cài đặt môi trường ảo hóa K3s/Docker trên Raspberry Pi/Jetson Nano. Xây dựng microservices tiếp nhận và xử lý luồng dữ liệu thời gian thực từ IoT Node.
4. **Phát triển hệ thống Cloud & Mobile App:** Triển khai backend Spring Boot + TimescaleDB. Phát triển ứng dụng di động đa nền tảng (React Native) kết nối với Cloud/Gateway để trực quan hóa dữ liệu và nhận cảnh báo thời gian thực.
5. **Tổng hợp báo cáo & Demo:** Đóng gói thiết kế, triển khai hạ tầng và thực hiện demo hệ thống end-to-end.

### Giai đoạn 2: Luận văn tốt nghiệp (Tối ưu thuật toán & Đánh giá hệ thống)
**Mục tiêu:** Nâng cấp mô hình học sâu (Transformers, Federated Learning, SSL) và thực hiện đánh giá hiệu năng toàn diện theo các chỉ tiêu SOTA.

**Nhiệm vụ cụ thể:**
1. **Triển khai Time-Series Transformers (Tầng Edge):** Thay thế mô hình cơ bản bằng các kiến trúc tiên tiến (Informer/PatchTST) nhằm phân tích chuyên sâu và dự báo chuỗi thời gian sức khỏe.
2. **Tích hợp Federated Learning & Self-Supervised Learning:** Xây dựng cơ chế huấn luyện cục bộ tại Edge Gateway, triển khai FedAvg trên Cloud. Áp dụng Contrastive Learning để tận dụng dữ liệu chưa gán nhãn và hoàn thiện luồng CI/CD cho cập nhật OTA.
3. **Nâng cấp mô hình SNNs (Tầng IoT):** Áp dụng kiến trúc LIF (Leaky Integrate-and-Fire) hoặc Recurrent SNNs nhằm cải thiện độ chính xác và tối ưu hóa tiêu thụ điện năng.
4. **Đánh giá hiệu năng hệ thống (SOTA Metrics):** Đo lường Communication Cost, End-to-end Latency, Energy-Delay Product (EDP), Model Drift, Robustness và Scalability.
5. **Thử nghiệm lâm sàng & Báo cáo khoa học:** Thử nghiệm thực tế trên **10–30 người dùng trong 1–2 tháng**. Đánh giá **độ nhạy (Sensitivity)**, **độ đặc hiệu (Specificity)** và mức độ hài lòng người dùng (UX) để hoàn thiện báo cáo luận văn.
