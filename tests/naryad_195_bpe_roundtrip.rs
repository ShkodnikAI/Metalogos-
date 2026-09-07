// ── Наряд №195 Contract 2: BPE round-trip (Unicode) ────────────────
//
// `decode(encode(text, vocab), vocab) == text` for ASCII, Cyrillic,
// emoji, mixed — same strings as Наряд №194 roundtrip test.

use metalogos::nn::bpe::{decode_bpe, encode_bpe, train_bpe};

fn roundtrip(text: &str, vocab_size: usize) {
    let vocab = train_bpe(text, vocab_size).expect("train_bpe");
    let ids = encode_bpe(text, &vocab).expect("encode_bpe");
    let back = decode_bpe(&ids, &vocab).expect("decode_bpe");
    assert_eq!(back, text, "roundtrip failed for: {}", text);
}

#[test]
fn roundtrip_ascii() {
    let text = "hello world hello world hello world";
    roundtrip(text, 50);
    println!("✓ ASCII roundtrip");
}

#[test]
fn roundtrip_cyrillic() {
    let text = "Привет мир Привет мир Привет мир";
    roundtrip(text, 100);
    println!("✓ Cyrillic roundtrip");
}

#[test]
fn roundtrip_emoji() {
    let text = "Hello 🌍🚀❤️ Hello 🌍🚀❤️ Hello 🌍🚀❤️";
    roundtrip(text, 100);
    println!("✓ Emoji roundtrip");
}

#[test]
fn roundtrip_mixed() {
    let text = "Hello мир 🌍 — Привет! Hello мир 🌍 — Привет!";
    roundtrip(text, 100);
    println!("✓ Mixed roundtrip");
}

#[test]
fn roundtrip_empty() {
    let text = "";
    let vocab = train_bpe("ab", 10).expect("train");
    let ids = encode_bpe(text, &vocab).expect("encode");
    let back = decode_bpe(&ids, &vocab).expect("decode");
    assert_eq!(back, text);
    println!("✓ Empty string roundtrip");
}
