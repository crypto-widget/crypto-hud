# Laporan dampak kinerja Crypto HUD

[English](README.md) · [简体中文](README.zh-CN.md) ·
[繁體中文](README.zh-TW.md) · [Español](README.es.md) ·
[Português do Brasil](README.pt-BR.md) · [Tiếng Việt](README.vi.md) ·
[Bahasa Indonesia](README.id.md) · [Türkçe](README.tr.md) ·
[한국어](README.ko.md) · [日本語](README.ja.md) ·
[Русский](README.ru.md) · [العربية](README.ar.md)

> Tanggal pengujian: 2026-07-12<br>
> Produk: Crypto HUD 0.9.7<br>
> Commit: `b572ce82d2b1a7d95fa2c5ab6687b73b9ed76ae7`<br>
> SHA-256 executable:
> `3F50145630A74FB6E7265AFE8935101BDB872C743DA6BC8267C49E0DD7BE267D`

## Ringkasan laporan

Laporan ini mencatat CPU, memori, thread, struktur proses, waktu mulai, jumlah
request jaringan, dan stabilitas Crypto HUD dalam kondisi pengujian berikut.

Dengan tiga pasangan dan refresh lima detik, pengujian sepuluh menit dengan
cache hangat mencatat **0,070% CPU rata-rata seluruh mesin**, **20,27 MiB
private commit**, **47,96 MiB working set**, dan **19,29 MiB private working
set**.

Dengan 20 pasangan live dan interval yang sama, hasilnya **0,125% CPU
rata-rata**, **0,681% CPU P95**, **21,56 MiB private commit**, dan **49,02 MiB
working set**.

Tidak ada proses anak atau WebView2. Memori saat widget disembunyikan dari tray
mendekati skenario offline yang terlihat. Dalam pengujian sepuluh menit, memori
meningkat selama warmup dan pembaruan candlestick lima menit, lalu berada dalam
rentang stabil pada menit 7–10.

Angka hanya menggambarkan proses Crypto HUD pada mesin tersebut. GPU, komposisi
DWM, dan daya listrik tidak tercakup.

## Angka utama

| Metrik | Hasil |
| --- | ---: |
| CPU rata-rata bawaan | 0,070% |
| CPU P95 bawaan | 0,189% |
| Median private commit | 20,27 MiB |
| Median working set | 47,96 MiB |
| Median private working set | 19,29 MiB |
| Waktu data live siap | 1,705 detik |
| CPU rata-rata 20 pasangan | 0,125% |
| CPU P95 20 pasangan | 0,681% |
| Sampel CPU tertinggi | 0,916% |
| Proses anak | 0 |
| Proses WebView2 | 0 |

## Lingkungan pengujian

| Item | Nilai |
| --- | --- |
| Sistem operasi | Windows 11 Pro for Workstations, 10.0.26200, build 26200 |
| Virtualisasi | QEMU Standard PC (Q35 + ICH9, 2009) |
| Prosesor | AMD Ryzen 7 8845HS with Radeon 780M Graphics |
| Prosesor logis | 12 |
| Memori | 23,98 GiB |
| Adapter tampilan | OrayIddDriver Device; Red Hat QXL controller |
| Power plan | Balanced |
| Renderer | Slint software renderer |
| Toolchain | Rust 1.96.0 |

Executable release dijalankan dari tata letak staged dengan plugin dan resource.
Setiap run memakai state terisolasi dan ID single-instance unik.

## Definisi metrik

- **CPU seluruh mesin:** waktu CPU proses dibagi waktu dan 12 prosesor logis.
- **CPU P95:** 95% sampel stabil berada pada atau di bawah angka ini.
- **Private commit:** memori committed khusus proses.
- **Working set:** halaman yang sedang berada di RAM, termasuk halaman bersama.
- **Private working set:** RAM residen khusus proses.
- **Thread:** total thread per sampel; worker sementara mengubah nilainya.
- **Handle:** handle Windows milik proses.

CPU dihitung dari selisih `TotalProcessorTime`. Sepuluh detik awal dibuang;
30 detik untuk run lima menit dan satu menit untuk run sepuluh menit.

CPU offline adalah estimasi pembulatan dari counter ekuivalen satu core setelah
masalah pembulatan integer ditemukan pada ringkasan pertama. Nilai jaringan
nyata tidak terkena masalah tersebut.

## Hasil skenario

| Skenario | CPU rata-rata / P95 | Median private commit | Median working set | Thread median / maks. |
| --- | ---: | ---: | ---: | ---: |
| Live bawaan, 3 pasangan, 10 menit | 0,070% / 0,189% | 20,27 MiB | 47,96 MiB | 9 / 13 |
| Pengaturan terbuka, jaringan nyata | 0,071% / 0,233% | 23,74 MiB | 55,35 MiB | 14 / 17 |
| 20 pasangan live, refresh 5 detik | 0,125% / 0,681% | 21,56 MiB | 49,02 MiB | 14 / 18 |
| Proxy lokal langsung menolak | 0,049% / 0,232% | 18,66 MiB | 43,67 MiB | 14 / 17 |
| Satu widget terlihat, data offline | sekitar 0,04% / 0,25% | 17,43 MiB | 41,93 MiB | 12 / 12 |
| Widget sama disembunyikan dari tray | sekitar 0,03% / 0,25% | 17,43 MiB | 41,77 MiB | 12 / 12 |
| 10 jendela Quote Board, offline | sekitar 0,14% / 0,50% | 20,13 MiB | 45,48 MiB | 12 / 12 |
| Satu instance dari lima jenis widget | sekitar 0,17% / 0,50% | 26,63 MiB | 53,37 MiB | 12 / 12 |
| Satu Quote Board dengan 20 pasangan offline | sekitar 0,09% / 0,42% | 17,87 MiB | 42,78 MiB | 12 / 12 |
| Satu widget skala 300%, offline | sekitar 0,08% / 0,50% | 18,71 MiB | 44,30 MiB | 12 / 12 |

Sampel CPU tunggal tertinggi adalah **0,916%** pada 20 pasangan.

## Perbedaan yang diamati

- Pengaturan terbuka mencatat 23,74 MiB private commit dan 55,35 MiB working
  set; skenario bawaan mencatat 20,27 MiB dan 47,96 MiB.
- Dari 3 ke 20 pasangan, CPU berubah dari 0,070% menjadi 0,125%, private commit
  20,27 menjadi 21,56 MiB, dan working set 47,96 menjadi 49,02 MiB.
- 10 Quote Board offline mencatat tambahan 2,70 MiB private commit dan 3,55 MiB
  working set dibanding satu widget.
- Lima jenis widget mencatat tambahan 9,20 MiB private commit dan 11,44 MiB
  working set.
- Skala 300% mencatat tambahan 1,28 MiB private commit dan 2,37 MiB working set.
- Sebelum dan sesudah disembunyikan, private commit 17,43 MiB; working set
  berubah dari 41,93 menjadi 41,77 MiB. Estimasi CPU 0,04% dan 0,03%.

## Startup

Penanda semua data pasar siap tercapai dalam **1,705 detik**. Penanda ini
memerlukan data pada baris yang dikonfigurasi dan mencakup penemuan plugin,
jendela, request awal, serta timer satu detik. Ini bukan waktu frame pertama.

Pemeriksaan update mengikuti jalur error karena GitHub Releases API gagal.
Respons berhasil tidak diukur.

## Garis waktu memori

| Menit | Median private commit | Median working set |
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

Private commit meningkat sampai menit 6. Median menit 7–10 berada pada
20,27–20,36 MiB. Perubahan menit 6 bertepatan dengan area pembaruan candlestick
lima menit.

Satu request SOL gagal sementara. Aplikasi melanjutkan dan keluar dengan kode 0.

## Pengamatan jaringan dan file

`pasangan unik × (60 / detik refresh + 0,2)` request/menit.

| Konfigurasi | Perkiraan request/menit |
| --- | ---: |
| 3 pasangan, 5 detik | 36,6 |
| 8 pasangan, 5 detik | 97,6 |
| 20 pasangan, 5 detik | 244 |
| 3 pasangan, 60 detik | 3,6 |
| 20 pasangan, 60 detik | 24 |

`0,2` mewakili pembaruan candlestick lima menit. Startup menambah kira-kira
satu ticker dan satu candlestick per pasangan. Latensi dapat menurunkan laju.

Proxy yang ditolak menghasilkan empat siklus gagal dalam 90 detik. Timeout
delapan detik dan koneksi ulang tidak diukur.

Setelah warmup, I/O file saat idle hampir nol. Penyimpanan pada disk lambat tidak
diukur. Counter I/O tidak digunakan sebagai byte jaringan teratribusi.

## Struktur proses

- Proses anak: 0.
- Proses WebView2: 0.
- Thread bawaan: median 9, maksimum 13.
- Thread 20 pasangan: median 14, maksimum 18.
- Handle bawaan: median 301, maksimum 322.

## Cakupan dan batasan

- QEMU dan layar remote tidak mengukur GPU fisik, DWM, atau daya.
- Setiap skenario kustom dijalankan sekali; tidak ada interval keyakinan.
- Run terpanjang 10 menit; tidak ada pengujian 8–24 jam.
- DPI 150–300%, multi-monitor 4K, sleep/resume, pelepasan layar, dan animasi
  kontinu tidak diukur.
- DNS lambat, HTTP 429, timeout delapan detik, dan pemulihan tidak diukur.
- WPR/WPA tidak tersedia; tidak ada byte jaringan, GPU, context switch, wakeup
  independen, atau daya teratribusi.
- Penyimpanan pada disk lambat atau tersinkron tidak diukur.
- Hasil berlaku untuk versi dan hash di awal laporan.

## Catatan validasi

- Build release:
  `cargo +1.96.0 build --release --locked -p crypto-hud`
- Pemeriksaan staged dengan plugin/resource.
- Tujuh skenario offline, tujuh jaringan nyata, dan satu run 10 menit.
- CPU, tiga metrik memori, thread, handle, I/O, startup, proses, dan error.
- Pemeriksaan data, navigasi, link, UTF-8, dan isolasi dua arah.

Perilaku produk tidak diubah selama pengukuran.
