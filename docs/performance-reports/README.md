# Crypto HUD performance impact report

[English](README.md) · [简体中文](README.zh-CN.md) ·
[繁體中文](README.zh-TW.md) · [Español](README.es.md) ·
[Português do Brasil](README.pt-BR.md) · [Tiếng Việt](README.vi.md) ·
[Bahasa Indonesia](README.id.md) · [Türkçe](README.tr.md) ·
[한국어](README.ko.md) · [日本語](README.ja.md) ·
[Русский](README.ru.md) · [العربية](README.ar.md)

> Test date: 2026-07-12<br>
> Product: Crypto HUD 0.9.7<br>
> Commit: `b572ce82d2b1a7d95fa2c5ab6687b73b9ed76ae7`<br>
> Executable SHA-256:
> `3F50145630A74FB6E7265AFE8935101BDB872C743DA6BC8267C49E0DD7BE267D`

## Report summary

This report records the CPU, memory, thread, process, startup, network-request,
and stability measurements collected from Crypto HUD under the test conditions
listed below.

With the default configuration of three market pairs and a five-second refresh
interval, the ten-minute warm-cache run recorded **0.070% average total-machine
CPU**, **20.27 MiB median private committed memory**, **47.96 MiB median working
set**, and **19.29 MiB median private working set**.

With 20 live market pairs and the same five-second interval, the run recorded
**0.125% average CPU**, **0.681% CPU P95**, **21.56 MiB median private committed
memory**, and **49.02 MiB median working set**.

No measured scenario created a child process or a WebView2 process. Tray-hide
produced nearly the same process memory as the visible offline scenario. The
ten-minute run showed a memory increase during warmup and the five-minute
candle refresh, followed by a stable range during minutes 7–10.

The values in this report describe the Crypto HUD process on the stated test
machine. They do not include attributed GPU use, DWM composition, or electrical
power.

## Key measurements

| Measurement | Result |
| --- | ---: |
| Default average CPU | 0.070% |
| Default CPU P95 | 0.189% |
| Default median private commit | 20.27 MiB |
| Default median working set | 47.96 MiB |
| Default median private working set | 19.29 MiB |
| Live market-data-ready time | 1.705 seconds |
| 20-pair average CPU | 0.125% |
| 20-pair CPU P95 | 0.681% |
| Highest observed application CPU sample | 0.916% |
| Child processes | 0 |
| WebView2 processes | 0 |

## Test environment

| Item | Value |
| --- | --- |
| Operating system | Windows 11 Pro for Workstations, 10.0.26200, build 26200 |
| Virtualization | QEMU Standard PC (Q35 + ICH9, 2009) |
| Reported processor | AMD Ryzen 7 8845HS with Radeon 780M Graphics |
| Logical processors available | 12 |
| System memory | 23.98 GiB |
| Display adapters | OrayIddDriver Device; Red Hat QXL controller |
| Power plan | Balanced |
| Application renderer | Slint software renderer |
| Rust toolchain | Rust 1.96.0 |

The executable was built in release mode and launched from a staged release
layout containing the bundled plugins and resources. Each run used an isolated
state directory and a unique single-instance identifier.

## Metric definitions

- **Total-machine CPU:** process CPU time divided by elapsed time and 12 logical
  processors. This follows the overall-machine percentage convention used by
  Windows Task Manager.
- **CPU P95:** 95% of steady-state CPU samples were at or below this value.
- **Private commit:** committed memory assigned exclusively to the process.
- **Working set:** memory pages currently resident in RAM; shared pages may be
  included.
- **Private working set:** resident RAM private to the process. This is closest
  to the per-process Memory value commonly displayed by Task Manager.
- **Thread count:** the number of process threads at each sample. Temporary
  network workers make this value change during a run.
- **Handle count:** the number of Windows handles owned by the process.

CPU was calculated from `TotalProcessorTime` deltas. Samples from the first
10 seconds were excluded from normal steady-state scenarios. The five-minute
run excluded the first 30 seconds, and the ten-minute run excluded its first
minute.

Offline CPU values are rounded estimates reconstructed from the core-equivalent
counter after an integer-rounding issue was identified in the first raw
summary. These values show scenario-scale differences; the live-network CPU
values were calculated without that rounding issue.

## Scenario results

CPU values are percentages of the complete 12-logical-processor test machine.

| Scenario | CPU average / P95 | Median private commit | Median working set | Threads median / max |
| --- | ---: | ---: | ---: | ---: |
| Default live, 3 pairs, 10-minute warm run | 0.070% / 0.189% | 20.27 MiB | 47.96 MiB | 9 / 13 |
| Settings window open with live network | 0.071% / 0.233% | 23.74 MiB | 55.35 MiB | 14 / 17 |
| 20 live pairs, five-second refresh | 0.125% / 0.681% | 21.56 MiB | 49.02 MiB | 14 / 18 |
| Local proxy refusing connections immediately | 0.049% / 0.232% | 18.66 MiB | 43.67 MiB | 14 / 17 |
| One visible widget, deterministic offline data | about 0.04% / 0.25% | 17.43 MiB | 41.93 MiB | 12 / 12 |
| Same widget parked by tray-hide | about 0.03% / 0.25% | 17.43 MiB | 41.77 MiB | 12 / 12 |
| 10 Quote Board windows, offline | about 0.14% / 0.50% | 20.13 MiB | 45.48 MiB | 12 / 12 |
| One instance of each of the five widget types | about 0.17% / 0.50% | 26.63 MiB | 53.37 MiB | 12 / 12 |
| One Quote Board with 20 offline pairs | about 0.09% / 0.42% | 17.87 MiB | 42.78 MiB | 12 / 12 |
| One widget at 300% scale, offline | about 0.08% / 0.50% | 18.71 MiB | 44.30 MiB | 12 / 12 |

The highest individual application CPU sample was **0.916%** in the live
20-pair scenario.

## Differences observed between scenarios

- Opening Settings increased median private commit from 20.27 MiB in the
  ten-minute default run to 23.74 MiB, and median working set from 47.96 MiB to
  55.35 MiB. The scenarios had different durations, so these are observed
  values rather than a controlled allocation-only subtraction.
- Increasing live pairs from 3 to 20 changed average CPU from 0.070% to 0.125%,
  median private commit from 20.27 MiB to 21.56 MiB, and median working set from
  47.96 MiB to 49.02 MiB.
- In offline runs, 10 Quote Board windows used 2.70 MiB more private commit and
  3.55 MiB more working set than one visible widget.
- One instance of each of the five widget types used 9.20 MiB more private
  commit and 11.44 MiB more working set than the one-widget offline scenario.
- A 300% widget scale used 1.28 MiB more private commit and 2.37 MiB more
  working set than the default-scale offline scenario.
- Tray-hide changed private commit from 17.43 MiB to 17.43 MiB and working set
  from 41.93 MiB to 41.77 MiB. The rounded offline CPU estimates were 0.04%
  visible and 0.03% hidden.

## Startup measurement

The application reached the existing all-market-data-ready marker in **1.705
seconds** during the live startup scenario.

This marker requires the configured market rows to have data. Its time includes
plugin discovery, window creation, initial market requests, and the
once-per-second UI timer that consumes market events. It is not a measurement
of first frame or first visible window.

The production-style update-check scenario followed the error path because the
GitHub Releases API request failed in the test environment. The process
continued running; a successful update response was not measured.

## Ten-minute memory timeline

| Minute | Median private commit | Median working set |
| ---: | ---: | ---: |
| 1 | 19.05 MiB | 46.46 MiB |
| 2 | 18.95 MiB | 46.52 MiB |
| 3 | 19.16 MiB | 46.74 MiB |
| 4 | 19.42 MiB | 47.03 MiB |
| 5 | 19.72 MiB | 47.27 MiB |
| 6 | 20.33 MiB | 47.96 MiB |
| 7 | 20.36 MiB | 48.02 MiB |
| 8 | 20.36 MiB | 48.05 MiB |
| 9 | 20.27 MiB | 48.02 MiB |
| 10 | 20.27 MiB | 48.02 MiB |

Private commit rose through minute 6. It then remained between 20.27 MiB and
20.36 MiB by minute median for minutes 7–10. The increase near minute 6
coincided with the five-minute candle-refresh region.

The ten-minute run recorded one transient SOL request failure. The application
continued processing and exited with code 0.

## Network and file observations

The approximate steady request count is:

`unique pairs × (60 / refresh seconds + 0.2)` requests per minute.

The `0.2` term represents one candle refresh every five minutes.

| Configuration | Approximate requests per minute |
| --- | ---: |
| 3 pairs, 5-second refresh | 36.6 |
| 8 pairs, 5-second refresh | 97.6 |
| 20 pairs, 5-second refresh | 244 |
| 3 pairs, 60-second refresh | 3.6 |
| 20 pairs, 60-second refresh | 24 |

Startup adds approximately one ticker request and one candle request per pair.
Network latency extends a cycle, so observed request rates may be below the
formula.

The refused local proxy produced four failed market cycles during 90 seconds.
This scenario covered immediate connection refusal. It did not cover an
eight-second read timeout or reconnection after an outage.

After startup and warmup, measured file read and write activity was effectively
zero in the idle scenarios. Dragging and settings-save latency on a slow disk
was not measured. Windows process I/O counters were not used as attributed
network-byte counters.

## Process structure

- Child processes observed: 0.
- WebView2 processes observed: 0.
- Default live thread count: median 9, maximum 13 in the ten-minute run.
- Live 20-pair thread count: median 14, maximum 18.
- Default live handle count in the ten-minute run: median 301, maximum 322.

## Test scope and limitations

- The test machine used QEMU and a remote virtual display stack. The results do
  not quantify physical GPU utilization, DWM composition cost, or battery
  power.
- Each custom scenario was run once; the report does not contain multi-run
  confidence intervals.
- The longest run was ten minutes. An 8–24 hour run was not performed.
- System DPI at 150–300%, 4K multi-monitor layouts, display removal,
  sleep/resume, and minimum-interval continuous animation were not measured.
- The network-failure case used immediate proxy refusal. DNS stalls, slow
  responses, HTTP 429, an eight-second timeout, and network recovery were not
  measured.
- Windows Performance Recorder and WPA attribution were unavailable. The
  report does not provide process-attributed network bytes, GPU utilization,
  context-switch counts, independent wakeup counts, or electrical power.
- State-save latency during dragging and settings changes was not measured on a
  slow or synchronized disk.
- The results apply to version 0.9.7 and the executable hash shown at the top of
  this report.

## Validation record

- Release build:
  `cargo +1.96.0 build --release --locked -p crypto-hud`
- Staged release-process check with bundled plugins and resources.
- Seven deterministic offline scenarios.
- Seven live-network scenarios.
- One ten-minute warm-cache run.
- CPU, three memory definitions, threads, handles, file I/O, startup readiness,
  process children, and failure-path observations.
- Documentation fact, navigation, relative-link, UTF-8, and Arabic bidirectional
  isolation checks.

The product's runtime behavior was not changed while collecting the
measurements.
