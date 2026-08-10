# todo

Bu dosya iki iş görüyor:

1. **Format örneği** — `claude.md`'deki sözdizimi tablosunun çalışan hali.
   Kullanıcının `~/.config/ratodo/todo.md` dosyası tam böyle görünecek.
2. **İlk test fixture'ı** — `tests/fixtures/simple.md` olarak kopyalanacak.

Bu paragrafın kendisi de bir testtir: araç tanımadığı satırlara dokunmaz,
dosyayı yeniden yazdığında bunlar byte-byte aynı kalır.

Referans "bugün": **2026-08-10**

## Ops

- [ ] rotate the backup keys @2026-08-08 #ops !high
- [ ] review the deploy PR @2026-08-10 16:00 #work
- [ ] sunucu taşımayı planla @2026-09-01 #ops
- [x] eski PR'ları kapat #work

## Home

- [ ] pay the invoice @2026-08-10 #home
- [ ] book a dentist appointment @2026-08-14 09:30 #health
- [ ] fatura öde @2026-08-17 #ev !med
- [ ] tarihsiz bir şey, ne zaman olursa
- [x] migrate the server #ops

## Someday

- [ ] Rust kitabının 13. bölümünü bitir !low
- [ ] klavye firmware'ini güncelle #hobby

---

> Bu alıntı da korunacak. Aşağıdaki tablo da.

| Beklenen grup | Hangi görevler |
|---|---|
| OVERDUE | rotate the backup keys (2 gün gecikmiş) |
| TODAY | review the deploy PR (16:00), pay the invoice |
| THIS WEEK | book a dentist appointment (14 Ağu), fatura öde (17 Ağu) |
| LATER | sunucu taşımayı planla (1 Eyl) |
| *(tarihsiz)* | dosyadaki `##` bölümleri altında, dosya sırasıyla |
| *(tamamlanmış)* | eski PR'ları kapat, migrate the server |
