<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Crypto HUD 圖示">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>行情隨時都在，需要時看一眼。</strong><br>
  常駐 Windows 桌面的原生加密行情小工具，不打斷你的專注。
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
  <img alt="平台：Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="使用 Rust 建置" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="原生 Slint 介面" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="MIT 授權" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="Market Compass 小工具，顯示 BTC 價格、K 線與周邊市場">
  </picture>
</p>

<p align="center"><sub>行情就在工作區邊緣。不必切換交易所、不連接錢包，也不製造干擾。</sub></p>

---

Crypto HUD 是一款輕量、本機優先的桌面行情工具，適合想關注幾個幣種、
又不想一直待在交易終端裡的人。把小工具放在順眼的位置，繼續工作；
只有需要時再看一眼。

## 為桌面背景而生

- **原生輕量**：Rust + Slint，不依賴 Electron、Tauri、WebView 或內建瀏覽器。
- **實測輕量占用**：預設測試設定下（單一小工具、3 個幣種對、每 5 秒更新），
  穩定運作時平均 CPU 占用約 **0.070%**，程序私有記憶體約 **20 MiB**。
  [查看完整效能報告](docs/performance-reports/README.zh-TW.md)。
- **一眼掌握**：小工具可拖曳、可置頂，重要數字始終在視線內。
- **本機優先**：版面與偏好保存在電腦上，不需要帳號或 API Key。
- **隨時安靜**：按 <kbd>Alt</kbd> + <kbd>C</kbd> 一次隱藏或還原所有小工具。
- **多資料來源**：支援 Binance、Coinbase、OKX 與 Hyperliquid 的公開行情。
- **彈性外觀**：提供多種樣式、深淺色主題、透明度與漲跌顏色設定。

> [!IMPORTANT]
> Crypto HUD 只用來查看公開行情。它不會下單、連接錢包或保管資產，
> 也不會要求私鑰、助記詞、交易所帳號或 API Key。

## 小工具預覽

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="Focus Ticker，顯示 BTC 價格、漲跌幅與走勢圖">
  </picture>
</p>

你可以選擇精簡行情列、完整圖表卡片或多幣種看板。內建小工具與自訂小工具
使用同一套外掛協定。

## 快速開始

Crypto HUD 為 Windows 打造。專案使用 `mise` 固定 Rust `1.96`，
並提供一條指令即可啟動的本機任務。

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

啟動後可以拖曳小工具、從系統匣開啟設定、選擇幣種、切換主題並調整透明度。
小工具位置與偏好會自動保存。

## 自訂與擴充

- 閱讀[自訂 UI 外掛開發指南](CUSTOM_UI_PLUGIN_DEVELOPMENT.md)。
- 查看[外掛協定與內建範例](crates/crypto-hud/plugins/README.md)。
- 使用 Slint 建立自己的行情小工具。

## 開發

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

歡迎參與貢獻。請查看[貢獻指南](CONTRIBUTING.md)、[更新日誌](CHANGELOG.md)
與[安全政策](SECURITY.md)。

## 路線圖

目前重點包括更清楚的資料來源健康狀態、價格與 24 小時漲跌提醒、
更完整的小工具管理、更好的首次啟動擺放，以及更完整的安裝程式。

## 授權

MIT © Crypto HUD Contributors
