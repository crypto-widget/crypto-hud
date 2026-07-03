# Crypto HUD

[English](README.md) | [简体中文](README.zh-CN.md)

Crypto HUD 让加密货币价格安静地显示在你的 Windows 桌面上。

不用再一次次切到交易所，只为了看一眼行情。把一个小小的价格卡片放在顺眼的位置，
然后继续工作、学习、写代码或者过自己的生活。想看的时候扫一眼，不想看的时候它不会打扰你。

项目目前处于 alpha 阶段。它已经可以用来查看价格，但仍属于早期软件，功能和界面可能会快速变化。

## 产品特点

- **轻量常驻桌面**：使用原生 Rust + Slint 桌面窗口，不依赖 Electron、Tauri、WebView 或内置浏览器运行时。
- **低占用、低打扰**：面向长期常驻场景，尽量保持启动快、内存占用低、后台 CPU 占用低。
- **本地优先体验**：界面、组件布局和偏好设置都在本机运行和保存，不依赖云端账号或托管控制面。
- **无需登录和授权**：不需要账号、OAuth、交易所 API key、钱包访问权限、私钥或助记词。
- **扫一眼就够**：价格留在桌面边上，不用反复打开交易所，也不打断工作、学习和生活。
- **安全边界清晰**：只读取公开行情数据，不下单、不连接钱包、不托管资产。

## 它能做什么

- 在桌面上显示小型悬浮行情组件。
- 可以让组件保持在其他窗口上方，也可以放在不打扰的位置。
- 提供多种组件样式，从紧凑 ticker 到更大的市场卡片都有。
- 可以选择每个组件要关注的币种/交易对。
- 从 Binance、OKX 和 Hyperliquid 获取实时行情。
- 支持浅色/深色主题、英文/简体中文，以及绿涨红跌或红涨绿跌。
- 会记住组件位置和设置，下次启动自动恢复。

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
- 已包含设置窗口、托盘控制、全局隐藏/显示快捷键、本地持久化、插件加载和 Windows 打包脚本。
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
- 点击托盘图标打开设置。
- 右键托盘图标可以打开设置或退出。
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
powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1 -Version v0.1.0-alpha.1
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
