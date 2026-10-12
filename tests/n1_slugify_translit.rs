// N1 (gh#1210, PE-1): slugify — the Russian/Belarusian transliteration.
//
// Before: any non-ASCII byte was DROPPED (the documented decision —
// "transliteration is locale-dependent"), so slugify("Привет мир")
// returned "". The owner's dispatch replaces that decision with a
// DETERMINISTIC per-character table (lowercase output):
//
//   а-a б-b в-v г-g д-d е-e ё-e ж-zh з-z и-i й-y к-k л-l м-m н-n о-o
//   п-p р-r с-s т-t у-u ф-f х-kh ц-ts ч-ch ш-sh щ-shch ъ-(dropped)
//   ы-y ь-(dropped) э-e ю-yu я-ya; Belarusian: і-i ў-u ґ-g;
//   the apostrophe is dropped. Other non-ASCII stays dropped, ASCII
//   unchanged.
//
// The owner's four contract cases live in the owner_cases test below;
// the table edges (multi-char mappings, ъ/ь, apostrophe, mixed ASCII)
// in the rest. The golden FILE contract (slug_cyr:privet-mir in
// p117_string_builtins.expected) rides the standard golden suite —
// all_golden_tests_pass compares the run output with the file, so a
// stale file fails there (no fs access needed here — the fs_gate
// ratchet, naryad #475).

fn slug_of(input: &str) -> String {
    let src = format!(
        "pattern Case() -> String {{\n  return slugify(\"{}\")\n}}\npattern RunAll(_input: String) -> String {{\n  return Case()\n}}\nflow Main {{ input: String = \"x\" -> RunAll -> output }}",
        input
    );
    let out = metalogos::run_program(&src)
        .unwrap_or_else(|e| panic!("slugify({input:?}) failed to run: {e}"))
        .unwrap_or_default();
    out.trim_end().to_string()
}

#[test]
fn n1_owner_cases() {
    for (input, expected) in [
        ("Ремонт офисов", "remont-ofisov"),
        ("Привет мир", "privet-mir"),
        ("Шамякина Мінск", "shamyakina-minsk"),
        ("Щётка", "shchetka"),
    ] {
        assert_eq!(slug_of(input), expected, "slugify({input:?})");
    }
}

#[test]
fn n1_multichar_mappings() {
    // One Cyrillic char -> up to three ASCII chars (ж→zh, х→kh, ц→ts,
    // щ→shch, ю→yu, я→ya), NOT pairwise products.
    for (input, expected) in [
        ("ж", "zh"),
        ("х", "kh"),
        ("ц", "ts"),
        ("щ", "shch"),
        ("ю", "yu"),
        ("я", "ya"),
        ("ъ", ""),
        ("ь", ""),
        ("ы", "y"),
        ("э", "e"),
    ] {
        assert_eq!(slug_of(input), expected, "slugify({input:?})");
    }
}

#[test]
fn n1_belarusian_and_apostrophe() {
    for (input, expected) in [
        ("і", "i"),
        ("ў", "u"),
        ("ґ", "g"),
        ("воўк", "vouk"),   // Belarusian ў inside a word (the table: ў->u)
        ("ґрунт", "grunt"), // ґ leads
        ("п'ять", "pyat"),  // the apostrophe is dropped, no separator
    ] {
        assert_eq!(slug_of(input), expected, "slugify({input:?})");
    }
}

#[test]
fn n1_ascii_unchanged_and_others_dropped() {
    for (input, expected) in [
        ("Hello World 123", "hello-world-123"),
        ("HELLO", "hello"),             // lowercase first
        ("中文", ""),                   // other non-ASCII still dropped
        ("αβ", ""),                     // Greek dropped too
        ("Привет, мир!", "privet-mir"), // punctuation -> separators
    ] {
        assert_eq!(slug_of(input), expected, "slugify({input:?})");
    }
}
