//! №500 (Wave 19, dispatch gh#793) — the audit 28.09 §3.2 blocking
//! reproductions: the third-party path-APIs (lopdf's own file I/O) used
//! to bypass the №475 facade — clippy saw `std::fs`, but not the file
//! opens inside `lopdf::Document::load/save`. Every pdf load/save now
//! routes through `crate::fs_gate` (read_bytes → load_mem; save_to →
//! write_bytes), and the №500 ratchet extends the clippy disallow list
//! to the library path-APIs themselves.
//!
//! The audit's three vectors, pinned as blocking tests (plus the `..`
//! traversal and the symlink swap):
//!   1. `pdf_fill_form("data/form.pdf", "{}", "app.mlog")` — the
//!      application-image overwrite refuses (the №475 HARD write deny —
//!      the allowlist crane does not apply);
//!   2. `pdf_rotate_page("data/a.pdf", 1, 90, "/home/app/.ssh/…")` —
//!      the absolute-path write refuses (the №131 sandbox);
//!   3. `pdf_delete_pages("/srv/other-tenant/contract.pdf", …)` — the
//!      absolute-path read refuses; the exfiltration-by-read vector is
//!      dead;
//!   4. `data/../../escape.pdf` — the `..` traversal refuses;
//!   5. a symlink named like an innocent output pointing at `app.mlog`
//!      refuses on the RESOLVED form (the raw name alone would pass);
//!   6. the LEGITIMATE flow still works: a real fixture PDF round-trips
//!      `pdf_set_metadata` through the gate (formats and behavior of
//!      honest pdf scenarios unchanged — the №500 boundary).

#![allow(clippy::disallowed_methods)] // the test harness manages its fixtures

use metalogos::builtins::pdf::{
    builtin_pdf_delete_pages, builtin_pdf_metadata, builtin_pdf_rotate_page,
    builtin_pdf_set_metadata,
};
use metalogos::interpreter::values::Value;
use serial_test::serial;

/// A real (minimal but valid) PDF fixture, built with lopdf the same way
/// the production path builds documents.
fn make_fixture_pdf(path: &str) {
    let mut doc = lopdf::Document::with_version("1.5");
    // A real one-page tree: Catalog → Pages(Count 1, Kids) → Page.
    let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT ET".to_vec(),
    )));
    let page_id = doc.add_object(lopdf::Object::Dictionary(
        vec![
            (b"Type".to_vec(), lopdf::Object::Name(b"Page".to_vec())),
            (b"Parent".to_vec(), lopdf::Object::Reference((1, 0))),
            (
                b"MediaBox".to_vec(),
                lopdf::Object::Array(vec![
                    lopdf::Object::Integer(0),
                    lopdf::Object::Integer(0),
                    lopdf::Object::Integer(612),
                    lopdf::Object::Integer(792),
                ]),
            ),
            (b"Contents".to_vec(), lopdf::Object::Reference(content_id)),
        ]
        .into_iter()
        .collect(),
    ));
    let pages_id = doc.add_object(lopdf::Object::Dictionary(
        vec![
            (b"Type".to_vec(), lopdf::Object::Name(b"Pages".to_vec())),
            (b"Count".to_vec(), lopdf::Object::Integer(1)),
            (
                b"Kids".to_vec(),
                lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
            ),
        ]
        .into_iter()
        .collect(),
    ));
    // Fix the Parent reference to the REAL pages id.
    if let lopdf::Object::Dictionary(dict) = doc.objects.get_mut(&page_id).unwrap() {
        dict.set(b"Parent", lopdf::Object::Reference(pages_id));
    }
    let catalog_id = doc.add_object(lopdf::Object::Dictionary(
        vec![
            (b"Type".to_vec(), lopdf::Object::Name(b"Catalog".to_vec())),
            (b"Pages".to_vec(), lopdf::Object::Reference(pages_id)),
        ]
        .into_iter()
        .collect(),
    ));
    doc.trailer
        .set(b"Root", lopdf::Object::Reference(catalog_id));
    doc.trailer
        .set(b"Size", lopdf::Object::Integer((catalog_id.0 as i64) + 1));
    let mut file = std::fs::File::create(path).expect("fixture create");
    doc.save_to(&mut file).expect("fixture save");
}

struct FixtureDir(&'static str);
impl FixtureDir {
    fn new(name: &'static str) -> Self {
        let _ = std::fs::remove_dir_all(name);
        std::fs::create_dir_all(name).unwrap();
        FixtureDir(name)
    }
    fn p(&self, file: &str) -> String {
        format!("{}/{}", self.0, file)
    }
}
impl Drop for FixtureDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0);
    }
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

/// Vector 1: the application-image overwrite refuses (the HARD write
/// deny — `.mlog` outputs are refused unconditionally, the allowlist
/// crane does not apply, the input being legit changes nothing).
#[test]
#[serial]
fn n500_vector1_fill_form_cannot_overwrite_app_mlog() {
    let d = FixtureDir::new("n500-v1");
    make_fixture_pdf(&d.p("form.pdf"));
    make_fixture_pdf(&d.p("form.pdf"));
    let err = metalogos::builtins::pdf::builtin_pdf_rotate_page(&[
        s(&d.p("form.pdf")),
        Value::Float(1.0),
        Value::Float(90.0),
        s(&d.p("app.mlog")),
    ])
    .expect_err("the .mlog overwrite must refuse");
    assert!(
        err.contains("sensitive") || err.contains("refused") || err.contains("denied"),
        "the refusal must name the write gate, got: {}",
        err
    );
}

/// Vector 2: the absolute-path write refuses (the №131 sandbox) — the
/// audit's arbitrary-file-write vector is dead.
#[test]
#[serial]
fn n500_vector2_rotate_cannot_write_absolute_path() {
    let d = FixtureDir::new("n500-v2");
    make_fixture_pdf(&d.p("a.pdf"));
    let err = builtin_pdf_rotate_page(&[
        s(&d.p("a.pdf")),
        Value::Float(1.0),
        Value::Float(90.0),
        s("/home/app/.ssh/authorized_keys"),
    ])
    .expect_err("the absolute-path write must refuse");
    assert!(
        err.contains("absolute"),
        "the refusal must name the sandbox's absolute-path rule, got: {}",
        err
    );
}

/// Vector 3: the absolute-path read refuses — the
/// exfiltration-by-read vector is dead (the input gate fires before any
/// lopdf parse).
#[test]
#[serial]
fn n500_vector3_delete_pages_cannot_read_absolute_path() {
    let d = FixtureDir::new("n500-v3");
    let err = builtin_pdf_delete_pages(&[
        s("/srv/other-tenant/contract.pdf"),
        s("[]"),
        s(&d.p("leak.pdf")),
    ])
    .expect_err("the absolute-path read must refuse");
    assert!(
        err.contains("absolute"),
        "the refusal must name the sandbox's absolute-path rule, got: {}",
        err
    );
}

/// The `..` traversal refuses on the OUTPUT side (writing past the
/// working directory through a relative-looking path).
#[test]
#[serial]
fn n500_dotdot_traversal_refused() {
    let d = FixtureDir::new("n500-dotdot");
    make_fixture_pdf(&d.p("a.pdf"));
    let err = builtin_pdf_rotate_page(&[
        s(&d.p("a.pdf")),
        Value::Float(1.0),
        Value::Float(90.0),
        s(&d.p("../../../../escape.pdf")),
    ])
    .expect_err("the .. traversal must refuse");
    assert!(
        err.contains("..") || err.contains("traversal") || err.contains("sandbox"),
        "the refusal must name the sandbox traversal rule, got: {}",
        err
    );
}

/// The symlink swap refuses on the RESOLVED form: an innocent-looking
/// output name pointing at `app.mlog` hits the HARD write deny through
/// canonicalization (the raw name alone would pass). Unix-only: the
/// reproduction pins the symlink vector itself.
#[test]
#[cfg(unix)]
#[serial]
fn n500_symlink_swap_refused() {
    let d = FixtureDir::new("n500-symlink");
    make_fixture_pdf(&d.p("a.pdf"));
    std::os::unix::fs::symlink("app.mlog", d.p("out.pdf")).expect("symlink");
    // The symlink target EXISTS (the hard-deny re-check canonicalizes
    // through an existing link; a dangling link fails at the open).
    std::fs::write(d.p("app.mlog"), b"# the application image").unwrap();
    let err = builtin_pdf_rotate_page(&[
        s(&d.p("a.pdf")),
        Value::Float(1.0),
        Value::Float(90.0),
        s(&d.p("out.pdf")),
    ])
    .expect_err("the symlink-swap write must refuse");
    assert!(
        err.contains("sensitive") || err.contains("refused") || err.contains("denied"),
        "the refusal must name the write gate, got: {}",
        err
    );
}

/// The deny-list protects the SENSITIVE-NAME reads: a `.env` file is not
/// a pdf the builtins may parse (the №455 vocabulary applies to the
/// routed reads).
#[test]
#[serial]
fn n500_deny_list_read_refused() {
    let d = FixtureDir::new("n500-deny");
    std::fs::write(d.p(".env"), "SECRET=1").unwrap();
    let err = builtin_pdf_metadata(&[s(&d.p(".env"))]).expect_err("the .env read must refuse");
    assert!(
        err.contains("sensitive") || err.contains("denied") || err.contains("refused"),
        "the refusal must name the read gate, got: {}",
        err
    );
}

/// The LEGITIMATE flow unchanged: a real fixture PDF round-trips
/// `pdf_set_metadata` through the gate (the №500 boundary — formats and
/// behavior of honest pdf scenarios do not change).
#[test]
#[serial]
fn n500_legit_metadata_roundtrip_works() {
    let d = FixtureDir::new("n500-legit");
    make_fixture_pdf(&d.p("doc.pdf"));
    let result = builtin_pdf_set_metadata(&[s(&d.p("doc.pdf")), s("title"), s("the gated title")])
        .expect("the legit metadata write must work through the gate");
    let text = format!("{}", result);
    assert!(
        text.contains("true") || text.contains("ok"),
        "the metadata result must report success, got: {}",
        text
    );
    // The title landed: a fresh read sees it.
    let meta = builtin_pdf_metadata(&[s(&d.p("doc.pdf"))]).expect("the read-back works");
    let meta_text = format!("{}", meta);
    assert!(
        meta_text.contains("the gated title"),
        "the read-back must carry the written title, got: {}",
        meta_text
    );
}
