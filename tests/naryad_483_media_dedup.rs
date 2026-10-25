// ── Naryad #483 (gh#731): the media/vision duplicate-name transfer ──
//
// The FINAL group of the №466 лекало: the twenty media/vision builtin
// names are spelled in `src/media_ops.rs` and nowhere else outside
// `BUILTIN_REGISTRY`. The №462 counter reads the backends' literals —
// after the transfer the count is ZERO and the baseline threshold is
// the fact (0; a new duplicate name fails CI at once).
//
// Pins (the module-vs-registry drift ratchet):
// 1. every name `media_ops::handles()` owns IS a registered builtin;
// 2. the registry's media/vision surface equals the handles() set
//    EXACTLY — a new media/vision builtin added to the registry must be
//    media_ops membership (the counter gate then demands its literal be
//    kept out of the backends); a removed one must not linger;
// 3. the constants spell exactly the registry names (the backends
//    compare against these — a typo here would silently break dispatch).
use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::media_ops;

fn handles_names() -> Vec<&'static str> {
    // The single spelling lives in media_ops; the membership hook is the
    // module's own surface — walk the registry candidate space through
    // handles() to recover the set.
    // №799 (T2, gh#1488): the candidate space is registry ∪ parked — the
    // parked rows (vision_edit/export/export_raw/generate/lora_*) left
    // the default-build registry, but the NAMES stay in the language
    // (builtins::parked) and the media_ops dispatch tables still own
    // them; a name known only to the parked table must not silently
    // drop out of the handle-set pin.
    let mut owned: Vec<&'static str> = Vec::new();
    for s in BUILTIN_REGISTRY.iter() {
        if media_ops::handles(s.name) {
            owned.push(s.name);
        }
    }
    for p in metalogos::builtins::parked::PARKED_REGISTRY {
        if media_ops::handles(p.name) {
            owned.push(p.name);
        }
    }
    owned
}

#[test]
fn n483_every_handles_name_is_a_registered_builtin() {
    for name in handles_names() {
        // №799 (T2, gh#1488): "registered" becomes "known to the
        // language" — the row compiles into THIS build, OR the name is
        // on the parked surface (builtins::parked; the row gates with
        // its feature). A handle name known to NEITHER is the drift.
        assert!(
            BUILTIN_REGISTRY.iter().any(|s| s.name == name)
                || metalogos::builtins::parked::parked_spec(name).is_some(),
            "media_ops owns `{}`, but it is neither in BUILTIN_REGISTRY nor \
             on the parked surface — the transfer drifted",
            name
        );
    }
}

#[test]
fn n483_handles_set_is_exactly_the_transferred_group() {
    // The transferred set = the 20 names the №462 counter found in BOTH
    // backends — NOT the whole registry media/vision surface (23): the
    // registry also carries `media_manifest_read`, `vision_fetch_weights`
    // and `vision_understand`, which were never TW∩VM duplicates (their
    // dispatch literals never appeared in both backends). The pin fixes
    // the membership: a name added to handles() without being part of
    // the transferred group is drift, a removed one is a hole.
    let mut owned = handles_names();
    owned.dedup();
    owned.sort_unstable();
    let mut expected = vec![
        "media_bind_origin",
        "media_manifest",
        "media_meta",
        "media_release",
        "media_retain",
        "media_save",
        "media_source_capture",
        "media_store_audio",
        "media_store_image",
        "media_store_video_frame",
        "media_store_video_segment",
        "vision_edit",
        "vision_export",
        "vision_export_raw",
        "vision_generate",
        "vision_list",
        "vision_load",
        "vision_lora_generate",
        "vision_lora_load",
        "vision_save",
    ];
    expected.sort_unstable();
    assert_eq!(
        owned, expected,
        "media_ops::handles() drifted from the transferred group"
    );
    assert_eq!(
        owned.len(),
        20,
        "the transferred group is exactly the twenty duplicates"
    );
}

#[test]
fn n483_constants_spell_the_registry_names() {
    // The backends compare `name` against these constants — a typo would
    // silently unhook the dispatch. The spellings are pinned literally.
    assert_eq!(media_ops::MEDIA_SAVE, "media_save");
    assert_eq!(media_ops::MEDIA_STORE_IMAGE, "media_store_image");
    assert_eq!(media_ops::MEDIA_STORE_AUDIO, "media_store_audio");
    assert_eq!(
        media_ops::MEDIA_STORE_VIDEO_FRAME,
        "media_store_video_frame"
    );
    assert_eq!(
        media_ops::MEDIA_STORE_VIDEO_SEGMENT,
        "media_store_video_segment"
    );
    assert_eq!(media_ops::MEDIA_BIND_ORIGIN, "media_bind_origin");
    assert_eq!(media_ops::MEDIA_MANIFEST, "media_manifest");
    assert_eq!(media_ops::MEDIA_META, "media_meta");
    assert_eq!(media_ops::MEDIA_RELEASE, "media_release");
    assert_eq!(media_ops::MEDIA_RETAIN, "media_retain");
    assert_eq!(media_ops::MEDIA_SOURCE_CAPTURE, "media_source_capture");
    assert_eq!(media_ops::VISION_GENERATE, "vision_generate");
    assert_eq!(media_ops::VISION_LIST, "vision_list");
    assert_eq!(media_ops::VISION_EXPORT, "vision_export");
    assert_eq!(media_ops::VISION_EXPORT_RAW, "vision_export_raw");
    assert_eq!(media_ops::VISION_SAVE, "vision_save");
    assert_eq!(media_ops::VISION_LOAD, "vision_load");
    assert_eq!(media_ops::VISION_EDIT, "vision_edit");
    assert_eq!(media_ops::VISION_LORA_GENERATE, "vision_lora_generate");
    assert_eq!(media_ops::VISION_LORA_LOAD, "vision_lora_load");
}
