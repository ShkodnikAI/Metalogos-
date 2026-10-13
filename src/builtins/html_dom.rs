// ── ADR-0192 (N5, gh#1215): the `html` feature — the DOM builtins ──────
//
// The whole module is `#[cfg(feature = "html")]` (declared in mod.rs):
// the default build links NO html code and NO html5ever/url crates
// (the ADR-0122 off-by-default capability-gate rhythm; the №518 license
// gate verified the stack in the N5-pre probe, issue #1214 — html5ever
// 0.27 + markup5ever_rcdom 0.3 + url 2, MIT/Apache-2.0/Unicode-3.0
// only).
//
// The three builtins (the tender-agent extraction family):
//
//   `html_select(html, css) -> List<Struct<HtmlMatch>>` — every element
//       matching ANY comma-separated selector becomes
//       `{tag, text, attrs, html}` (tag lowercase; text = the normalized
//       subtree text, script/style subtrees excluded; attrs = a Dict of
//       the element's attributes; html = the serialized subtree).
//   `html_text(html, css?) -> String` — the normalized concatenated text
//       of the matched elements (no selector = the whole document);
//       script/style never reach the text.
//   `html_links(html, base_url) -> List<Struct<HtmlLink>>` — every <a>
//       with a resolvable href becomes `{href, text}`; hrefs are
//       ABSOLUTE (the WHATWG resolution against base_url via the url
//       crate), only http/https survive — `javascript:` and friends are
//       DROPPED by contract, never an error.
//
// The loud limits (the repo's fail-loud discipline, ADR-0192 §2.4):
//   - the input ≤ 5 MiB (`HTML_INPUT_LIMIT`) — beyond, a loud error,
//     never a silent parse of a truncated document;
//   - the result ≤ 10 000 nodes (`HTML_RESULT_LIMIT`) — beyond, a loud
//     error, never a silent truncate.
//
// The broken-HTML tolerance is html5ever's job: the parser implements
// the HTML5 spec error-recovery algorithm, so a malformed tender page
// still yields its readable parts.
//
// Selector support (the naryad's minimal contract, deliberately NOT
// more): tag, `.class`, `#id`, `[attr]`, `[attr=value]`, descendant
// (space), direct child (`>`), comma groups. Everything else is a loud
// parse error naming the position.

#[cfg(not(feature = "html"))]
use crate::interpreter::Value;

#[cfg(not(feature = "html"))]
fn feature_refusal(builtin: &str) -> String {
    format!(
        "{}() requires the `html` feature (build with --features html; ADR-0192). \
         The off-by-default capability gate keeps the default build free of the \
         html5ever stack — the refusal is LOUD, never a silent empty result.",
        builtin
    )
}

/// The per-call input cap: 5 MiB (ADR-0192 §2.4).
#[cfg(feature = "html")]
pub(crate) const HTML_INPUT_LIMIT: usize = 5 * 1024 * 1024;
/// The per-call result cap: 10 000 nodes (ADR-0192 §2.4).
#[cfg(feature = "html")]
pub(crate) const HTML_RESULT_LIMIT: usize = 10_000;

// ── Without the feature: the registry surface is ALWAYS present (the
// N559 arity export moves with the registry, feature-independent — the
// vision/voice rhythm), and the handlers refuse LOUDLY.
#[cfg(not(feature = "html"))]
pub(crate) fn builtin_html_select(_args: &[Value]) -> Result<Value, String> {
    Err(feature_refusal("html_select"))
}
#[cfg(not(feature = "html"))]
pub(crate) fn builtin_html_text(_args: &[Value]) -> Result<Value, String> {
    Err(feature_refusal("html_text"))
}
#[cfg(not(feature = "html"))]
pub(crate) fn builtin_html_links(_args: &[Value]) -> Result<Value, String> {
    Err(feature_refusal("html_links"))
}

// ── With the feature: the real html5ever-backed implementation. ──────
#[cfg(feature = "html")]
mod real {
    pub(super) use super::{HTML_INPUT_LIMIT, HTML_RESULT_LIMIT};
    use crate::builtins::core::expect_string_arg;
    use crate::interpreter::Value;
    use html5ever::parse_document;
    use html5ever::tendril::TendrilSink;
    use markup5ever_rcdom::{Handle, NodeData, RcDom};

    // ── Parsing ────────────────────────────────────────────────────────────

    /// Parse an HTML document fragment with html5ever's error recovery.
    /// The 5 MiB input cap fires BEFORE any parse work (fail loud).
    pub(super) fn parse_html(builtin: &str, html: &str) -> Result<RcDom, String> {
        if html.len() > HTML_INPUT_LIMIT {
            return Err(format!(
                "{}(): input exceeds the 5 MiB limit ({} bytes > {}); \
             split the source or raise HTML_INPUT_LIMIT consciously",
                builtin,
                html.len(),
                HTML_INPUT_LIMIT
            ));
        }
        parse_document(RcDom::default(), html5ever::ParseOpts::default())
            .from_utf8()
            .read_from(&mut html.as_bytes())
            .map_err(|e| format!("{}(): HTML parse error: {}", builtin, e))
    }

    /// A lowercase element tag name (empty for non-elements).
    fn tag_name(handle: &Handle) -> Option<String> {
        match &handle.data {
            NodeData::Element { name, .. } => Some(name.local.to_string().to_lowercase()),
            _ => None,
        }
    }

    /// The element's attribute map (names lowercased — html5ever already
    /// normalizes them on parse).
    fn element_attrs(handle: &Handle) -> Vec<(String, String)> {
        match &handle.data {
            NodeData::Element { attrs, .. } => attrs
                .borrow()
                .iter()
                .map(|a| (a.name.local.to_string().to_lowercase(), a.value.to_string()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The parent ELEMENT (skipping the document root) — None at the top.
    /// rcdom 0.3 stores the parent in a `Cell<Option<Weak<Node>>>` — the
    /// read is a take-and-restore dance (Weak is Clone, the cell is left
    /// exactly as it was).
    fn parent_element(handle: &Handle) -> Option<Handle> {
        let weak = handle.parent.take();
        handle.parent.set(weak.clone());
        let parent = weak.and_then(|w| w.upgrade())?;
        match &parent.data {
            NodeData::Document => None,
            NodeData::Element { .. } => Some(parent),
            _ => None,
        }
    }

    // ── Text extraction ───────────────────────────────────────────────────

    /// Collect the subtree text; script/style subtrees NEVER contribute
    /// (the naryad's contract) — a <script> body is code, not content.
    fn collect_text(handle: &Handle, out: &mut String) {
        match &handle.data {
            NodeData::Text { contents } => out.push_str(&contents.borrow()),
            NodeData::Element { name, .. } => {
                let tag = name.local.to_string().to_lowercase();
                if tag == "script" || tag == "style" {
                    return;
                }
                for child in handle.children.borrow().iter() {
                    collect_text(child, out);
                }
            }
            NodeData::Document => {
                for child in handle.children.borrow().iter() {
                    collect_text(child, out);
                }
            }
            _ => {}
        }
    }

    /// The whitespace normalization contract: every run of whitespace
    /// collapses to ONE space, leading/trailing whitespace trimmed.
    pub(crate) fn normalize_ws(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The normalized text of an element subtree (script/style excluded).
    fn element_text(handle: &Handle) -> String {
        let mut raw = String::new();
        collect_text(handle, &mut raw);
        normalize_ws(&raw)
    }

    // ── The minimal selector engine ────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    enum SimpleSel {
        /// An element name (`div`) or `*` (any).
        Tag(String),
        /// `.class`
        Class(String),
        /// `#id`
        Id(String),
        /// `[attr]`
        AttrExists(String),
        /// `[attr=value]`
        AttrEq(String, String),
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Comb {
        /// ` ` — any ancestor
        Descendant,
        /// `>` — the direct parent
        Child,
    }

    #[derive(Debug, Clone)]
    pub(super) struct Compound {
        simples: Vec<SimpleSel>,
    }

    /// One comma group: the compounds left-to-right with the combinators
    /// BETWEEN them (combinators[i] joins compounds[i] and compounds[i+1]).
    #[derive(Debug, Clone)]
    pub(super) struct Complex {
        compounds: Vec<Compound>,
        combinators: Vec<Comb>,
    }

    fn is_ident_char(c: char) -> bool {
        c.is_alphanumeric() || c == '-' || c == '_' || (c as u32) > 0x7F
    }

    struct SelectorParser<'a> {
        src: &'a str,
        bytes: Vec<char>,
        pos: usize,
    }

    impl<'a> SelectorParser<'a> {
        fn new(src: &'a str) -> Self {
            SelectorParser {
                src,
                bytes: src.chars().collect(),
                pos: 0,
            }
        }

        fn peek(&self) -> Option<char> {
            self.bytes.get(self.pos).copied()
        }

        fn bump(&mut self) -> Option<char> {
            let c = self.peek()?;
            self.pos += 1;
            Some(c)
        }

        fn skip_ws(&mut self) -> bool {
            // Returns true when at least one whitespace char was consumed.
            let start = self.pos;
            while matches!(
                self.peek(),
                Some(' ') | Some('\t') | Some('\n') | Some('\r')
            ) {
                self.pos += 1;
            }
            self.pos != start
        }

        fn read_ident(&mut self) -> Result<String, String> {
            let start = self.pos;
            while let Some(c) = self.peek() {
                if is_ident_char(c) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos == start {
                return Err(format!(
                    "expected an identifier at position {} in the selector {:?}",
                    start, self.src
                ));
            }
            Ok(self.bytes[start..self.pos].iter().collect())
        }

        fn read_compound(&mut self) -> Result<Compound, String> {
            let mut simples = Vec::new();
            loop {
                match self.peek() {
                    Some('*') => {
                        self.bump();
                        simples.push(SimpleSel::Tag("*".to_string()));
                    }
                    Some('.') => {
                        self.bump();
                        let name = self.read_ident()?;
                        simples.push(SimpleSel::Class(name));
                    }
                    Some('#') => {
                        self.bump();
                        let name = self.read_ident()?;
                        simples.push(SimpleSel::Id(name));
                    }
                    Some('[') => {
                        self.bump();
                        self.skip_ws();
                        let attr = self.read_ident()?.to_lowercase();
                        self.skip_ws();
                        match self.peek() {
                            Some(']') => {
                                self.bump();
                                simples.push(SimpleSel::AttrExists(attr));
                            }
                            Some('=') => {
                                self.bump();
                                self.skip_ws();
                                let value = self.read_attr_value()?;
                                self.skip_ws();
                                if self.peek() != Some(']') {
                                    return Err(format!(
                                        "expected ']' at position {} in the selector {:?}",
                                        self.pos, self.src
                                    ));
                                }
                                self.bump();
                                simples.push(SimpleSel::AttrEq(attr, value));
                            }
                            other => {
                                return Err(format!(
                                    "expected ']' or '=' after [{}], got {:?} at position {} \
                                 in the selector {:?}",
                                    attr, other, self.pos, self.src
                                ));
                            }
                        }
                    }
                    Some(c) if is_ident_char(c) && simples.is_empty() => {
                        // A tag name may open a compound only (`.a.b` has no tag;
                        // `div.a` has one). tag names are matched case-insensitively
                        // (html5ever lowercases them on parse).
                        let tag = self.read_ident()?.to_lowercase();
                        simples.push(SimpleSel::Tag(tag));
                    }
                    _ => break,
                }
                // After the first simple selector, only . # [ may continue the
                // compound; anything else ends it.
                match self.peek() {
                    Some('.') | Some('#') | Some('[') => continue,
                    _ => break,
                }
            }
            if simples.is_empty() {
                return Err(format!(
                    "empty compound selector at position {} in the selector {:?}",
                    self.pos, self.src
                ));
            }
            Ok(Compound { simples })
        }

        fn read_attr_value(&mut self) -> Result<String, String> {
            match self.peek() {
                Some(q @ ('"' | '\'')) => {
                    self.bump();
                    let mut value = String::new();
                    loop {
                        match self.bump() {
                            Some(c) if c == q => break,
                            Some(c) => value.push(c),
                            None => {
                                return Err(format!(
                                    "unterminated quoted attribute value in the selector {:?}",
                                    self.src
                                ))
                            }
                        }
                    }
                    Ok(value)
                }
                _ => {
                    let start = self.pos;
                    while let Some(c) = self.peek() {
                        if c == ']' || c.is_whitespace() {
                            break;
                        }
                        self.pos += 1;
                    }
                    if self.pos == start {
                        return Err(format!(
                            "empty attribute value at position {} in the selector {:?}",
                            start, self.src
                        ));
                    }
                    Ok(self.bytes[start..self.pos].iter().collect())
                }
            }
        }

        /// One comma group.
        fn read_complex(&mut self) -> Result<Complex, String> {
            self.skip_ws();
            let mut compounds = vec![self.read_compound()?];
            let mut combinators = Vec::new();
            loop {
                // A combinator or the end of the group.
                let had_ws = self.skip_ws();
                match self.peek() {
                    Some('>') => {
                        self.bump();
                        self.skip_ws();
                        combinators.push(Comb::Child);
                        compounds.push(self.read_compound()?);
                    }
                    Some(c) if c != ',' && had_ws => {
                        combinators.push(Comb::Descendant);
                        compounds.push(self.read_compound()?);
                    }
                    _ => break,
                }
            }
            Ok(Complex {
                compounds,
                combinators,
            })
        }

        fn parse_all(&mut self) -> Result<Vec<Complex>, String> {
            let mut groups = vec![self.read_complex()?];
            loop {
                self.skip_ws();
                match self.peek() {
                    Some(',') => {
                        self.bump();
                        groups.push(self.read_complex()?);
                    }
                    Some(_) => {
                        return Err(format!(
                            "unexpected character {:?} at position {} in the selector {:?} \
                         (the supported subset: tag, .class, #id, [attr], [attr=value], \
                         descendant, >, comma)",
                            self.peek(),
                            self.pos,
                            self.src
                        ));
                    }
                    None => break,
                }
            }
            Ok(groups)
        }
    }

    pub(super) fn parse_selectors(builtin: &str, css: &str) -> Result<Vec<Complex>, String> {
        if css.trim().is_empty() {
            return Err(format!("{}(): the selector must not be empty", builtin));
        }
        SelectorParser::new(css).parse_all()
    }

    // ── Matching ───────────────────────────────────────────────────────────

    fn matches_simple(handle: &Handle, sel: &SimpleSel) -> bool {
        match sel {
            SimpleSel::Tag(t) => {
                if t == "*" {
                    true
                } else {
                    tag_name(handle).as_deref() == Some(t.as_str())
                }
            }
            SimpleSel::Class(class) => element_attrs(handle)
                .iter()
                .any(|(k, v)| k == "class" && v.split_whitespace().any(|c| c == class.as_str())),
            SimpleSel::Id(id) => element_attrs(handle)
                .iter()
                .any(|(k, v)| k == "id" && v == id.as_str()),
            SimpleSel::AttrExists(attr) => element_attrs(handle)
                .iter()
                .any(|(k, _)| k == attr.as_str()),
            SimpleSel::AttrEq(attr, value) => element_attrs(handle)
                .iter()
                .any(|(k, v)| k == attr.as_str() && v == value.as_str()),
        }
    }

    fn matches_compound(handle: &Handle, compound: &Compound) -> bool {
        compound.simples.iter().all(|s| matches_simple(handle, s))
    }

    /// Right-to-left complex matching: the LAST compound must match the
    /// element itself; the combinators walk UP the ancestor chain.
    fn matches_complex(handle: &Handle, complex: &Complex) -> bool {
        let last = complex.compounds.len() - 1;
        if !matches_compound(handle, &complex.compounds[last]) {
            return false;
        }
        matches_rest(handle, complex, last)
    }

    fn matches_rest(handle: &Handle, complex: &Complex, ci: usize) -> bool {
        if ci == 0 {
            return true;
        }
        let comb = complex.combinators[ci - 1];
        match comb {
            Comb::Child => match parent_element(handle) {
                Some(p) if matches_compound(&p, &complex.compounds[ci - 1]) => {
                    matches_rest(&p, complex, ci - 1)
                }
                _ => false,
            },
            Comb::Descendant => {
                let mut cur = parent_element(handle);
                while let Some(p) = cur {
                    if matches_compound(&p, &complex.compounds[ci - 1])
                        && matches_rest(&p, complex, ci - 1)
                    {
                        return true;
                    }
                    cur = parent_element(&p);
                }
                false
            }
        }
    }

    pub(super) fn matches_any(handle: &Handle, groups: &[Complex]) -> bool {
        groups.iter().any(|c| matches_complex(handle, c))
    }

    // ── The element walk ───────────────────────────────────────────────────

    /// Pre-order walk of every element; the callback's Err aborts the walk
    /// (the loud-limit path).
    pub(super) fn walk_elements(
        handle: &Handle,
        out: &mut dyn FnMut(&Handle) -> Result<(), String>,
    ) -> Result<(), String> {
        if let NodeData::Element { .. } = &handle.data {
            out(handle)?;
        }
        for child in handle.children.borrow().iter() {
            walk_elements(child, out)?;
        }
        Ok(())
    }

    // ── Serialization (the HtmlMatch.html field) ──────────────────────────

    const VOID_ELEMENTS: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ];

    fn escape_text(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    fn escape_attr(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    /// Serialize an element subtree back to HTML (the honest fragment: the
    /// DOM is the parse product, the serialization re-escapes the text and
    /// the attribute values).
    fn serialize_element(handle: &Handle, out: &mut String) {
        match &handle.data {
            NodeData::Element { name, attrs, .. } => {
                let tag = name.local.to_string().to_lowercase();
                out.push('<');
                out.push_str(&tag);
                for a in attrs.borrow().iter() {
                    out.push(' ');
                    out.push_str(&a.name.local.to_string().to_lowercase());
                    out.push_str("=\"");
                    out.push_str(&escape_attr(a.value.as_ref()));
                    out.push('"');
                }
                out.push('>');
                if VOID_ELEMENTS.contains(&tag.as_str()) {
                    return;
                }
                serialize_children(handle, out);
                out.push_str("</");
                out.push_str(&tag);
                out.push('>');
            }
            NodeData::Text { contents } => out.push_str(&escape_text(&contents.borrow())),
            NodeData::Document => serialize_children(handle, out),
            NodeData::Comment { contents } => {
                out.push_str("<!--");
                out.push_str(contents);
                out.push_str("-->");
            }
            NodeData::Doctype { .. } | NodeData::ProcessingInstruction { .. } => {}
        }
    }

    fn serialize_children(handle: &Handle, out: &mut String) {
        for child in handle.children.borrow().iter() {
            serialize_element(child, out);
        }
    }

    // ── The builtin handlers ───────────────────────────────────────────────

    fn make_dict(pairs: Vec<(String, String)>) -> Value {
        Value::Struct {
            type_name: "Dict".to_string(),
            fields: pairs
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        }
    }

    /// `html_select(html, css) -> List<Struct<HtmlMatch>{tag, text, attrs, html}>`
    /// — every element matching ANY comma-separated selector (document
    /// order). The loud limits: the 5 MiB input, the 10 000-node result.
    pub(crate) fn builtin_html_select(args: &[Value]) -> Result<Value, String> {
        let html = expect_string_arg("html_select", args, 0)?;
        let css = expect_string_arg("html_select", args, 1)?;
        let dom = parse_html("html_select", &html)?;
        let groups = parse_selectors("html_select", &css)?;

        let mut matches: Vec<Value> = Vec::new();
        walk_elements(&dom.document, &mut |el: &Handle| {
            if matches_any(el, &groups) {
                if matches.len() >= HTML_RESULT_LIMIT {
                    return Err(format!(
                        "html_select(): the result exceeds the {}-node limit; \
                     narrow the selector or split the source (never a silent truncate)",
                        HTML_RESULT_LIMIT
                    ));
                }
                let tag = tag_name(el).unwrap_or_default();
                let attrs = make_dict(element_attrs(el));
                let mut frag = String::new();
                serialize_element(el, &mut frag);
                matches.push(Value::Struct {
                    type_name: "HtmlMatch".to_string(),
                    fields: [
                        ("tag".to_string(), Value::String(tag)),
                        ("text".to_string(), Value::String(element_text(el))),
                        ("attrs".to_string(), attrs),
                        ("html".to_string(), Value::String(frag)),
                    ]
                    .into_iter()
                    .collect(),
                });
            }
            Ok(())
        })?;
        Ok(Value::List(matches))
    }

    /// `html_text(html, css?) -> String` — the normalized concatenated text
    /// of the matched elements (no selector = the whole document). The
    /// script/style subtrees NEVER contribute; whitespace runs collapse.
    pub(crate) fn builtin_html_text(args: &[Value]) -> Result<Value, String> {
        let html = expect_string_arg("html_text", args, 0)?;
        let dom = parse_html("html_text", &html)?;
        // The optional selector: present-and-String, present-and-not-String
        // is a loud type error (the parity with the String-typed lanes).
        let groups = match args.get(1) {
            None | Some(Value::Unit) => None,
            Some(Value::String(s)) if s.trim().is_empty() => None,
            Some(Value::String(s)) => Some(parse_selectors("html_text", s)?),
            Some(other) => {
                return Err(format!(
                    "html_text() expected String as the selector, got {}",
                    other.type_name()
                ))
            }
        };

        let mut pieces: Vec<String> = Vec::new();
        match &groups {
            None => {
                let mut raw = String::new();
                collect_text(&dom.document, &mut raw);
                let normalized = normalize_ws(&raw);
                if !normalized.is_empty() {
                    pieces.push(normalized);
                }
            }
            Some(groups) => {
                let mut matched = 0usize;
                walk_elements(&dom.document, &mut |el: &Handle| {
                    if matches_any(el, groups) {
                        matched += 1;
                        if matched > HTML_RESULT_LIMIT {
                            return Err(format!(
                                "html_text(): the result exceeds the {}-node limit; \
                             narrow the selector or split the source",
                                HTML_RESULT_LIMIT
                            ));
                        }
                        let text = element_text(el);
                        if !text.is_empty() {
                            pieces.push(text);
                        }
                    }
                    Ok(())
                })?;
            }
        }
        Ok(Value::String(pieces.join(" ")))
    }

    /// `html_links(html, base_url) -> List<Struct<HtmlLink>{href, text}>` —
    /// every <a> with a resolvable href; the hrefs are ABSOLUTE (resolved
    /// against base_url), only http/https survive — `javascript:` (and any
    /// other scheme) is DROPPED by contract. The base_url must itself be an
    /// http/https URL (loud otherwise).
    pub(crate) fn builtin_html_links(args: &[Value]) -> Result<Value, String> {
        let html = expect_string_arg("html_links", args, 0)?;
        let base = expect_string_arg("html_links", args, 1)?;
        let dom = parse_html("html_links", &html)?;

        let base_url = url::Url::parse(&base)
            .map_err(|e| format!("html_links(): base_url is not a valid URL: {}", e))?;
        if base_url.scheme() != "http" && base_url.scheme() != "https" {
            return Err(format!(
                "html_links(): base_url must be an http/https URL, got scheme {:?}",
                base_url.scheme()
            ));
        }

        let mut links: Vec<Value> = Vec::new();
        walk_elements(&dom.document, &mut |el: &Handle| {
            if tag_name(el).as_deref() != Some("a") {
                return Ok(());
            }
            let href = element_attrs(el)
                .iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.trim().to_string());
            let Some(href) = href else {
                return Ok(()); // an <a> without href is an anchor, not a link
            };
            if href.is_empty() {
                return Ok(());
            }
            // The scheme contract: only absolute http/https survive. The
            // WHATWG resolution handles the relative forms; a `javascript:`
            // href resolves to a non-http scheme and is dropped here.
            let resolved = match base_url.join(&href) {
                Ok(u) if u.scheme() == "http" || u.scheme() == "https" => u,
                _ => return Ok(()), // javascript:, mailto:, data:, garbage — dropped
            };
            if links.len() >= HTML_RESULT_LIMIT {
                return Err(format!(
                    "html_links(): the result exceeds the {}-node limit; \
                 split the source (never a silent truncate)",
                    HTML_RESULT_LIMIT
                ));
            }
            links.push(Value::Struct {
                type_name: "HtmlLink".to_string(),
                fields: [
                    ("href".to_string(), Value::String(resolved.to_string())),
                    ("text".to_string(), Value::String(element_text(el))),
                ]
                .into_iter()
                .collect(),
            });
            Ok(())
        })?;
        Ok(Value::List(links))
    }

    // ── Tests (the N5 unit legs; the integration/golden legs live in
    // tests/naryad_n5_html_builtins.rs, feature-gated) ─────────────────────
}

// The real handlers surface at the module level only with the feature.
#[cfg(feature = "html")]
pub(crate) use real::{builtin_html_links, builtin_html_select, builtin_html_text};

#[cfg(all(test, feature = "html"))]
mod tests {
    use super::real::{matches_any, parse_html, parse_selectors, walk_elements, HTML_INPUT_LIMIT};

    #[test]
    fn test_selector_parse_and_match_basics() {
        let groups = parse_selectors("html_select", "div.lot, a[href]").expect("parse");
        assert_eq!(groups.len(), 2);
        let html = r#"<div class="lot">One</div><span>x</span><a href="/p">P</a>"#;
        let dom = parse_html("html_select", html).expect("parse html");
        let mut hits = 0usize;
        walk_elements(&dom.document, &mut |el| {
            if matches_any(el, &groups) {
                hits += 1;
            }
            Ok(())
        })
        .expect("walk");
        assert_eq!(hits, 2, "div.lot and a[href] match; span does not");
    }

    #[test]
    fn test_child_and_descendant_combinators() {
        let html = r#"<div id="a"><p>direct</p><section><p>nested</p></section></div>"#;
        let dom = parse_html("html_text", html).expect("parse");

        let child = parse_selectors("html_text", "div > p").expect("parse");
        let descendant = parse_selectors("html_text", "div p").expect("parse");
        let mut child_hits = 0usize;
        let mut desc_hits = 0usize;
        walk_elements(&dom.document, &mut |el| {
            if matches_any(el, &child) {
                child_hits += 1;
            }
            if matches_any(el, &descendant) {
                desc_hits += 1;
            }
            Ok(())
        })
        .expect("walk");
        assert_eq!(child_hits, 1, "only the direct p");
        assert_eq!(desc_hits, 2, "both p elements");
    }

    #[test]
    fn test_unsupported_selector_is_loud() {
        let err = parse_selectors("html_select", "div p:first-child").expect_err("loud");
        assert!(
            err.contains("unexpected character"),
            "the error must name the position and the supported subset: {err}"
        );
    }

    #[test]
    fn test_input_limit_is_loud() {
        let big = "x".repeat(HTML_INPUT_LIMIT + 1);
        let err = match parse_html("html_select", &big) {
            Err(e) => e,
            Ok(_) => panic!("the 5 MiB input limit must fail loud"),
        };
        assert!(
            err.contains("5 MiB"),
            "the error must name the limit: {err}"
        );
    }
}
