// ── Наряд №287 (P2, testing/docs): doc-тесты документации ──
//
// rustdoc doc-tests-паттерн для Metalogos: каждый ```mlog-блок
// REFERENCE/README/docs — кандидат на исполнение; докачанная ложь
// невозможна (verified docs — грант-аргумент Testing Evidence).
//
//   mlog test --docs [GLOB...]        # дефолт: REFERENCE.md README.md docs/**/*.md
//
// ── Контракт блока (docs/doc-tests.md — SSOT) ──────────────────────
//   (без маркера)          — блок обязан исполниться без ошибки;
//   // expect: <значение>  — последняя строка output программы равна
//                            ожиданию (поддержан на flow-блоках);
//   // expect-error: <код?>— исполнение обязано упасть; опциональный
//                            код проверяется как substring ошибки;
//   // no-run              — только парсинг без исполнения.
//
// ── Read-only профиль исполнения ───────────────────────────────────
// 1. ephemeral cwd: весь прогон — в tempdir (все файловые/db-эффекты
//    doc-блоков изолированы — «нет побочек на CI-машине»);
// 2. сетевые/exec-билтины (http_*, smtp_*, imap_*, mcp_*, exec, exec_argv)
//    заменяются заглушками с громким отказом [DOC_SANDBOX] — подмена
//    ЛОКАЛЬНА для интерпретатора (реестр SSOT нетронут);
// 3. write_file-эскейпы наружу — громко штатной песочницей №131/№252;
// 4. call_llm/call_claude — mock-бэкенд (без ключей сеть не трогают).
//
// ── Классификация блока ────────────────────────────────────────────
//   содержит flow        → программа: полный запуск;
//   declarations-only    → parse + регистрация без исполнения
//                          (pattern/learnable/entity/import/config);
//   иначе (фрагмент)     → обёртка pattern __DocTest + flow Main.
//
// Паритет бэкендов: --backend vm исполняет через compiler+VM; ошибка
// КОМПИЛЯЦИИ VM = skip (ADR-0105: VM experimental, полный язык не
// покрывает), ошибка РАНТАЙМА VM = fail.

use crate::interpreter::{Interpreter, Value};

use std::path::{Path, PathBuf};

/// Ephemeral каталог (мини-tempdir без внешних зависимостей —
/// FEATURE_INTAKE: doc-тесты не добавляют crate). Уникальность —
/// pid + счётчик; Drop удаляет дерево рекурсивно (fail-silent, как
/// tempfile). Для изоляции doc-блоков этого достаточно.
struct EphemeralDir {
    path: PathBuf,
}

impl EphemeralDir {
    fn new(tag: &str) -> Result<Self, String> {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let base = std::env::temp_dir();
        let path = base.join(format!(
            "mlog-doc-tests-{}-{}-{}",
            tag,
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("doc-tests: cannot create {}: {e}", path.display()))?;
        Ok(EphemeralDir { path })
    }
}

impl Drop for EphemeralDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

// ── Модель ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocBackend {
    Tw,
    Vm,
}

/// Один ```mlog-блок с семантическим якорем «файл: секция: блок №k»
/// (строки меняются — якорь не).
#[derive(Debug, Clone)]
pub struct DocBlock {
    pub file: String,
    pub section: String,
    pub index: usize, // № mlog-блока в файле, 1-based
    pub body: String,
}

impl DocBlock {
    pub fn anchor(&self) -> String {
        format!("{}: {}: block #{}", self.file, self.section, self.index)
    }
}

/// Результат прогона doc-тестов (формат отчёта issue #342:
/// N извлечено / M исполнено / K no-run / S ошибок).
#[derive(Debug, Default)]
pub struct DocReport {
    pub files: usize,
    pub extracted: usize,
    pub executed: usize,
    pub no_run: usize,
    pub expect_error: usize,
    pub skipped: usize,
    pub failures: Vec<String>, // якорь + причина
}

impl DocReport {
    /// Однострочная сводка (stderr/CI-лог).
    pub fn summary(&self) -> String {
        format!(
            "doc-tests: {} files, {} extracted, {} executed, {} no-run, {} expect-error, {} skipped, {} failures",
            self.files, self.extracted, self.executed, self.no_run, self.expect_error,
            self.skipped, self.failures.len()
        )
    }
}

// ── Извлечение ──────────────────────────────────────────────────────

/// Извлечь все ```mlog-блоки из markdown. Фенсы без языка и ```json
/// пропускаются ВЫШЕ (вызова) — здесь только mlog. Якорь секции —
/// ближайший markdown-заголовок `#`..`######` над фенсом.
pub fn extract_doc_blocks(markdown: &str, file_label: &str) -> Vec<DocBlock> {
    let mut blocks = Vec::new();
    let mut section = String::new();
    let mut index: usize = 0;
    let lines: Vec<&str> = markdown.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // Заголовок секции (семантический якорь).
        if let Some(rest) = strip_heading(line) {
            section = rest.trim().to_string();
            i += 1;
            continue;
        }
        // Открытие фенса: ```mlog (язык после ``` — ровно mlog).
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            let lang = trimmed.strip_prefix("```").unwrap_or("").trim();
            if lang == "mlog" {
                index += 1;
                let mut body: Vec<&str> = Vec::new();
                i += 1;
                while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                    body.push(lines[i]);
                    i += 1;
                }
                blocks.push(DocBlock {
                    file: file_label.to_string(),
                    section: section.clone(),
                    index,
                    body: body.join("\n"),
                });
            }
        }
        i += 1;
    }
    blocks
}

fn strip_heading(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
        Some(&trimmed[hashes..])
    } else {
        None
    }
}

/// Разрешить globs/файлы: пустой список → дефолт (REFERENCE.md,
/// README.md, docs/**/*.md); элемент с "**" → рекурсивный *.md-walk
/// от указанного каталога; иначе — literal-файл. Sorted, dedup.
pub fn resolve_doc_files(root: &Path, globs: &[String]) -> Vec<PathBuf> {
    if globs.is_empty() {
        return default_doc_files(root);
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for g in globs {
        if g.contains("**") {
            let (dir_part, _) = match g.split_once("**") {
                Some(p) => p,
                None => continue,
            };
            let dir = if dir_part.is_empty() {
                root.to_path_buf()
            } else {
                root.join(dir_part)
            };
            if dir.is_dir() {
                collect_markdown(&dir, &mut out);
            }
        } else {
            let p = root.join(g);
            if p.is_file() {
                out.push(p);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Дефолтные файлы: REFERENCE.md, README.md, docs/**/*.md (рекурсивно),
/// отсортированные — детерминизм отчёта.
pub fn default_doc_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for name in ["REFERENCE.md", "README.md"] {
        let p = root.join(name);
        if p.is_file() {
            out.push(p);
        }
    }
    // Дефолт — живая документация ЯЗЫКА: docs/book/** (руководство).
    // docs/adr/** и docs/research/** — исторические записи решений:
    // исполняются только по явному --docs glob (ADR фиксирует состояние
    // на момент решения; пиннинг их примеров — отдельное решение).
    let book = root.join("docs").join("book");
    if book.is_dir() {
        collect_markdown(&book, &mut out);
    }
    out.sort();
    out
}

fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out);
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            out.push(path);
        }
    }
}

// ── Маркеры блока ───────────────────────────────────────────────────

#[derive(Debug, Default)]
struct BlockMarkers {
    no_run: bool,
    skip: bool,
    expect_error: Option<Option<String>>, // Some(None) — без кода
    expect: Option<String>,
}

fn scan_markers(body: &str) -> BlockMarkers {
    let mut m = BlockMarkers::default();
    for line in body.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("//") {
            let rest = rest.trim_start();
            if let Some(code) = rest.strip_prefix("expect-error") {
                m.expect_error = Some(code.trim().strip_prefix(':').map(|c| c.trim().to_string()));
                continue;
            }
            if rest.starts_with("expect-error") {
                m.expect_error = Some(None);
                continue;
            }
            if let Some(v) = rest.strip_prefix("expect:") {
                m.expect = Some(v.trim().to_string());
                continue;
            }
            if rest.starts_with("no-run") {
                m.no_run = true;
                continue;
            }
            if rest.starts_with("doc-test: skip") || rest.starts_with("doc-test:skip") {
                m.skip = true;
            }
        }
    }
    m
}

// ── Исполнение ──────────────────────────────────────────────────────

macro_rules! doc_blocked_fn {
    ($fname:ident, $name:literal) => {
        fn $fname(_: &[Value]) -> Result<Value, String> {
            Err(format!(
                "[DOC_SANDBOX] {} is disabled in doc-tests (read-only profile: no network/exec)",
                $name
            ))
        }
    };
}

fn apply_doc_profile(interp: &mut Interpreter) {
    doc_blocked_fn!(b_http_get, "http_get");
    interp.override_builtin("http_get", b_http_get);
    doc_blocked_fn!(b_http_post, "http_post");
    interp.override_builtin("http_post", b_http_post);
    doc_blocked_fn!(b_http_post_multipart, "http_post_multipart");
    interp.override_builtin("http_post_multipart", b_http_post_multipart);
    doc_blocked_fn!(b_http_download, "http_download");
    interp.override_builtin("http_download", b_http_download);
    doc_blocked_fn!(b_smtp_send, "smtp_send");
    interp.override_builtin("smtp_send", b_smtp_send);
    doc_blocked_fn!(b_smtp_send_html, "smtp_send_html");
    interp.override_builtin("smtp_send_html", b_smtp_send_html);
    doc_blocked_fn!(b_imap_list, "imap_list");
    interp.override_builtin("imap_list", b_imap_list);
    doc_blocked_fn!(b_imap_read, "imap_read");
    interp.override_builtin("imap_read", b_imap_read);
    doc_blocked_fn!(b_imap_search, "imap_search");
    interp.override_builtin("imap_search", b_imap_search);
    doc_blocked_fn!(b_imap_mark_read, "imap_mark_read");
    interp.override_builtin("imap_mark_read", b_imap_mark_read);
    doc_blocked_fn!(b_mcp_call, "mcp_call");
    interp.override_builtin("mcp_call", b_mcp_call);
    doc_blocked_fn!(b_mcp_list_tools, "mcp_list_tools");
    interp.override_builtin("mcp_list_tools", b_mcp_list_tools);
    doc_blocked_fn!(b_exec, "exec");
    interp.override_builtin("exec", b_exec);
    doc_blocked_fn!(b_exec_argv, "exec_argv");
    interp.override_builtin("exec_argv", b_exec_argv);
}

/// Классификация блока по содержимому.
enum Kind {
    /// Полная программа (есть flow) — полный запуск.
    Program,
    /// Declarations-only (pattern/learnable/entity/import/config) —
    /// parse + регистрация, исполнения нет.
    Decls,
    /// Фрагмент statements — обёртка pattern + flow.
    Fragment,
}

fn classify(body: &str) -> Kind {
    if body.contains("flow ") {
        return Kind::Program;
    }
    for line in body.lines() {
        let t = line.trim_start();
        for kw in [
            "pattern ",
            "learnable ",
            "entity ",
            "config ",
            "fluid ",
            "reflex ",
            "reflex_seq ",
            "reflex_gen ",
            "sandbox ",
            "rule ",
            "hook ",
            "db ",
            "schema ",
            "skill_index ",
            "memory ",
            "mlogserver ",
            "server ",
            "template ",
            "tool ",
            "eval ",
            "mutate ",
            "adapt ",
            "memorize ",
            "forget ",
            "relate ",
            "conversation ",
            "llm ",
            "test ",
            "type ",
            "context_budget ",
            "vision ",
            "goal ",
            "todo ",
            // №325: the compatibility profile is a declaration.
            "profile ",
        ] {
            if t.starts_with(kw) {
                return Kind::Decls;
            }
        }
    }
    Kind::Fragment
}

/// Собрать исполняемый source из блока.
fn build_source(body: &str, kind: &Kind) -> String {
    match kind {
        Kind::Program | Kind::Decls => body.to_string(),
        Kind::Fragment => {
            // Import-строки фрагмента поднимаются на верхний уровень
            // (import — declaration, внутри pattern невалиден; доки
            // часто показывают import посреди примера).
            let mut imports = Vec::new();
            let mut rest = Vec::new();
            for line in body.lines() {
                if line.trim_start().starts_with("import ") {
                    imports.push(line.trim().to_string());
                } else {
                    rest.push(line);
                }
            }
            let body_text = rest.join("\n");
            let imports_text = if imports.is_empty() {
                String::new()
            } else {
                format!("{}\n", imports.join("\n"))
            };
            format!(
                "{imports_text}pattern __DocTest(_input: String) -> String {{\n{body_text}\n  return \"\"\n}}\n\nflow Main {{ input: String = \"x\" -> __DocTest -> output }}"
            )
        }
    }
}

/// Исполнить собранный source. cwd вызывающего должен быть ephemeral
/// (run_doc_tests ставит tempdir). Returns flow output.
fn execute_source(
    source: &str,
    backend: DocBackend,
    base_dir: &Path,
) -> Result<Option<String>, String> {
    match backend {
        DocBackend::Tw => {
            let declarations =
                crate::parser::parse(source).map_err(|e| format!("parse error: {e}"))?;
            let mut interp = Interpreter::new();
            // base_dir — РЕЗОЛЮЦИЯ import/std (репо-корень); файловые
            // эффекты билтинов идут от cwd (ephemeral tempdir вызывающего).
            interp.set_base_dir(base_dir.to_path_buf());
            apply_doc_profile(&mut interp);
            interp.run(declarations)
        }
        DocBackend::Vm => {
            let declarations =
                crate::parser::parse(source).map_err(|e| format!("parse error: {e}"))?;
            let mut comp = crate::compiler::Compiler::with_std_root(base_dir.to_path_buf());
            let program = comp
                .compile(declarations)
                .map_err(|e| format!("vm-compile: {e}"))?;
            let mut vm = crate::vm::Vm::new();
            vm.run(program)
        }
    }
}

// ── Прогон ──────────────────────────────────────────────────────────

/// Прогнать doc-тесты по списку файлов. Каждому блоку — свежий
/// ephemeral cwd (tempdir): идемпотентность блоков, изоляция побочек.
/// `root` — база import/std-резолюции (репо-корень); cwd блоков —
/// ephemeral tempdir (файловые/db-эффекты туда, не на машину).
pub fn run_doc_tests(
    files: &[PathBuf],
    backend: DocBackend,
    root: &Path,
) -> Result<DocReport, String> {
    let mut report = DocReport {
        files: files.len(),
        ..Default::default()
    };
    let prev_cwd = std::env::current_dir().map_err(|e| format!("doc-tests: cwd: {e}"))?;
    struct CwdGuard(std::path::PathBuf);
    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }
    let _guard = CwdGuard(prev_cwd);

    for file in files {
        let label = file
            .strip_prefix(std::env::current_dir().unwrap_or_default())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| file.display().to_string());
        let markdown = match std::fs::read_to_string(file) {
            Ok(m) => m,
            Err(e) => {
                report
                    .failures
                    .push(format!("{label}: cannot read file: {e}"));
                continue;
            }
        };
        for block in extract_doc_blocks(&markdown, &label) {
            report.extracted += 1;
            let markers = scan_markers(&block.body);
            let anchor = block.anchor();

            // doc-test: skip — полный пропуск (напр. research-эскизы
            // нереализованного синтаксиса); учитывается в счётчике.
            if markers.skip {
                report.skipped += 1;
                continue;
            }

            // Свежий ephemeral cwd на КАЖДЫЙ блок (идемпотентность,
            // изоляция побочек); возвращение cwd — CwdGuard выше.
            let dir = EphemeralDir::new("block")?;
            std::env::set_current_dir(&dir.path).map_err(|e| format!("doc-tests: chdir: {e}"))?;

            // no-run: только парсинг (в обоих бэкендах).
            if markers.no_run {
                let source = build_source(&block.body, &classify(&block.body));
                match crate::parser::parse(&source) {
                    Ok(_) => {
                        report.no_run += 1;
                        continue;
                    }
                    Err(e) => {
                        report
                            .failures
                            .push(format!("{anchor}: no-run block failed to parse: {e}"));
                        continue;
                    }
                }
            }

            let kind = classify(&block.body);
            let source = build_source(&block.body, &kind);
            match execute_source(&source, backend, root) {
                Ok(output) => {
                    // Мягкая ошибка интерпретатора: вызов неизвестной
                    // функции возвращает строку "[ERROR: …]" — для доков
                    // это провал (тихая ложь недопустима).
                    if output.as_deref().unwrap_or("").contains("[ERROR: ") {
                        report.failures.push(format!(
                            "{anchor}: soft runtime error in output: {}",
                            output.unwrap_or_default()
                        ));
                        continue;
                    }
                    if let Some(expected) = &markers.expect {
                        // expect: последняя строка output == ожиданию.
                        let last = output
                            .as_deref()
                            .unwrap_or("")
                            .lines()
                            .rev()
                            .find(|l| !l.trim().is_empty())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if last != *expected {
                            report
                                .failures
                                .push(format!("{anchor}: expect '{expected}', got '{last}'"));
                            continue;
                        }
                    }
                    match &markers.expect_error {
                        // Ожидаемая ошибка не случилась.
                        Some(_) => {
                            report.failures.push(format!(
                                "{anchor}: expect-error, but the block executed without error"
                            ));
                        }
                        None => report.executed += 1,
                    }
                }
                Err(err) => {
                    // VM-компиляция: известные ограничения ADR-0105 — skip.
                    if backend == DocBackend::Vm && err.starts_with("vm-compile:") {
                        report.skipped += 1;
                        continue;
                    }
                    match &markers.expect_error {
                        Some(code) => {
                            if let Some(code) = code {
                                if !err.contains(code.as_str()) {
                                    report.failures.push(format!(
                                        "{anchor}: expect-error code '{code}' not found in: {err}"
                                    ));
                                    continue;
                                }
                            }
                            report.expect_error += 1;
                        }
                        None => {
                            report.failures.push(format!("{anchor}: {err}"));
                        }
                    }
                }
            }
        }
    }
    Ok(report)
}
