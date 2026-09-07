// ── Наряд №195 Contract 3: BPE efficiency ──────────────────────────
//
// BPE produces fewer tokens than character-level tokenization
// on the same text. Concrete numbers, not abstract claims.

use metalogos::nn::bpe::{encode_bpe, train_bpe};

#[test]
fn bpe_fewer_tokens_than_char_level() {
    // A text with repeated patterns — BPE should merge common pairs.
    let text = "abababababababababababababababababababababababab";

    // Character-level count (Наряд №194 scheme)
    let char_count = text.chars().count();

    // BPE with vocab_size=10 (small — will merge "ab" into one token)
    let vocab = train_bpe(text, 10).expect("train_bpe");
    let bpe_ids = encode_bpe(text, &vocab).expect("encode_bpe");
    let bpe_count = bpe_ids.len();

    println!(
        "text: '{}'\nchar tokens: {}, bpe tokens: {}, bpe vocab_size: {}",
        text, char_count, bpe_count, vocab.vocab_size
    );

    assert!(
        bpe_count < char_count,
        "BPE should produce fewer tokens than char-level: bpe={}, char={}",
        bpe_count,
        char_count
    );

    // Specific expectation: "ab" merged → each "ab" pair = 1 token
    // Text "ababab...ab" (24 pairs) → after merge, 24 tokens (not 48)
    // Even better if "abab" gets merged at higher vocab_size.
    assert!(
        bpe_count <= char_count / 2,
        "BPE should at least halve token count for repeating 'ab': bpe={}, char={}",
        bpe_count,
        char_count
    );

    println!(
        "✓ BPE efficiency: {} tokens vs {} char tokens ({}% reduction)",
        bpe_count,
        char_count,
        (1.0 - bpe_count as f64 / char_count as f64) * 100.0
    );
}

#[test]
fn bpe_efficiency_on_longer_text() {
    // Longer text with more variety — BPE should still help.
    let text =
        "the quick brown fox jumps over the lazy dog the quick brown fox jumps over the lazy dog";

    let char_count = text.chars().count();
    let vocab = train_bpe(text, 100).expect("train_bpe");
    let bpe_ids = encode_bpe(text, &vocab).expect("encode_bpe");
    let bpe_count = bpe_ids.len();

    println!(
        "text length: {} chars, bpe tokens: {}, vocab: {} merges",
        char_count,
        bpe_count,
        vocab.merges.len()
    );

    assert!(
        bpe_count < char_count,
        "BPE should produce fewer tokens: bpe={}, char={}",
        bpe_count,
        char_count
    );

    println!(
        "✓ BPE efficiency on longer text: {} vs {} ({:.0}% reduction)",
        bpe_count,
        char_count,
        (1.0 - bpe_count as f64 / char_count as f64) * 100.0
    );
}
