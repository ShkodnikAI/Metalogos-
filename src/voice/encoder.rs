// ── Speaker encoder contract (Наряд №303, issue #371) ────────────────
//
// ECAPA-class speaker encoder contract — built on existing nn-blocks
// (src/nn/: attention, dense, rmsnorm, activation, layer, persist).
// The actual encoder implementation requires candle-core conv1d layers
// which are not yet available in the nn-blocks API. This file documents
// the embedding contract and provides a stub encoder for testing.
//
// Real ECAPA implementation: phase A4 (after conv1d support, separate ADR).
// Contract is fixed HERE so that store/ledger/encryption can be tested
// against a known embedding shape.

/// Embedding dimension for ECAPA-class speaker encoder.
/// 192 is the standard ECAPA-TDNN output dimension (industry standard).
pub const EMBEDDING_DIM: usize = 192;

/// Speaker encoder contract:
/// - Input: raw audio waveform (16kHz, mono, float32)
/// - Output: 192-dim embedding, L2-normalized
/// - Serialization: little-endian f32 × 192 = 768 bytes
/// - At rest: AES-256-GCM encrypted (Наряд №172, secret() stack)
/// - In Value: never (opaque VoiceId handle, ADR-0144)
pub struct SpeakerEncoder;

impl SpeakerEncoder {
    /// Stub encoder — produces a deterministic embedding from audio.
    /// Real implementation: ECAPA-TDNN on candle-core (phase A4).
    /// This stub is for testing store/ledger/encryption mechanics.
    pub fn encode_stub(audio: &[f32]) -> Vec<f32> {
        // Deterministic hash-based embedding (not real ECAPA)
        let mut embedding = vec![0.0f32; EMBEDDING_DIM];
        for (i, &sample) in audio.iter().enumerate() {
            embedding[i % EMBEDDING_DIM] += sample;
        }
        // L2 normalize
        let norm: f32 = embedding.iter().map(|f| f * f).sum::<f32>().sqrt();
        if norm > 0.0 {
            for f in &mut embedding {
                *f /= norm;
            }
        }
        embedding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_dimension() {
        let audio = vec![0.1; 16000];
        let embedding = SpeakerEncoder::encode_stub(&audio);
        assert_eq!(embedding.len(), EMBEDDING_DIM);
    }

    #[test]
    fn l2_normalized() {
        let audio = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let embedding = SpeakerEncoder::encode_stub(&audio);
        let norm: f32 = embedding.iter().map(|f| f * f).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "embedding must be L2-normalized, got norm={}",
            norm
        );
    }

    #[test]
    fn deterministic() {
        let audio = vec![0.5; 100];
        let e1 = SpeakerEncoder::encode_stub(&audio);
        let e2 = SpeakerEncoder::encode_stub(&audio);
        assert_eq!(e1, e2);
    }

    #[test]
    fn different_audio_different_embedding() {
        let e1 = SpeakerEncoder::encode_stub(&[1.0, 2.0, 3.0]);
        let e2 = SpeakerEncoder::encode_stub(&[3.0, 2.0, 1.0]);
        assert_ne!(e1, e2);
    }
}
