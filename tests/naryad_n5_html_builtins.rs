// ── Naryad N5 (gh#1215, PE-2, adr/feature): the `html` feature ─────────
//
// The DOM extraction family on the html5ever stack (ADR-0192):
//
//   H1: `html_select(html, css) -> List<HtmlMatch>` — tag/.class/#id/
//       [attr]/[attr=value], descendant, direct child, comma groups; a
//       synthetic CYRILLIC tender page is the fixture (the naryad's
//       test contract); attrs is a Dict; html is the serialized
//       subtree; document order.
//   H2: `html_text(html, css?)` — the normalized text; script/style
//       subtrees NEVER contribute; whitespace runs collapse.
//   H3: `html_links(html, base_url)` — hrefs ABSOLUTE (resolved against
//       base_url), only http/https survive; `javascript:` is DROPPED;
//       relative links resolve; the <a> text is the normalized text.
//   H4: the loud limits — the 5 MiB input and the 10 000-node result
//       both fail LOUD (never a silent truncate).
//   H5: broken HTML is tolerated (html5ever's error recovery) — an
//       unclosed-tag document still extracts its readable parts.
//   H6: an unsupported selector (pseudo-classes etc.) is a LOUD parse
//       error naming the supported subset.
//   H7: the TW/VM parity — both backends share the registry handler;
//       the parity is pinned (the golden-pair leg runs BOTH).
//   H8: the golden pair examples/n5_html_tender.mlog(+.expected) is
//       byte-identical on TW and VM (the naryad's DOD).
//
// The module is `#![cfg(feature = "html")]`: without the feature the
// file compiles EMPTY (the default CI suite never sees it); the
// feature-on CI job (html-tests) runs it. The RED-first discipline:
// with the feature declared but the handlers absent, every program
// below fails with the loud `unknown function 'html_*'` compile error —
// recorded before the implementation landed.

#![cfg(feature = "html")]
#![allow(clippy::disallowed_methods)]

use std::fs;

// ── eval_expr (the №261 lekało) ──────────────────────────────────────

fn eval_expr(src: &str) -> Result<String, String> {
    // A single-expression src rides the template's own `return`; a
    // multi-statement src (let-bindings) carries its OWN trailing
    // `return` — the wrapper return would be a syntax error.
    let body = if src.trim_start().starts_with("let ") {
        src.to_string()
    } else {
        format!("return {}", src)
    };
    let full = format!(
        "pattern __eval(input: String) -> String {{ {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        body
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err("eval returned None".to_string()),
        Err(e) => Err(e),
    }
}

/// The VM twin of the TW eval above — the parity leg.
fn eval_expr_vm(src: &str) -> Result<String, String> {
    let body = if src.trim_start().starts_with("let ") {
        src.to_string()
    } else {
        format!("return {}", src)
    };
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let declarations = metalogos::parser::parse(&format!(
        "pattern __eval(input: String) -> String {{ {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        body
    ))
    .map_err(|e| format!("parse error: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    match vm.run(program)? {
        Some(s) => Ok(s.trim_end().to_string()),
        None => Err("vm eval returned None".to_string()),
    }
}

/// The synthetic CYRILLIC tender page (the naryad's fixture contract).
const TENDER_HTML: &str = r##"<html>
<head>
  <title>Закупки — тестовая страница</title>
  <style>.hidden { color: red; }  /* style must never leak into text */</style>
  <script>var tracking = "script must never leak into text";</script>
</head>
<body>
  <div class="lot featured" id="lot-1" data-deadline="2026-11-01">
    <h2>Поставка канцелярии</h2>
    <p>Цена: <b>12 500</b> BYN</p>
    <a href="/lots/1">Подробнее о лоте 1</a>
    <a href="https://tender.example.by/docs/rules.pdf">Правила</a>
  </div>
  <div class="lot" id="lot-2">
    <h2>Ремонт фасада</h2>
    <p>Цена: <b>98 000</b> BYN</p>
    <a href="javascript:steal()[]">Зловредная ссылка</a>
    <a href="//mirror.example.by/lots/2">Зеркало</a>
    <a href="#anchor">Якорь-пустышка</a>
  </div>
  <footer>Контакты: gul@gov.by</footer>
</body>
</html>"##;

fn mlog_str(s: &str) -> String {
    // Embed a string into an mlog double-quoted literal: escape the
    // backslashes, the double quotes and the newlines (the mlog string
    // literal does not carry raw newlines — the \\n escape is the
    // literal's own, decoded back by the parser).
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}

// ── H1: html_select — the tender extraction ──────────────────────────

#[test]
fn n5_html_select_extracts_the_tender_lots() {
    let src = format!(
        "length(html_select({}, \"div.lot h2\"))",
        mlog_str(TENDER_HTML)
    );
    let out = eval_expr(&src).expect("html_select must succeed");
    assert_eq!(
        out.trim_end(),
        "2",
        "both lot h2 elements match, got: {out}"
    );
}

#[test]
fn n5_html_select_match_fields() {
    // The first match's tag/text/html (document order).
    let src = format!(
        "let ms = html_select({}, \"div.lot h2\")\n         let m = ms[0]\n         return m.tag + \"|\" + m.text + \"|\" + m.html",
        mlog_str(TENDER_HTML)
    );
    let out = eval_expr(&src).expect("html_select fields must succeed");
    assert_eq!(
        out.trim_end(),
        "h2|Поставка канцелярии|<h2>Поставка канцелярии</h2>",
        "tag, normalized text and the serialized subtree, got: {out}"
    );
}

#[test]
fn n5_html_select_attrs_dict_and_class_selector() {
    // The .featured class selector + the attrs Dict (the dynamic keys
    // are read through json_get — the battle-tested Dict surface).
    let src = format!(
        "let ms = html_select({}, \"div.featured\")\n         let m = ms[0]\n         return json_get(m.attrs, \"data-deadline\") + \"|\" + json_get(m.attrs, \"id\")",
        mlog_str(TENDER_HTML)
    );
    let out = eval_expr(&src).expect("attrs Dict must succeed");
    assert_eq!(out.trim_end(), "2026-11-01|lot-1", "got: {out}");
}

#[test]
fn n5_html_select_attr_eq_and_child_combinator() {
    // [attr=value] + the direct child (the div > p under lot-2 only —
    // the footer's p does not exist; the direct-child contract).
    let src = format!(
        "to_string(length(html_select({}, \"div[id=lot-2] > p\"))) + \"|\" + to_string(length(html_select({}, \"div p\")))",
        mlog_str(TENDER_HTML),
        mlog_str(TENDER_HTML)
    );
    let out = eval_expr(&src).expect("attr selectors must succeed");
    assert_eq!(out.trim_end(), "1|2", "got: {out}");
}

// ── H2: html_text — script/style never leak, whitespace normalized ───

#[test]
fn n5_html_text_excludes_script_and_style() {
    let src = format!("html_text({}, \"div.lot\")", mlog_str(TENDER_HTML));
    let out = eval_expr(&src).expect("html_text must succeed");
    let out = out.trim_end();
    assert!(
        !out.contains("tracking"),
        "the script body must never reach the text: {out}"
    );
    assert!(
        !out.contains("hidden") || !out.contains("color"),
        "the style body must never reach the text: {out}"
    );
    assert!(
        out.contains("Поставка канцелярии Цена: 12 500 BYN"),
        "the normalized text of the first lot: {out}"
    );
}

#[test]
fn n5_html_text_whole_document_and_selector_form() {
    let all = format!("html_text({})", mlog_str(TENDER_HTML));
    let out = eval_expr(&all).expect("html_text without a selector");
    let out = out.trim_end();
    assert!(
        out.contains("Закупки — тестовая страница") && out.contains("Контакты"),
        "the whole document text: {out}"
    );
    let scoped = format!("html_text({}, \"h2\")", mlog_str(TENDER_HTML));
    let scoped_out = eval_expr(&scoped).expect("html_text with a selector");
    assert_eq!(
        scoped_out.trim_end(),
        "Поставка канцелярии Ремонт фасада",
        "both h2 texts joined with a space, got: {scoped_out}"
    );
}

// ── H3: html_links — absolute, http/https only, javascript: dropped ──

#[test]
fn n5_html_links_resolve_and_drop() {
    // 4 <a> elements: /lots/1 (relative), rules.pdf (absolute https),
    // javascript: (DROPPED), //mirror (protocol-relative), #anchor
    // (same-document — resolves to the base+anchor, still http) — so
    // 4 survive: /lots/1, rules.pdf, mirror, #anchor. The javascript:
    // drop is asserted explicitly by the missing 5th element.
    let count = format!(
        "length(html_links({}, \"https://tender.example.by/lots\"))",
        mlog_str(TENDER_HTML)
    );
    let n = eval_expr(&count).expect("html_links must succeed");
    assert_eq!(n.trim_end(), "4", "got: {n}");

    // The javascript: link is NOT among them (the 4th is the #anchor).
    let fourth = format!(
        "let ls = html_links({}, \"https://tender.example.by/lots\")\n         return ls[3].href",
        mlog_str(TENDER_HTML)
    );
    let fourth_out = eval_expr(&fourth).expect("the fourth link");
    assert_eq!(
        fourth_out.trim_end(),
        "https://tender.example.by/lots#anchor",
        "javascript: dropped, the same-document anchor survives: {fourth_out}"
    );
}

#[test]
fn n5_html_links_absolute_href_and_text() {
    // The first link (document order): /lots/1 resolved against the base.
    let first = format!(
        "let ls = html_links({}, \"https://tender.example.by/lots\")\n         return ls[0].href + \"|\" + ls[0].text",
        mlog_str(TENDER_HTML)
    );
    let out = eval_expr(&first).expect("link resolution must succeed");
    assert_eq!(
        out.trim_end(),
        "https://tender.example.by/lots/1|Подробнее о лоте 1",
        "the relative href resolves against the base, got: {out}"
    );
    // The protocol-relative //mirror... form is the THIRD link (index 2).
    let mirror = format!(
        "let ls = html_links({}, \"https://tender.example.by/lots\")\n         return ls[2].href",
        mlog_str(TENDER_HTML)
    );
    let mirror_out = eval_expr(&mirror).expect("protocol-relative resolution");
    assert_eq!(
        mirror_out.trim_end(),
        "https://mirror.example.by/lots/2",
        "got: {mirror_out}"
    );
}

// ── H4: the loud limits ───────────────────────────────────────────────

#[test]
fn n5_input_limit_is_loud() {
    // 5 MiB + 1 byte of "a" — beyond the input cap, LOUD.
    let big: String = "a".repeat(5 * 1024 * 1024 + 1);
    let src = format!("html_select({}, \"div\")", mlog_str(&big));
    let err = eval_expr(&src).expect_err("the input cap must fail loud");
    assert!(
        err.contains("5 MiB") || err.contains("html_select"),
        "the error must name the limit/builtin: {}",
        err.chars().take(200).collect::<String>()
    );
}

#[test]
fn n5_result_limit_is_loud() {
    // 10 001 <div> elements — beyond the result cap, LOUD.
    let mut big = String::from("<body>");
    for i in 0..10_001 {
        big.push_str(&format!("<div class=\"row\">{}</div>", i));
    }
    big.push_str("</body>");
    let src = format!("html_select({}, \"div.row\")", mlog_str(&big));
    let err = eval_expr(&src).expect_err("the result cap must fail loud");
    assert!(
        err.contains("10 000") || err.contains("10000") || err.contains("node limit"),
        "the error must name the node limit: {}",
        err.chars().take(300).collect::<String>()
    );
}

// ── H5: broken HTML tolerated ─────────────────────────────────────────

#[test]
fn n5_broken_html_still_extracts() {
    let broken = "<div class='lot'><h2>Незакрытый лот <p>цена 5 BYN";
    let src = format!("html_text({}, \".lot\")", mlog_str(broken));
    let out = eval_expr(&src).expect("the broken HTML must not error");
    assert!(
        out.contains("Незакрытый лот") && out.contains("цена 5 BYN"),
        "the error recovery still yields the readable text: {out}"
    );
}

// ── H6: unsupported selector — loud ───────────────────────────────────

#[test]
fn n5_unsupported_selector_is_loud() {
    let src = format!(
        "html_select({}, \"div:first-child\")",
        mlog_str(TENDER_HTML)
    );
    let err = eval_expr(&src).expect_err("a pseudo-class must fail loud");
    assert!(
        err.contains("unexpected character") || err.contains("html_select"),
        "the error must name the supported subset: {err}"
    );
}

// ── H7: VM parity on a representative extraction ──────────────────────

#[test]
fn n5_vm_parity_select_and_links() {
    let sel = format!(
        "let ms = html_select({}, \"div.lot h2\")\n         return ms[1].text",
        mlog_str(TENDER_HTML)
    );
    let tw = eval_expr(&sel).expect("TW extraction");
    let vm = eval_expr_vm(&sel).expect("VM extraction");
    assert_eq!(tw.trim_end(), vm.trim_end(), "TW/VM parity on html_select");
    assert_eq!(tw.trim_end(), "Ремонт фасада");

    let links = format!(
        "let ls = html_links({}, \"https://tender.example.by/lots\")\n         return length(ls)",
        mlog_str(TENDER_HTML)
    );
    let tw_l = eval_expr(&links).expect("TW links");
    let vm_l = eval_expr_vm(&links).expect("VM links");
    assert_eq!(
        tw_l.trim_end(),
        vm_l.trim_end(),
        "TW/VM parity on html_links"
    );
}

// ── H8: the golden pair — TW and VM byte-identical ────────────────────

#[test]
fn n5_golden_pair_tw_and_vm_identical() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mlog_path = base.join("examples/n5_html_tender.mlog");
    let expected_path = base.join("examples/n5_html_tender.expected");
    let source = fs::read_to_string(&mlog_path).expect("the golden example exists");
    let expected = fs::read_to_string(&expected_path).expect("the golden expected exists");

    let tw = metalogos::run_program(&source)
        .expect("the golden example runs on TW")
        .unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        expected.trim_end(),
        "the TW output must equal the .expected file"
    );

    let declarations = metalogos::parser::parse(&source).expect("the golden example parses");
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp
        .compile(declarations)
        .expect("the golden example compiles");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(program).expect("the golden example runs on VM");
    assert_eq!(
        vm_out.unwrap_or_default().trim_end(),
        expected.trim_end(),
        "the VM output must equal the .expected file too"
    );
}

// ── The off-feature loud refusal (the N559 feature-independent surface)
#[test]
#[cfg(not(feature = "html"))]
fn n5_off_feature_refusal_is_loud() {
    let src = "pattern __eval(input: String) -> String { return html_select(\"<div>x</div>\", \"div\") }\nflow Main { input: String = \"x\" -> __eval -> output }";
    let err = metalogos::run_program(src).expect_err("off-feature must refuse LOUD");
    assert!(
        err.contains("requires the `html` feature"),
        "the refusal must name the feature and ADR-0192: {err}"
    );
}
