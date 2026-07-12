<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Логотип Crypto HUD">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>Рынок всегда перед глазами.</strong><br>
  Нативные криптовалютные виджеты на рабочем столе Windows, которые не отвлекают от дел.
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
  <img alt="Платформа: Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Написано на Rust" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="Нативный интерфейс Slint" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="Лицензия MIT" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="Виджет Market Compass с ценой BTC, свечным графиком и другими рынками">
  </picture>
</p>

<p align="center"><sub>Цены в реальном времени у края рабочего пространства. Без вкладки биржи, подключения кошелька и лишнего шума.</sub></p>

---

Crypto HUD — лёгкий локальный экран рынка для тех, кто хочет следить за несколькими
монетами, не проводя весь день в торговом терминале. Разместите виджет в удобном
месте, продолжайте работать и смотрите на рынок только тогда, когда это нужно.

## Создан для спокойной работы в фоне

- **Нативный и лёгкий**: Rust + Slint, без Electron, Tauri, WebView и встроенного браузера.
- **Измеренное небольшое потребление ресурсов**: в стандартном тесте с одним
  виджетом, 3 рыночными парами и обновлением каждые 5 секунд средняя загрузка
  CPU в установившемся режиме составила **0,070%**, а частная память процесса —
  около **20 MiB**.
  [Открыть полный отчёт о производительности](docs/performance-reports/README.ru.md).
- **Всё одним взглядом**: перемещаемые виджеты поверх окон держат важные цифры на виду.
- **Локальный подход**: расположение и настройки остаются на компьютере; аккаунт и API Key не нужны.
- **Скрытие по команде**: <kbd>Alt</kbd> + <kbd>C</kbd> скрывает или возвращает все виджеты.
- **Четыре открытых источника**: Binance, Coinbase, OKX и Hyperliquid.
- **Гибкий внешний вид**: разные стили, светлая и тёмная темы, прозрачность и цвета рынка.

> [!IMPORTANT]
> Crypto HUD предназначен только для просмотра открытых рыночных данных. Он не
> совершает сделки, не подключает кошельки, не хранит активы и не запрашивает
> приватные ключи, seed-фразы, аккаунты бирж или API Key.

## Предпросмотр виджетов

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="Focus Ticker с ценой BTC, изменением и мини-графиком">
  </picture>
</p>

Выберите компактный тикер, карточку с подробным графиком или панель нескольких рынков.
Встроенные и пользовательские виджеты используют один и тот же контракт плагинов.

## Быстрый старт

Crypto HUD создан для Windows. Репозиторий использует `mise`, чтобы закрепить
Rust `1.96`, и предоставляет задачу локального запуска одной командой.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

После запуска перемещайте виджеты, открывайте настройки из системного трея,
выбирайте рынки и темы, настраивайте прозрачность. Позиции и параметры сохраняются автоматически.

## Настройка и плагины

- Прочитайте [руководство по разработке UI-плагинов](CUSTOM_UI_PLUGIN_DEVELOPMENT.md).
- Изучите [контракт плагинов и встроенные примеры](crates/crypto-hud/plugins/README.md).
- Создайте собственный рыночный виджет на Slint.

## Разработка

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

Мы рады вкладу сообщества. Ознакомьтесь с [руководством участника](CONTRIBUTING.md),
[историей изменений](CHANGELOG.md) и [политикой безопасности](SECURITY.md).

## Планы

В приоритете более ясные состояния источников данных, оповещения о цене и изменении
за 24 часа, расширенное управление виджетами, улучшенное первое размещение и более
полный установщик.

## Лицензия

MIT © Crypto HUD Contributors
