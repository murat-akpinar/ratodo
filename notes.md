# Çalışma defteri

> `claude.md` **kararların** kaydı — orası oturmuş şeyler.
> Bu dosya **ham düşünce**: açık uçlar, yapılacaklar, reddedilenler, riskler.
> Bir madde burada kararlaşırsa `claude.md`'ye taşınır ve buradan silinir.

---

## Başlangıç fikri (2026-08-10, ham hali — silinmedi)

> - CLI rust ile Ratatui kütüphanesi ile bir todo, planlayıcı araç düşünüyorum.
> - Bir çok kişi vim açıp yazıp txt veya md kaydediyor düşüncem şu bu araç linux
>   kurduğumda i3 hyprland sway tarzı tiling kullanan kişiler için kolay olmasını
>   düşünyorum.
> - Terminal açıp komut girerek veya direkt vscode gibi code . diyip
>   açabilecekleri bir şekilde düşünüyorum. Böylece bir iş yaprken bir anda oraya
>   veri todo girebilecek.
> - .config/proje-ismi/dosyalar şeklinde tutmayı düşnüyüorum
> - todo listemde ki şeyler 1 dosyada tutmak istiyorum böylece kişi dotfile içine
>   github'a yedekleyebilri oradak çebilir değişikleri diğer pclerine secron
>   edebilir.
> - linux ortamında ki takvime entegre olabilmesi lazım

Bu altı madde **tasarımın anayasası.** Bir özellik tartışılırken sorulacak soru:
*bu maddelerden hangisine hizmet ediyor?* Hiçbirine hizmet etmiyorsa v1'de yok.

### Maddeler nereye oturdu

| # | Ham madde | Karar |
|---|---|---|
| 1 | Rust + ratatui todo/planlayıcı | Temel. Tek binary, çevrimdışı |
| 2 | i3 / Hyprland / sway kullanıcıları | Hedef kitle. Palet (Catppuccin), tuş haritası ve v4'teki waybar modülü buradan geliyor |
| 3 | Komutla hızlı giriş, akışı bölmeden | `ratodo add "..."` → yazar, çıkar, TUI açılmaz. Ürünün varlık sebebi |
| 4 | `.config/proje-ismi/` | `~/.config/ratodo/todo.md`. XDG'den bilinçli sapma — gerekçesi `claude.md`'de |
| 5 | Tek dosya, dotfiles, GitHub, çoklu PC | Tek Markdown dosyası. **Senkron aracın işi değil** — kullanıcının git'i |
| 6 | Linux takvimine entegrasyon | Tek yönlü `todo.ics` (VTODO). Dosyayı üretiyoruz, abone etmek kullanıcının işi |

**4 ile 5 çelişiyordu**, 5 kazandı: XDG'ye göre kullanıcı verisi
`~/.local/share/`'a ait, ama kimse orayı dotfiles'a koymuyor.

---

## Reddedilenler ve nedenleri

Bunlar "sonra bakarız" değil, **bakıldı ve hayır denildi**. Yeniden tartışmaya
açmak için yeni bir bilgi gerekiyor.

| Ne | Neden hayır |
|---|---|
| TOML / JSON depolama | Parse'ı bedava ama elle düzenlenmiyor, `git diff` gürültülü. Madde 2'yi öldürüyor |
| SQLite depolama | Hızlı ama binary — `git diff` yok, vim'le açılmıyor. Madde 5'i öldürüyor |
| todo.txt standardı | Ekosistemi var ama tarih/tekrar desteği zayıf, takvim çıkışı için yetmiyor. Madde 6'yı öldürüyor |
| CalDAV çift yönlü senkron | ETag, çakışma çözümü, offline kuyruk, auth saklama. Tek başına bir alt-proje |
| Kanban / board görünümü | taskell zaten yapıyor ve iyi yapıyor |
| Bulut senkron / hesap | "Veri yerinde kalıyor" ürünün en güçlü cümlesi, geri alınmaz |
| `tokio` | Async'e ihtiyaç yok — tek yerel dosya, blocking IO yeterli |
| Tema yükleyici (TOML + hot reload) | YAGNI. 11 sabit bir `theme.rs` yeterli |
| İki görünüm kipi (ajanda / dosya) | İki kip = durum yönetimi + tuş çakışması + iki çizim yolu. Tek kip: ajanda |
| Strikethrough ile tamamlanmış görev | Terminal desteği tutarsız, yarı kullanıcıda okunmaz oluyor |

---

## Açık sorular

- [ ] **`ratodo` müsait mi?** İsim karara bağlandı (ratatui + todo), ama
      müsaitlik kontrol edilmedi. Çakışma çıkarsa yedek: `tuido`.
      - [ ] crates.io'da müsait mi?
      - [ ] GitHub'da aynı isimde belirgin bir proje var mı?
      - [ ] `command -v ratodo` — yaygın dağıtımlarda çakışıyor mu?
- [ ] **README'nin ilk cümlesi:** *"A todo TUI, built **with** ratatui"* —
      `for` değil. İsim akrabalığı ratatui eklentisi sanılma riski taşıyor,
      ilk cümle bunu kapatmalı.
- [ ] Tamamlanan görev yerinde mi kalsın, `## Done`'a mı taşınsın?
      Yerinde kalırsa dosya şişiyor; taşınırsa her tamamlamada `git diff`
      iki satır oynatıyor. *Sezgi: v1'de yerinde kalsın, v2'de `ratodo archive`.*
- [ ] Çoklu liste için `--file` yetiyor mu, isimli liste kavramı mı lazım?
      *Önce `--file` ile yaşayıp görelim.*
- [ ] `ratodo add` her çağrıldığında `.ics` yenilensin mi, yoksa sadece TUI
      kapanışında mı? *Sezgi: her `add`'de — basit, ve dosya küçük.*
- [ ] `- [ ]` dışında `* [ ]` ve `+ [ ]` de tanınsın mı? (Markdown hepsini
      liste sayıyor.) *Sezgi: okurken tanı, yazarken hep `- [ ]` kullan.*

---

## Yapılacaklar — v1

Sıralama kasıtlı: **2. adım biterse elinde çalışan bir CLI todo var** (çirkin ama
işleyen). 4. adım tıkanırsa proje ölmüyor.

### 0 — Kurulum
- [ ] Rust (rustup) — Linux / WSL
- [ ] `git init` (bu klasör henüz depo değil)
- [ ] Truecolor terminal doğrula: `printf "\x1b[38;2;203;166;247mmauve\x1b[0m\n"`
- [ ] khal veya Thunderbird (`.ics` doğrulaması için)
- [ ] `ratodo` müsaitliğini doğrula (aşağıdaki açık soru), sonra
      `cargo init --name ratodo`

### 1 — Fixture'lar (terminal gerekmiyor)
- [ ] `tests/fixtures/simple.md` — düzgün, sıradan bir liste
- [ ] `tests/fixtures/gnarly.md` — kasten zor olan (taslak aşağıda)
- [ ] Her fixture için beklenen `Vec<Task>` çıktısını yaz

### 2 — parse + write (terminal gerekmiyor) ← ürünün kalbi
- [ ] `model.rs`: `Task { raw: String, line_no: usize, done, title, due, tags, priority, dirty: bool }`
- [ ] `parse.rs`: satır → `Task`. **Ham satır her zaman saklanır**
- [ ] `write.rs`: `dirty == false` ise ham satırı olduğu gibi yaz
- [ ] Atomik yazma: temp dosya → `fsync` → `rename`, öncesinde `.bak`
- [ ] mtime kontrolü — okuduğumuzdan beri değiştiyse yazma, uyar
- [ ] **Round-trip testi:** `parse(write(parse(x))) == parse(x)`
- [ ] **Sadakat testi:** dokunulmamış her satır byte-byte aynı
- [ ] `ratodo list` → `println!`. Burada ürün çalışıyor olacak

### 3 — agenda + ics (terminal gerekmiyor)
- [ ] `agenda(&[Task], today) -> Vec<Group>` — `today` **parametre**,
      `Local::now()` fonksiyonun içinde değil (yoksa test edilemez)
- [ ] Grup testleri: gecikmiş / bugün / bu hafta / sonra / tarihsiz
- [ ] Sınır testleri: tam bugün 00:00, tam +7 gün, geçmiş yıl, geçersiz tarih
- [ ] `ics.rs`: VTODO çıktısı (~30 satır string biçimlendirme, crate yok)
- [ ] Snapshot testi + **gerçek doğrulama:** çıktıyı khal'e ver, okunuyor mu

### 4 — ratatui (asıl yeni olan kısım)
- [ ] **Panic hook ilk gün yazılsın** — ham modda panikleyen TUI kullanıcının
      terminalini bozuk bırakır
- [ ] Aptal liste: görev başlıklarını bas, `↑↓`, `q` ile çık
- [ ] Olay döngüsü: `crossterm::event::poll` + `notify` mpsc kanalı
- [ ] **Sabit FPS yok** — olay geldiğinde çiz, boştayken blokla (idle'da %0 CPU)
- [ ] inotify: dosya dışarıdan değişince yeniden oku

### 5 — Birleştir + tasarımı uygula
- [ ] `theme.rs` (11 sabit)
- [ ] Gruplu ajanda çizimi, `○ ✓ !` sembolleri
- [ ] ASCII fallback: `[ ]` `[x]` `[!]`
- [ ] `a` ekle · `⏎` işaretle · `d` sil · `e` `$EDITOR` · `q` çık
- [ ] `clap`: `ratodo` · `ratodo add` · `ratodo list` · `ratodo done` · `ratodo sync`
- [ ] README: khal ve Thunderbird için `.ics` abone olma adımları

---

## Fixture taslakları

### `tests/fixtures/gnarly.md` — kasten zor olan

Parser'ın **hiçbirini bozmaması** gereken şeyler. Her satır ayrı bir tuzak:

```markdown
# Benim listem

Bu bir paragraf. Görev değil, dokunulmayacak.

## İş
- [ ] deploy planını yaz @2026-08-12 #ops !high
- [X] eski PR'ları kapat          <- büyük X
* [ ] yıldızlı liste öğesi        <- - yerine *
  - [ ] girintili alt görev       <- girinti korunmalı
- [ ]    üç boşluklu başlık       <- fazladan boşluk korunmalı
- [ ] geçersiz tarih @2026-13-45  <- ayrıştırılamamalı, satır bozulmamalı
- [ ] üç etiket #a #b #c @2026-09-01
- [ ] çöp @ ve # tek başına
- [ ] Türkçe karakter: şğüöçİI ✓ emoji 🚀

> Alıntı satırı. Dokunma.

| tablo | var |
|-------|-----|
| bunu  | da  |

## Kişisel
- [ ] tarihsiz görev
- [x] bitmiş görev

---
Son satırdan sonra newline var mı yok mu — ikisi de korunmalı.
```

Beklenen davranış:
- Görev sayılanlar: `- [ ]`, `- [x]`, `- [X]`, `* [ ]`, girintili olan
- **Değişmeden kalanlar:** başlık, paragraf, alıntı, tablo, `---`, boş satırlar
- `@2026-13-45` → `due = None`, ama satır aynen yazılır
- Kullanıcı sadece bir görevi tamamlarsa, dosyadaki **diğer 20 satır byte-byte aynı**

### `tests/fixtures/simple.md`

Kök dizindeki `todo.md` bunun kendisi — format örneği ve ilk fixture aynı dosya.

---

## Bilinen riskler

| Risk | Etki | Ne yapıyoruz |
|---|---|---|
| **Round-trip sadakati bozulursa** | Kullanıcının el yazması dosyası bozulur → güven biter, araç silinir. **En kritik risk** | Ham satır saklama + byte-byte test + `.bak` + atomik yazma |
| Truecolor yok (TTY, eski `screen`) | Renkler bozuk | Şikâyet gelirse `COLORTERM` bakıp 16 renge düş (~5 satır) |
| GNOME Takvim yerel `.ics` okumuyor | Madde 6 yarım kalmış hissi | README'de dürüst ol: khal ✅ Thunderbird ✅ GNOME ⚠️ Google ❌ |
| Google Calendar VTODO'yu yok sayıyor | "Takvimimde görünmüyor" şikâyeti | v2'de `--as-events` bayrağı |
| İsim ratatui'nin alt-projesi sanılır | Bağımsız ürün olarak görülmez, "ratatui eklentisi" diye geçer | README'nin ilk cümlesi: *"built **with** ratatui"*. Logo fare değil, **checklist tutan fare** |
| `ratodo` müsait değilse | İsim baştan alınır, `~/.config/` yolu ve paket adı değişir | Kod yazılmadan önce kontrol. Yedek hazır: `tuido` |
| `ratodo` 6 harf, günde 20 kez yazılıyor | Sürtünme — `jot`'un 3 harfi buydu | `alias r=ratodo` README'de örnek olarak verilecek |
| Kapsam kayması | **Projeyi öldüren asıl şey** | `claude.md`'deki "Kapsam dışı" listesi. Her yeni fikir önce oraya bakacak |

---

## Fikir çöplüğü

v1'de **yok** ama atılmadı. Buraya yazılıyor ki kafanın içinde dolaşıp
v1'i şişirmesin.

- `ratodo status --json` → **waybar / eww modülü.** Bar'da "3 open · 1 overdue".
  Hedef kitle için muhtemelen en büyük tek kazanç. v4.
- `notify-send` ile gecikmiş görev bildirimi. v4.
- `ratodo done "fatura"` — bulanık eşleme ile TUI açmadan işaretle.
- `ratodo log` — "bugün ne bitirdim". Haftalık rapor yazanlar için.
- `ratodo undo` — son değişikliği `.bak`'tan geri al.
- Otomatik git commit (`--commit` bayrağı). Cazip ama kullanıcının git'ine
  karışmak tehlikeli; opt-in bile olsa dikkatli.
- tmux popup / Hyprland scratchpad binding örneği — README'ye örnek config.
- fzf ile görev seçme (`ratodo done $(ratodo list | fzf)`). `--porcelain` çıktı
  formatı gerektirir.
- `~2026-09-01` erteleme sözdizimi (bu tarihe kadar gizle). v3.
- Şifreli liste — **hayır.** Dosya düz metin kalmalı, ürünün tüm mantığı bu.
