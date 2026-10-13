// ── ADR-0192 (N6, gh#1216): the `xml` feature — the feed/XML builtins ──
//
// The whole module is cfg-neutral (declared in mod.rs): the registry
// surface is ALWAYS present (the N559 feature-independent rhythm), the
// handlers refuse LOUDLY without the feature. The `xml` feature implies
// `html` (the shared text/limit helpers, ADR-0192 §2.2) and adds
// xml5ever 0.18 (MIT/Apache-2.0 — the transitive dep of
// markup5ever_rcdom, promoted to a direct optional dependency).
//
// The three builtins (the feed/sitemap family):
//
//   `feed_parse(xml) -> List<Struct<FeedItem>>` — RSS 2.0
//       (`rss > channel > item`) and Atom 1.0 (`feed > entry`) detected
//       by the root element; every item becomes
//       `{title, link, published, summary, id}` — a missing field is the
//       EMPTY STRING (the query_param fail-closed convention), never a
//       per-item error: real-world feeds are ragged, one broken item
//       must not kill the batch.
//   `sitemap_parse(xml) -> List<Struct<SitemapEntry>>` — the `urlset >
//       url` and `sitemapindex > sitemap` shapes uniformly: every
//       element named `url` or `sitemap` carrying a `loc` child becomes
//       `{loc, lastmod}` (lastmod often absent → "").
//   `xml_parse(xml) -> Struct<XmlNode>{tag, attrs, text, children}` —
//       the general tree: `tag` (local name, case-SENSITIVE — XML is),
//       `attrs` (a Dict), `text` (the DIRECT text children, normalized),
//       `children` (the recursive XmlNode list; non-element children are
//       not nodes — comments/PIs/doctype never surface).
//
// Security (the naryad's hard contract):
//   - DTD and external entities are NEVER expanded. XML5 (the grammar
//     xml5ever implements) has no custom-entity mechanism — a DOCTYPE
//     internal subset is discarded by the tokenizer, an undefined
//     entity reference degrades to literal text. The XXE / billion-
//     laughs negative tests pin exactly this (no file reads, no
//     exponential blowup).
//   - the input ≤ 5 MiB (`XML_INPUT_LIMIT`) — beyond, a loud error;
//   - the result ≤ 10 000 nodes (`XML_RESULT_LIMIT`) — beyond, a loud
//     error, never a silent truncate;
//   - the tree depth ≤ 256 (`XML_DEPTH_LIMIT`) — beyond, a loud error
//     (the recursive XmlNode build must not blow the stack on an
//     adversarial 10k-deep chain).
//
// Taint: every extracted string is untrusted — the builtins are Sources
// in the audit vocabulary (the request_header N2 / html N5 lane).

#[cfg(not(feature = "xml"))]
use crate::interpreter::Value;

#[cfg(not(feature = "xml"))]
fn feature_refusal(builtin: &str) -> String {
    format!(
        "{}() requires the `xml` feature (build with --features xml; ADR-0192). \
         The off-by-default capability gate keeps the default build free of the \
         xml5ever stack — the refusal is LOUD, never a silent empty result.",
        builtin
    )
}

// ── Without the feature: the registry surface is ALWAYS present, the
// handlers refuse LOUDLY. ─────────────────────────────────────────────
#[cfg(not(feature = "xml"))]
pub(crate) fn builtin_feed_parse(_args: &[Value]) -> Result<Value, String> {
    Err(feature_refusal("feed_parse"))
}
#[cfg(not(feature = "xml"))]
pub(crate) fn builtin_sitemap_parse(_args: &[Value]) -> Result<Value, String> {
    Err(feature_refusal("sitemap_parse"))
}
#[cfg(not(feature = "xml"))]
pub(crate) fn builtin_xml_parse(_args: &[Value]) -> Result<Value, String> {
    Err(feature_refusal("xml_parse"))
}

// ── With the feature: the real xml5ever-backed implementation. ────────
#[cfg(feature = "xml")]
mod real {
    use crate::builtins::core::expect_string_arg;
    use crate::interpreter::Value;
    use markup5ever_rcdom::{Handle, NodeData, RcDom};
    use xml5ever::driver::parse_document;
    use xml5ever::driver::XmlParseOpts;
    use xml5ever::tendril::TendrilSink;

    /// The per-call input cap: 5 MiB (ADR-0192 §2.4, the naryad's limit).
    pub(crate) const XML_INPUT_LIMIT: usize = 5 * 1024 * 1024;
    /// The per-call result cap: 10 000 nodes.
    pub(crate) const XML_RESULT_LIMIT: usize = 10_000;
    /// The tree-depth cap for the recursive XmlNode build.
    pub(crate) const XML_DEPTH_LIMIT: usize = 256;

    // ── Parsing ───────────────────────────────────────────────────────

    /// Parse an XML document with xml5ever. The 5 MiB input cap fires
    /// BEFORE any parse work (fail loud). DTDs are never expanded: the
    /// XML5 grammar has no custom-entity mechanism, so an XXE payload
    /// degrades to literal text — pinned by the negative tests.
    pub(super) fn parse_xml(builtin: &str, xml: &str) -> Result<RcDom, String> {
        if xml.len() > XML_INPUT_LIMIT {
            return Err(format!(
                "{}(): input exceeds the 5 MiB limit ({} bytes > {}); \
                 split the source or process it in chunks (never a silent \
                 parse of a truncated document)",
                builtin,
                xml.len(),
                XML_INPUT_LIMIT
            ));
        }
        parse_document(RcDom::default(), XmlParseOpts::default())
            .from_utf8()
            .read_from(&mut xml.as_bytes())
            .map_err(|e| format!("{}(): XML parse error: {}", builtin, e))
    }

    /// The element's local name (XML is case-SENSITIVE — no lowering).
    fn local_name(handle: &Handle) -> Option<String> {
        match &handle.data {
            NodeData::Element { name, .. } => Some(name.local.to_string()),
            _ => None,
        }
    }

    fn element_attrs(handle: &Handle) -> Vec<(String, String)> {
        match &handle.data {
            NodeData::Element { attrs, .. } => attrs
                .borrow()
                .iter()
                .map(|a| (a.name.local.to_string(), a.value.to_string()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Direct element children (non-elements skipped).
    fn child_elements(handle: &Handle) -> Vec<Handle> {
        handle
            .children
            .borrow()
            .iter()
            .filter(|c| matches!(&c.data, NodeData::Element { .. }))
            .cloned()
            .collect()
    }

    /// First direct child element with the given local name.
    fn child_by_name(handle: &Handle, tag: &str) -> Option<Handle> {
        child_elements(handle)
            .into_iter()
            .find(|c| matches!(&c.data, NodeData::Element { name, .. } if &*name.local == tag))
    }

    /// The concatenated DIRECT text children, whitespace-normalized
    /// (CDATA segments arrive as Text nodes — they contribute; nested
    /// element text does NOT — that belongs to the child's own node).
    fn direct_text(handle: &Handle) -> String {
        let mut raw = String::new();
        for child in handle.children.borrow().iter() {
            if let NodeData::Text { contents } = &child.data {
                raw.push_str(&contents.borrow());
            }
        }
        raw.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    // ── Field extraction (the fail-closed item shape) ─────────────────

    /// The text of the FIRST direct child element named `tag` ("" when
    /// absent — a missing field is empty, never a per-item error).
    fn field_text(item: &Handle, tag: &str) -> String {
        child_by_name(item, tag)
            .map(|c| direct_text(&c))
            .unwrap_or_default()
    }

    /// The Atom link: the `href` of the first `<link rel="alternate">`
    /// when present, else the first `<link>`'s href, else "".
    fn atom_link(entry: &Handle) -> String {
        let links: Vec<Handle> = child_elements(entry)
            .into_iter()
            .filter(|c| matches!(&c.data, NodeData::Element { name, .. } if &*name.local == "link"))
            .collect();
        let pick = links
            .iter()
            .find(|l| {
                element_attrs(l)
                    .iter()
                    .any(|(k, v)| k == "rel" && v == "alternate")
            })
            .or_else(|| links.first());
        pick.and_then(|l| {
            element_attrs(l)
                .iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.clone())
        })
        .unwrap_or_default()
    }

    fn make_dict(pairs: Vec<(String, String)>) -> Value {
        Value::Struct {
            type_name: "Dict".to_string(),
            fields: pairs
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        }
    }

    fn feed_item(
        title: String,
        link: String,
        published: String,
        summary: String,
        id: String,
    ) -> Value {
        Value::Struct {
            type_name: "FeedItem".to_string(),
            fields: [
                ("title".to_string(), Value::String(title)),
                ("link".to_string(), Value::String(link)),
                ("published".to_string(), Value::String(published)),
                ("summary".to_string(), Value::String(summary)),
                ("id".to_string(), Value::String(id)),
            ]
            .into_iter()
            .collect(),
        }
    }

    // ── feed_parse ────────────────────────────────────────────────────

    /// `feed_parse(xml) -> List<Struct<FeedItem>{title, link, published,
    /// summary, id}>` — RSS 2.0 and Atom 1.0 detected by the root
    /// element; neither → a loud error naming the two supported shapes.
    pub(crate) fn builtin_feed_parse(args: &[Value]) -> Result<Value, String> {
        let xml = expect_string_arg("feed_parse", args, 0)?;
        let dom = parse_xml("feed_parse", &xml)?;

        // The root element (the Document's first element child).
        let root = child_elements(&dom.document)
            .into_iter()
            .next()
            .ok_or_else(|| "feed_parse(): the document has no root element".to_string())?;
        let root_tag = local_name(&root).unwrap_or_default();

        let items: Vec<Handle> = match root_tag.as_str() {
            // RSS 2.0: rss > channel > item*
            "rss" => child_by_name(&root, "channel")
                .map(|ch| {
                    child_elements(&ch)
                        .into_iter()
                        .filter(|c| matches!(&c.data, NodeData::Element { name, .. } if &*name.local == "item"))
                        .collect()
                })
                .unwrap_or_default(),
            // Atom 1.0: feed > entry*
            "feed" => child_elements(&root)
                .into_iter()
                .filter(|c| matches!(&c.data, NodeData::Element { name, .. } if &*name.local == "entry"))
                .collect(),
            other => {
                return Err(format!(
                    "feed_parse(): the root element is <{}>, not <rss> (RSS 2.0) \
                     or <feed> (Atom 1.0) — the two supported shapes",
                    other
                ))
            }
        };

        let mut out = Vec::new();
        for item in items {
            if out.len() >= XML_RESULT_LIMIT {
                return Err(format!(
                    "feed_parse(): the result exceeds the {}-item limit; \
                     split the source (never a silent truncate)",
                    XML_RESULT_LIMIT
                ));
            }
            let (title, link, published, summary, id) = if root_tag == "rss" {
                (
                    field_text(&item, "title"),
                    field_text(&item, "link"),
                    // RSS has no fixed date element: pubDate is the
                    // convention; dc:date (the Dublin Core variant)
                    // is the common alternative.
                    {
                        let d = field_text(&item, "pubDate");
                        if d.is_empty() {
                            field_text(&item, "date")
                        } else {
                            d
                        }
                    },
                    field_text(&item, "description"),
                    // guid is the RSS id; fall back to the link.
                    {
                        let g = field_text(&item, "guid");
                        if g.is_empty() {
                            field_text(&item, "link")
                        } else {
                            g
                        }
                    },
                )
            } else {
                (
                    field_text(&item, "title"),
                    atom_link(&item),
                    // Atom: published, else updated (the common ragged form).
                    {
                        let p = field_text(&item, "published");
                        if p.is_empty() {
                            field_text(&item, "updated")
                        } else {
                            p
                        }
                    },
                    {
                        // summary, else content (the full-text variant).
                        let s = field_text(&item, "summary");
                        if s.is_empty() {
                            field_text(&item, "content")
                        } else {
                            s
                        }
                    },
                    field_text(&item, "id"),
                )
            };
            out.push(feed_item(title, link, published, summary, id));
        }
        Ok(Value::List(out))
    }

    // ── sitemap_parse ─────────────────────────────────────────────────

    /// `sitemap_parse(xml) -> List<Struct<SitemapEntry>{loc, lastmod}>` —
    /// the `urlset > url` and `sitemapindex > sitemap` shapes uniformly:
    /// every `url`/`sitemap` element carrying a `loc` child becomes an
    /// entry (lastmod often absent → "").
    pub(crate) fn builtin_sitemap_parse(args: &[Value]) -> Result<Value, String> {
        let xml = expect_string_arg("sitemap_parse", args, 0)?;
        let dom = parse_xml("sitemap_parse", &xml)?;

        let mut out = Vec::new();
        // Walk every element; the entry shapes are matched locally so a
        // wrapped/_namespaced container still parses.
        walk_for_sitemap(&dom.document, &mut out)?;
        Ok(Value::List(out))
    }

    fn walk_for_sitemap(handle: &Handle, out: &mut Vec<Value>) -> Result<(), String> {
        if let NodeData::Element { name, .. } = &handle.data {
            if &*name.local == "url" || &*name.local == "sitemap" {
                let loc = field_text(handle, "loc");
                if !loc.is_empty() {
                    if out.len() >= XML_RESULT_LIMIT {
                        return Err(format!(
                            "sitemap_parse(): the result exceeds the {}-entry limit; \
                             split the source (never a silent truncate)",
                            XML_RESULT_LIMIT
                        ));
                    }
                    let lastmod = field_text(handle, "lastmod");
                    out.push(Value::Struct {
                        type_name: "SitemapEntry".to_string(),
                        fields: [
                            ("loc".to_string(), Value::String(loc)),
                            ("lastmod".to_string(), Value::String(lastmod)),
                        ]
                        .into_iter()
                        .collect(),
                    });
                }
            }
        }
        for child in child_elements(handle) {
            walk_for_sitemap(&child, out)?;
        }
        Ok(())
    }

    // ── xml_parse ─────────────────────────────────────────────────────

    /// `xml_parse(xml) -> Struct<XmlNode>{tag, attrs, text, children}` —
    /// the general tree. The depth cap (256) and the node cap (10 000)
    /// both fail LOUD: the recursive build must not blow the stack on an
    /// adversarial chain.
    pub(crate) fn builtin_xml_parse(args: &[Value]) -> Result<Value, String> {
        let xml = expect_string_arg("xml_parse", args, 0)?;
        let dom = parse_xml("xml_parse", &xml)?;

        let root = child_elements(&dom.document)
            .into_iter()
            .next()
            .ok_or_else(|| "xml_parse(): the document has no root element".to_string())?;
        let mut budget = XML_RESULT_LIMIT;
        build_node(&root, 0, &mut budget)
    }

    fn build_node(handle: &Handle, depth: usize, budget: &mut usize) -> Result<Value, String> {
        if depth > XML_DEPTH_LIMIT {
            return Err(format!(
                "xml_parse(): the tree exceeds the {}-level depth limit — \
                 the input is adversarially nested (never a stack overflow)",
                XML_DEPTH_LIMIT
            ));
        }
        if *budget == 0 {
            return Err(format!(
                "xml_parse(): the result exceeds the {}-node limit; \
                 split the source (never a silent truncate)",
                XML_RESULT_LIMIT
            ));
        }
        *budget -= 1;

        let (tag, attrs) = match &handle.data {
            NodeData::Element { name, attrs, .. } => {
                let pairs = attrs
                    .borrow()
                    .iter()
                    .map(|a| (a.name.local.to_string(), a.value.to_string()))
                    .collect();
                (name.local.to_string(), pairs)
            }
            _ => (String::new(), Vec::new()),
        };

        let mut children = Vec::new();
        for child in child_elements(handle) {
            children.push(build_node(&child, depth + 1, budget)?);
        }

        Ok(Value::Struct {
            type_name: "XmlNode".to_string(),
            fields: [
                ("tag".to_string(), Value::String(tag)),
                ("attrs".to_string(), make_dict(attrs)),
                ("text".to_string(), Value::String(direct_text(handle))),
                ("children".to_string(), Value::List(children)),
            ]
            .into_iter()
            .collect(),
        })
    }

    // ── Tests (the N6 unit legs; the integration/golden legs live in
    // tests/naryad_n6_xml_builtins.rs, feature-gated) ─────────────────────
}

// The real handlers surface at the module level only with the feature.
#[cfg(feature = "xml")]
pub(crate) use real::{builtin_feed_parse, builtin_sitemap_parse, builtin_xml_parse};

#[cfg(all(test, feature = "xml"))]
mod tests {
    use super::real::{parse_xml, XML_INPUT_LIMIT};
    use markup5ever_rcdom::NodeData;

    #[test]
    fn test_rss_root_detected() {
        let dom = parse_xml(
            "feed_parse",
            r#"<?xml version="1.0"?><rss version="2.0"><channel><item><title>T</title></item></channel></rss>"#,
        )
        .expect("parse");
        let root_tag = {
            let children = dom.document.children.borrow();
            children
                .iter()
                .find_map(|c| match &c.data {
                    NodeData::Element { name, .. } => Some(name.local.to_string()),
                    _ => None,
                })
                .expect("root")
        };
        assert_eq!(&*root_tag, "rss");
    }

    #[test]
    fn test_input_limit_is_loud() {
        let big = "<a>".to_string() + &"x".repeat(XML_INPUT_LIMIT) + "</a>";
        let err = match parse_xml("xml_parse", &big) {
            Err(e) => e,
            Ok(_) => panic!("the 5 MiB input limit must fail loud"),
        };
        assert!(
            err.contains("5 MiB"),
            "the error must name the limit: {err}"
        );
    }
}
