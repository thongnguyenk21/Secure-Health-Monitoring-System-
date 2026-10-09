# Kế hoạch triển khai - IoMT 3-Tier Edge AI + Federated Learning

## 1. Phân chia Work Packages (WP)

| STT | Mã WP | Tên Work Package | Giai đoạn | Mục tiêu chính |
|---|---|---|---|---|
| 1 | WP1 | Thiết kế phần cứng IoT Node | GĐ1 | Chế tạo thiết bị đeo thu thập PPG/ECG/IMU |
| 2 | WP2 | Firmware + SNNs cơ bản + BLE | GĐ1 | Đọc cảm biến, lọc nhiễu, suy luận SNN, truyền BLE |
| 3 | WP3 | Edge Gateway (K3s/Docker + Microservices) | GĐ1 | Xây dựng gateway nhận, xử lý, huấn luyện cục bộ |
| 4 | WP4 | Cloud (Spring Boot + TimescaleDB) + Mobile App (React Native) | GĐ1 | Backend lưu trữ + ứng dụng di động trực quan hóa & cảnh báo |
| 5 | WP5 | Tích hợp E2E + Báo cáo + Demo GĐ1 | GĐ1 | Kiểm thử xuyên suốt & bảo vệ đồ án chuyên ngành |
| 6 | WP6 | Time-Series Transformers (Informer/PatchTST) tại Edge | GĐ2 | Nâng cấp khả năng phân tích chuỗi thời gian |
| 7 | WP7 | Federated Learning + SSL + CI/CD OTA | GĐ2 | Huấn luyện phân tán bảo mật + cập nhật mô hình |
| 8 | WP8 | Nâng cấp SNNs (LIF/Recurrent SNN) tại IoT | GĐ2 | Tối ưu độ chính xác & tiết kiệm năng lượng |
| 9 | WP9 | Đo lường SOTA Metrics | GĐ2 | Đánh giá định lượng toàn bộ hệ thống |
| 10 | WP10 | Thử nghiệm lâm sàng + Báo cáo khoa học | GĐ2 | Thử nghiệm thực tế & hoàn thiện luận văn |

## 2. Lộ trình theo mốc thời gian

| Mốc | Nội dung | Ước tính | Phụ thuộc |
|---|---|---|---|
| M1 | Hoàn thiện PCB, firmware đọc cảm biến & bộ lọc tín hiệu | 4–6 tuần | — |
| M2 | Nhúng SNN cơ bản + giao thức BLE tối ưu | 4–6 tuần | M1 |
| M3 | Xây dựng Edge Gateway (K3s/Docker) + microservices ingest | 4–5 tuần | M2 |
| M4 | Cloud Backend (Spring Boot + TimescaleDB) + API | 4–6 tuần | M3 |
| M5 | Mobile App (React Native) + dashboard + cảnh báo thời gian thực | 4–6 tuần | M4 |
| M6 | Tích hợp E2E + demo + hoàn thiện báo cáo Đồ án CN | 2–3 tuần | M1–M5 |
| M7 | Triển khai Time-Series Transformers (Informer/PatchTST) | 6–8 tuần | M6 |
| M8 | Tích hợp FL + SSL (Contrastive Learning) + CI/CD OTA | 6–8 tuần | M7 |
| M9 | Nâng cấp SNN (LIF/Recurrent SNN) | 4–6 tuần | M6 |
| M10 | Đo lường các chỉ tiêu SOTA | 4–6 tuần | M8, M9 |
| M11 | Thử nghiệm lâm sàng 10–30 người / 1–2 tháng | 6–10 tuần | M10 |
| M12 | Hoàn thiện báo cáo khoa học + tổng kết | 3–4 tuần | M11 |

**Tổng thời lượng dự kiến:** ~53–74 tuần

## 3. Rủi ro & biện pháp giảm thiểu

| Rủi ro | Mức độ | Biện pháp giảm thiểu |
|---|---|---|
| Tài nguyên hạn chế ESP32-C3 | Cao | Lượng tử hóa int8, pruning, giảm time-step |
| BLE nghẽn băng thông | Trung bình | Gửi theo batch, chỉ truyền feature/event, tối ưu MTU |
| Hội tụ FL chậm / Non-IID | Cao | FedProx, weighted FedAvg, SSL pretraining |
| Thiếu dữ liệu gán nhãn | Cao | Contrastive Learning, data augmentation sinh lý |
| Tích hợp OTA phức tạp | Trung bình | Model versioning, CI/CD canary, rollback |
| Vấn đề đạo đức khi thử nghiệm | Cao | Xin phê duyệt IRB, consent form, ẩn danh hóa PII |
| Kết nối Edge–Cloud không ổn định | Trung bình | Buffer cục bộ, retry policy, offline mode |

## 4. Milestones & Deliverables

| Milestone | Ngày dự kiến (tham khảo) | Deliverables |
|---|---|---|
| MS1 - POC IoT | Sau M1-M2 | PCB, Firmware, SNN cơ bản chạy được |
| MS2 - Edge Ready | Sau M3 | Gateway hoạt động, ingest realtime |
| MS3 - GĐ1 Complete | Sau M6 | Demo E2E + Báo cáo Đồ án CN |
| MS4 - AI nâng cao | Sau M7-M9 | Transformers + FL + SNN nâng cấp + Metrics |
| MS5 - Validation | Sau M11-M12 | Thử nghiệm lâm sàng + Luận văn hoàn chỉnh |
