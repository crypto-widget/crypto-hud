<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Logotipo de Crypto HUD">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>Tu mercado, siempre a la vista.</strong><br>
  Widgets nativos de criptomonedas en tu escritorio de Windows, sin quitarte el enfoque.
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
  <img alt="Plataforma: Windows" src="https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Creado con Rust" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="Interfaz nativa con Slint" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="Licencia MIT" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="Widget Market Compass con el precio de BTC, velas y otros mercados">
  </picture>
</p>

<p align="center"><sub>Precios en vivo al borde de tu espacio de trabajo. Sin pestañas del exchange, sin billeteras y sin ruido.</sub></p>

---

Crypto HUD es una pantalla de mercado ligera y local para quienes quieren seguir
algunas monedas sin vivir dentro de una terminal de trading. Coloca un widget
donde te resulte cómodo, sigue trabajando y mira el mercado solo cuando importe.

## Diseñado para permanecer en segundo plano

- **Nativo y ligero**: Rust + Slint, sin Electron, Tauri, WebView ni navegador integrado.
- **De un vistazo**: widgets movibles y siempre visibles mantienen los datos importantes a la vista.
- **Local primero**: el diseño y las preferencias se guardan en tu equipo; no requiere cuenta ni API Key.
- **Silencio cuando quieras**: oculta o restaura todos los widgets con <kbd>Alt</kbd> + <kbd>C</kbd>.
- **Cuatro fuentes públicas**: Binance, Coinbase, OKX y Hyperliquid.
- **Apariencia flexible**: varios estilos, temas claro y oscuro, opacidad y colores de mercado configurables.

> [!IMPORTANT]
> Crypto HUD está diseñado solo para consultar información pública. No ejecuta
> operaciones, no conecta billeteras ni custodia fondos, y nunca solicita claves
> privadas, frases semilla, cuentas de exchange o API Keys.

## Vista previa de los widgets

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="Focus Ticker con precio de BTC, variación y minigráfico">
  </picture>
</p>

Elige un ticker compacto, una tarjeta con gráfico o un panel de varios mercados.
Los widgets incluidos usan el mismo contrato de plugins que los personalizados.

## Inicio rápido

Crypto HUD está creado para Windows. El repositorio usa `mise` para fijar Rust
`1.96` e incluye una tarea de inicio local con un solo comando.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

Después de iniciarlo, arrastra los widgets, abre la configuración desde la bandeja,
selecciona mercados, cambia el tema y ajusta la opacidad. Las posiciones y
preferencias se guardan automáticamente.

## Personalización y plugins

- Lee la [guía para crear plugins de interfaz](CUSTOM_UI_PLUGIN_DEVELOPMENT.md).
- Consulta el [contrato de plugins y los ejemplos incluidos](crates/crypto-hud/plugins/README.md).
- Crea tu propio widget de mercado con Slint.

## Desarrollo

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

Las contribuciones son bienvenidas. Consulta la [guía de contribución](CONTRIBUTING.md),
el [registro de cambios](CHANGELOG.md) y la [política de seguridad](SECURITY.md).

## Hoja de ruta

Las prioridades incluyen mejores estados de salud de los proveedores, alertas de
precio y cambio de 24 horas, una gestión de widgets más completa, una mejor
ubicación inicial y un instalador más completo.

## Licencia

MIT © Crypto HUD Contributors
