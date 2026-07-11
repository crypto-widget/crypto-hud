<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Crypto HUD logosu">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>Piyasa her zaman gözünüzün önünde.</strong><br>
  Odağınızı bozmadan Windows masaüstünde duran yerel kripto widget'ları.
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
  <img alt="Rust ile geliştirildi" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="Yerel Slint arayüzü" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="MIT lisansı" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="BTC fiyatını, mum grafiğini ve diğer piyasaları gösteren Market Compass widget'ı">
  </picture>
</p>

<p align="center"><sub>Canlı fiyatlar çalışma alanınızın kenarında. Borsa sekmesi yok, cüzdan bağlantısı yok, gürültü yok.</sub></p>

---

Crypto HUD, birkaç coini takip etmek isteyen ancak tüm gününü işlem terminalinde
geçirmek istemeyenler için hafif ve yerel öncelikli bir piyasa ekranıdır. Widget'ı
uygun bir yere koyun, işinize devam edin ve yalnızca gerektiğinde göz atın.

## Arka planda sessizce çalışmak için tasarlandı

- **Yerel ve hafif**: Rust + Slint; Electron, Tauri, WebView veya gömülü tarayıcı yok.
- **Tek bakışta bilgi**: taşınabilir ve her zaman üstte kalan widget'lar önemli rakamları görünür tutar.
- **Önce yerel**: düzen ve tercihler bilgisayarınızda kalır; hesap veya API Key gerekmez.
- **İstediğinizde gizleyin**: tüm widget'ları <kbd>Alt</kbd> + <kbd>C</kbd> ile gizleyin veya geri getirin.
- **Dört genel veri kaynağı**: Binance, Coinbase, OKX ve Hyperliquid.
- **Esnek görünüm**: farklı stiller, açık/koyu temalar, saydamlık ve ayarlanabilir piyasa renkleri.

> [!IMPORTANT]
> Crypto HUD yalnızca genel piyasa verilerini görüntüler. İşlem yapmaz, cüzdan
> bağlamaz, varlık saklamaz; özel anahtar, kurtarma ifadesi, borsa hesabı veya
> API Key istemez.

## Widget önizlemesi

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="BTC fiyatını, değişimi ve mini grafiği gösteren Focus Ticker">
  </picture>
</p>

Kompakt bir ticker, ayrıntılı grafik kartı veya çoklu piyasa panosu seçin. Dahili
ve özel widget'lar aynı eklenti sözleşmesini kullanır.

## Hızlı başlangıç

Crypto HUD Windows için geliştirilmiştir. Depo, Rust `1.96` sürümünü sabitlemek
için `mise` kullanır ve tek komutlu yerel başlatma görevi sunar.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

Başlattıktan sonra widget'ları taşıyın, sistem tepsisinden ayarları açın, piyasaları
seçin, temayı değiştirin ve saydamlığı ayarlayın. Konumlar ve tercihler otomatik kaydedilir.

## Özelleştirme ve eklentiler

- [Özel arayüz eklentisi geliştirme kılavuzunu](CUSTOM_UI_PLUGIN_DEVELOPMENT.md) okuyun.
- [Eklenti sözleşmesini ve dahili örnekleri](crates/crypto-hud/plugins/README.md) inceleyin.
- Slint ile kendi piyasa widget'ınızı oluşturun.

## Geliştirme

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

Katkılarınızı bekliyoruz. [Katkı kılavuzuna](CONTRIBUTING.md),
[değişiklik günlüğüne](CHANGELOG.md) ve [güvenlik politikasına](SECURITY.md) bakın.

## Yol haritası

Öncelikler arasında daha net sağlayıcı durumları, fiyat ve 24 saatlik değişim
uyarıları, daha kapsamlı widget yönetimi, daha iyi ilk yerleşim ve daha eksiksiz
bir yükleyici bulunuyor.

## Lisans

MIT © Crypto HUD Contributors
