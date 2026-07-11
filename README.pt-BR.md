<p align="center">
  <img src="crates/crypto-hud/ui/icon.png" width="88" alt="Logotipo do Crypto HUD">
</p>

<h1 align="center">Crypto HUD</h1>

<p align="center">
  <strong>Seu mercado, sempre ao alcance dos olhos.</strong><br>
  Widgets nativos de cripto na área de trabalho do Windows, sem tirar o seu foco.
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
  <img alt="Feito com Rust" src="https://img.shields.io/badge/built_with-Rust-dea584?style=flat-square&logo=rust&logoColor=white">
  <img alt="Interface nativa com Slint" src="https://img.shields.io/badge/UI-native_Slint-2379f4?style=flat-square">
  <img alt="Licença MIT" src="https://img.shields.io/badge/license-MIT-22c55e?style=flat-square">
</p>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.market-compass/ui/preview-dark.png" width="500" alt="Widget Market Compass com preço do BTC, candles e outros mercados">
  </picture>
</p>

<p align="center"><sub>Preços ao vivo na borda do seu espaço de trabalho. Sem aba da corretora, sem carteira e sem ruído.</sub></p>

---

Crypto HUD é um painel de mercado leve e local para quem quer acompanhar algumas
moedas sem viver dentro de um terminal de negociação. Posicione um widget onde
for mais confortável, continue trabalhando e olhe o mercado apenas quando precisar.

## Feito para ficar em segundo plano

- **Nativo e leve**: Rust + Slint, sem Electron, Tauri, WebView ou navegador integrado.
- **Informação em um olhar**: widgets móveis e sempre visíveis mantêm os números importantes por perto.
- **Local primeiro**: layout e preferências ficam no seu computador; sem conta ou API Key.
- **Silencioso quando quiser**: oculte ou restaure tudo com <kbd>Alt</kbd> + <kbd>C</kbd>.
- **Quatro fontes públicas**: Binance, Coinbase, OKX e Hyperliquid.
- **Visual flexível**: vários estilos, temas claro e escuro, opacidade e cores configuráveis.

> [!IMPORTANT]
> Crypto HUD serve apenas para consultar dados públicos. Ele não realiza ordens,
> não conecta carteiras, não guarda fundos e nunca pede chaves privadas, frases
> de recuperação, contas de corretora ou API Keys.

## Prévia dos widgets

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-light.png">
    <img src="crates/crypto-hud/plugins/com.cryptohud.focus-ticker/ui/preview-dark.png" width="820" alt="Focus Ticker com preço do BTC, variação e minigráfico">
  </picture>
</p>

Escolha um ticker compacto, um cartão com gráfico ou um painel de vários mercados.
Os widgets incluídos usam o mesmo contrato de plugins disponível para widgets personalizados.

## Início rápido

Crypto HUD foi criado para Windows. O repositório usa `mise` para fixar o Rust
`1.96` e oferece uma tarefa local de inicialização com um único comando.

```powershell
git clone https://github.com/crypto-widget/crypto-hud.git
cd crypto-hud
mise trust
mise install
mise run run-app
```

Depois de iniciar, arraste os widgets, abra as configurações pela bandeja,
selecione mercados, troque o tema e ajuste a opacidade. Posições e preferências
são salvas automaticamente.

## Personalização e plugins

- Leia o [guia de desenvolvimento de plugins de interface](CUSTOM_UI_PLUGIN_DEVELOPMENT.md).
- Veja o [contrato de plugins e os exemplos incluídos](crates/crypto-hud/plugins/README.md).
- Crie seu próprio widget de mercado com Slint.

## Desenvolvimento

```powershell
mise run format-check
mise run check
mise run test
mise run run-app
```

Contribuições são bem-vindas. Consulte o [guia de contribuição](CONTRIBUTING.md),
o [histórico de mudanças](CHANGELOG.md) e a [política de segurança](SECURITY.md).

## Próximos passos

As prioridades incluem melhores estados de saúde dos provedores, alertas de preço
e variação de 24 horas, gerenciamento mais completo de widgets, melhor posição
inicial e um instalador mais completo.

## Licença

MIT © Crypto HUD Contributors
