# Informe del impacto de Crypto HUD en el rendimiento

[English](README.md) · [简体中文](README.zh-CN.md) ·
[繁體中文](README.zh-TW.md) · [Español](README.es.md) ·
[Português do Brasil](README.pt-BR.md) · [Tiếng Việt](README.vi.md) ·
[Bahasa Indonesia](README.id.md) · [Türkçe](README.tr.md) ·
[한국어](README.ko.md) · [日本語](README.ja.md) ·
[Русский](README.ru.md) · [العربية](README.ar.md)

> Fecha de prueba: 2026-07-12<br>
> Producto: Crypto HUD 0.9.7<br>
> Commit: `b572ce82d2b1a7d95fa2c5ab6687b73b9ed76ae7`<br>
> SHA-256 del ejecutable:
> `3F50145630A74FB6E7265AFE8935101BDB872C743DA6BC8267C49E0DD7BE267D`

## Resumen del informe

Este informe registra las mediciones de CPU, memoria, hilos, estructura del
proceso, inicio, solicitudes de red y estabilidad de Crypto HUD bajo las
condiciones indicadas.

Con tres pares y actualización cada cinco segundos, la ejecución de diez
minutos con caché caliente registró **0,070% de CPU promedio de toda la
máquina**, **20,27 MiB de memoria privada comprometida**, **47,96 MiB de
conjunto de trabajo** y **19,29 MiB de conjunto de trabajo privado**.

Con 20 pares en vivo y el mismo intervalo se registraron **0,125% de CPU
promedio**, **0,681% de CPU P95**, **21,56 MiB de memoria privada** y
**49,02 MiB de conjunto de trabajo**.

No se observaron procesos secundarios ni procesos WebView2. La memoria del
proceso al ocultar el widget desde la bandeja fue casi igual a la del escenario
offline visible. En la prueba de diez minutos, la memoria aumentó durante el
calentamiento y la actualización de velas de cinco minutos, y permaneció en un
intervalo estable durante los minutos 7–10.

Las cifras describen el proceso Crypto HUD en la máquina indicada. No incluyen
uso de GPU atribuido, composición DWM ni consumo eléctrico.

## Mediciones principales

| Medición | Resultado |
| --- | ---: |
| CPU promedio predeterminado | 0,070% |
| CPU P95 predeterminado | 0,189% |
| Mediana de memoria privada | 20,27 MiB |
| Mediana del conjunto de trabajo | 47,96 MiB |
| Mediana del conjunto de trabajo privado | 19,29 MiB |
| Tiempo hasta datos en vivo listos | 1,705 segundos |
| CPU promedio con 20 pares | 0,125% |
| CPU P95 con 20 pares | 0,681% |
| Muestra máxima de CPU de la aplicación | 0,916% |
| Procesos secundarios | 0 |
| Procesos WebView2 | 0 |

## Entorno de prueba

| Elemento | Valor |
| --- | --- |
| Sistema operativo | Windows 11 Pro for Workstations, 10.0.26200, build 26200 |
| Virtualización | QEMU Standard PC (Q35 + ICH9, 2009) |
| Procesador informado | AMD Ryzen 7 8845HS with Radeon 780M Graphics |
| Procesadores lógicos | 12 |
| Memoria del sistema | 23,98 GiB |
| Adaptadores de pantalla | OrayIddDriver Device; Red Hat QXL controller |
| Plan de energía | Equilibrado |
| Renderizador | Renderizador de software de Slint |
| Toolchain | Rust 1.96.0 |

El ejecutable se compiló en release y se inició desde una distribución staged
con plugins y recursos incluidos. Cada ejecución usó un directorio de estado
aislado y un identificador de instancia único.

## Definición de las métricas

- **CPU de toda la máquina:** tiempo de CPU del proceso dividido entre el tiempo
  transcurrido y 12 procesadores lógicos.
- **CPU P95:** el 95% de las muestras estables fue igual o inferior a este valor.
- **Memoria privada comprometida:** memoria comprometida exclusivamente para el
  proceso.
- **Conjunto de trabajo:** páginas residentes en RAM; puede incluir páginas
  compartidas.
- **Conjunto de trabajo privado:** RAM residente exclusiva del proceso.
- **Hilos:** total de hilos en cada muestra; los trabajadores de red temporales
  hacen variar el valor.
- **Handles:** identificadores de Windows propiedad del proceso.

El CPU se calculó con diferencias de `TotalProcessorTime`. Se excluyeron los
primeros 10 segundos; 30 segundos en la prueba de cinco minutos y el primer
minuto en la prueba de diez minutos.

Los valores de CPU offline son estimaciones redondeadas reconstruidas desde el
contador equivalente a un núcleo tras detectar redondeo entero en el primer
resumen. Los valores con red real no presentan ese problema.

## Resultados por escenario

| Escenario | CPU promedio / P95 | Memoria privada mediana | Working set mediano | Hilos mediana / máx. |
| --- | ---: | ---: | ---: | ---: |
| Predeterminado en vivo, 3 pares, 10 minutos | 0,070% / 0,189% | 20,27 MiB | 47,96 MiB | 9 / 13 |
| Configuración abierta y red real | 0,071% / 0,233% | 23,74 MiB | 55,35 MiB | 14 / 17 |
| 20 pares en vivo, actualización de 5 s | 0,125% / 0,681% | 21,56 MiB | 49,02 MiB | 14 / 18 |
| Proxy local con rechazo inmediato | 0,049% / 0,232% | 18,66 MiB | 43,67 MiB | 14 / 17 |
| Un widget visible, datos offline | aprox. 0,04% / 0,25% | 17,43 MiB | 41,93 MiB | 12 / 12 |
| El mismo widget oculto desde la bandeja | aprox. 0,03% / 0,25% | 17,43 MiB | 41,77 MiB | 12 / 12 |
| 10 ventanas Quote Board, offline | aprox. 0,14% / 0,50% | 20,13 MiB | 45,48 MiB | 12 / 12 |
| Una instancia de cada uno de los cinco widgets | aprox. 0,17% / 0,50% | 26,63 MiB | 53,37 MiB | 12 / 12 |
| Un Quote Board con 20 pares offline | aprox. 0,09% / 0,42% | 17,87 MiB | 42,78 MiB | 12 / 12 |
| Un widget a escala 300%, offline | aprox. 0,08% / 0,50% | 18,71 MiB | 44,30 MiB | 12 / 12 |

La muestra individual máxima de CPU fue **0,916%**, en el escenario de 20 pares.

## Diferencias observadas

- Configuración abierta registró 23,74 MiB privados y 55,35 MiB de working set;
  el escenario predeterminado de diez minutos registró 20,27 MiB y 47,96 MiB.
- De 3 a 20 pares, el CPU promedio cambió de 0,070% a 0,125%, la memoria privada
  de 20,27 MiB a 21,56 MiB y el working set de 47,96 MiB a 49,02 MiB.
- Diez Quote Board offline registraron 2,70 MiB privados y 3,55 MiB de working
  set más que un widget visible offline.
- Los cinco tipos de widget registraron 9,20 MiB privados y 11,44 MiB de working
  set más que el escenario offline de un widget.
- La escala 300% registró 1,28 MiB privados y 2,37 MiB de working set más que la
  escala predeterminada offline.
- Antes y después de ocultar, la memoria privada fue 17,43 MiB; el working set
  cambió de 41,93 MiB a 41,77 MiB. Las estimaciones de CPU fueron 0,04% y 0,03%.

## Inicio

La marca existente de todos los datos de mercado listos se alcanzó en
**1,705 segundos**. Requiere datos para las filas configuradas e incluye
descubrimiento de plugins, creación de ventanas, solicitudes iniciales y el
temporizador de un segundo que consume eventos. No representa el primer
fotograma ni la primera ventana visible.

La prueba de actualización siguió la ruta de error porque GitHub Releases API
falló en este entorno. No se midió una respuesta exitosa.

## Cronología de memoria de diez minutos

| Minuto | Memoria privada mediana | Working set mediano |
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

La memoria privada aumentó hasta el minuto 6. Las medianas de los minutos 7–10
permanecieron entre 20,27 y 20,36 MiB. El cambio del minuto 6 coincidió con la
zona de actualización de velas de cinco minutos.

Se registró un fallo transitorio de SOL. La aplicación continuó y terminó con
código 0.

## Observaciones de red y archivos

`pares únicos × (60 / segundos de actualización + 0,2)` solicitudes/minuto.

| Configuración | Solicitudes aproximadas/minuto |
| --- | ---: |
| 3 pares, 5 s | 36,6 |
| 8 pares, 5 s | 97,6 |
| 20 pares, 5 s | 244 |
| 3 pares, 60 s | 3,6 |
| 20 pares, 60 s | 24 |

El término `0,2` representa una actualización de velas cada cinco minutos.
El inicio agrega aproximadamente una solicitud de ticker y una de velas por
par. La latencia puede reducir la frecuencia real.

El proxy rechazado produjo cuatro ciclos fallidos en 90 segundos. No se midió
un timeout de ocho segundos ni la reconexión posterior.

Después del calentamiento, el I/O de archivos en reposo fue prácticamente cero.
No se midió la latencia de guardado en un disco lento. Los contadores de I/O no
se usaron como bytes de red atribuidos.

## Estructura del proceso

- Procesos secundarios: 0.
- Procesos WebView2: 0.
- Hilos predeterminados en diez minutos: mediana 9, máximo 13.
- Hilos con 20 pares: mediana 14, máximo 18.
- Handles predeterminados: mediana 301, máximo 322.

## Alcance y límites

- QEMU y la pantalla remota no cuantifican GPU física, DWM ni energía.
- Cada escenario personalizado se ejecutó una vez; no hay intervalos de
  confianza de múltiples ejecuciones.
- La ejecución más larga fue de diez minutos; no hubo prueba de 8–24 horas.
- No se midieron DPI del sistema de 150–300%, varios monitores 4K, suspensión,
  retirada de pantalla ni animación continua mínima.
- No se midieron DNS lento, HTTP 429, timeout de ocho segundos o recuperación.
- WPR/WPA no estaban disponibles; no hay bytes de red atribuidos, GPU, cambios
  de contexto, despertares independientes ni energía.
- No se midió guardado en discos lentos o sincronizados.
- Los resultados corresponden a la versión y hash indicados al inicio.

## Registro de validación

- Build release:
  `cargo +1.96.0 build --release --locked -p crypto-hud`
- Comprobación staged con plugins y recursos.
- Siete escenarios offline y siete con red real.
- Una ejecución de diez minutos.
- CPU, tres medidas de memoria, hilos, handles, I/O, inicio, procesos y errores.
- Comprobación de datos, navegación, enlaces, UTF-8 y aislamiento bidireccional.

No se cambió el comportamiento del producto durante las mediciones.
