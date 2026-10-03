// ── src/module_path.rs ───────────────────────────────────────────────
// №558 (issue #919; the audit 02.10 M-8; dispatch gh#925): the ONE
// module-search rule.
//
// The audit found THREE implementations of the "module_path → file"
// rule: the interpreter's runtime loader (src/interpreter/modules.rs),
// the compiler's `resolve_import` (src/compiler.rs) and the semantic
// static resolution (`resolve_imports_statically`, src/semantic.rs) —
// the third copy lived outside the mirror metrics. They composed the
// same path two ways: `format!("{}.mlog")` (append) vs
// `with_extension("mlog")` — EQUIVALENT on every reachable input,
// because the grammar (`grammar.pest`: `import_path_segments = { IDENT
// ~ (SLASH ~ IDENT)* }`) admits no dots in module paths, so a
// `with_extension` replace can never fire (the honest equivalence note;
// the append form is the SSOT below). The base directory stays a
// site's own choice (the runtime loader and the semantic pass use their
// base_dir; the compiler passes its std_root) — the RULE is shared, the
// roots differ by design.
//
// Leaf module: no crate-internal dependencies (the C4 acyclicity
// ratchet sees no new edge); the file-existence rule stays at each
// site's read (the loud per-site refusals keep their exact pinned
// wording — the rule this SSOT unifies is the path composition only).

use std::path::{Path, PathBuf};

/// The module-search rule: `module_path` resolved against `base_dir`
/// becomes `<base_dir>/<module_path>.mlog` (nested `std/string` paths
/// compose naturally; the optional grammar `./` prefix passes through
/// `Path::join` unchanged). The one implementation all three loaders
/// call (№558): the runtime loader, the compiler's resolve_import, the
/// semantic static resolution.
pub fn resolve_module_file(base_dir: &Path, module_path: &str) -> PathBuf {
    base_dir.join(format!("{}.mlog", module_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_simple_nested_and_dot_paths() {
        assert_eq!(
            resolve_module_file(Path::new("."), "std/string"),
            PathBuf::from("./std/string.mlog")
        );
        assert_eq!(
            resolve_module_file(Path::new("/base"), "my_utils"),
            PathBuf::from("/base/my_utils.mlog")
        );
        // the grammar's optional "./" prefix composes unchanged
        assert_eq!(
            resolve_module_file(Path::new("."), "./my_utils"),
            PathBuf::from(".//my_utils.mlog")
        );
    }
}
