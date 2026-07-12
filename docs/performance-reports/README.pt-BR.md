# Relatório de impacto do Crypto HUD no desempenho

[English](README.md) · [简体中文](README.zh-CN.md) ·
[繁體中文](README.zh-TW.md) · [Español](README.es.md) ·
[Português do Brasil](README.pt-BR.md) · [Tiếng Việt](README.vi.md) ·
[Bahasa Indonesia](README.id.md) · [Türkçe](README.tr.md) ·
[한국어](README.ko.md) · [日本語](README.ja.md) ·
[Русский](README.ru.md) · [العربية](README.ar.md)

> Data do teste: 2026-07-12<br>
> Produto: Crypto HUD 0.9.7<br>
> Commit: `b572ce82d2b1a7d95fa2c5ab6687b73b9ed76ae7`<br>
> SHA-256 do executável:
> `3F50145630A74FB6E7265AFE8935101BDB872C743DA6BC8267C49E0DD7BE267D`

## Resumo do relatório

Este relatório registra CPU, memória, threads, estrutura do processo,
inicialização, solicitações de rede e estabilidade do Crypto HUD nas condições
descritas.

Com três pares e atualização a cada cinco segundos, a execução de dez minutos
com cache aquecido registrou **0,070% de CPU média da máquina**, **20,27 MiB de
memória privada comprometida**, **47,96 MiB de conjunto de trabalho** e
**19,29 MiB de conjunto de trabalho privado**.

Com 20 pares ao vivo e o mesmo intervalo, foram registrados **0,125% de CPU
média**, **0,681% de CPU P95**, **21,56 MiB de memória privada** e **49,02 MiB
de conjunto de trabalho**.

Não foram observados processos filhos ou WebView2. A memória com o widget
oculto pela bandeja ficou próxima do cenário offline visível. Na execução de
dez minutos, a memória aumentou durante o aquecimento e a atualização de velas
de cinco minutos, mantendo uma faixa estável nos minutos 7–10.

Os números descrevem o processo Crypto HUD na máquina informada. Não incluem
GPU atribuída, composição DWM ou energia elétrica.

## Medições principais

| Medição | Resultado |
| --- | ---: |
| CPU média padrão | 0,070% |
| CPU P95 padrão | 0,189% |
| Mediana de memória privada | 20,27 MiB |
| Mediana do conjunto de trabalho | 47,96 MiB |
| Mediana do conjunto de trabalho privado | 19,29 MiB |
| Tempo até dados ao vivo prontos | 1,705 segundo |
| CPU média com 20 pares | 0,125% |
| CPU P95 com 20 pares | 0,681% |
| Maior amostra de CPU da aplicação | 0,916% |
| Processos filhos | 0 |
| Processos WebView2 | 0 |

## Ambiente de teste

| Item | Valor |
| --- | --- |
| Sistema operacional | Windows 11 Pro for Workstations, 10.0.26200, build 26200 |
| Virtualização | QEMU Standard PC (Q35 + ICH9, 2009) |
| Processador informado | AMD Ryzen 7 8845HS with Radeon 780M Graphics |
| Processadores lógicos | 12 |
| Memória | 23,98 GiB |
| Adaptadores de vídeo | OrayIddDriver Device; Red Hat QXL controller |
| Plano de energia | Equilibrado |
| Renderizador | Renderizador por software do Slint |
| Toolchain | Rust 1.96.0 |

O executável release foi iniciado de uma distribuição staged com plugins e
recursos. Cada execução usou estado isolado e ID de instância exclusivo.

## Definição das métricas

- **CPU da máquina:** tempo de CPU do processo dividido pelo tempo decorrido e
  por 12 processadores lógicos.
- **CPU P95:** 95% das amostras estáveis ficaram nesse valor ou abaixo.
- **Memória privada comprometida:** memória comprometida somente para o processo.
- **Conjunto de trabalho:** páginas residentes em RAM, inclusive compartilhadas.
- **Conjunto de trabalho privado:** RAM residente exclusiva do processo.
- **Threads:** total de threads por amostra; workers temporários alteram o valor.
- **Handles:** identificadores do Windows mantidos pelo processo.

O CPU foi calculado por diferenças de `TotalProcessorTime`. Foram excluídos os
primeiros 10 segundos; 30 segundos na execução de cinco minutos e o primeiro
minuto na execução de dez minutos.

Os valores offline são estimativas arredondadas reconstruídas do contador
equivalente a um núcleo após um problema de arredondamento no primeiro resumo.
Os valores com rede real não apresentam esse problema.

## Resultados por cenário

| Cenário | CPU média / P95 | Memória privada mediana | Working set mediano | Threads mediana / máx. |
| --- | ---: | ---: | ---: | ---: |
| Padrão ao vivo, 3 pares, 10 minutos | 0,070% / 0,189% | 20,27 MiB | 47,96 MiB | 9 / 13 |
| Configurações abertas e rede real | 0,071% / 0,233% | 23,74 MiB | 55,35 MiB | 14 / 17 |
| 20 pares ao vivo, atualização de 5 s | 0,125% / 0,681% | 21,56 MiB | 49,02 MiB | 14 / 18 |
| Proxy local recusando imediatamente | 0,049% / 0,232% | 18,66 MiB | 43,67 MiB | 14 / 17 |
| Um widget visível, dados offline | aprox. 0,04% / 0,25% | 17,43 MiB | 41,93 MiB | 12 / 12 |
| O mesmo widget oculto pela bandeja | aprox. 0,03% / 0,25% | 17,43 MiB | 41,77 MiB | 12 / 12 |
| 10 janelas Quote Board, offline | aprox. 0,14% / 0,50% | 20,13 MiB | 45,48 MiB | 12 / 12 |
| Uma instância de cada um dos cinco widgets | aprox. 0,17% / 0,50% | 26,63 MiB | 53,37 MiB | 12 / 12 |
| Um Quote Board com 20 pares offline | aprox. 0,09% / 0,42% | 17,87 MiB | 42,78 MiB | 12 / 12 |
| Um widget em escala de 300%, offline | aprox. 0,08% / 0,50% | 18,71 MiB | 44,30 MiB | 12 / 12 |

A maior amostra individual foi **0,916%**, no cenário com 20 pares.

## Diferenças observadas

- Configurações abertas registraram 23,74 MiB privados e 55,35 MiB de working
  set; o cenário padrão registrou 20,27 MiB e 47,96 MiB.
- De 3 para 20 pares, a CPU média mudou de 0,070% para 0,125%, a memória privada
  de 20,27 MiB para 21,56 MiB e o working set de 47,96 MiB para 49,02 MiB.
- Dez Quote Board offline registraram 2,70 MiB privados e 3,55 MiB de working
  set a mais que um widget offline.
- Os cinco tipos de widget registraram 9,20 MiB privados e 11,44 MiB de working
  set a mais que um widget.
- A escala 300% registrou 1,28 MiB privados e 2,37 MiB de working set a mais.
- Antes e depois de ocultar, a memória privada foi 17,43 MiB; o working set
  mudou de 41,93 MiB para 41,77 MiB. As estimativas de CPU foram 0,04% e 0,03%.

## Inicialização

O marcador de todos os dados de mercado prontos foi alcançado em **1,705
segundo**. Ele exige dados para as linhas configuradas e inclui descoberta de
plugins, janelas, solicitações iniciais e o timer de um segundo. Não representa
o primeiro quadro nem a primeira janela visível.

A verificação de atualização seguiu o caminho de erro porque a API do GitHub
Releases falhou no ambiente. Uma resposta bem-sucedida não foi medida.

## Linha do tempo de memória

| Minuto | Memória privada mediana | Working set mediano |
| ---: | ---: | ---: |
| 1 | 19,05 MiB | 46,46 MiB |
| 2 | 18,95 MiB | 46,52 MiB |
| 3 | 19,16 MiB | 46,74 MiB |
| 4 | 19,42 MiB | 47,03 MiB |
| 5 | 19,72 MiB | 47,27 MiB |
| 6 | 20,33 MiB | 47,96 MiB |
| 7 | 20,36 MiB | 48,02 MiB |
| 8 | 20,36 MiB | 48,05 MiB |
| 9 | 20,27 MiB | 48,02 MiB |
| 10 | 20,27 MiB | 48,02 MiB |

A memória privada aumentou até o minuto 6. As medianas dos minutos 7–10 ficaram
entre 20,27 e 20,36 MiB. O minuto 6 coincidiu com a região de atualização de
velas de cinco minutos.

Houve uma falha transitória de SOL. A aplicação continuou e saiu com código 0.

## Observações de rede e arquivos

`pares únicos × (60 / segundos de atualização + 0,2)` solicitações/minuto.

| Configuração | Solicitações aproximadas/minuto |
| --- | ---: |
| 3 pares, 5 s | 36,6 |
| 8 pares, 5 s | 97,6 |
| 20 pares, 5 s | 244 |
| 3 pares, 60 s | 3,6 |
| 20 pares, 60 s | 24 |

O termo `0,2` representa uma atualização de velas a cada cinco minutos.
O início adiciona aproximadamente uma solicitação de ticker e uma de velas por
par. A latência pode reduzir a frequência observada.

O proxy recusado produziu quatro ciclos com falha em 90 segundos. Não foram
medidos timeout de oito segundos nem reconexão.

Após o aquecimento, o I/O de arquivos em repouso foi praticamente zero. Não foi
medida a latência em disco lento. Os contadores de I/O não foram usados como
bytes de rede atribuídos.

## Estrutura do processo

- Processos filhos: 0.
- Processos WebView2: 0.
- Threads padrão em dez minutos: mediana 9, máximo 13.
- Threads com 20 pares: mediana 14, máximo 18.
- Handles padrão: mediana 301, máximo 322.

## Escopo e limitações

- QEMU e vídeo remoto não quantificam GPU física, DWM nem energia.
- Cada cenário personalizado foi executado uma vez; não há intervalos de
  confiança.
- A execução mais longa teve dez minutos; não houve teste de 8–24 horas.
- Não foram medidos DPI do sistema de 150–300%, vários monitores 4K, suspensão,
  remoção de tela ou animação contínua mínima.
- Não foram medidos DNS lento, HTTP 429, timeout de oito segundos ou recuperação.
- WPR/WPA não estava disponível; não há bytes de rede atribuídos, GPU, trocas de
  contexto, wakeups independentes nem consumo elétrico.
- Não foi medido salvamento em disco lento ou sincronizado.
- Os resultados correspondem à versão e ao hash indicados no início.

## Registro de validação

- Build release:
  `cargo +1.96.0 build --release --locked -p crypto-hud`
- Verificação staged com plugins e recursos.
- Sete cenários offline e sete com rede real.
- Uma execução de dez minutos.
- CPU, três métricas de memória, threads, handles, I/O, início, processos e erros.
- Verificações de dados, navegação, links, UTF-8 e isolamento bidirecional.

O comportamento do produto não foi alterado durante as medições.
