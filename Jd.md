1. Tóm tắt đề tài (Abstract)

Sự bùng nổ của các thiết bị Internet of Medical Things (IoMT) mang lại tiềm năng to lớn trong việc theo dõi sức khỏe liên tục. Tuy nhiên, kiến trúc xử lý tập trung (Client-Cloud) hiện tại đang bộc lộ những hạn chế nghiêm trọng về độ trễ mạng, băng thông và đặc biệt là rủi ro xâm phạm quyền riêng tư dữ liệu y tế nhạy cảm.

Đề tài này đề xuất và phát triển một kiến trúc phân tán 3 tầng (IoT Node – Edge Gateway – Cloud) tích hợp Học máy liên kết (Federated Learning - FL). Thay vì truyền dữ liệu thô, hệ thống thực hiện suy luận cục bộ tại các nút biên bằng Mạng nơ-ron xung (SNNs) siêu nhẹ và Time-Series Transformers, sau đó chỉ chia sẻ trọng số mô hình lên Cloud để tổng hợp (Federated Averaging). Giải pháp này nhằm tối ưu hóa tính toán (Energy-Delay Product), giảm thiểu chi phí truyền thông (Communication Cost) và bảo vệ tuyệt đối dữ liệu cá nhân theo cơ chế Privacy-by-Design.

2. Mục tiêu nghiên cứu (Research Objectives)

Thiết kế kiến trúc: Xây dựng hoàn chỉnh đường ống dữ liệu (data pipeline) xuyên suốt 3 tầng tính toán, đảm bảo tính liền mạch và khả năng mở rộng (scalability).
Tối ưu hóa Edge AI: Nhúng và tối ưu các mô hình học sâu lên phần cứng giới hạn tài nguyên. Cụ thể, triển khai SNNs trên vi điều khiển (IoT Node) và Time-Series Transformers (như Informer/PatchTST) trên Edge Gateway để phân tích chuỗi thời gian y tế.
Bảo mật dữ liệu: Tích hợp giao thức Federated Learning kết hợp Học tự giám sát (Self-Supervised Learning) nhằm cập nhật mô hình học máy liên tục mà không cần thu thập dữ liệu thô về máy chủ trung tâm.
3. Phương pháp và Nội dung thực hiện (Methodology & Implementation)

Tầng IoT (Cảm biến & Thu thập): Phát triển phần cứng dựa trên ESP32-C3 kết hợp cảm biến PPG/ECG và IMU. Ứng dụng TensorFlow Lite Micro để chạy suy luận các bài toán phân loại trạng thái cơ bản ngay tại thiết bị (<5ms).
Tầng Edge (Xử lý luồng & Huấn luyện cục bộ): Ảo hóa môi trường bằng K3s/Docker trên Raspberry Pi hoặc Jetson Nano. Xây dựng các Microservices để tiếp nhận dữ liệu thời gian thực và thực hiện huấn luyện cục bộ (Local Training).
Tầng Cloud (Tổng hợp & Quản lý): Xây dựng kho dữ liệu chuỗi thời gian phân tán (PostgreSQL + TimescaleDB) và triển khai thuật toán Federated Averaging. Thiết lập luồng CI/CD cho quá trình cập nhật mô hình OTA (Over-The-Air).
4. Đánh giá hệ thống (System Evaluation) Đề tài sẽ không chỉ dừng ở mức xây dựng nguyên mẫu (prototype) mà còn tiến hành đánh giá định lượng nghiêm ngặt dựa trên các tham số SOTA (State-of-the-Art Metrics):

Hiệu suất tính toán: Đánh giá độ trễ đầu cuối (End-to-end Latency) và chỉ số Energy-Delay Product (EDP) tại các nút biên.
Hiệu suất mạng: Đo lường chi phí truyền tải (Communication Cost) trong quá trình hội tụ của mô hình Federated Learning.
Độ tin cậy của mô hình: Đánh giá hiện tượng suy giảm hiệu suất (Model Drift) và độ bền vững (Robustness) của hệ thống khi dữ liệu sinh lý có sự thay đổi theo thời gian.
5. Kết quả dự kiến và Đóng góp (Expected Contributions)

Một nguyên mẫu hệ thống (Hardware & Software) hoạt động hoàn chỉnh theo mô hình 3 tầng.
Bộ mã nguồn và tài liệu thiết kế hệ thống chứng minh được tính khả thi của việc kết hợp GNN/SNNs, Transformers và FL trên các thiết bị tài nguyên hạn chế.
Cung cấp cơ sở thực nghiệm và dữ liệu đánh giá hiệu năng mang tính tham khảo cao cho các nghiên cứu tiếp theo về ứng dụng Edge AI trong y tế số.
Yêu cầu

3.1. Giai đoạn 1: Đồ án chuyên ngành (Xây dựng hạ tầng và POC)
Mục tiêu: Thiết lập thành công hạ tầng phần cứng, xây dựng đường ống dữ liệu (data pipeline) xuyên suốt 3 tầng (IoT-Edge-Cloud) và tích hợp các mô hình AI suy luận cơ bản.

Nhiệm vụ cụ thể:

Thiết kế phần cứng IoT Node: Lắp ráp thiết bị đeo dựa trên vi điều khiển ESP32-C3 tích hợp các cảm biến y sinh (MAX30102, MPU6050). Hoàn thiện bản vẽ sơ đồ nguyên lý, layout mạch in PCB 2 lớp và thiết kế vỏ hộp in 3D.
Phát triển Firmware & SNNs cơ bản: Lập trình các module đọc cảm biến, bộ lọc nhiễu tín hiệu. Nghiên cứu nhúng Mạng nơ-ron xung (SNNs) cơ bản vào ESP32-C3 để phân loại trạng thái vận động, đồng thời thiết lập giao thức truyền dữ liệu BLE tối ưu băng thông.
Xây dựng Edge Gateway cơ bản: Cài đặt môi trường ảo hóa K3s/Docker trên thiết bị vi máy tính (Raspberry Pi/Jetson Nano). Xây dựng các dịch vụ microservice để nhận và xử lý luồng dữ liệu liên tục từ IoT Node.
Phát triển hệ thống Cloud & Mobile App: Triển khai server lưu trữ dữ liệu chuỗi thời gian sử dụng Spring Boot và TimescaleDB. Song song, phát triển ứng dụng di động đa nền tảng (React Native) kết nối với Cloud/Gateway để trực quan hóa dữ liệu y tế và nhận cảnh báo thời gian thực.
Tổng hợp báo cáo: Đóng gói toàn bộ quá trình thiết kế, triển khai hạ tầng và thực hiện demo hệ thống chạy xuyên suốt từ đầu cuối (end-to-end).
3.2. Giai đoạn 2: Luận văn tốt nghiệp (Tối ưu thuật toán & Đánh giá hệ thống)
Mục tiêu: Nâng cấp các mô hình học sâu (Transformers, Federated Learning, SSL) và thực hiện đánh giá hiệu năng hệ thống toàn diện (System Evaluation) dựa trên các chuẩn đo lường SOTA.

Nhiệm vụ cụ thể:

Triển khai Time-Series Transformers (Tầng Edge): Thay thế các mô hình suy luận cơ bản tại Edge Gateway bằng các kiến trúc Transformer tiên tiến (như Informer hoặc PatchTST) nhằm phân tích chuyên sâu và dự báo chuỗi thời gian sức khỏe.
Tích hợp Federated Learning & Học tự giám sát (SSL): Xây dựng cơ chế huấn luyện cục bộ (local training) tại Edge Gateway và thuật toán Federated Averaging trên Cloud. Áp dụng Contrastive Learning để tận dụng tối đa dữ liệu chưa gán nhãn và hoàn thiện luồng CI/CD cho cập nhật OTA.
Nâng cấp mô hình SNNs (Tầng IoT): Áp dụng các kiến trúc Mạng nơ-ron xung mới (như Leaky Integrate-and-Fire hoặc Recurrent SNNs) trên vi điều khiển để cải thiện độ chính xác trong phát hiện sự cố y tế và tối ưu hóa tối đa điện năng.
Đánh giá hiệu năng hệ thống (SOTA Metrics): Tiến hành đo lường các chỉ số cốt lõi:
Communication Cost & Latency: Chi phí băng thông trong quá trình hội tụ mô hình và độ trễ đầu cuối.
Energy-Delay Product (EDP): Sự đánh đổi giữa mức tiêu thụ điện và thời gian suy diễn.
Model Drift & Robustness: Mức độ suy giảm hiệu suất khi phân phối dữ liệu thay đổi.
Scalability: Khả năng mở rộng kiến trúc và nhận diện các điểm nghẽn (bottlenecks).
Thử nghiệm lâm sàng và Báo cáo: Triển khai thử nghiệm thực tế trên nhóm 10-30 người dùng trong 1-2 tháng. Đánh giá độ nhạy (sensitivity), độ đặc hiệu (specificity) của hệ thống và mức độ hài lòng về trải nghiệm người dùng (UX) để hoàn thiện báo cáo khoa học cuối cùng.