// ── Наряд №195 Contract 1: BPE training determinism ─────────────────
//
// Same corpus + vocab_size → bitwise identical vocab (merges list).

use metalogos::nn::bpe::train_bpe;

#[test]
fn same_corpus_same_vocab_size_produces_identical_merges() {
    let corpus = "ababababababababababababababababababababababababab";
    let vocab1 = train_bpe(corpus, 10).expect("train 1");
    let vocab2 = train_bpe(corpus, 10).expect("train 2");

    assert_eq!(vocab1.merges.len(), vocab2.merges.len());
    for (i, (a, b)) in vocab1.merges.iter().zip(vocab2.merges.iter()).enumerate() {
        assert_eq!(a.0, b.0, "merge {} first part mismatch", i);
        assert_eq!(a.1, b.1, "merge {} second part mismatch", i);
    }
    assert_eq!(vocab1.vocab_size, vocab2.vocab_size);
    println!(
        "✓ determinism: {} merges, vocab_size={}",
        vocab1.merges.len(),
        vocab1.vocab_size
    );
}

#[test]
fn different_corpus_different_vocab() {
    let v1 = train_bpe("aaaabbbb", 10).expect("train 1");
    let v2 = train_bpe("abcdabcd", 10).expect("train 2");
    assert_ne!(
        v1.merges, v2.merges,
        "different corpora should produce different merge orders"
    );
    println!("✓ different corpora → different vocab");
}

#[test]
fn tie_break_is_lexicographic() {
    // "baba" has pairs: (b,a) and (a,b) each appearing once.
    // Tie-break: lexicographic order of "ba" vs "ab" → "ab" wins (smaller).
    // So the first merge should be (a,b), not (b,a).
    let vocab = train_bpe("baba", 10).expect("train");
    assert!(!vocab.merges.is_empty(), "should have at least one merge");
    // The first merge is the pair whose merged string sorts first lexicographically.
    // For "baba": pairs are (b,a)="ba" and (a,b)="ab". "ab" < "ba", so merge (a,b) first.
    let first = &vocab.merges[0];
    let merged = format!("{}{}", first.0, first.1);
    // The merged string should be "ab" (the lexicographically smaller one)
    assert_eq!(
        merged, "ab",
        "tie-break: first merge should be the lexicographically smaller pair, got '{}'",
        merged
    );
    println!(
        "✓ tie-break: first merge is ('a','b') — lexicographic order (merged='{}')",
        merged
    );
}
