# Наряд MLG-3: PDF-функции для офисной автоматизации — полный цикл

Проект: Metalogos (ShkodnikAI/Metalogos-)
Слой: core (pdf-модуль builtin-функций)
Дата: 2026-08-12
Версия наряда: 1
База: HEAD 5ec0a3e (Наряд MLG-2: PDF manipulation на чистом Rust)
Зависимость: наряды MLG-1 и MLG-2 приняты; наряд MLG-3 расширяет их

## Цель прохода

Расширить Metalogos v0.12.0 полноценным PDF-стеком для офисной автоматизации:
1. **Дополнить** существующие PDF-функции новыми возможностями (таблицы, изображения, колонтитулы, нумерация страниц, водяные знаки)
2. **Улучшить** `html_to_pdf` — переход с внешнего `wkhtmltopdf` на чистый Rust
3. **Добавить** `pdf_fill_form` — заполнение PDF-форм (AcroForm)
4. **Покрыть тестами** — интеграционные + контрактные + arity
5. **Обновить** бинарник `mlog`, git commit + push

Грантовая стратегия: язык должен быть **самодостаточным** — без Python-прокси, без внешних CLI-утилит, чистый Rust.

## Текущее состояние (после MLG-2)

### Уже реализовано (pdf.rs, 1534 строки):
| Функция | Arity | Реализация | Крейт |
|---------|-------|------------|-------|
| `pdf_classify(path)` | 1 | Полная | pdf-inspector |
| `pdf_to_markdown(path)` | 1 | Полная | pdf-inspector |
| `pdf_extract_regions(path, regions_json)` | 2 | Полная | pdf-inspector |
| `pdf_ocr(path)` | 1 | Feature-gated (pdf-ocr) | tesseract |
| `pdf_create()` | 0 | Полная | lopdf |
| `pdf_add_page(id, width, height)` | 3 | Полная | lopdf |
| `pdf_write_text(id, x, y, text [,font, size])` | 4..6 | Полная | lopdf |
| `pdf_draw_line(id, x1, y1, x2, y2 [,width])` | 5..6 | Полная | lopdf |
| `pdf_draw_rect(id, x, y, w, h [,stroke, fill])` | 5..7 | Полная | lopdf |
| `pdf_save(id, path)` | 2 | Полная | lopdf |
| `pdf_merge(paths_json, output)` | 2 | Полная | lopdf |
| `pdf_split(path, ranges_json, output_dir)` | 3 | Полная | lopdf |
| `pdf_metadata(path)` | 1 | Полная | lopdf |
| `pdf_set_metadata(path, key, value)` | 3 | Полная | lopdf |
| `html_to_pdf(html, path)` | 2 | Через wkhtmltopdf | внешний CLI |
| `send_document(chat_id, file_path [,caption])` | 2..3 | Полная | reqwest |

### Что отсутствует / требует улучшения:
| Что | Почему важно |
|-----|-------------|
| `pdf_draw_table` | Офисные документы = таблицы. Нет таблиц — нет автоматизации |
| `pdf_add_image` | Логотипы, подписи, сканы — базовая потребность |
| `pdf_set_page_header` / `pdf_set_page_footer` | Колонтитулы — стандарт требование к документам |
| `pdf_page_numbers` | Нумерация страниц — обязательна для официальных документов |
| `pdf_watermark` | Водные знаки — «ДЛЯ СЛУЖЕБНОГО ПОЛЬЗОВАНИЯ», «КОНФИДЕНЦИАЛЬНО» |
| `pdf_fill_form` | Заполнение готовых PDF-форм (AcroForm) — налоговые, договоры |
| `html_to_pdf` на чистом Rust | Зависимость от wkhtmltopdf нарушает грантовую стратегию |
| `pdf_rotate_page` / `pdf_delete_page` | Манипуляция страницами — базовый набор |
| `pdf_extract_images` | Извлечение изображений из PDF — обратная операция |
| Тесты для новых функций | Без тестов — нет гарантии работоспособности |

## Запреты прохода

- **НЕ** ломать существующие функции (pdf_classify, pdf_to_markdown, pdf_create и т.д.)
- **НЕ** добавлять Python-зависимости (PyPDF2, weasyprint, reportlab)
- **НЕ** менять arity существующих функций
- **НЕ** менять порядок в BUILTIN_REGISTRY (bytecode-индексы)
- **НЕ** трогать grammar.pest
- **НЕ** добавлять новые feature-флаги без веской причины

## Перед началом

1. Ветка: `git checkout -b feat/mlg3-pdf-office` (от HEAD 5ec0a3e)
2. Лог: `docs/naryads/2026-08-12-pdf-office-automation/report.md`
3. Проверить: `cargo build --release` компилируется, существующие тесты проходят

## Блок A. Новые PDF-функции создания

### A.1. `pdf_draw_table(id, x, y, col_widths_json, rows_json [,style_json])`

Arity: 5..6. Рисует таблицу на текущей странице документа.

- `id` — строка, идентификатор документа (из pdf_create)
- `x`, `y` — координаты верхнего левого угла таблицы (points)
- `col_widths_json` — JSON-массив ширин столбцов, e.g. `[150, 100, 200]`
- `rows_json` — JSON-массив массивов, e.g. `[["Name","Age","City"],["Alice","30","Moscow"]]`
- `style_json` (опционально) — JSON-объект: `{"font":"Helvetica","font_size":10,"border":true,"header_bg":"0.9,0.9,0.9"}`

Реализация: итерация по rows, отрисовка ячеек через lopdf text operators + rect operators. Заголовок строки (row 0) выделяется полужирным шрифтом и/или фоном.

Регистрация:
- registry.rs: `spec!("pdf_draw_table", 5, 6, "pdf")`
- mod.rs: `funcs.insert("pdf_draw_table".to_string(), builtin_pdf_draw_table as BuiltinFn);`

### A.2. `pdf_add_image(id, x, y, image_path [,width, height])`

Arity: 4..6. Вставляет изображение (PNG/JPEG) на текущую страницу.

- `id` — идентификатор документа
- `x`, `y` — координаты нижнего левого угла (PDF coordinate system)
- `image_path` — путь к файлу PNG или JPEG
- `width`, `height` (опционально) — целевые размеры; если опущены — использовать intrinsic

Реализация через lopdf: создать XObject (Image stream), добавить в Resources страницы, отрисовать через `Do` operator. Поддержка PNG (декодирование через `png` crate) и JPEG (pass-through).

Зависимость: добавить `png = "0.17"` в Cargo.toml (или `image = "0.25"` для универсальности).

Регистрация:
- registry.rs: `spec!("pdf_add_image", 4, 6, "pdf")`
- mod.rs: `funcs.insert("pdf_add_image".to_string(), builtin_pdf_add_image as BuiltinFn);`

### A.3. `pdf_set_page_header(id, text [,font, size])` / `pdf_set_page_footer(id, text [,font, size])`

Arity: 2..4. Устанавливает колонтитул для всех страниц документа (включая будущие).

Реализация: сохранить header/footer текст в PdfDocument struct (добавить поля `header: Option<PdfElement>`, `footer: Option<PdfElement>`). При `pdf_save` — рендерить колонтитул на каждую страницу.

Изменения в PdfDocument:
```rust
struct PdfDocument {
    title: String,
    author: String,
    pages: Vec<PdfPage>,
    header: Option<PdfElement>,  // NEW
    footer: Option<PdfElement>,  // NEW
}
```

Регистрация:
- registry.rs: `spec!("pdf_set_page_header", 2, 4, "pdf")`, `spec!("pdf_set_page_footer", 2, 4, "pdf")`
- mod.rs: соответствующие insert

### A.4. `pdf_page_numbers(id [,format, x, y])`

Arity: 1..4. Включает автоматическую нумерацию страниц при сохранении.

- `format` (опционально) — строка формата: `"page N"`, `"N/M"`, `"N of M"`. По умолчанию: `"N/M"`
- `x`, `y` (опционально) — координаты размещения. По умолчанию: по центру нижнего поля

Реализация: добавить `page_number_format: Option<String>` и `page_number_pos: Option<(f64, f64)>` в PdfDocument. При save — вторым проходом проставить номера.

Регистрация:
- registry.rs: `spec!("pdf_page_numbers", 1, 4, "pdf")`

### A.5. `pdf_watermark(id, text [,font, size, opacity])`

Arity: 2..5. Добавляет диагональный водяной знак на все страницы.

- `opacity` — от 0.0 до 1.0, по умолчанию 0.3
- Текст рисуется по диагонали (45°), по центру страницы

Реализация: при save — для каждой страницы добавить текстовый оператор с `Tm` (text matrix) для поворота + `gs` (graphics state) для прозрачности (ExtGState /ca).

Регистрация:
- registry.rs: `spec!("pdf_watermark", 2, 5, "pdf")`

## Блок B. Манипуляция существующими PDF

### B.1. `pdf_fill_form(path, fields_json, output_path)`

Arity: 3. Заполняет AcroForm-поля в существующем PDF.

- `path` — путь к PDF с формой
- `fields_json` — JSON-объект: `{"field_name": "value", ...}`
- `output_path` — куда сохранить заполненный PDF

Реализация через lopdf: найти AcroForm dictionary в корневом объекте, перечислить поля, найти совпадения по имени, подставить значения (V entry), обновить Appearance (AP) для визуального отображения.

Регистрация:
- registry.rs: `spec!("pdf_fill_form", 3, "pdf")`

### B.2. `pdf_rotate_page(path, page_number, degrees, output_path)`

Arity: 4. Поворачивает указанную страницу на 90/180/270 градусов.

Регистрация:
- registry.rs: `spec!("pdf_rotate_page", 4, "pdf")`

### B.3. `pdf_delete_pages(path, pages_json, output_path)`

Arity: 3. Удаляет указанные страницы из PDF.

- `pages_json` — JSON-массив номеров страниц (1-indexed): `[2, 5, 7]`

Регистрация:
- registry.rs: `spec!("pdf_delete_pages", 3, "pdf")`

### B.4. `pdf_extract_images(path [,output_dir])`

Arity: 1..2. Извлекает все изображения из PDF.

- Возвращает JSON-массив путей к извлечённым файлам
- `output_dir` по умолчанию = директория исходного PDF

Реализация: пройти XObject-дерево, найти Image streams, декодировать и сохранить.

Регистрация:
- registry.rs: `spec!("pdf_extract_images", 1, 2, "pdf")`

## Блок C. Улучшение html_to_pdf

### C.1. Замена wkhtmltopdf на чистый Rust

Текущая реализация вызывает внешний `wkhtmltopdf` через `std::process::Command`. Это нарушает грантовую стратегию самодостаточности.

Варианты (выбрать наиболее практичный):
1. **printpdf** crate — чистый Rust, но нет HTML-рендеринга
2. **Кастомный мини-рендерер** — парсить упрощённый HTML (h1-h6, p, table, ul/ol, b/i/em) и рисовать через lopdf (уже есть)
3. **Гибридный подход** — сохранить wkhtmltopdf как fallback, но добавить базовый HTML→PDF на Rust для простых документов

Рекомендация: **вариант 3** — реализовать `html_to_pdf_rust` для простых HTML (без CSS, без JavaScript), при сложном HTML — возвращать ошибку с рекомендацией установить wkhtmltopdf. Это даёт:
- 80% офисных кейсов (отчёты, таблицы, служебные записки) — чистый Rust
- 20% (сложная вёрстка) — понятная ошибка, не молчаливый fallback

Добавить функцию `html_to_pdf` переписать: сначала попытка Rust-рендера, при ошибке — fallback на wkhtmltopdf (если доступен).

## Блок D. Тесты

### D.1. Интеграционные тесты

Создать `tests/phase_mlg3_pdf_office.rs`:

```rust
// Тесты:
// 1. test_pdf_draw_table_basic — создать PDF с таблицей, проверить что файл создан и > 0
// 2. test_pdf_draw_table_with_style — таблица со стилями
// 3. test_pdf_add_image_png — вставить PNG
// 4. test_pdf_add_image_jpeg — вставить JPEG
// 5. test_pdf_set_page_header_footer — колонтитулы
// 6. test_pdf_page_numbers — нумерация страниц (многостраничный PDF)
// 7. test_pdf_watermark — водяной знак
// 8. test_pdf_fill_form — заполнение формы (нужен fixture с AcroForm)
// 9. test_pdf_rotate_page — поворот страницы
// 10. test_pdf_delete_pages — удаление страниц
// 11. test_pdf_extract_images — извлечение изображений
// 12. test_pdf_merge_multi — слияние 3+ PDF
// 13. test_pdf_split_ranges — разделение по диапазонам
```

### D.2. Inline-тесты в pdf.rs

Добавить в `#[cfg(test)] mod tests`:
- `test_pdf_draw_table_basic`
- `test_pdf_add_image_file_not_found`
- `test_pdf_set_header_then_save`
- `test_pdf_watermark_basic`
- `test_pdf_page_numbers_format`
- `test_pdf_fill_form_no_fields`

### D.3. Registry-тесты

Новые функции автоматически покрываются `registry_sync_check.rs` и `registry_arity_check.rs`.
Убедиться, что после добавления spec!-записей тесты проходят.

### D.4. Контрактные тесты (examples/)

Создать `examples/p_pdf_office.mlog`:
```mlog
// Демонстрация полного PDF-стека
let doc = pdf_create()
pdf_add_page(doc.id, 595.28, 841.89)
pdf_set_page_header(doc.id, "Metalogos Office Automation Report")
pdf_page_numbers(doc.id, "N of M")
pdf_write_text(doc.id, 72, 750, "Q1 Financial Summary", "Helvetica-Bold", 18)
pdf_draw_table(doc.id, 72, 650, [150,100,100], [["Metric","Q1","Q2"],["Revenue","$1.2M","$1.5M"],["Costs","$0.8M","$0.9M"]])
pdf_watermark(doc.id, "DRAFT")
pdf_save(doc.id, "/tmp/metalogos_report.pdf")
print("PDF saved successfully")
```

## Блок E. Бинарник и релиз

### E.1. Сборка

```bash
cargo build --release
cp target/release/mlog download/mlog
```

### E.2. Обновление версии

В Cargo.toml (workspace.package.version): `0.12.0` → `0.13.0`

### E.3. Changelog

Добавить секцию в CHANGELOG.md:
```markdown
## [0.13.0] - 2026-08-12

### Added
- pdf_draw_table — таблицы в PDF (Наряд MLG-3)
- pdf_add_image — вставка PNG/JPEG изображений
- pdf_set_page_header / pdf_set_page_footer — колонтитулы
- pdf_page_numbers — автоматическая нумерация страниц
- pdf_watermark — водяные знаки
- pdf_fill_form — заполнение AcroForm-полей
- pdf_rotate_page — поворот страниц
- pdf_delete_pages — удаление страниц
- pdf_extract_images — извлечение изображений
- html_to_pdf улучшен: базовый рендер на чистом Rust с fallback на wkhtmltopdf

### Changed
- PdfDocument struct: добавлены поля header, footer, watermark, page_numbers
- html_to_pdf: приоритет Rust-рендера над wkhtmltopdf
```

## Блок F. Git commit + push

### F.1. Коммит

```bash
git add -A
git commit -m "Наряд MLG-3: PDF-функции для офисной автоматизации (таблицы, изображения, колонтитулы, формы, водяные знаки, чистый Rust html_to_pdf)"
```

### F.2. Push

```bash
git push origin feat/mlg3-pdf-office
```

Если нет прав на push в origin — сделать fork и push туда, затем создать PR.

## Порядок выполнения

| Шаг | Блок | Что | Ожидаемый результат |
|-----|------|-----|---------------------|
| 1 | — | git checkout -b feat/mlg3-pdf-office | Ветка создана |
| 2 | A.1 | pdf_draw_table | spec! + handler + inline test |
| 3 | A.2 | pdf_add_image | spec! + handler + png crate + inline test |
| 4 | A.3 | pdf_set_page_header / pdf_set_page_footer | PdfDocument расширение + handlers |
| 5 | A.4 | pdf_page_numbers | PdfDocument расширение + handler |
| 6 | A.5 | pdf_watermark | PdfDocument расширение + handler |
| 7 | B.1 | pdf_fill_form | spec! + handler (lopdf AcroForm) |
| 8 | B.2 | pdf_rotate_page | spec! + handler |
| 9 | B.3 | pdf_delete_pages | spec! + handler |
| 10 | B.4 | pdf_extract_images | spec! + handler |
| 11 | C.1 | html_to_pdf Rust-рендер | Улучшение существующей функции |
| 12 | D.1 | Интеграционные тесты | tests/phase_mlg3_pdf_office.rs |
| 13 | D.2 | Inline-тесты | Расширение mod tests в pdf.rs |
| 14 | D.3 | Registry-тесты | cargo test registry_sync_check + arity |
| 15 | D.4 | Контрактный пример | examples/p_pdf_office.mlog |
| 16 | E | Бинарник + версия + changelog | mlog v0.13.0 |
| 17 | F | Git commit + push | Ветка в origin |
| 18 | — | report.md | Отчёт о выполнении |

## Чек-лист сдачи

- [ ] Ветка feat/mlg3-pdf-office создана от HEAD 5ec0a3e
- [ ] pdf_draw_table: spec! + handler + inline test проходят
- [ ] pdf_add_image: spec! + handler + png crate + inline test проходят
- [ ] pdf_set_page_header / pdf_set_page_footer: handler + сохранение колонтитулов
- [ ] pdf_page_numbers: handler + нумерация на многостраничном PDF
- [ ] pdf_watermark: handler + диагональный текст с прозрачностью
- [ ] pdf_fill_form: handler + AcroForm заполнение
- [ ] pdf_rotate_page: handler + поворот 90/180/270
- [ ] pdf_delete_pages: handler + удаление страниц
- [ ] pdf_extract_images: handler + извлечение PNG/JPEG
- [ ] html_to_pdf: Rust-рендер для простых HTML + wkhtmltopdf fallback
- [ ] Все spec! записи добавлены в registry.rs
- [ ] Все dispatch insert добавлены в mod.rs
- [ ] cargo build --release — успех
- [ ] cargo test — все тесты проходят (включая registry_sync и arity)
- [ ] Бинарник mlog v0.13.0 собран
- [ ] CHANGELOG.md обновлён
- [ ] Git commit + push выполнены
- [ ] report.md заполнен

## Структура отчёта исполнителя (report.md)

```markdown
# Отчёт: MLG-3 PDF Office Automation
Дата, ветка, коммиты.

## Реализованные функции:
- [функция]: [краткое описание реализации, строки кода]

## Тесты:
- Inline: [количество] тестов, [результат]
- Интеграционные: [количество] тестов, [результат]
- Registry sync: [результат]
- Registry arity: [результат]

## Изменения в зависимостях:
- [новый crate]: [версия, причина]

## Изменения в структурах:
- PdfDocument: [новые поля]

## Версия:
- 0.12.0 → 0.13.0

## Git:
- Commit: [hash]
- Branch: feat/mlg3-pdf-office
- Push: [статус]

## Затруднения:
- [список или «нет»]

## Нерешённые вопросы:
- [если есть]
```
