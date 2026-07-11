<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Biểu trưng Crypto HUD">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>Thị trường luôn trong tầm mắt.</strong><br>
  Widget crypto native trên màn hình Windows, luôn hữu ích mà không làm bạn mất tập trung.
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="README.zh-TW.md">繁體中文</a> ·
  <a href="README.es.md">Español</a> ·
  <a href="README.pt-BR.md">Português</a> ·
  <a href="README.vi.md">Tiếng Việt</a><br>
  <a href="README.id.md">Bahasa Indonesia</a> ·
  <a href="README.tr.md">Türkçe</a> ·
  <a href="README.ko.md">한국어</a> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.ru.md">Русский</a> ·
  <a href="README.ar.md">العربية</a>
</p>

<p align="center">
  <img alt="Nền tảng: Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Xây dựng bằng Rust" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="Giao diện Slint native" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="Giấy phép MIT" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="Widget Market Compass hiển thị giá BTC, biểu đồ nến và các thị trường xung quanh">
  </picture>
</p>

<p align="center"><sub>Giá trực tiếp ngay bên cạnh không gian làm việc. Không cần mở sàn, không kết nối ví, không gây nhiễu.</sub></p>

---

Crypto HUD là bảng theo dõi thị trường nhẹ, ưu tiên dữ liệu cục bộ, dành cho người
muốn theo dõi vài đồng coin mà không phải ở mãi trong terminal giao dịch. Đặt
widget ở vị trí thuận mắt, tiếp tục công việc và chỉ liếc nhìn khi cần.

## Được thiết kế để chạy yên lặng trong nền

- **Native và nhẹ**: Rust + Slint, không dùng Electron, Tauri, WebView hay trình duyệt nhúng.
- **Nắm bắt trong một ánh nhìn**: widget có thể kéo, luôn nổi và giữ số liệu quan trọng trong tầm mắt.
- **Ưu tiên cục bộ**: bố cục và tùy chọn nằm trên máy của bạn; không cần tài khoản hay API Key.
- **Ẩn khi cần tập trung**: nhấn <kbd>Alt</kbd> + <kbd>C</kbd> để ẩn hoặc khôi phục mọi widget.
- **Bốn nguồn công khai**: Binance, Coinbase, OKX và Hyperliquid.
- **Giao diện linh hoạt**: nhiều kiểu widget, chủ đề sáng/tối, độ trong suốt và màu tăng giảm tùy chỉnh.

> [!IMPORTANT]
> Crypto HUD chỉ dùng để xem dữ liệu thị trường công khai. Ứng dụng không đặt
> lệnh, không kết nối ví, không lưu giữ tài sản và không bao giờ yêu cầu khóa bí mật,
> cụm từ khôi phục, tài khoản sàn hay API Key.

## Xem trước widget

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="Focus Ticker hiển thị giá BTC, mức thay đổi và biểu đồ nhỏ">
  </picture>
</p>

Chọn ticker nhỏ gọn, thẻ biểu đồ chi tiết hoặc bảng nhiều thị trường. Widget đi kèm
và widget tùy chỉnh đều dùng chung một hợp đồng plugin.

## Bắt đầu nhanh

Crypto HUD được xây dựng cho Windows. Kho mã dùng `mise` để cố định Rust `1.96`
và cung cấp tác vụ khởi chạy cục bộ bằng một lệnh.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

Sau khi chạy, bạn có thể kéo widget, mở cài đặt từ khay hệ thống, chọn thị trường,
đổi chủ đề và điều chỉnh độ trong suốt. Vị trí và tùy chọn được lưu tự động.

## Tùy chỉnh và plugin

- Đọc [hướng dẫn phát triển plugin giao diện](CUSTOM_UI_PLUGIN_DEVELOPMENT.md).
- Xem [hợp đồng plugin và các ví dụ đi kèm](crates/crypto-hud/plugins/README.md).
- Tạo widget thị trường của riêng bạn bằng Slint.

## Phát triển

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

Mọi đóng góp đều được chào đón. Xem [hướng dẫn đóng góp](CONTRIBUTING.md),
[nhật ký thay đổi](CHANGELOG.md) và [chính sách bảo mật](SECURITY.md).

## Lộ trình

Các ưu tiên gồm trạng thái nguồn dữ liệu rõ ràng hơn, cảnh báo giá và biến động 24 giờ,
quản lý widget đầy đủ hơn, bố trí lần đầu tốt hơn và trình cài đặt hoàn thiện hơn.

## Giấy phép

MIT © Crypto HUD Contributors
