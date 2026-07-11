<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Crypto HUD ロゴ">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>相場を、いつでもひと目で。</strong><br>
  集中を邪魔せず Windows デスクトップに常駐する、ネイティブな暗号資産ウィジェット。
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
  <img alt="プラットフォーム：Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Rust 製" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="ネイティブ Slint UI" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="MIT ライセンス" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="BTC 価格、ローソク足、周辺市場を表示する Market Compass ウィジェット">
  </picture>
</p>

<p align="center"><sub>作業スペースの端にリアルタイム価格を。取引所タブも、ウォレット接続も、余計なノイズもありません。</sub></p>

---

Crypto HUD は、いくつかの通貨を追いかけたいものの、取引端末に居続けたくは
ない人のための軽量でローカルファーストな市場表示ツールです。見やすい場所に
ウィジェットを置き、作業を続けながら必要なときだけ確認できます。

## 背景で静かに動くための設計

- **ネイティブで軽量**：Rust + Slint。Electron、Tauri、WebView、内蔵ブラウザは使いません。
- **ひと目で確認**：移動可能で常に手前に置けるウィジェットが重要な数字を見せ続けます。
- **ローカルファースト**：レイアウトと設定は端末内に保存。アカウントや API Key は不要です。
- **必要なときに非表示**：<kbd>Alt</kbd> + <kbd>C</kbd> ですべてのウィジェットを一括表示・非表示。
- **4 つの公開データソース**：Binance、Coinbase、OKX、Hyperliquid に対応。
- **柔軟な見た目**：複数のスタイル、ライト／ダークテーマ、透明度、騰落色を設定できます。

> [!IMPORTANT]
> Crypto HUD は公開市場情報の閲覧専用です。注文、ウォレット接続、資産保管は行わず、
> 秘密鍵、シードフレーズ、取引所アカウント、API Key を求めることもありません。

## ウィジェットプレビュー

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="BTC 価格、変動率、ミニチャートを表示する Focus Ticker">
  </picture>
</p>

コンパクトなティッカー、詳しいチャートカード、複数市場のボードから選べます。
同梱ウィジェットとカスタムウィジェットは同じプラグイン仕様を使用します。

## クイックスタート

Crypto HUD は Windows 向けです。リポジトリは `mise` で Rust `1.96` を固定し、
1 コマンドで起動できるローカルタスクを用意しています。

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

起動後はウィジェットをドラッグし、システムトレイから設定を開いて市場、テーマ、
透明度を調整できます。位置と設定は自動保存されます。

## カスタマイズとプラグイン

- [カスタム UI プラグイン開発ガイド](CUSTOM_UI_PLUGIN_DEVELOPMENT.md)を読む。
- [プラグイン仕様と同梱サンプル](crates/crypto-hud/plugins/README.md)を見る。
- Slint で自分だけの市場ウィジェットを作る。

## 開発

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

コントリビューションを歓迎します。[コントリビューションガイド](CONTRIBUTING.md)、
[変更履歴](CHANGELOG.md)、[セキュリティポリシー](SECURITY.md)をご覧ください。

## ロードマップ

データ提供元の状態表示、価格と 24 時間変動アラート、より充実したウィジェット管理、
初回配置の改善、より完成度の高いインストーラーを優先しています。

## ライセンス

MIT © Crypto HUD Contributors
