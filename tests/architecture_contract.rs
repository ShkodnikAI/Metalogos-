//! Executable architecture contracts for Metalogos- (naryad #382, issue #502, idea A10).
//!
//! Mechanization of the ADR culture after the donor reference: memorax-code
//! (star 1299, MIT, v0.1.18) `ARCHITECTURE.md` §5.2 (the ownership table
//! "MUST own / MUST NOT own") and §5.3 (executable contracts with the
//! discipline "do not weaken a boundary to get a green build"). The frozen
//! inventory and every rule below were verified on live main `c60651b`
//! (2026-09-18) by an independent audit prototype; re-verified by the
//! mutation demonstrations logged in the naryad report.
//!
//! # Ownership table (§5.2)
//!
//! | Component             | MUST own                                                | MUST NOT own                              |
//! |-----------------------|---------------------------------------------------------|-------------------------------------------|
//! | root crate metalogos  | language core: parser, semantic, execution, gates, CLI  | mlogpkg / mlog-lsp (satellite crates)     |
//! | builtins              | builtin registry and implementations                    | transport (server, mcp_server)            |
//! | vm / interpreter      | execution, evaluation, runtime state                    | transport (server, mcp_server)            |
//! | parser / ast          | syntax, AST, lowering                                   | transport (server, mcp_server)            |
//! | ledger / consent      | provenance substrate: records, chain, consent surface   | builtin surface; transport                |
//! | mlogpkg / mlog-lsp    | packaging / LSP tooling (may depend on metalogos)       | -                                         |
//!
//! # Boundary-change rule (§5.3)
//!
//! Do NOT weaken a contract to make a build pass. An intentional boundary
//! change lands as the ownership table + the contract test updated in ONE
//! PR, with the reason stated in the PR description. A red test is a
//! signal, not an obstacle: fix the dependency direction, or consciously
//! move the boundary — never silence the check.
//!
//! # Loud boundary of the scanner (precedent ADR-0125 / naryad #241)
//!
//! * the scan is textual: comments are stripped; string literals are NOT
//!   parsed (a module path inside a string literal is a rare
//!   false-positive risk, accepted loudly);
//! * macros are NOT expanded; `build.rs` is not analyzed; cfg-feature
//!   graphs are not modeled — a `#[cfg(feature = "server")] use
//!   crate::server;` is still an architectural edge at source level and
//!   still counts;
//! * `use super::X` resolves to the TOP-LEVEL module of the file (the
//!   naryad spec), not to the immediate parent path component;
//! * transitive dependencies are out of scope — these contracts are
//!   targeted checks of DIRECT source edges, not a replacement for
//!   cargo-deny;
//! * rustfmt-canonical sources are assumed (`use crate::{`, no space
//!   between the path and the brace group).
//!
//! # The C4 ratchet (frozen cycle inventory)
//!
//! The file-level `crate::` graph carries exactly two cycles today
//! (Tarjan SCCs over 149 files / ~300 direct edges at `c60651b`):
//!
//! * SCC-1: src/ast.rs, src/builtins/mod.rs, src/bytecode.rs,
//!   src/interpreter/mod.rs, src/llm.rs;
//! * SCC-2: src/audit.rs, src/semantic.rs.
//!
//! A NEW cycle, a NEW intra-cycle edge, or the DISAPPEARANCE of a frozen
//! cycle/edge turns `c4_acyclicity_ratchet` red. Shrinkage (boundary
//! compression) is welcome — and is still a loud event: update
//! `FROZEN_SCCS` in the same PR so the inventory never drifts silently.
//!
//! Runtime budget: source scan only, no compilation — well under 5 s.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ── Scan primitives ─────────────────────────────────────────────────────

/// Root of the crate sources (the workspace root when run via `cargo test`).
fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Recursively collect `.rs` files under `dir` as sorted `src/...` paths.
fn collect_rs_files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(format!("src/{rel}"));
        }
    }
}

/// Textual comment stripper: replaces `// ...` to end of line and
/// `/* ... */` spans with spaces (newlines inside block comments are kept
/// so line numbers stay valid). Naive by design — string literals are not
/// parsed (see the loud boundary in the module docs).
fn strip_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(b' ');
                i += 1;
            }
        } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            out.push(b' ');
            out.push(b' ');
            i += 2;
            while i < bytes.len() {
                if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                    break;
                }
                if bytes[i] == b'\n' {
                    out.push(b'\n');
                } else {
                    out.push(b' ');
                }
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Top-level module name -> representative module file (`X.rs` preferred
/// over `X/mod.rs`, mirroring the audit prototype that produced the
/// frozen C4 inventory).
fn module_map(files: &[String]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for f in files {
        let rel = f.strip_prefix("src/").unwrap_or(f);
        if let Some(stem) = rel.strip_suffix(".rs") {
            if !stem.contains('/') {
                map.entry(stem.to_string()).or_insert_with(|| f.clone());
            }
        }
    }
    for f in files {
        let rel = f.strip_prefix("src/").unwrap_or(f);
        if let Some(dir) = rel.strip_suffix("/mod.rs") {
            if !dir.contains('/') {
                map.entry(dir.to_string()).or_insert_with(|| f.clone());
            }
        }
    }
    map
}

// ── Edge extraction ─────────────────────────────────────────────────────

/// One direct source edge `from` -> `to` (module file), via `head`
/// (the top-level module name, or `super`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    from: String,
    to: String,
    via: String,
    line: usize,
}

struct Scan {
    files: Vec<String>,
    modules: BTreeMap<String, String>,
    edges: BTreeSet<Edge>,
}

const USE_CRATE: &str = "use crate::";
const USE_SUPER: &str = "use super::";
const CRATE_PATH: &str = "crate::";

fn scan_sources() -> Scan {
    let dir = src_dir();
    let files = collect_rs_files(&dir);
    let modules = module_map(&files);
    let mut edges = BTreeSet::new();
    for rel in &files {
        let path = dir.join(rel.strip_prefix("src/").unwrap_or(rel));
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let stripped = strip_comments(&text);
        for edge in extract_edges(rel, &stripped, &modules) {
            edges.insert(edge);
        }
    }
    Scan {
        files,
        modules,
        edges,
    }
}

/// Shared scan: computed once, reused by every contract test.
fn scan() -> &'static Scan {
    static SCAN: OnceLock<Scan> = OnceLock::new();
    SCAN.get_or_init(scan_sources)
}

fn matching_brace(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if b.first() != Some(&b'{') {
        return None;
    }
    let mut depth = 1usize;
    for (i, &c) in b.iter().enumerate().skip(1) {
        match c {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split on commas that sit at brace depth zero.
fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '{' => {
                depth += 1;
                cur.push(c);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            ',' if depth == 0 => {
                items.push(cur.clone());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        items.push(cur);
    }
    items
}

/// ` a::{b, c} as d ` -> `a` (mirrors the prototype: strip `as`-rename,
/// take the first path segment).
fn head_of_item(item: &str) -> String {
    let base = item.trim().split(" as ").next().unwrap_or("").trim();
    base.split("::").next().unwrap_or("").trim().to_string()
}

fn read_ident(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            break;
        }
    }
    out
}

fn line_of(stripped: &str, pos: usize) -> usize {
    stripped[..pos].matches('\n').count() + 1
}

fn extract_edges(rel: &str, stripped: &str, modules: &BTreeMap<String, String>) -> Vec<Edge> {
    let mut out = Vec::new();
    let add = |out: &mut Vec<Edge>, pos: usize, head: &str| {
        if let Some(to) = modules.get(head) {
            if to != rel {
                out.push(Edge {
                    from: rel.to_string(),
                    to: to.clone(),
                    via: head.to_string(),
                    line: line_of(stripped, pos),
                });
            }
        }
    };

    // `use crate::...` — group and single forms (`pub use` shares the needle).
    let mut from = 0usize;
    while let Some(found) = stripped[from..].find(USE_CRATE) {
        let pos = from + found;
        let after = pos + USE_CRATE.len();
        let rest = &stripped[after..];
        let rb = rest.as_bytes();
        if rb.first() == Some(&b'{') {
            let close = matching_brace(rest).unwrap_or(rest.len());
            let body = &rest[1..close];
            for item in split_top_level_commas(body) {
                let head = head_of_item(&item);
                if !head.is_empty() {
                    add(&mut out, pos, &head);
                }
            }
        } else if rb
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        {
            let head = read_ident(rest);
            if !head.is_empty() {
                add(&mut out, pos, &head);
            }
        }
        from = after;
    }

    // `use super::X` — resolve to the TOP-LEVEL module of this file
    // (naryad spec); a top-level file's `super` is the crate root facade
    // and produces no edge.
    if rel.contains('/') {
        let parent_name = rel.split('/').next().unwrap_or("");
        if let Some(parent_file) = modules.get(parent_name) {
            let mut from = 0usize;
            while let Some(found) = stripped[from..].find(USE_SUPER) {
                let pos = from + found;
                let rest = &stripped[pos + USE_SUPER.len()..];
                let rb = rest.as_bytes();
                if rb
                    .first()
                    .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
                {
                    let to = parent_file.clone();
                    if to != rel {
                        out.push(Edge {
                            from: rel.to_string(),
                            to,
                            via: "super".to_string(),
                            line: line_of(stripped, pos),
                        });
                    }
                }
                from = pos + USE_SUPER.len();
            }
        }
    }

    // Bare `crate::head` mentions in code and type positions (lowercase
    // heads, mirroring the prototype; includes the use-statements above —
    // the edge set dedups).
    let mut from = 0usize;
    while let Some(found) = stripped[from..].find(CRATE_PATH) {
        let pos = from + found;
        let rest = &stripped[pos + CRATE_PATH.len()..];
        let rb = rest.as_bytes();
        if rb
            .first()
            .is_some_and(|c| c.is_ascii_lowercase() || *c == b'_')
        {
            let head = read_ident(rest);
            if !head.is_empty() {
                add(&mut out, pos, &head);
            }
        }
        from = pos + CRATE_PATH.len();
    }

    out
}

// ── Tarjan SCC over the file-level graph ────────────────────────────────

fn tarjan_sccs(nodes: &[String], adj: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    struct State {
        index: HashMap<String, usize>,
        low: HashMap<String, usize>,
        on_stack: HashSet<String>,
        stack: Vec<String>,
        counter: usize,
        sccs: Vec<Vec<String>>,
    }

    fn connect(v: String, adj: &BTreeMap<String, BTreeSet<String>>, st: &mut State) {
        st.index.insert(v.clone(), st.counter);
        st.low.insert(v.clone(), st.counter);
        st.counter += 1;
        st.stack.push(v.clone());
        st.on_stack.insert(v.clone());
        if let Some(neigh) = adj.get(&v) {
            for w in neigh {
                if !st.index.contains_key(w) {
                    connect(w.clone(), adj, st);
                    let low_w = st.low[w];
                    let low_v = st.low[&v];
                    st.low.insert(v.clone(), low_w.min(low_v));
                } else if st.on_stack.contains(w) {
                    let idx_w = st.index[w];
                    let low_v = st.low[&v];
                    st.low.insert(v.clone(), idx_w.min(low_v));
                }
            }
        }
        if st.low[&v] == st.index[&v] {
            let mut comp = Vec::new();
            while let Some(w) = st.stack.pop() {
                st.on_stack.remove(&w);
                comp.push(w.clone());
                if w == v {
                    break;
                }
            }
            if comp.len() > 1 {
                st.sccs.push(comp);
            }
        }
    }

    let mut st = State {
        index: HashMap::new(),
        low: HashMap::new(),
        on_stack: HashSet::new(),
        stack: Vec::new(),
        counter: 0,
        sccs: Vec::new(),
    };
    for n in nodes {
        if !st.index.contains_key(n) {
            connect(n.clone(), adj, &mut st);
        }
    }
    st.sccs
}

// ── Frozen cycle inventory (C4 ratchet), verified at c60651b ────────────

struct FrozenScc {
    files: &'static [&'static str],
    edges: &'static [&'static [&'static str; 2]],
}

const FROZEN_SCCS: &[FrozenScc] = &[
    // SCC-1: the language-core tangle — builtins need interpreter Value
    // plumbing, the interpreter needs builtins' registry, bytecode links
    // both, ast is the shared vocabulary, llm rounds the ring.
    FrozenScc {
        files: &[
            "src/ast.rs",
            "src/builtins/mod.rs",
            "src/bytecode.rs",
            "src/interpreter/mod.rs",
            "src/llm.rs",
        ],
        edges: &[
            &["src/ast.rs", "src/interpreter/mod.rs"],
            &["src/builtins/mod.rs", "src/interpreter/mod.rs"],
            &["src/bytecode.rs", "src/ast.rs"],
            &["src/bytecode.rs", "src/interpreter/mod.rs"],
            &["src/interpreter/mod.rs", "src/ast.rs"],
            &["src/interpreter/mod.rs", "src/builtins/mod.rs"],
            &["src/interpreter/mod.rs", "src/bytecode.rs"],
            &["src/interpreter/mod.rs", "src/llm.rs"],
            &["src/llm.rs", "src/ast.rs"],
            &["src/llm.rs", "src/interpreter/mod.rs"],
        ],
    },
    // SCC-2: the audit <-> semantic analysis pair.
    FrozenScc {
        files: &["src/audit.rs", "src/semantic.rs"],
        edges: &[
            &["src/audit.rs", "src/semantic.rs"],
            &["src/semantic.rs", "src/audit.rs"],
        ],
    },
];

// ── Forbidden-edge rules (C2 / C3 / C5 / C6) ────────────────────────────

/// Importer patterns are exact paths or `src/dir/` prefixes.
fn importer_matches(rel: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| rel == *p || rel.starts_with(p))
}

fn assert_no_forbidden_edges(id: &str, importers: &[&str], forbidden: &[&str], why: &str) {
    let scan = scan();
    let mut bad: Vec<String> = Vec::new();
    for e in &scan.edges {
        if importer_matches(&e.from, importers) && forbidden.contains(&e.via.as_str()) {
            bad.push(format!(
                "{}:{} -> crate::{} (resolved: {})",
                e.from, e.line, e.via, e.to
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "[{id}] forbidden dependency edges found ({}):\n  {}\nrule: {why}\nDo not weaken the contract to make the build pass; move the boundary deliberately (ownership table + test, one PR).",
        bad.len(),
        bad.join("\n  "),
    );
}

#[test]
fn c2_builtins_never_touch_transport() {
    assert_no_forbidden_edges(
        "C2",
        &["src/builtins/"],
        &["server", "mcp_server"],
        "builtins own the builtin registry and implementations; transport (server / mcp_server) lives strictly above the builtin layer",
    );
}

#[test]
fn c3_execution_core_never_touches_transport() {
    assert_no_forbidden_edges(
        "C3",
        &["src/vm.rs", "src/interpreter/"],
        &["server", "mcp_server"],
        "the execution core (tree-walking interpreter, VM) stays transport-blind; media handle types re-exported from interpreter/values.rs are data, not transport",
    );
}

#[test]
fn c5_frontend_never_touches_transport() {
    assert_no_forbidden_edges(
        "C5",
        &["src/parser/", "src/ast.rs"],
        &["server", "mcp_server"],
        "the frontend (parser, ast) knows nothing about transport",
    );
}

#[test]
fn c6_provenance_substrate_isolation() {
    assert_no_forbidden_edges(
        "C6",
        &["src/ledger.rs", "src/consent.rs"],
        &["builtins", "server", "mcp_server"],
        "the provenance substrate (signed action ledger, consent surface) is a low layer: no builtin surface, no transport",
    );
}

// ── C4: the acyclicity ratchet ──────────────────────────────────────────

#[test]
fn c4_acyclicity_ratchet() {
    let scan = scan();

    let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for e in &scan.edges {
        adj.entry(e.from.clone()).or_default().insert(e.to.clone());
    }

    let mut computed: Vec<Vec<String>> = tarjan_sccs(&scan.files, &adj);
    for comp in computed.iter_mut() {
        comp.sort();
    }
    computed.sort();

    let mut expected: Vec<Vec<String>> = FROZEN_SCCS
        .iter()
        .map(|f| f.files.iter().map(|s| (*s).to_string()).collect())
        .collect();
    expected.sort();

    if computed != expected {
        panic!(
            "C4: module-cycle inventory drift.\ncomputed cycles: {computed:#?}\nfrozen cycles:  {expected:#?}\nA cycle here that is not in FROZEN_SCCS is a violation.\nA frozen cycle that vanished means the inventory drifted - update FROZEN_SCCS deliberately (ownership table + test, one PR).\nDo not weaken a contract to make a build pass."
        );
    }

    // Exact intra-cycle edge ratchet: growth is a violation, shrinkage is
    // compression that must still be loud.
    for f in FROZEN_SCCS {
        let set: HashSet<&str> = f.files.iter().copied().collect();
        let mut intra: BTreeSet<(String, String)> = BTreeSet::new();
        for e in &scan.edges {
            if set.contains(e.from.as_str()) && set.contains(e.to.as_str()) {
                intra.insert((e.from.clone(), e.to.clone()));
            }
        }
        let frozen: BTreeSet<(String, String)> = f
            .edges
            .iter()
            .map(|p| (p[0].to_string(), p[1].to_string()))
            .collect();
        let extra: Vec<_> = intra.difference(&frozen).collect();
        let gone: Vec<_> = frozen.difference(&intra).collect();
        assert!(
            extra.is_empty() && gone.is_empty(),
            "C4: intra-cycle edge drift in cycle {:#?}\nnew edges (VIOLATION): {:#?}\nvanished edges (if deliberate compression - update FROZEN_SCCS in the same PR): {:#?}\nDo not weaken a contract to make a build pass.",
            f.files,
            extra,
            gone
        );
    }
}

// ── C1: root-crate independence from the satellite crates ───────────────

fn is_dep_section(section: &str) -> bool {
    section == "dependencies"
        || section == "dev-dependencies"
        || section == "build-dependencies"
        || section.ends_with("dependencies")
        || section.starts_with("dependencies.")
        || section.starts_with("dev-dependencies.")
        || section.starts_with("build-dependencies.")
}

fn mentions_satellite(s: &str) -> bool {
    s.contains("mlogpkg") || s.contains("mlog-lsp") || s.contains("mlog_lsp")
}

#[test]
fn c1_root_crate_independence() {
    // (a) manifest scan: no satellite crate in the root crate's dependency
    // sections (inline key or table-style `[dependencies.mlogpkg]`).
    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let toml = fs::read_to_string(&toml_path).expect("root Cargo.toml is readable");
    let mut section = String::new();
    for (i, raw) in toml.lines().enumerate() {
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') {
            section = t.trim_matches(|c| c == '[' || c == ']').to_string();
            assert!(
                !(is_dep_section(&section) && mentions_satellite(&section)),
                "C1: root Cargo.toml:{} - satellite crate declared as a dependency section: [{section}]",
                i + 1
            );
            continue;
        }
        assert!(
            !(is_dep_section(&section) && mentions_satellite(t)),
            "C1: root Cargo.toml:{} - the root crate depends on a satellite crate: `{t}` (section [{section}]).\nAllowed direction: mlogpkg / mlog-lsp -> metalogos only.",
            i + 1
        );
    }

    // (b) source scan: no satellite crate is even mentioned in src/.
    let scan = scan();
    let dir = src_dir();
    let mut bad: Vec<String> = Vec::new();
    for rel in &scan.files {
        let path = dir.join(rel.strip_prefix("src/").unwrap_or(rel));
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let stripped = strip_comments(&text);
        for token in ["mlogpkg", "mlog_lsp", "mlog-lsp"] {
            if stripped.contains(token) {
                bad.push(format!("{rel} mentions `{token}`"));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "C1: root-crate sources reference satellite crates:\n  {}\nThe root crate `metalogos` must not depend on `mlogpkg` / `mlog-lsp`.",
        bad.join("\n  ")
    );
}

// ── Meta-guard: anchors and scanner sanity ──────────────────────────────

/// Every rule above would pass vacuously if the scanner silently saw
/// nothing (wrong path, renamed module, deleted frozen file). This test
/// pins the anchors: when one breaks, the contracts must be updated
/// deliberately, not skipped.
#[test]
fn contract_anchors_hold() {
    let scan = scan();

    assert!(
        scan.files.len() >= 100,
        "scanner found only {} source files - the walk is broken and every contract below would pass vacuously",
        scan.files.len()
    );
    assert!(
        scan.modules.len() >= 25,
        "scanner resolved only {} top-level modules - the walk is broken",
        scan.modules.len()
    );
    assert!(
        scan.edges.len() >= 100,
        "scanner found only {} direct edges - the parser is broken and forbidden-edge rules would pass vacuously",
        scan.edges.len()
    );

    for anchor in [
        "src/builtins/mod.rs",
        "src/interpreter/mod.rs",
        "src/vm.rs",
        "src/parser/mod.rs",
        "src/ast.rs",
        "src/ledger.rs",
        "src/consent.rs",
        "src/server.rs",
        "src/mcp_server.rs",
    ] {
        assert!(
            scan.files.iter().any(|f| f == anchor),
            "anchor file {anchor} is gone - the architecture moved; update the ownership table and the contracts deliberately (one PR)"
        );
    }

    for head in ["server", "mcp_server", "builtins", "interpreter", "parser"] {
        assert!(
            scan.modules.contains_key(head),
            "forbidden-head module `{head}` no longer resolves - rename the heads in the contracts deliberately"
        );
    }

    let toml_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let toml = fs::read_to_string(&toml_path).expect("root Cargo.toml is readable");
    assert!(
        toml.contains("[workspace]"),
        "root Cargo.toml lost its [workspace] section - the three-crate topology is a pinned fact"
    );
    let members = toml
        .lines()
        .find(|l| l.trim().starts_with("members"))
        .expect("workspace members declared");
    assert!(
        members.contains("mlogpkg") && members.contains("mlog-lsp"),
        "workspace members no longer declare the satellite crates: {members}"
    );

    for f in FROZEN_SCCS {
        for file in f.files {
            assert!(
                scan.files.iter().any(|x| x == file),
                "frozen cycle file {file} is gone - update FROZEN_SCCS deliberately (that is good news: the tangle shrank)"
            );
        }
    }
}
