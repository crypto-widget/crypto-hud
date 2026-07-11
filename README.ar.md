<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="شعار Crypto HUD">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center" dir="rtl">
  <strong>السوق أمامك بنظرة واحدة.</strong><br>
  أدوات عملات رقمية أصلية تبقى على سطح مكتب Windows من دون أن تشتت تركيزك.
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
  <img alt="المنصة: Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="مبني باستخدام Rust" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="واجهة Slint أصلية" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="ترخيص MIT" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="أداة Market Compass تعرض سعر BTC ومخطط الشموع والأسواق المحيطة">
  </picture>
</p>

<p align="center" dir="rtl"><sub>أسعار مباشرة عند طرف مساحة العمل. بلا تبويب منصة تداول، وبلا اتصال بمحفظة، وبلا ضوضاء.</sub></p>

---

Crypto HUD شاشة سوق خفيفة ومحلية أولاً لمن يريد متابعة عدد قليل من العملات من
دون البقاء طوال اليوم داخل منصة تداول. ضع الأداة في المكان المناسب، وواصل عملك،
ثم ألقِ نظرة على السوق عندما تحتاج فقط.

## مصمم ليعمل بهدوء في الخلفية

- **أصلي وخفيف**: Rust + Slint، بلا Electron أو Tauri أو WebView أو متصفح مضمّن.
- **المهم بنظرة واحدة**: أدوات قابلة للسحب وتبقى فوق النوافذ لتعرض الأرقام المهمة دائماً.
- **محلي أولاً**: التخطيط والتفضيلات محفوظة على جهازك؛ لا حاجة إلى حساب أو API Key.
- **يختفي عند الحاجة**: استخدم <kbd>Alt</kbd> + <kbd>C</kbd> لإخفاء كل الأدوات أو إعادتها.
- **أربعة مصادر عامة**: Binance وCoinbase وOKX وHyperliquid.
- **مظهر مرن**: أنماط متعددة، ووضع فاتح أو داكن، وشفافية وألوان سوق قابلة للتخصيص.

> [!IMPORTANT]
> صُمم Crypto HUD لعرض بيانات السوق العامة فقط. لا ينفذ صفقات، ولا يتصل بالمحافظ،
> ولا يحتفظ بالأصول، ولا يطلب مفتاحاً خاصاً أو عبارة استرداد أو حساب منصة أو API Key.

## معاينة الأدوات

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="أداة Focus Ticker تعرض سعر BTC والتغير ومخططاً مصغراً">
  </picture>
</p>

اختر شريط أسعار صغيراً أو بطاقة بمخطط مفصل أو لوحة لعدة أسواق. تستخدم الأدوات
المضمنة والمخصصة عقد الإضافات نفسه.

## البدء السريع

صُمم Crypto HUD لنظام Windows. يستخدم المستودع `mise` لتثبيت Rust `1.96`،
ويوفر مهمة تشغيل محلية بأمر واحد.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

بعد التشغيل، اسحب الأدوات إلى مكانها، وافتح الإعدادات من شريط النظام، واختر
الأسواق والسمات واضبط الشفافية. تُحفظ المواضع والتفضيلات تلقائياً.

## التخصيص والإضافات

- اقرأ [دليل تطوير إضافات الواجهة](CUSTOM_UI_PLUGIN_DEVELOPMENT.md).
- راجع [عقد الإضافات والأمثلة المضمنة](crates/crypto-hud/plugins/README.md).
- أنشئ أداة السوق الخاصة بك باستخدام Slint.

## التطوير

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

نرحب بالمساهمات. راجع [دليل المساهمة](CONTRIBUTING.md) و[سجل التغييرات](CHANGELOG.md)
و[سياسة الأمان](SECURITY.md).

## خارطة الطريق

تشمل الأولويات حالات أوضح لمصادر البيانات، وتنبيهات السعر والتغير خلال 24 ساعة،
وإدارة أشمل للأدوات، ومواضع أفضل عند التشغيل الأول، ومثبتاً أكثر اكتمالاً.

## الترخيص

MIT © Crypto HUD Contributors
