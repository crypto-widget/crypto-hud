<h1 align="center">Crypto HUD</h1>

<p align="center">
  轻量、本地、安全的 Windows 桌面行情 HUD。
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img alt="状态：alpha" src="https://img.shields.io/badge/status-alpha-f59e0b">
  <img alt="平台：Windows" src="https://img.shields.io/badge/platform-Windows-0078d4">
  <img alt="运行方式：原生" src="https://img.shields.io/badge/runtime-native-22c55e">
  <img alt="无需账号" src="https://img.shields.io/badge/account-not%20required-14b8a6">
  <img alt="许可证：MIT" src="https://img.shields.io/badge/license-MIT-111827">
</p>

> 把行情放在桌面边缘，需要时扫一眼，不需要时安静待着。

Crypto HUD 让加密货币价格安静地显示在你的 Windows 桌面上。

不用再一次次切到交易所，只为了看一眼行情。把一个小小的价格卡片放在顺眼的位置，
然后继续工作、学习、写代码或者过自己的生活。想看的时候扫一眼，不想看的时候它不会打扰你。

项目目前处于 alpha 阶段。它已经可以用来查看价格，但仍属于早期软件，功能和界面可能会快速变化。

## 产品特点

- **扫一眼行情**：价格常驻桌面边缘，不用反复打开交易所，也不打断工作和生活。
- **轻量低占用**：原生 Rust + Slint，不依赖 Electron、Tauri、WebView 或内置浏览器运行时。
- **本地运行更安心**：布局和偏好保存在本机，不需要账号、OAuth、API key、钱包权限、私钥或助记词。
- **一键隐藏/显示**：按 `Alt+C` 隐藏所有组件，需要时再一键唤回。
- **只看行情，不碰资产**：只读取公开行情，不下单、不连接钱包、不托管资产。

## 它能做什么

- 在桌面显示可拖拽、可置顶的悬浮行情组件。
- 选择关注的币种/交易对，并从 Binance、Coinbase、OKX 和 Hyperliquid 获取实时行情。
- 支持不同组件样式、浅色/深色主题、中英文和涨跌颜色偏好。
- 自动保存组件位置和设置，下次打开继续使用。

Crypto HUD 只用于查看公开行情信息。它不会下单，不连接钱包，不托管资产，也不会要求你提供交易所 API key。
它的安全边界刻意保持简单：公开行情来自市场数据源，布局和偏好设置保存在本机。

## 适合谁使用

如果你有这些需求，Crypto HUD 可能适合你：

- 白天会一直关注几个币种。
- 不想反复打开交易所，只为了确认一下价格。
- 想要一个轻量原生桌面工具，而不是完整交易终端。
- 喜欢把小组件固定摆在桌面上。

如果你现在需要完整图表、投资组合统计、交易下单或复杂提醒自动化，它暂时还不是最合适的工具。

## 当前状态

Crypto HUD 是一个 alpha 阶段的原生 Windows 桌面应用，使用 Rust 和 Slint 构建。

- 作为单个原生桌面进程运行。
- 使用真正的桌面窗口，而不是 WebView 或浏览器宿主 UI。
- 已包含主界面、托盘控制、全局隐藏/显示快捷键、本地持久化、插件加载和 Windows 打包脚本。
- 默认快捷键：`Alt+C` 隐藏或显示所有组件。

## 本地试用

你需要 Rust。项目使用 `mise` 固定预期的 Rust 工具链版本。

请先审阅 `mise.toml`，然后安装工具链：

```powershell
mise trust
mise install
```

检查项目能否构建：

```powershell
mise run check
```

运行应用：

```powershell
mise run run-app
```

如果想指定启动几个组件：

```powershell
cargo run -p crypto-hud -- --widgets 3
```

## 基本使用

- 拖拽价格卡片即可移动。
- 点击托盘图标打开主界面。
- 右键托盘图标可以打开主界面或退出。
- 在设置里可以添加组件、选择币种、调整透明度、切换主题、配置开机启动，以及修改市场数据偏好。
- 使用 `Alt+C` 隐藏或显示所有组件。

布局和设置会自动保存。做隔离测试时，可以设置自定义状态目录：

```powershell
$env:CRYPTO_HUD_STATE_DIR = "$PWD\.crypto-hud-state"
mise run run-app
```

## 给贡献者

常用开发命令：

```powershell
mise run format-check
mise run check
mise run test
mise run format
mise run run-app
powershell -ExecutionPolicy Bypass -File .\scripts\gui-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\gui-settings-interaction-smoke.ps1
```

代码拆分为几个小型 Rust crate：

```text
crates/
  crypto-hud-core/          市场符号、格式化和提醒基础逻辑
  crypto-hud-market/        行情数据获取
  crypto-hud-runtime/       小组件运行时视图合同
  crypto-hud-shell-state/   设置和布局状态持久化
  crypto-hud/              原生桌面外壳和 Slint UI
```

内置和本地小组件插件合同位于 `crates/crypto-hud/plugins/README.md`。

贡献指南见 `CONTRIBUTING.md`，安全问题报告方式见 `SECURITY.md`。

## 发布打包

Crypto HUD 目前使用本地 Windows 发布脚本，而不是 GitHub Actions 自动发布流程。

```powershell
cargo test --workspace
powershell -ExecutionPolicy Bypass -File .\scripts\gui-smoke.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\release-process-check.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\package-smoke.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File .\scripts\update-smoke.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1 -Version v0.8.2
```

打包脚本会在 `dist/` 中生成 Windows zip、校验和和 release manifest。安装器会先校验包内容，
再复制文件。项目支持可选 Windows Authenticode 签名，相关环境变量记录在
`scripts/package-windows.ps1` 中。

## 路线图

- 更好的数据源健康状态、过期数据状态和错误状态。
- 价格和 24 小时涨跌幅提醒。
- 复制、重命名、排序和按组件显示/隐藏。
- 更好的首次启动摆放和恢复。
- 更完整的安装器格式。

## 许可证

Crypto HUD 使用 MIT License 授权。详见 `LICENSE`。
