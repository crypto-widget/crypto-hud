<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Crypto HUD 로고">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>시장을, 언제나 한눈에.</strong><br>
  집중을 방해하지 않고 Windows 바탕 화면에 머무는 네이티브 암호화폐 위젯입니다.
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
  <img alt="플랫폼: Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Rust로 제작" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="네이티브 Slint UI" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="MIT 라이선스" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="BTC 가격, 캔들 차트, 주변 시장을 보여 주는 Market Compass 위젯">
  </picture>
</p>

<p align="center"><sub>작업 공간 가장자리에서 실시간 가격을 확인하세요. 거래소 탭도, 지갑 연결도, 소음도 없습니다.</sub></p>

---

Crypto HUD는 몇 개의 코인을 계속 확인하고 싶지만 거래 터미널에 머물고 싶지는
않은 사람을 위한 가볍고 로컬 우선인 시장 표시 도구입니다. 편한 위치에 위젯을
놓고 작업을 계속하다가 필요할 때만 가볍게 확인하세요.

## 조용한 백그라운드를 위한 설계

- **네이티브 & 가벼움**: Rust + Slint. Electron, Tauri, WebView, 내장 브라우저를 사용하지 않습니다.
- **한눈에 확인**: 이동 가능하고 항상 위에 표시되는 위젯으로 중요한 숫자를 놓치지 않습니다.
- **로컬 우선**: 레이아웃과 설정은 내 컴퓨터에 저장되며 계정이나 API Key가 필요 없습니다.
- **필요할 때 숨김**: <kbd>Alt</kbd> + <kbd>C</kbd>로 모든 위젯을 한 번에 숨기거나 복원합니다.
- **4개의 공개 데이터 소스**: Binance, Coinbase, OKX, Hyperliquid를 지원합니다.
- **유연한 디자인**: 다양한 스타일, 밝은/어두운 테마, 투명도와 등락 색상을 설정할 수 있습니다.

> [!IMPORTANT]
> Crypto HUD는 공개 시장 정보를 보는 용도로만 설계되었습니다. 주문을 실행하거나
> 지갑을 연결하고 자산을 보관하지 않으며, 개인 키·시드 문구·거래소 계정·API Key를
> 요구하지 않습니다.

## 위젯 미리 보기

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="BTC 가격, 변동률, 미니 차트를 보여 주는 Focus Ticker">
  </picture>
</p>

작은 티커, 자세한 차트 카드, 여러 시장을 보여 주는 보드 중에서 선택할 수 있습니다.
기본 위젯과 사용자 정의 위젯은 동일한 플러그인 규격을 사용합니다.

## 빠른 시작

Crypto HUD는 Windows용으로 제작되었습니다. 저장소는 `mise`로 Rust `1.96`을
고정하고 한 번의 명령으로 실행할 수 있는 로컬 작업을 제공합니다.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

실행 후 위젯을 끌어 배치하고, 시스템 트레이에서 설정을 열어 시장과 테마를
선택하고 투명도를 조절하세요. 위치와 설정은 자동으로 저장됩니다.

## 사용자 정의와 플러그인

- [사용자 정의 UI 플러그인 개발 가이드](CUSTOM_UI_PLUGIN_DEVELOPMENT.md)를 읽어 보세요.
- [플러그인 규격과 기본 예제](crates/crypto-hud/plugins/README.md)를 확인하세요.
- Slint로 나만의 시장 위젯을 만들 수 있습니다.

## 개발

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

기여를 환영합니다. [기여 가이드](CONTRIBUTING.md), [변경 기록](CHANGELOG.md),
[보안 정책](SECURITY.md)을 확인해 주세요.

## 로드맵

더 명확한 데이터 공급자 상태, 가격 및 24시간 변동 알림, 향상된 위젯 관리,
더 나은 최초 배치와 완성도 높은 설치 프로그램을 우선 개발하고 있습니다.

## 라이선스

MIT © Crypto HUD Contributors
