# Báo cáo tác động hiệu năng của Crypto HUD

[English](README.md) · [简体中文](README.zh-CN.md) ·
[繁體中文](README.zh-TW.md) · [Español](README.es.md) ·
[Português do Brasil](README.pt-BR.md) · [Tiếng Việt](README.vi.md) ·
[Bahasa Indonesia](README.id.md) · [Türkçe](README.tr.md) ·
[한국어](README.ko.md) · [日本語](README.ja.md) ·
[Русский](README.ru.md) · [العربية](README.ar.md)

> Ngày kiểm thử: 2026-07-12<br>
> Sản phẩm: Crypto HUD 0.9.7<br>
> Commit: `b572ce82d2b1a7d95fa2c5ab6687b73b9ed76ae7`<br>
> SHA-256 của tệp thực thi:
> `3F50145630A74FB6E7265AFE8935101BDB872C743DA6BC8267C49E0DD7BE267D`

## Tóm tắt báo cáo

Báo cáo ghi lại CPU, bộ nhớ, thread, cấu trúc tiến trình, khởi động, số yêu cầu
mạng và độ ổn định của Crypto HUD trong các điều kiện nêu dưới đây.

Với ba cặp và chu kỳ làm mới năm giây, lần chạy mười phút có cache nóng ghi nhận
**0,070% CPU trung bình toàn máy**, **20,27 MiB private commit**, **47,96 MiB
working set** và **19,29 MiB private working set**.

Với 20 cặp trực tuyến và cùng chu kỳ, số đo là **0,125% CPU trung bình**,
**0,681% CPU P95**, **21,56 MiB private commit** và **49,02 MiB working set**.

Không ghi nhận tiến trình con hoặc WebView2. Bộ nhớ khi ẩn tiện ích xuống khay
gần với kịch bản offline hiển thị. Trong bài mười phút, bộ nhớ tăng ở giai đoạn
làm nóng và cập nhật nến năm phút, sau đó nằm trong một khoảng ổn định ở phút
7–10.

Các số liệu chỉ mô tả tiến trình Crypto HUD trên máy thử nghiệm đã nêu. Báo cáo
không bao gồm GPU được quy gán, chi phí DWM hoặc điện năng.

## Số liệu chính

| Chỉ số | Kết quả |
| --- | ---: |
| CPU trung bình mặc định | 0,070% |
| CPU P95 mặc định | 0,189% |
| Private commit trung vị | 20,27 MiB |
| Working set trung vị | 47,96 MiB |
| Private working set trung vị | 19,29 MiB |
| Thời gian dữ liệu trực tuyến sẵn sàng | 1,705 giây |
| CPU trung bình với 20 cặp | 0,125% |
| CPU P95 với 20 cặp | 0,681% |
| Mẫu CPU cao nhất | 0,916% |
| Tiến trình con | 0 |
| Tiến trình WebView2 | 0 |

## Môi trường kiểm thử

| Hạng mục | Giá trị |
| --- | --- |
| Hệ điều hành | Windows 11 Pro for Workstations, 10.0.26200, build 26200 |
| Ảo hóa | QEMU Standard PC (Q35 + ICH9, 2009) |
| CPU được báo cáo | AMD Ryzen 7 8845HS with Radeon 780M Graphics |
| Bộ xử lý logic | 12 |
| Bộ nhớ hệ thống | 23,98 GiB |
| Bộ điều hợp hiển thị | OrayIddDriver Device; Red Hat QXL controller |
| Chế độ nguồn | Balanced |
| Bộ kết xuất | Slint software renderer |
| Toolchain | Rust 1.96.0 |

Tệp release được chạy từ bố cục staged có đầy đủ plugin và tài nguyên. Mỗi lần
chạy dùng thư mục trạng thái riêng và ID single-instance duy nhất.

## Định nghĩa chỉ số

- **CPU toàn máy:** thời gian CPU của tiến trình chia cho thời gian và 12 bộ xử
  lý logic.
- **CPU P95:** 95% mẫu ổn định không vượt quá giá trị này.
- **Private commit:** bộ nhớ cam kết chỉ cho tiến trình.
- **Working set:** trang đang cư trú trong RAM, có thể gồm trang dùng chung.
- **Private working set:** RAM cư trú chỉ thuộc tiến trình.
- **Thread:** tổng thread ở từng mẫu; worker mạng tạm thời làm số này thay đổi.
- **Handle:** handle Windows do tiến trình giữ.

CPU được tính từ chênh lệch `TotalProcessorTime`. Bỏ 10 giây đầu; 30 giây với
bài năm phút và phút đầu với bài mười phút.

CPU offline là ước lượng làm tròn dựng lại từ bộ đếm tương đương một lõi sau khi
phát hiện làm tròn số nguyên trong bản tổng hợp đầu tiên. CPU mạng thật không bị
vấn đề này.

## Kết quả theo kịch bản

| Kịch bản | CPU trung bình / P95 | Private commit trung vị | Working set trung vị | Thread trung vị / tối đa |
| --- | ---: | ---: | ---: | ---: |
| Mặc định trực tuyến, 3 cặp, 10 phút | 0,070% / 0,189% | 20,27 MiB | 47,96 MiB | 9 / 13 |
| Mở Cài đặt và dùng mạng thật | 0,071% / 0,233% | 23,74 MiB | 55,35 MiB | 14 / 17 |
| 20 cặp trực tuyến, làm mới 5 giây | 0,125% / 0,681% | 21,56 MiB | 49,02 MiB | 14 / 18 |
| Proxy cục bộ từ chối ngay | 0,049% / 0,232% | 18,66 MiB | 43,67 MiB | 14 / 17 |
| Một tiện ích hiển thị, dữ liệu offline | khoảng 0,04% / 0,25% | 17,43 MiB | 41,93 MiB | 12 / 12 |
| Cùng tiện ích bị ẩn xuống khay | khoảng 0,03% / 0,25% | 17,43 MiB | 41,77 MiB | 12 / 12 |
| 10 cửa sổ Quote Board, offline | khoảng 0,14% / 0,50% | 20,13 MiB | 45,48 MiB | 12 / 12 |
| Một phiên bản của mỗi loại trong năm tiện ích | khoảng 0,17% / 0,50% | 26,63 MiB | 53,37 MiB | 12 / 12 |
| Một Quote Board với 20 cặp offline | khoảng 0,09% / 0,42% | 17,87 MiB | 42,78 MiB | 12 / 12 |
| Một tiện ích ở tỷ lệ 300%, offline | khoảng 0,08% / 0,50% | 18,71 MiB | 44,30 MiB | 12 / 12 |

Mẫu CPU đơn cao nhất là **0,916%** trong kịch bản 20 cặp.

## Chênh lệch quan sát được

- Cài đặt mở ghi nhận 23,74 MiB private commit và 55,35 MiB working set; bài
  mặc định ghi nhận 20,27 MiB và 47,96 MiB.
- Từ 3 lên 20 cặp, CPU trung bình đổi từ 0,070% thành 0,125%, private commit từ
  20,27 MiB thành 21,56 MiB, working set từ 47,96 MiB thành 49,02 MiB.
- 10 Quote Board offline nhiều hơn một tiện ích 2,70 MiB private commit và
  3,55 MiB working set.
- Năm loại tiện ích nhiều hơn một tiện ích 9,20 MiB private commit và 11,44 MiB
  working set.
- Tỷ lệ 300% nhiều hơn 1,28 MiB private commit và 2,37 MiB working set.
- Trước và sau khi ẩn, private commit đều là 17,43 MiB; working set đổi từ
  41,93 MiB thành 41,77 MiB. CPU ước lượng là 0,04% và 0,03%.

## Khởi động

Mốc tất cả dữ liệu thị trường sẵn sàng đạt sau **1,705 giây**. Mốc này yêu cầu
dữ liệu cho các hàng đã cấu hình và gồm khám phá plugin, tạo cửa sổ, yêu cầu đầu
tiên và timer một giây. Đây không phải thời gian khung hình đầu tiên.

Kiểm tra cập nhật đi theo nhánh lỗi vì GitHub Releases API thất bại. Phản hồi
thành công chưa được đo.

## Dòng thời gian bộ nhớ

| Phút | Private commit trung vị | Working set trung vị |
| ---: | ---: | ---: |
| 1 | 19,05 MiB | 46,46 MiB |
| 2 | 18,95 MiB | 46,52 MiB |
| 3 | 19,16 MiB | 46,74 MiB |
| 4 | 19,42 MiB | 47,03 MiB |
| 5 | 19,72 MiB | 47,27 MiB |
| 6 | 20,33 MiB | 47,96 MiB |
| 7 | 20,36 MiB | 48,02 MiB |
| 8 | 20,36 MiB | 48,05 MiB |
| 9 | 20,27 MiB | 48,02 MiB |
| 10 | 20,27 MiB | 48,02 MiB |

Private commit tăng đến phút 6. Trung vị phút 7–10 nằm trong khoảng
20,27–20,36 MiB. Thay đổi ở phút 6 trùng vùng cập nhật nến năm phút.

Có một lỗi SOL tạm thời. Ứng dụng tiếp tục và thoát với mã 0.

## Quan sát mạng và tệp

`số cặp duy nhất × (60 / số giây làm mới + 0,2)` yêu cầu/phút.

| Cấu hình | Yêu cầu ước tính/phút |
| --- | ---: |
| 3 cặp, 5 giây | 36,6 |
| 8 cặp, 5 giây | 97,6 |
| 20 cặp, 5 giây | 244 |
| 3 cặp, 60 giây | 3,6 |
| 20 cặp, 60 giây | 24 |

`0,2` biểu thị một lần cập nhật nến mỗi năm phút. Khởi động thêm khoảng một
yêu cầu ticker và một yêu cầu nến mỗi cặp. Độ trễ có thể giảm tần suất thực tế.

Proxy bị từ chối tạo bốn chu kỳ lỗi trong 90 giây. Chưa đo timeout tám giây hoặc
kết nối lại.

Sau khi làm nóng, I/O tệp khi nhàn rỗi gần bằng không. Chưa đo lưu trên đĩa
chậm. Counter I/O không được dùng làm byte mạng quy gán.

## Cấu trúc tiến trình

- Tiến trình con: 0.
- Tiến trình WebView2: 0.
- Thread mặc định: trung vị 9, tối đa 13.
- Thread với 20 cặp: trung vị 14, tối đa 18.
- Handle mặc định: trung vị 301, tối đa 322.

## Phạm vi và giới hạn

- QEMU và màn hình từ xa không định lượng GPU vật lý, DWM hoặc điện năng.
- Mỗi kịch bản tùy chỉnh chạy một lần; không có khoảng tin cậy nhiều lần chạy.
- Lần chạy dài nhất là 10 phút; chưa có bài 8–24 giờ.
- Chưa đo DPI hệ thống 150–300%, nhiều màn hình 4K, sleep/resume, tháo màn hình
  hoặc hoạt ảnh liên tục.
- Chưa đo DNS chậm, HTTP 429, timeout tám giây hoặc phục hồi mạng.
- Không có WPR/WPA; không có byte mạng quy gán, GPU, context switch, wakeup độc
  lập hoặc điện năng.
- Chưa đo lưu trên đĩa chậm hoặc đồng bộ.
- Kết quả chỉ tương ứng phiên bản và hash ở đầu báo cáo.

## Hồ sơ xác minh

- Build release:
  `cargo +1.96.0 build --release --locked -p crypto-hud`
- Kiểm tra staged với plugin và tài nguyên.
- Bảy kịch bản offline, bảy kịch bản mạng thật và một lần chạy 10 phút.
- CPU, ba chỉ số bộ nhớ, thread, handle, I/O, khởi động, tiến trình và lỗi.
- Kiểm tra dữ liệu, điều hướng, liên kết, UTF-8 và cách ly hai chiều.

Không thay đổi hành vi sản phẩm trong quá trình đo.
