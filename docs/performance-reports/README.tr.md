# Crypto HUD performans etkisi raporu

[English](README.md) · [简体中文](README.zh-CN.md) ·
[繁體中文](README.zh-TW.md) · [Español](README.es.md) ·
[Português do Brasil](README.pt-BR.md) · [Tiếng Việt](README.vi.md) ·
[Bahasa Indonesia](README.id.md) · [Türkçe](README.tr.md) ·
[한국어](README.ko.md) · [日本語](README.ja.md) ·
[Русский](README.ru.md) · [العربية](README.ar.md)

> Test tarihi: 2026-07-12<br>
> Ürün: Crypto HUD 0.9.7<br>
> Commit: `b572ce82d2b1a7d95fa2c5ab6687b73b9ed76ae7`<br>
> Çalıştırılabilir dosyanın SHA-256 değeri:
> `3F50145630A74FB6E7265AFE8935101BDB872C743DA6BC8267C49E0DD7BE267D`

## Rapor özeti

Bu rapor, aşağıda belirtilen test koşullarında Crypto HUD işlemi için ölçülen
CPU, bellek, iş parçacığı, işlem yapısı, başlangıç, ağ isteği ve kararlılık
verilerini kaydeder.

Üç piyasa çifti ve beş saniyelik yenileme aralığından oluşan varsayılan
yapılandırmanın on dakikalık sıcak önbellek çalışmasında toplam makine CPU
ortalaması **%0,070**, medyan özel ayrılmış bellek **20,27 MiB**, medyan çalışma
kümesi **47,96 MiB** ve medyan özel çalışma kümesi **19,29 MiB** olarak ölçüldü.

Aynı yenileme aralığında 20 canlı piyasa çifti kullanıldığında CPU ortalaması
**%0,125**, CPU P95 değeri **%0,681**, medyan özel ayrılmış bellek **21,56 MiB**
ve medyan çalışma kümesi **49,02 MiB** oldu.

Ölçülen senaryoların hiçbirinde alt işlem veya WebView2 işlemi oluşmadı.
Sistem tepsisine gizleme durumundaki işlem belleği, çevrimdışı görünür pencere
senaryosuna yakın kaldı. On dakikalık çalışmada bellek, ısınma ve beş dakikalık
mum verisi yenilemesi sırasında yükseldi; 7–10. dakikalarda dar bir aralıkta
kaldı.

Bu değerler belirtilen test makinesindeki Crypto HUD işlemini kapsar. GPU
kullanımı, DWM birleştirme maliyeti ve elektriksel güç ölçümü kapsama dahil
değildir.

## Temel ölçümler

| Ölçüm | Sonuç |
| --- | ---: |
| Varsayılan CPU ortalaması | %0,070 |
| Varsayılan CPU P95 | %0,189 |
| Varsayılan medyan özel ayrılmış bellek | 20,27 MiB |
| Varsayılan medyan çalışma kümesi | 47,96 MiB |
| Varsayılan medyan özel çalışma kümesi | 19,29 MiB |
| Canlı piyasa verisinin hazır olma süresi | 1,705 saniye |
| 20 çift CPU ortalaması | %0,125 |
| 20 çift CPU P95 | %0,681 |
| Gözlenen en yüksek uygulama CPU örneği | %0,916 |
| Alt işlemler | 0 |
| WebView2 işlemleri | 0 |

## Test ortamı

| Öğe | Değer |
| --- | --- |
| İşletim sistemi | Windows 11 Pro for Workstations, 10.0.26200, derleme 26200 |
| Sanallaştırma | QEMU Standard PC (Q35 + ICH9, 2009) |
| Bildirilen işlemci | AMD Ryzen 7 8845HS with Radeon 780M Graphics |
| Kullanılabilir mantıksal işlemci | 12 |
| Sistem belleği | 23,98 GiB |
| Görüntü bağdaştırıcıları | OrayIddDriver Device; Red Hat QXL controller |
| Güç planı | Dengeli |
| Uygulama oluşturucusu | Slint yazılım oluşturucusu |
| Rust araç zinciri | Rust 1.96.0 |

Çalıştırılabilir dosya sürüm kipinde derlendi ve paketlenmiş eklentiler ile
kaynakları içeren aşamalı bir sürüm düzeninden başlatıldı. Her çalışmada
yalıtılmış bir durum dizini ve benzersiz bir tek örnek kimliği kullanıldı.

## Metrik tanımları

- **Toplam makine CPU değeri:** İşlem CPU süresinin geçen süreye ve 12
  mantıksal işlemciye bölünmesiyle hesaplanır. Windows Görev Yöneticisi'nin
  genel makine yüzdesi yaklaşımıyla aynıdır.
- **CPU P95:** Kararlı durum CPU örneklerinin %95'i bu değere eşit veya daha
  düşüktür.
- **Özel ayrılmış bellek:** Yalnızca bu işleme ayrılan taahhüt edilmiş bellek.
- **Çalışma kümesi:** O anda RAM'de bulunan sayfalar; paylaşılan sayfalar da
  dahil olabilir.
- **Özel çalışma kümesi:** RAM'de bulunan ve yalnızca işleme ait sayfalar.
- **İş parçacığı sayısı:** Her örnek anındaki işlem iş parçacığı sayısı.
- **Tanıtıcı sayısı:** İşlemin sahip olduğu Windows tanıtıcılarının sayısı.

CPU, `TotalProcessorTime` farklarından hesaplandı. Olağan kararlı durum
senaryolarında ilk 10 saniye, beş dakikalık çalışmada ilk 30 saniye ve on
dakikalık çalışmada ilk dakika örneklerden çıkarıldı.

İlk ham özetlemede tamsayı yuvarlama sorunu saptandığından çevrimdışı CPU
değerleri çekirdek eşdeğeri sayaçtan yeniden oluşturulan yuvarlanmış yaklaşık
değerlerdir. Canlı ağ senaryolarındaki CPU değerleri bu yuvarlama sorunundan
etkilenmedi.

## Senaryo sonuçları

CPU değerleri, 12 mantıksal işlemcili test makinesinin tamamının yüzdesidir.

| Senaryo | CPU ortalama / P95 | Medyan özel ayrılmış bellek | Medyan çalışma kümesi | İş parçacığı medyan / en çok |
| --- | ---: | ---: | ---: | ---: |
| Varsayılan canlı, 3 çift, 10 dakikalık sıcak çalışma | %0,070 / %0,189 | 20,27 MiB | 47,96 MiB | 9 / 13 |
| Canlı ağ ile Ayarlar penceresi açık | %0,071 / %0,233 | 23,74 MiB | 55,35 MiB | 14 / 17 |
| 20 canlı çift, beş saniyelik yenileme | %0,125 / %0,681 | 21,56 MiB | 49,02 MiB | 14 / 18 |
| Bağlantıları hemen reddeden yerel proxy | %0,049 / %0,232 | 18,66 MiB | 43,67 MiB | 14 / 17 |
| Bir görünür bileşen, belirlenmiş çevrimdışı veri | yaklaşık %0,04 / %0,25 | 17,43 MiB | 41,93 MiB | 12 / 12 |
| Aynı bileşen sistem tepsisine gizlenmiş | yaklaşık %0,03 / %0,25 | 17,43 MiB | 41,77 MiB | 12 / 12 |
| 10 Quote Board penceresi, çevrimdışı | yaklaşık %0,14 / %0,50 | 20,13 MiB | 45,48 MiB | 12 / 12 |
| Beş bileşen türünün her birinden bir örnek | yaklaşık %0,17 / %0,50 | 26,63 MiB | 53,37 MiB | 12 / 12 |
| 20 çevrimdışı çift içeren bir Quote Board | yaklaşık %0,09 / %0,42 | 17,87 MiB | 42,78 MiB | 12 / 12 |
| %300 ölçekli bir bileşen, çevrimdışı | yaklaşık %0,08 / %0,50 | 18,71 MiB | 44,30 MiB | 12 / 12 |

Gözlenen en yüksek tek uygulama CPU örneği, 20 çiftlik canlı senaryoda
**%0,916** oldu.

## Senaryolar arasında gözlenen farklar

- Ayarlar penceresi açıkken medyan özel ayrılmış bellek 20,27 MiB'den
  23,74 MiB'ye, medyan çalışma kümesi 47,96 MiB'den 55,35 MiB'ye çıktı.
  Senaryo süreleri farklı olduğundan bunlar yalnızca gözlenen değerlerdir.
- Canlı çift sayısının 3'ten 20'ye çıkmasıyla CPU ortalaması %0,070'ten
  %0,125'e, özel ayrılmış bellek 20,27 MiB'den 21,56 MiB'ye ve çalışma kümesi
  47,96 MiB'den 49,02 MiB'ye değişti.
- Çevrimdışı testte 10 Quote Board penceresi, tek görünür bileşene göre
  2,70 MiB daha fazla özel ayrılmış bellek ve 3,55 MiB daha fazla çalışma
  kümesi kullandı.
- Beş bileşen türünden birer örnek, tek bileşenli çevrimdışı senaryodan
  9,20 MiB daha fazla özel ayrılmış bellek ve 11,44 MiB daha fazla çalışma
  kümesi kullandı.
- %300 ölçek, varsayılan ölçekli çevrimdışı senaryoya göre 1,28 MiB daha fazla
  özel ayrılmış bellek ve 2,37 MiB daha fazla çalışma kümesiyle ölçüldü.
- Sistem tepsisine gizlemede özel ayrılmış bellek 17,43 MiB'den 17,43 MiB'ye,
  çalışma kümesi 41,93 MiB'den 41,77 MiB'ye değişti. Yuvarlanmış çevrimdışı
  CPU tahminleri görünür durumda %0,04 ve gizli durumda %0,03 idi.

## Başlangıç ölçümü

Canlı başlangıç senaryosunda uygulama, mevcut tüm-piyasa-verisi-hazır işaretine
**1,705 saniyede** ulaştı.

Bu işaret, yapılandırılmış piyasa satırlarında veri bulunmasını gerektirir.
Süre; eklenti keşfini, pencere oluşturmayı, ilk piyasa isteklerini ve piyasa
olaylarını tüketen saniyelik UI zamanlayıcısını içerir. İlk kare veya ilk
görünür pencere süresi değildir.

Üretim biçimindeki güncelleme kontrolünde GitHub Releases API isteği test
ortamında başarısız olduğu için hata yolu izlendi. İşlem çalışmaya devam etti;
başarılı güncelleme yanıtı ölçülmedi.

## On dakikalık bellek zaman çizelgesi

| Dakika | Medyan özel ayrılmış bellek | Medyan çalışma kümesi |
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

Özel ayrılmış bellek 6. dakikaya kadar yükseldi. 7–10. dakikalarda dakika
medyanı 20,27–20,36 MiB aralığında kaldı. 6. dakika civarındaki yükseliş, beş
dakikalık mum yenileme bölgesiyle aynı zamana denk geldi.

On dakikalık çalışmada bir geçici SOL isteği hatası kaydedildi. Uygulama işlem
yapmayı sürdürdü ve 0 koduyla kapandı.

## Ağ ve dosya gözlemleri

Yaklaşık kararlı istek sayısı:

`benzersiz çift sayısı × (60 / yenileme saniyesi + 0,2)` istek/dakika.

`0,2`, her beş dakikada bir mum yenilemesini temsil eder.

| Yapılandırma | Dakikada yaklaşık istek |
| --- | ---: |
| 3 çift, 5 saniyelik yenileme | 36,6 |
| 8 çift, 5 saniyelik yenileme | 97,6 |
| 20 çift, 5 saniyelik yenileme | 244 |
| 3 çift, 60 saniyelik yenileme | 3,6 |
| 20 çift, 60 saniyelik yenileme | 24 |

Başlangıç, her çift için yaklaşık bir fiyat ve bir mum isteği ekler. Ağ
gecikmesi bir çevrimi uzatabildiğinden gözlenen hız formülden düşük olabilir.

Bağlantıyı reddeden yerel proxy, 90 saniyede dört başarısız piyasa çevrimi
oluşturdu. Bu test hemen bağlantı reddini kapsadı; sekiz saniyelik okuma zaman
aşımını veya kesinti sonrası yeniden bağlanmayı kapsamadı.

Başlangıç ve ısınmadan sonra, boşta senaryolarda ölçülen dosya okuma/yazma
etkinliği fiilen sıfırdı. Yavaş diskte sürükleme ve ayar kaydetme gecikmesi
ölçülmedi. Windows işlem I/O sayaçları, işleme atfedilen ağ baytı olarak
kullanılmadı.

## İşlem yapısı

- Gözlenen alt işlem: 0.
- Gözlenen WebView2 işlemi: 0.
- Varsayılan canlı iş parçacığı sayısı: medyan 9, en çok 13.
- 20 çiftlik canlı iş parçacığı sayısı: medyan 14, en çok 18.
- Varsayılan canlı tanıtıcı sayısı: medyan 301, en çok 322.

## Test kapsamı ve sınırlamalar

- Test makinesi QEMU ve uzak sanal görüntü yığını kullandı. Fiziksel GPU
  kullanımı, DWM birleştirme maliyeti ve pil gücü ölçülmedi.
- Her özel senaryo bir kez çalıştırıldı; çoklu çalışma güven aralıkları yoktur.
- En uzun çalışma on dakikaydı; 8–24 saatlik çalışma yapılmadı.
- %150–300 sistem DPI, 4K çoklu ekran, ekran kaldırma, uyku/uyanma ve en kısa
  aralıkta sürekli animasyon ölçülmedi.
- Ağ arızası hemen proxy reddiyle oluşturuldu. DNS beklemesi, yavaş yanıt,
  HTTP 429, sekiz saniyelik zaman aşımı ve ağın geri gelmesi ölçülmedi.
- Windows Performance Recorder ve WPA atfı kullanılamadı. İşleme atfedilen ağ
  baytları, GPU kullanımı, bağlam değiştirme sayıları, bağımsız uyanma sayıları
  ve elektriksel güç raporda yoktur.
- Yavaş veya eşzamanlanan diskte sürükleme ve ayar değişikliği sırasındaki
  durum kaydetme gecikmesi ölçülmedi.
- Sonuçlar 0.9.7 sürümüne ve raporun başındaki çalıştırılabilir dosya karmasına
  aittir.

## Doğrulama kaydı

- Sürüm derlemesi:
  `cargo +1.96.0 build --release --locked -p crypto-hud`
- Paketlenmiş eklentiler ve kaynaklarla aşamalı sürüm süreci kontrolü.
- Yedi belirlenmiş çevrimdışı senaryo.
- Yedi canlı ağ senaryosu.
- Bir on dakikalık sıcak önbellek çalışması.
- CPU, üç bellek tanımı, iş parçacıkları, tanıtıcılar, dosya I/O, başlangıç
  hazır olma süresi, alt işlemler ve hata yolu gözlemleri.
- Belge olguları, gezinme, göreli bağlantılar, UTF-8 ve Arapça çift yönlü metin
  yalıtımı kontrolleri.

Ölçümler toplanırken ürünün çalışma zamanı davranışı değiştirilmedi.
