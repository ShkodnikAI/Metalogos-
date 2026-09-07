// ── Наряд №195 Contract 4: BPE persistence round-trip ──────────────
//
// Serialize a BPE vocab → deserialize → encode/decode gives identical
// results to the original vocab.

use metalogos::nn::bpe::{decode_bpe, deserialize_vocab, encode_bpe, serialize_vocab, train_bpe};

#[test]
fn serialize_deserialize_preserves_vocab() {
    let corpus = "hello world hello world hello world hello world";
    let original = train_bpe(corpus, 50).expect("train_bpe");

    // Serialize
    let bytes = serialize_vocab(&original);
    assert!(!bytes.is_empty(), "serialized data should not be empty");

    // Deserialize
    let restored = deserialize_vocab(&bytes).expect("deserialize_vocab");

    // Vocab sizes match
    assert_eq!(original.vocab_size, restored.vocab_size);
    assert_eq!(original.merges.len(), restored.merges.len());

    // Merges match exactly
    for (i, (a, b)) in original
        .merges
        .iter()
        .zip(restored.merges.iter())
        .enumerate()
    {
        assert_eq!(a.0, b.0, "merge {} first mismatch", i);
        assert_eq!(a.1, b.1, "merge {} second mismatch", i);
    }

    // Vocab entries match
    assert_eq!(original.vocab.len(), restored.vocab.len());
    for (token, &id) in &original.vocab {
        assert_eq!(
            restored.vocab.get(token),
            Some(&id),
            "vocab mismatch for token '{}'",
            token
        );
    }

    println!(
        "✓ serialize/deserialize: {} bytes, {} merges, vocab_size={}",
        bytes.len(),
        restored.merges.len(),
        restored.vocab_size
    );
}

#[test]
fn encode_decode_identical_after_persist() {
    let corpus = "abababab abababab abababab";
    let original = train_bpe(corpus, 20).expect("train_bpe");

    // Encode with original
    let text = "abab";
    let ids_original = encode_bpe(text, &original).expect("encode original");

    // Serialize + deserialize
    let bytes = serialize_vocab(&original);
    let restored = deserialize_vocab(&bytes).expect("deserialize");

    // Encode with restored
    let ids_restored = encode_bpe(text, &restored).expect("encode restored");

    // IDs must be identical
    assert_eq!(
        ids_original, ids_restored,
        "token IDs should match after persist"
    );

    // Decode must give the same text
    let decoded = decode_bpe(&ids_restored, &restored).expect("decode");
    assert_eq!(decoded, text);

    println!(
        "✓ encode/decode identical after persist: ids={:?}",
        ids_restored
    );
}

#[test]
fn unicode_vocab_persists_correctly() {
    let corpus = "Привет мир Привет мир Привет";
    let original = train_bpe(corpus, 100).expect("train_bpe");

    let text = "Привет";
    let ids_original = encode_bpe(text, &original).expect("encode original");

    let bytes = serialize_vocab(&original);
    let restored = deserialize_vocab(&bytes).expect("deserialize");

    let ids_restored = encode_bpe(text, &restored).expect("encode restored");
    assert_eq!(ids_original, ids_restored);

    let decoded = decode_bpe(&ids_restored, &restored).expect("decode");
    assert_eq!(decoded, text);

    println!(
        "✓ Unicode vocab persists: '{}' → {:?} → '{}'",
        text, ids_restored, decoded
    );
}
