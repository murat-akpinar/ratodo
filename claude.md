# ratodo — Karar Kaydı

> Son güncelleme: 2026-08-10 · Durum: tasarım aşaması, kod yok
>
> İsim: **ratodo** — ratatui + todo. Gerekçe ve elenen adaylar en altta.
> Müsaitlik kontrolü (crates.io / GitHub / `command -v`) hâlâ yapılmadı.

## Tek cümlede

i3 / Hyprland / sway kullanan birinin, iş akışını bölmeden terminalden saniyeler
içinde görev yakaladığı; verisini dotfiles'ına giren **tek bir Markdown
dosyasında** tutan, buluta hiçbir şey göndermeyen, tek binary'lik bir
Rust + ratatui TUI'si.

## Neden var — boşluk nerede

| Araç | Ne yapıyor | Eksiği |
|---|---|---|
| `vim ~/todo.md` | Herkesin şu an yaptığı şey. Sıfır bağımlılık | "Bugün ne var?" sorusuna cevap yok. Tarih hesabı kafada, gecikmiş görev görünmüyor |
| taskwarrior | Çok güçlü veri modeli, olgun CLI | Verisi `~/.task/*.data` — **kendi formatı**. Araç olmadan okunmuyor, dotfiles'a girmiyor. Öğrenme eğrisi dik, TUI'si (`vit`) ayrı bir proje |
| todo.txt / todo.sh | Düz metin, standart var, ekosistem var | TUI yok. Tarih/ajanda desteği zayıf, takvim çıkışı yok. Shell script |
| taskell | Markdown + TUI, tam bizim alan | **Kanban odaklı** — sütunlar var, tarih yok, ajanda yok, takvim yok |
| dstask | Git tabanlı, düz dosya | Zorunlu git repo + her görev ayrı YAML dosyası. "Tek dosya" değil |
| Todoist / TickTick TUI istemcileri | Senkron çalışıyor | Hesap gerekiyor, veri dışarı çıkıyor, çevrimdışı zayıf |

Boşluk: **düz Markdown + hızlı yakalama + ajanda** üçünü birden veren yok.
Her araç ikisini alıp üçüncüsünü düşürüyor.

Kritik ayrım — veri deposu değil, **dosyanın kendisi**:

```
taskwarrior  : ~/.task/pending.data
               araç silinirse veri okunamaz. Format aracın malı.

bizim araç   : ~/.config/ratodo/todo.md
               araç silinse dosya hâlâ işe yarar. Format kullanıcının malı.
```

Bu tek cümle ürünün omurgası: **araç dosyanın sahibi değil, misafiri.**
Aşağıdaki her mimari karar bundan türüyor.

## Verilen kararlar

### Ürün

- **Hedef kitle:** tiling WM (i3 / Hyprland / sway) kullanan, terminalde yaşayan,
  dotfiles'ını git'te tutan kişi. Zaten `vim todo.md` yapıyor — onu bırakması
  için sebep vermek zorundayız, dosyasını elinden almak için değil.
- **Dosya kullanıcınındır.** Elle düzenlenebilir olmalı. Araç tanımadığı hiçbir
  satıra dokunmaz, biçimlendirmesini bozmaz, sırasını değiştirmez.
- **İki giriş yolu, ikisi de v1'de:**

  ```
  ratodo                              → TUI açılır
  ratodo add "fatura öde @yarın"      → yazar ve çıkar, TUI hiç açılmaz
  ```

  İkincisi ürünün varlık sebebi. Bir iş yaparken akılda beliren şeyi 2 saniyede
  dosyaya atmak. TUI açmak zorunda kalmak bu akışı öldürür.
- **`e` tuşu `$EDITOR`'ı açar.** TUI'den vim'e kaçış kapısı. On satır kod, ve
  hedef kitleye tam oturuyor — "aracın yapamadığı şeyi dosyada yaparım"
  garantisi, kullanıcıyı kilitlenmiş hissetmekten kurtarıyor.
- **Yerel ve çevrimdışı.** Hesap yok, sunucu yok, telemetri yok. Senkron
  kullanıcının kendi git'i — bizim işimiz değil, olmasın da.
- **v1 yazma yapar ama yıkıcı değil:** atomik yazma (geçici dosya + `rename`)
  ve yazmadan önce `todo.md.bak`. K8s aracındaki "read-only" garantisinin
  buradaki karşılığı bu — burada yazmak zorundayız, o yüzden garanti
  "hiçbir şeyi bozamaz" değil "**kaybettiremez**" oluyor.

### Depolama — format

Tek dosya, Markdown checklist. Sözdizimi:

| # | Sözdizimi | Örnek | Anlamı |
|---|---|---|---|
| 1 | `- [ ]` | `- [ ] fatura öde` | açık görev |
| 2 | `- [x]` | `- [x] pr review et` | tamamlanmış görev |
| 3 | `## Başlık` | `## Work` | bölüm |
| 4 | `@YYYY-MM-DD` | `@2026-08-12` | son tarih |
| 5 | `@YYYY-MM-DD HH:MM` | `@2026-08-12 16:00` | saatli son tarih |
| 6 | `#etiket` | `#ops #home` | etiket, birden çok olabilir |
| 7 | `!high` `!med` `!low` | `!high` | öncelik |
| 8 | geri kalan her şey | serbest metin | görevin başlığı |
| 9 | **tanınmayan satır** | `> not`, tablo, boş satır, `---` | **dokunulmaz, aynen korunur** |

9 numara bir ayrıntı değil, ürün kararı. Kullanıcının dosyasının yarısı bizim
anlamadığımız şeyler olabilir; hepsi yerinde kalır.

**Girdi esnek, depolama katı.** Yazarken kısayol serbest, dosyaya her zaman ISO
tarih yazılır:

```
ratodo add "fatura öde @yarın"     →  - [ ] fatura öde @2026-08-11
ratodo add "rapor @pzt !high"      →  - [ ] rapor @2026-08-17 !high
ratodo add "yedek al @3g"          →  - [ ] yedek al @2026-08-13
```

Kabul edilen kısayollar: `@today @tomorrow @mon…@sun @3d @2w`.
Dosyada asla görünmezler — dosya makinede de insanda da aynı okunmalı.

### Depolama — konum

**XDG'den bilinçli bir sapma var.** Kullanıcı verisi teknik olarak
`$XDG_DATA_HOME`'a (`~/.local/share/`) ait. Ama amaç dosyanın dotfiles'a
girmesi ve kimse `~/.local/share`'i dotfiles'a koymuyor. Standarda uymak
ürünün ana vaadini kırıyor, o yüzden uymuyoruz:

| Ne | Nerede | Neden |
|---|---|---|
| `todo.md` | `~/.config/ratodo/todo.md` | **Kullanıcının.** Dotfiles'a girer, elle düzenlenir, git'te versiyonlanır. XDG sapması burada, bilerek |
| `todo.ics` | `~/.local/share/ratodo/todo.ics` | **Türetilmiş.** Yedeklenmesi anlamsız, silinse yeniden üretilir. XDG doğru yer |
| `config.toml` | `~/.config/ratodo/config.toml` | v2. v1'de config dosyası **yok** |

Ezme yolları: `$XDG_CONFIG_HOME` ve `--file <yol>`. İkincisi "iş listesi ayrı,
kişisel liste ayrı" isteyen için kaçış kapısı — çoklu liste özelliği yazmadan
önce bunun yetip yetmediğini görelim.

### Eşzamanlı düzenleme

Dosya tek gerçek kaynak, bellekteki model değil. Bu yüzden:

- TUI açıkken dosya dışarıdan değişirse (vim, `ratodo add` başka bir terminalde,
  `git pull`) → `notify` / inotify yakalar, dosya yeniden okunur.
- Yazmadan önce mtime kontrol edilir. Okuduğumuzdan beri değiştiyse **üzerine
  yazılmaz**, kullanıcı uyarılır.
- Yazma atomik: geçici dosyaya yaz → `fsync` → `rename`. Yarım dosya diye bir
  şey oluşmaz, güç kesilse bile.

Karmaşık bir merge yapmıyoruz. Çakışma nadir, ve yanlış merge sessizce veri
kaybettirir — uyarıp geri çekilmek dürüst olan.

### Tasarım

- **Palet: Catppuccin Mocha, accent = mauve.** Gerekçe dürüst olsun: hedef kitle
  (tiling WM kullanıcıları) zaten Catppuccin kullanıyor, araç ekranın geri
  kalanına yabancı görünmesin. Tek dosya `theme.rs`, 11 sabit — tema
  *yükleyici* yazma (TOML, hot reload), YAGNI.

| Rol | Catppuccin Mocha | Hex |
|---|---|---|
| zemin | `base` | `#1e1e2e` |
| seçili satır | `surface0` | `#313244` |
| kenarlık | `overlay0` | `#6c7086` |
| ana metin | `text` | `#cdd6f4` |
| tarih (dim) | `subtext0` | `#a6adc8` |
| tamamlanmış görev | `overlay1` | `#7f849c` |
| accent / seçim | `mauve` | `#cba6f7` |
| gecikmiş | `red` | `#f38ba8` |
| bugün | `peach` | `#fab387` |
| tamamlandı ✓ | `green` | `#a6e3a1` |
| etiket `#tag` | `blue` | `#89b4fa` |

⚠️ Catppuccin 24-bit RGB. `Color::Rgb` truecolor terminal ister. Hedef kitle
terminalde yaşadığı için risk k8s aracına göre çok daha düşük (alacritty, kitty,
wezterm, foot — hepsi destekliyor), ama TTY'de ve eski `screen` içinde bozulur.
Şimdilik Rgb ile git, şikâyet gelirse `COLORTERM` bakıp 16 renge düş (~5 satır).

- **Tek vurgu rengi (mauve) + gri tonları.** Her şey renkliyse hiçbir şey
  vurgulu değildir.
- **Kırmızı sadece gecikmiş görev için.** Başka hiçbir yerde yok.
- **Yeşil sadece tamamlanmış için.** İkisi de kazanılmış anlamlar, sulandırma.
- İki seviye hiyerarşi: görev başlığı parlak, tarih/etiket `dim`. Üçüncü yok.
- Bol boşluk. Gruplar arası boş satır tasarımın yarısı.
- **Tek yerleşim, panel bölme yok.** Sidebar yok, modal yok. Bir liste var.
- `○ ✓ !` sembolleri — renge tek başına güvenme (renk körlüğü + kopyalanabilirlik).
  ⚠️ Bunlar **sadece ekran sembolü**, dosya sözdizimi değil. Dosyada yalnızca
  `[ ]` ve `[x]` var; `!` gecikmiş demek ve tarihten türetiliyor. Dosyaya asla
  `- [!]` yazılmaz.
- **Nerd font'a bağımlı olma**, ASCII fallback şart: `[ ]` `[x]` `[!]`.
- Üstü çizili (strikethrough) **kullanma** — crossterm destekliyor ama terminal
  desteği tutarsız, yarı kullanıcıda tamamlanmış görev okunmaz hale geliyor.
  Sönük renk + `✓` yeterli.

Ekran taslağı:

```
┌─ ratodo ────────────────────────────── 3 open · 1 overdue ─┐
│                                                            │
│  OVERDUE                                                   │
│  ! rotate the backup keys              2 days ago #ops     │
│                                                            │
│  TODAY                                                     │
│  ○ pay the invoice                                #home    │
│  ○ review the deploy PR                     16:00 #work    │
│                                                            │
│  THIS WEEK                                                 │
│  ○ book a dentist appointment              Aug 20 #health  │
│  ✓ migrate the server                                      │
│                                                            │
├────────────────────────────────────────────────────────────┤
│ ↑↓ move   ⏎ toggle   a add   d del   e $EDITOR   q quit    │
└────────────────────────────────────────────────────────────┘
```

Hızlı yakalama TUI açmadan:

```
$ ratodo add "fatura öde @yarın #ev"
added: fatura öde  ·  due tomorrow (2026-08-11)  ·  #ev
$
```

Tek satır çıktı, geri dön. Süslü bir şey yok — kullanıcı zaten başka bir işin
ortasında.

### v1 görünüm kuralları

Ajanda saf bir fonksiyon: `agenda(&[Task], today) -> Vec<Group>`.
Bugünün tarihi **parametre**, `Local::now()` çağrısı fonksiyonun içinde değil —
yoksa test edilemez. Ürünün asıl mantığı burada ve tamamı TUI'siz test edilebilir.

| # | Grup | Koşul | Nasıl görünüyor |
|---|---|---|---|
| 1 | OVERDUE | `due < bugün` | `!` · kırmızı · "2 days ago" |
| 2 | TODAY | `due == bugün` | `○` · peach · saat varsa `16:00` |
| 3 | THIS WEEK | `bugün < due ≤ +7g` | `○` · dim · `Aug 20` |
| 4 | LATER | `due > +7g` | `○` · dim · katlanmış, `l` ile açılır |
| 5 | *(dosyadaki `##` başlığı)* | tarihi yok | `○` · dosyadaki sırayla |
| 6 | — | `[x]` tamamlanmış | `✓` · sönük · kendi grubunun sonunda |

Tarihsiz görevler dosyadaki `##` bölümleri altında, **dosyadaki sıraylalarında**
kalıyor. Kullanıcının kendi düzenini yeniden sıralamıyoruz.

Görünüm kipi tek: **ajanda**. "Dosya görünümü / ajanda görünümü" diye ikinci bir
kip yok — iki kip demek durum yönetimi, tuş çakışması ve iki ayrı çizim yolu
demek. v1'de gerek yok.

### Takvim — ne veriyoruz, ne vermiyoruz

Tarihi olan **açık** görevler `todo.ics`'e VTODO olarak yazılır. Tek yön.
Takvimde yapılan düzenleme `todo.md`'ye geri dönmez ve bu bilinçli — geri dönüş
demek çakışma çözümü demek, o da ayrı bir alt-proje.

Ne zaman yazılır: her `todo.md` yazımından sonra, ve `ratodo sync` ile elle.

⚠️ **"Linux takvimi" diye tek bir şey yok — bu bölüm dürüst okunmalı:**

| İstemci | Yerel `.ics` dosyasını okur mu | Not |
|---|---|---|
| khal | ✅ | Zaten dosya tabanlı, doğrudan dizini gösteriyorsun |
| Thunderbird | ✅ | "Takvim ekle → Bu bilgisayarda / dosya" |
| Evolution | ⚠️ | Sürüme göre değişiyor, "On This Computer" kaynağı gerekiyor |
| GNOME Takvim | ⚠️ | Çoğunlukla `webcal://` / HTTPS bekliyor, yerel dosya güvenilir değil |
| Google Calendar | ❌ | Hem yerel dosya okumuyor, hem **VTODO'yu tamamen yok sayıyor** |

Yani `.ics` üretmek kolay; **abone ettirmek kullanıcının işi.** v1'de dosyayı
üretiyoruz ve README'ye khal + Thunderbird için abone olma adımlarını yazıyoruz.
Otomatik kayıt, dbus ile takvim servisine konuşma vb. **yok**.

VTODO mu VEVENT mi: görev semantik olarak VTODO'dur ve doğru olan o. Ama birçok
istemci VTODO göstermiyor. Melez yazmak (saatli olanlar VEVENT, saatsizler
VTODO) kafa karıştırıcı — v1'de **VTODO**, v2'de `--as-events` bayrağı.

## Mimari

Yükü belirleyen şey burada performans değil (dosya birkaç KB), **veri
sadakati**. Asıl mimari karar bu:

> **Round-trip sadakati:** parser her görevin ham satırını saklar. Araç bir alanı
> değiştirmediyse satır byte-byte aynı yazılır.

Kullanıcının el yazması girintisi, fazladan boşluğu, kendi eklediği notu
kaybolmaz. "Dosya kullanıcınındır" kararının teknik karşılığı bu, ve testi
terminal gerektirmiyor:

```
parse(write(parse(x))) == parse(x)
ve: dokunulmamış her satır, byte-byte aynı
```

Akış:

```
~/.config/ratodo/todo.md
  → parse   : satır -> Task { ham satır + ayrıştırılmış alanlar }
  → model   : Vec<Task>, dosya sırası korunur
  → agenda  : (Vec<Task>, bugün) -> Vec<Group>     ← ürün burada
  → ratatui : sadece olay geldiğinde çiz
  ← write   : Task -> satır (değişmediyse ham satır), atomik + .bak
  → ics     : tarihi olan açık görevler -> VTODO
```

Olay döngüsü:

- **Sabit FPS ile çizme.** Tuşa basıldığında veya dosya değiştiğinde çiz,
  boştayken blokla → idle'da %0 CPU. Bir todo aracı arka planda pil yakmamalı.
- **Panic hook terminali geri versin.** Ham modda panikleyen bir TUI kullanıcının
  terminalini bozuk bırakır. `std::panic::set_hook` ile ekran her durumda
  eski haline döner — bu ilk gün yazılacak, sonradan değil.

## Bağımlılıklar

```toml
ratatui       # TUI
crossterm     # terminal backend + olaylar
clap          # add / done / list alt komutları
chrono        # tarih ayrıştırma + "yarın / 3g" hesapları
notify        # inotify — dosya dışarıdan değişirse
directories   # XDG yolları
anyhow        # hata
```

Yedi crate, hepsi zorunlu.

Kasten **yok** olanlar ve nedenleri:

- **`tokio` yok.** K8s aracının aksine burada async'e gerek yok — tek yerel
  dosya, blocking IO fazlasıyla yeterli. Olay döngüsü `crossterm::event::poll`
  + `notify`'ın mpsc kanalı. Async runtime eklemek derleme süresini ve binary
  boyutunu bedava büyütürdü.
- **`serde` yok.** Markdown parser'ını kendimiz yazıyoruz (zaten ürünün kalbi),
  v1'de config dosyası yok. `config.toml` geldiğinde eklenir.
- **`regex` yok.** `@tarih`, `#etiket`, `!öncelik` ayrıştırması kelime kelime
  taramayla yapılır — regex'ten hem hızlı hem hata mesajı üretmesi kolay.
- **`icalendar` crate'i yok.** VTODO çıktısı ~30 satır string biçimlendirme;
  bir crate'e bağımlı olmaya değmez.

## Dosya düzeni

```
src/
  main.rs      clap alt komutları, terminal kurulumu/temizliği, panic hook
  model.rs     Task, Section, Due, Priority
  parse.rs     todo.md -> Vec<Task>, ham satır korunur     ← ürün burada
  write.rs     Vec<Task> -> todo.md, atomik + yedek
  agenda.rs    (Vec<Task>, bugün) -> Vec<Group>            ← ürün burada
  ics.rs       Vec<Task> -> todo.ics (VTODO)
  theme.rs     Catppuccin sabitleri (11 tane)
  ui.rs        ratatui çizim
tests/
  fixtures/    elle yazılmış todo.md'ler — düzgün olanlar ve kasten garip olanlar
```

Sekiz dosya. `mod.rs` piramidi, trait katmanı, plugin sistemi yok.

## Kapsam dışı (en önemli bölüm)

Bu projeyi öldürecek şey teknik zorluk değil, **kapsam kayması**. Bir todo aracı
sonsuza kadar büyüyebilir; herkesin aklında eklenmesi gereken bir özellik var.
Aşağıdakiler bilinçli olarak YOK:

- ❌ **Bulut senkron / hesap / sunucu.** Git zaten var. "Veri yerinde kalıyor"
  garantisini geri almayız.
- ❌ **CalDAV çift yönlü senkron.** ETag, çakışma, offline kuyruk. Ayrı ürün.
- ❌ **Kanban / board görünümü.** taskell bunu iyi yapıyor. Yeniden yazma.
- ❌ **Tekrarlayan görevler (RRULE).** Tek başına bir hafta. v3.
- ❌ **Alt görev / bağımlılık grafiği.** taskwarrior'ın alanı, oraya girme.
- ❌ **Zaman takibi / pomodoro.** Farklı ürün, farklı kullanım anı.
- ❌ **Tema yükleyici, hot reload, eklenti sistemi.**
- ❌ **Genel amaçlı Markdown editörü.** `e` ile vim açılıyor, yeter.
- ❌ **Windows / macOS.** crossterm ve notify taşınabilir, kasten kırmıyoruz —
  ama XDG yolları ve hedef kitle Linux. Test edilmiyor, vaat edilmiyor.

## Yol haritası

1. **v1 — Yakala + işaretle.** `ratodo` (TUI) + `ratodo add` + ajanda + `.ics` export.
   Tek ekran, tek binary. Tek başına kullanılabilir olmalı.
2. **v2 — Filtre / arama / arşiv.** Etiket ve önceliğe göre filtre, `/` ile
   arama, tamamlananları `## Done`'a taşıma. `config.toml` burada geliyor.
3. **v3 — Tekrarlayan görevler + erteleme.** RRULE alt kümesi ve `~tarih`
   (bu tarihe kadar gizle).
4. **v4 — Masaüstü entegrasyonu.** `ratodo status --json` → waybar / eww modülü,
   ve `notify-send` ile gecikmiş görev bildirimi. Hedef kitle için asıl kazanç
   burada — bar'ında "3 open · 1 overdue" görmek.
5. **v5 — CalDAV.** vdirsyncer dizinine yazarak, opt-in.

## Karara bağlananlar

- ✅ **İsim: `ratodo`** — ratatui + todo. Akrabalık isimde duruyor ama isim
  kütüphaneyi değil ürünü anlatıyor. Gerekçe ve elenenler en altta.
- ✅ **Depolama: tek Markdown dosyası**, satır içi meta veri. Gerekçe: kullanıcı
  zaten vim'de md yazıyor, araç yokken de dosya işe yarıyor, `git diff` satır
  satır anlamlı.
- ✅ **Konum: `~/.config/ratodo/todo.md`.** XDG'den bilinçli sapma — dotfiles'a
  girmesi standarda uymaktan önemli.
- ✅ **Takvim: tek yönlü `.ics`, VTODO.** Dosyayı üretiyoruz, abone etmek
  kullanıcının işi.
- ✅ **v1 kapsamı: yakala + işaretle.** Filtre/arama v2'ye.
- ✅ **`e` → `$EDITOR`.** Kaçış kapısı, on satır, kitleye tam oturuyor.
- ✅ **`tokio` yok, `serde` yok, `regex` yok.**
- ✅ **Arayüz dili: İngilizce.** Terimler ve arama sonuçları zaten İngilizce;
  açık kaynak yapılırsa kitle de öyle. i18n yok (YAGNI); gerekirse sonra
  ayırmak daha ucuz. *Bu doküman Türkçe — kod ve arayüz İngilizce.*
- ✅ **Test ortamı gerekmiyor.** K8s aracının kind'e ihtiyacı vardı; burada
  gereken tek şey elle yazılmış birkaç `todo.md`. Bu büyük bir avantaj,
  1. günden test yazılabilir.

## Açık sorular

- [ ] **`ratodo` müsait mi?** İsme karar verildi ama kontrol edilmedi:
      crates.io · GitHub · `command -v ratodo`. Çakışma çıkarsa yedek: `tuido`.
- [ ] Tamamlanan görev bulunduğu yerde mi kalsın, `## Done` bölümüne mi taşınsın?
      Yerinde kalırsa dosya şişiyor; taşınırsa `git diff` her tamamlamada iki
      satır oynatıyor.
- [ ] Çoklu liste (iş / kişisel ayrı dosya) için `--file` yetiyor mu, yoksa
      isimli liste kavramı mı lazım? Önce `--file` ile yaşayıp görelim.
- [ ] `ratodo add` çağrıldığında `.ics` de yenilensin mi, yoksa sadece TUI kapanışında mı?
      (Her `add`'de yenilemek basit ama gereksiz disk yazımı)

## İlk somut adımlar

**Bağlam: bu ilk TUI projesi.** Mimariyi buna göre kuruyoruz. Kritik özellik şu —
proje iki yarıya ayrılıyor ve zorluk dengesiz:

| | Zor / yeni | Kolay / tanıdık |
|---|---|---|
| Ne | ratatui olay döngüsü, terminal ham modu, inotify | `parse` / `write` / `agenda`: saf fonksiyonlar |
| Neden | Terminal ham moda alınır, olaylar beklenir, panikte terminal geri verilmeli | Girdi `String`, çıktı `Vec<Task>`. Yan etki yok |
| Test | Zor, gözle | **Kolay — fixture dosyalarına karşı düz unit test, terminal hiç gerekmez** |

Yani **ürünün asıl değeri (parse + agenda) TUI'den tamamen bağımsız yazılıp test
edilebilir.** Bu, TUI'ye yeni biri için riski büyük ölçüde siliyor.

Sıra:

1. **Fixture'ları yaz.** Bir tane düzgün `todo.md`, bir tane kasten zor olan:
   girintili alt liste, boş satırlar, `> alıntı`, tablo, `---`, emoji, Türkçe
   karakter, bozuk tarih, `[X]` büyük harf, `* [ ]` yıldızlı liste, aynı satırda
   üç etiket. Kural yazmak için gerçek bozukluklara bakmak şart.
2. **`parse` + `write` yaz — TUI yok.** `ratodo list` bulguları `println!` etsin.
   Round-trip testi: dokunulmamış her satır byte-byte aynı çıkmalı.
3. **`agenda` + `ics` — saf fonksiyonlar.** Sabit bir "bugün" ver, çıktıyı
   snapshot'la. `.ics` çıktısını khal'e verip gerçekten okunduğunu doğrula.
4. **ratatui'yi aptal bir listeyle öğren.** Görev başlıklarını ekrana bas,
   `q` ile çık, panic hook terminali geri versin. Olay döngüsü + inotify
   entegrasyonu burada oturur.
5. **İkisini birleştir**, sonra tasarımı uygula.

Sıralama kasıtlı: **2. adım biterse elinde çalışan bir CLI todo var** (çirkin ama
işleyen), 4. adım tıkanırsa proje ölmüyor.

> Cluster, sunucu, hesap gerekmiyor. Gereken tek şey bir metin dosyası.
> Bugün başlanabilir.

## Geliştirme ortamı kurulum listesi

*(Bu klasör şu an sadece doküman.)*

- [ ] **Rust** — rustup. Linux / WSL tercih edilir (hedef platform orası)
- [ ] **git init** — bu klasör henüz depo değil
- [ ] Truecolor destekleyen terminal (Catppuccin paleti için)
- [ ] **khal** veya **Thunderbird** — `.ics` çıktısını doğrulamak için
- [ ] *(opsiyonel)* i3 / Hyprland / sway — gerçek kullanım akışını test etmek için

---

# İsim — ratodo

**ratatui + todo.** Karar verildi.

## Kriterler

1. Todo / yakalama fikri isimde görünsün
2. **Günde 20 kez yazılacak** — kısa olmalı
3. Logo tek şekilde çizilebilsin — tek renk, 16px favicon'da okunur
4. TR ve EN'de aynı okunsun, açıklama gerektirmesin
5. crates.io ve arama sonuçlarında gürültüye karışmasın

## Neden ratodo

Fikir `ratado` olarak geldi — ratatui'yi çağrıştırsın diye. Yön doğruydu, ama
`ratado` 1. kriterde düşüyor: **ürünü değil çerçeveyi anlatıyor.** Bir metin
editörüne `Qtext` demek gibi; kullanıcı ne ile yazıldığını değil ne yaptığını
arıyor. Üstelik logosu ratatui'nin faresiyle doğrudan çakışıyor ve
İspanyolca/Portekizce'de `rata` (fare) çağrışımı taşıyor.

Tek harf düzeltiyor:

```
ratado  →  rat + ado      hiçbir şey demiyor
ratodo  →  rat + todo     "todo" harfi harfine içinde
```

`ratodo` akrabalığı aynen koruyor, tek kusurunu kapatıyor. Logo da düzeliyor:
sade bir fare değil, **elinde checklist tutan fare** — ratatui'nin logosuyla
akraba ama onunla karışmıyor.

⚠️ **Dikkat edilecek nokta:** akrabalık bir tuzak da taşıyor — proje ratatui'nin
bir eklentisi ya da alt-projesi sanılabilir. README'nin ilk cümlesi bunu
kapatmalı: *"A todo TUI, built with ratatui"* — **with**, `for` değil.

## Elenen adaylar

| İsim | Fikir | Neden elendi |
| ---- | ----- | ------------ |
| **ratado** | ratatui + (hiçbir şey) | Ürünü değil çerçeveyi anlatıyor. Logo ratatui'nin faresiyle çakışıyor. `rata` = fare çağrışımı |
| **jot** | "jot down" = çabucak not al — ürünün fiili | 3 harf ile en ucuz yazılan aday, ve tek gerçek rakip. Ama *todo* olduğu isimden belli değil, ve BSD/macOS'ta aynı adda bir komut var (sayı üreteci) |
| **tuido** | TUI + do | Sağlam yedek. "TUI" bilmeyene bir şey ifade etmiyor, ratatui akrabalığı yok |
| **tik** | TR "tik atmak" = EN "tick" — iki dilde aynı anlam, aynı okunuş | Çok genel, arama gürültüsü yüksek, `tick` crate'i mevcut |
| **tock** | tick-tock — hem onay hem zaman | Zaman takibi araçlarıyla karışıyor, bizde zaman takibi yok |
| **rusto** | Rust + to-do | Anlam zorlama, "rustic" çağrışımı |
| **kap** | TR "kapmak" = yakalamak | Türkçe-only, uluslararası okunmuyor |
| **nudge** | "dürtme" | Bildirim aracı gibi duruyor, todo değil |

## Kabul edilen bedel

`jot`'un 3 harfi kaybedildi. Günde 20 kez yazılan bir komutta bu önemsiz değil —
ama `alias r=ratodo` bunu 1 harfe indiriyor ve **kararı geri alınabilir kılıyor.**
İsim geri alınamaz, alias alınır. README'de örnek olarak verilecek.

## Kalan kontroller

- [ ] crates.io'da `ratodo` müsait mi
- [ ] GitHub'da aynı adda belirgin bir proje var mı
- [ ] `command -v ratodo` — yaygın dağıtımlarda çakışma
- [ ] Çakışma çıkarsa yedek: **tuido**

## Tagline

> A todo TUI, built with ratatui — one Markdown file, no cloud, no account.
