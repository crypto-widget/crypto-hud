<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Logo Crypto HUD">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>Pasar Anda, selalu dalam sekali pandang.</strong><br>
  Widget kripto native di desktop Windows yang tetap terlihat tanpa mengganggu fokus.
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
  <img alt="Platform: Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Dibuat dengan Rust" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="Antarmuka Slint native" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="Lisensi MIT" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="Widget Market Compass dengan harga BTC, grafik candlestick, dan pasar lainnya">
  </picture>
</p>

<p align="center"><sub>Harga langsung di tepi ruang kerja Anda. Tanpa tab bursa, tanpa koneksi dompet, tanpa kebisingan.</sub></p>

---

Crypto HUD adalah tampilan pasar yang ringan dan mengutamakan penyimpanan lokal
untuk Anda yang ingin memantau beberapa koin tanpa terus berada di terminal trading.
Letakkan widget di posisi yang nyaman, lanjutkan pekerjaan, dan lihat pasar saat diperlukan.

## Dibuat untuk tetap tenang di latar belakang

- **Native dan ringan**: Rust + Slint, tanpa Electron, Tauri, WebView, atau browser bawaan.
- **Penggunaan ringan yang terukur**: pada pengujian bawaan dengan satu widget,
  3 pasangan pasar, dan penyegaran setiap 5 detik, rata-rata CPU saat stabil
  adalah **0,070%** dengan sekitar **20 MiB memori privat proses**.
  [Lihat laporan performa lengkap](docs/performance-reports/README.id.md).
- **Sekilas langsung paham**: widget dapat digeser dan selalu di atas agar angka penting tetap terlihat.
- **Lokal lebih dulu**: tata letak dan preferensi tersimpan di komputer; tanpa akun atau API Key.
- **Sembunyikan kapan saja**: tekan <kbd>Alt</kbd> + <kbd>C</kbd> untuk menyembunyikan atau memulihkan semua widget.
- **Empat sumber publik**: Binance, Coinbase, OKX, dan Hyperliquid.
- **Tampilan fleksibel**: beragam gaya, tema terang/gelap, opasitas, dan warna pasar yang dapat diatur.

> [!IMPORTANT]
> Crypto HUD hanya untuk melihat data pasar publik. Aplikasi tidak melakukan
> transaksi, tidak menghubungkan dompet, tidak menyimpan aset, dan tidak pernah
> meminta kunci privat, seed phrase, akun bursa, atau API Key.

## Pratinjau widget

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="Focus Ticker dengan harga BTC, perubahan, dan grafik mini">
  </picture>
</p>

Pilih ticker ringkas, kartu dengan grafik lengkap, atau papan beberapa pasar.
Widget bawaan memakai kontrak plugin yang sama dengan widget kustom.

## Mulai cepat

Crypto HUD dibuat untuk Windows. Repositori menggunakan `mise` untuk mengunci
Rust `1.96` dan menyediakan tugas peluncuran lokal dengan satu perintah.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

Setelah berjalan, geser widget, buka pengaturan dari system tray, pilih pasar,
ganti tema, dan atur opasitas. Posisi serta preferensi tersimpan otomatis.

## Kustomisasi dan plugin

- Baca [panduan pengembangan plugin antarmuka](CUSTOM_UI_PLUGIN_DEVELOPMENT.md).
- Lihat [kontrak plugin dan contoh bawaan](crates/crypto-hud/plugins/README.md).
- Buat widget pasar Anda sendiri dengan Slint.

## Pengembangan

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

Kontribusi sangat diterima. Baca [panduan kontribusi](CONTRIBUTING.md),
[catatan perubahan](CHANGELOG.md), dan [kebijakan keamanan](SECURITY.md).

## Peta jalan

Prioritas mencakup status kesehatan penyedia yang lebih jelas, notifikasi harga
dan perubahan 24 jam, pengelolaan widget yang lebih lengkap, penempatan awal
yang lebih baik, serta installer yang lebih lengkap.

## Lisensi

MIT © Crypto HUD Contributors
