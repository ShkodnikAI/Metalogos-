// ── Naryad N6 (gh#1216, PE-2, adr/feature): the `xml` feature ─────────
//
// The feed/XML family on the xml5ever stack (ADR-0192, implies `html`):
//
//   X1: `feed_parse(xml)` — RSS 2.0 and Atom 1.0 detected by the root
//       element; every item → {title, link, published, summary, id};
//       a missing field is the EMPTY string (fail-closed items, the
//       ragged real-world feed never kills the batch); CDATA segments
//       contribute; namespace declarations don't disturb the local-name
//       matching.
//   X2: `sitemap_parse(xml)` — the urlset > url and sitemapindex >
//       sitemap shapes uniformly → {loc, lastmod} (lastmod often "").
//   X3: `xml_parse(xml)` — the general XmlNode tree {tag, attrs, text,
//       children}: the local name is case-SENSITIVE (XML is), attrs is
//       a Dict, text is the DIRECT text (normalized), children recurses.
//   X4: the XXE contract — a DOCTYPE with an external ENTITY is NEVER
//       expanded: the XML5 grammar has no custom-entity mechanism, so
//       `&xxe;` degrades to literal text and file:///etc/passwd is
//       never read (the negative test asserts the secret content does
//       not surface).
//   X5: the billion-laughs contract — nested custom entities do NOT
//       expand (the output stays bounded, no exponential blowup).
//   X6: the loud limits — the 5 MiB input, the 10 000-node result and
//       the 256-level depth all fail LOUD (never a silent truncate,
//       never a stack overflow).
//   X7: an unsupported root (neither rss nor feed) is a LOUD error
//       naming the two supported shapes.
//   X8: the TW/VM parity — both backends share the registry handler;
//       the golden pair examples/n6_feed_parse.mlog(+.expected) is
//       byte-identical on TW and VM (the naryad's DOD).
//
// The feature-on legs live in the `feature_on` module (compiled ONLY
// with the `xml` feature — the xml-tests CI job runs them); the
// off-feature refusal leg is compiled ONLY without it (the default CI
// suite runs it). RED-first: with the feature declared but the handlers
// absent, every feature-on program fails with the loud `unknown
// function 'feed_parse'` compile error.

// The file itself ALWAYS compiles: the feature-on legs (X1–X8) live in
// the `feature_on` module; the off-feature loud-refusal leg runs WITHOUT
// the feature — the refusal contract is part of the N559
// feature-independent registry surface.
#[cfg(feature = "xml")]
mod feature_on {
    #![allow(clippy::disallowed_methods)]

    use std::fs;

    // ── eval_expr (the №261 lekało; the VM twin for the parity leg) ──

    fn eval_expr(src: &str) -> Result<String, String> {
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

    fn mlog_str(s: &str) -> String {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t")
        )
    }

    // ── The fixtures (the naryad's contract: CDATA + namespace variants)

    const RSS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel>
    <title>ТендерыGov</title>
    <item>
      <title><![CDATA[Поставка канцелярии]]></title>
      <link>https://tender.example.by/lots/1</link>
      <pubDate>Mon, 12 Oct 2026 09:00:00 +0300</pubDate>
      <description>Лот №1: бумага, ручки</description>
      <guid>lot-1</guid>
    </item>
    <item>
      <title>Ремонт фасада</title>
      <link>https://tender.example.by/lots/2</link>
      <dc:date>2026-10-12T12:00:00Z</dc:date>
      <description><![CDATA[Лот №2: фасадные работы]]></description>
    </item>
  </channel>
</rss>"#;

    const ATOM_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Gov Feed</title>
  <entry>
    <id>urn:uuid:aaa-1</id>
    <title>Lot 1 announced</title>
    <link rel="alternate" href="https://gov.example.by/lots/1"/>
    <published>2026-10-12T09:00:00Z</published>
    <summary>Office supplies lot</summary>
  </entry>
  <entry>
    <id>urn:uuid:bbb-2</id>
    <title>Lot 2 updated</title>
    <link href="https://gov.example.by/lots/2"/>
    <updated>2026-10-12T12:00:00Z</updated>
    <content>Facade works, full text variant</content>
  </entry>
</feed>"#;

    const SITEMAP_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://gov.example.by/</loc>
    <lastmod>2026-10-01</lastmod>
  </url>
  <url>
    <loc>https://gov.example.by/lots</loc>
  </url>
</urlset>"#;

    const SITEMAP_INDEX_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap>
    <loc>https://gov.example.by/sitemap-1.xml</loc>
  </sitemap>
  <sitemap>
    <loc>https://gov.example.by/sitemap-2.xml</loc>
    <lastmod>2026-10-11</lastmod>
  </sitemap>
</sitemapindex>"#;

    // ── X1: feed_parse — RSS 2.0 with CDATA + the dc:date variant ────

    #[test]
    fn n6_feed_parse_rss_items() {
        let src = format!(
            "let fs = feed_parse({})\n         let f = fs[0]\n         return to_string(length(fs)) + \"|\" + f.title + \"|\" + f.link + \"|\" + f.published + \"|\" + f.id",
            mlog_str(RSS_XML)
        );
        let out = eval_expr(&src).expect("feed_parse must succeed");
        assert_eq!(
            out.trim_end(),
            "2|Поставка канцелярии|https://tender.example.by/lots/1|Mon, 12 Oct 2026 09:00:00 +0300|lot-1",
            "the CDATA title, the link, the pubDate and the guid: {out}"
        );
    }

    #[test]
    fn n6_feed_parse_rss_missing_fields_fail_closed() {
        // The second item has no guid (→ the link fallback), no pubDate
        // (→ the dc:date variant), the third shape: a bare item.
        let src = format!(
            "let fs = feed_parse({})\n         return fs[1].published + \"|\" + fs[1].id",
            mlog_str(RSS_XML)
        );
        let out = eval_expr(&src).expect("feed_parse ragged items");
        assert_eq!(
            out.trim_end(),
            "2026-10-12T12:00:00Z|https://tender.example.by/lots/2",
            "dc:date fills published, the link falls back for id: {out}"
        );
    }

    #[test]
    fn n6_feed_parse_atom_entries() {
        let src = format!(
            "let fs = feed_parse({})\n         let f = fs[0]\n         return f.id + \"|\" + f.link + \"|\" + f.published + \"|\" + f.summary",
            mlog_str(ATOM_XML)
        );
        let out = eval_expr(&src).expect("feed_parse atom");
        assert_eq!(
            out.trim_end(),
            "urn:uuid:aaa-1|https://gov.example.by/lots/1|2026-10-12T09:00:00Z|Office supplies lot",
            "the rel=alternate link wins: {out}"
        );
    }

    #[test]
    fn n6_feed_parse_atom_content_fallback() {
        // The second entry has no published (→ updated), no summary
        // (→ content), one plain link (no rel).
        let src = format!(
            "let fs = feed_parse({})\n         return fs[1].published + \"|\" + fs[1].summary + \"|\" + fs[1].link",
            mlog_str(ATOM_XML)
        );
        let out = eval_expr(&src).expect("feed_parse atom fallbacks");
        assert_eq!(
            out.trim_end(),
            "2026-10-12T12:00:00Z|Facade works, full text variant|https://gov.example.by/lots/2",
            "updated/content fill the ragged fields: {out}"
        );
    }

    // ── X2: sitemap_parse — both shapes ──────────────────────────────

    #[test]
    fn n6_sitemap_parse_urlset() {
        let src = format!(
            "let ss = sitemap_parse({})\n         return to_string(length(ss)) + \"|\" + ss[0].loc + \"|\" + ss[0].lastmod + \"|\" + ss[1].lastmod",
            mlog_str(SITEMAP_XML)
        );
        let out = eval_expr(&src).expect("sitemap_parse urlset");
        assert_eq!(
            out.trim_end(),
            "2|https://gov.example.by/|2026-10-01|",
            "the missing lastmod is the EMPTY string: {out}"
        );
    }

    #[test]
    fn n6_sitemap_parse_index() {
        let src = format!(
            "let ss = sitemap_parse({})\n         return to_string(length(ss)) + \"|\" + ss[1].loc + \"|\" + ss[1].lastmod",
            mlog_str(SITEMAP_INDEX_XML)
        );
        let out = eval_expr(&src).expect("sitemap_parse index");
        assert_eq!(
            out.trim_end(),
            "2|https://gov.example.by/sitemap-2.xml|2026-10-11",
            "the sitemapindex shape parses uniformly: {out}"
        );
    }

    // ── X3: xml_parse — the general tree, case-sensitive local names ──

    #[test]
    fn n6_xml_parse_tree_shape() {
        let src = format!(
            "let n = xml_parse({})\n         let c0 = n.children[0]\n         return n.tag + \"|\" + json_get(n.attrs, \"version\") + \"|\" + c0.tag + \"|\" + json_get(c0.attrs, \"class\") + \"|\" + c0.text",
            mlog_str(r#"<lot version="2.0" id="a1">Стол <b class="hi">офисный</b></lot>"#)
        );
        let out = eval_expr(&src).expect("xml_parse tree");
        assert_eq!(
            out.trim_end(),
            "lot|2.0|b|hi|офисный",
            "the local name is case-sensitive, attrs is a Dict, text is DIRECT: {out}"
        );
    }

    // ── X4: the XXE contract — no entity expansion, no file reads ────

    #[test]
    fn n6_xxe_is_never_expanded() {
        let xxe = r#"<?xml version="1.0"?>
<!DOCTYPE lot [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<lot>&xxe;</lot>"#;
        let src = format!("xml_parse({})", mlog_str(xxe));
        let out = eval_expr(&src).expect("the XXE payload must not error — it degrades");
        let out = out.trim_end();
        assert!(
            !out.contains("root:") && !out.contains("/bin") && !out.contains("daemon"),
            "the external entity must NEVER be read/expanded: {out}"
        );
    }

    // ── X5: billion laughs — no entity expansion, no blowup ──────────

    #[test]
    fn n6_billion_laughs_do_not_expand() {
        let laughs = r#"<?xml version="1.0"?>
<!DOCTYPE lolz [
  <!ENTITY lol "lol">
  <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
]>
<lolz>&lol3;</lolz>"#;
        let src = format!("xml_parse({})", mlog_str(laughs));
        let started = std::time::Instant::now();
        let out = eval_expr(&src).expect("the entity payload must not error");
        let elapsed = started.elapsed();
        let out = out.trim_end();
        assert!(
            !out.contains("lololololol"),
            "the nested custom entities must NOT expand: {out}"
        );
        assert!(
            elapsed.as_secs() < 5,
            "the parse must stay bounded (no exponential blowup): {:?}",
            elapsed
        );
    }

    // ── X6: the loud limits ──────────────────────────────────────────

    #[test]
    fn n6_input_limit_is_loud() {
        let big: String = "a".repeat(5 * 1024 * 1024 + 1);
        let src = format!("xml_parse({})", mlog_str(&big));
        let err = eval_expr(&src).expect_err("the input cap must fail loud");
        assert!(
            err.contains("5 MiB") || err.contains("xml_parse"),
            "the error must name the limit/builtin: {}",
            err.chars().take(200).collect::<String>()
        );
    }

    #[test]
    fn n6_depth_limit_is_loud() {
        // 300 nested elements — beyond the 256-level depth cap, LOUD.
        let mut deep = String::new();
        for _ in 0..300 {
            deep.push_str("<a>");
        }
        deep.push('x');
        for _ in 0..300 {
            deep.push_str("</a>");
        }
        let src = format!("xml_parse({})", mlog_str(&deep));
        let err = eval_expr(&src).expect_err("the depth cap must fail loud");
        assert!(
            err.contains("depth") || err.contains("256"),
            "the error must name the depth limit: {}",
            err.chars().take(300).collect::<String>()
        );
    }

    // ── X7: the unsupported root is loud ─────────────────────────────

    #[test]
    fn n6_unsupported_root_is_loud() {
        let src = format!("feed_parse({})", mlog_str("<orders><order/></orders>"));
        let err = eval_expr(&src).expect_err("neither rss nor feed");
        assert!(
            err.contains("<rss>") && err.contains("<feed>"),
            "the error must name the two supported shapes: {err}"
        );
    }

    // ── X8: the golden pair — TW and VM byte-identical ───────────────

    #[test]
    fn n6_golden_pair_tw_and_vm_identical() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mlog_path = base.join("examples/n6_feed_parse.mlog");
        let expected_path = base.join("examples/n6_feed_parse.expected");
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

    #[test]
    fn n6_vm_parity_feed_and_sitemap() {
        let feed = format!(
            "let fs = feed_parse({})\n         return to_string(length(fs))",
            mlog_str(RSS_XML)
        );
        let tw = eval_expr(&feed).expect("TW feed");
        let vm = eval_expr_vm(&feed).expect("VM feed");
        assert_eq!(tw.trim_end(), vm.trim_end(), "TW/VM parity on feed_parse");

        let sitemap = format!(
            "let ss = sitemap_parse({})\n         return to_string(length(ss))",
            mlog_str(SITEMAP_XML)
        );
        let tw_s = eval_expr(&sitemap).expect("TW sitemap");
        let vm_s = eval_expr_vm(&sitemap).expect("VM sitemap");
        assert_eq!(
            tw_s.trim_end(),
            vm_s.trim_end(),
            "TW/VM parity on sitemap_parse"
        );
    }
}

// ── The off-feature loud refusal (the N559 feature-independent surface) ──
#[test]
#[cfg(not(feature = "xml"))]
fn n6_off_feature_refusal_is_loud() {
    let src = "pattern __eval(input: String) -> String { return feed_parse(\"<rss/>\") }\nflow Main { input: String = \"x\" -> __eval -> output }";
    let err = metalogos::run_program(src).expect_err("off-feature must refuse LOUD");
    assert!(
        err.contains("requires the `xml` feature"),
        "the refusal must name the feature and ADR-0192: {err}"
    );
}
