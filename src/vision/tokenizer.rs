//! Tokenizer wrapper for Vision R3 (naryad №212).
//!
//! Thin wrapper around the HF `tokenizers` crate (canonical BPE implementation).
//!
//! ## Why not hand-roll?
//!
//! Qwen2Tokenizer uses byte-level BPE with a 151,643-token base vocab +
//! 119 special tokens. The pre-tokenizer is a GPT-2-style regex split;
//! getting the regex wrong silently changes tokenization for non-ASCII
//! inputs. The merges file is rank-ordered; reading it with the wrong
//! order silently produces different IDs.
//!
//! `tokenizers` is HF's verified reference implementation and is
//! deterministic (no RNG). Adding it as an optional dep gated behind
//! `vision` is the lowest-risk path (ADR-0124 update).
//!
//! ## NO network access
//!
//! Loads `tokenizer.json` from the local filesystem. No network.
//! Auto-download is R5 (ADR-0125).

use std::path::Path;

/// Thin wrapper around `tokenizers::Tokenizer`.
#[derive(Debug)]
pub struct Tokenizer {
    inner: tokenizers::Tokenizer,
}

impl Tokenizer {
    /// Load a tokenizer from `{tokenizer_dir}/tokenizer.json`.
    ///
    /// `tokenizer.json` is the canonical HF fast-tokenizer format (~11 MB for
    /// Qwen2). The `tokenizers` crate reads it directly.
    pub fn from_dir(tokenizer_dir: &Path) -> Result<Self, String> {
        let tokenizer_path = tokenizer_dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(format!(
                "Tokenizer::from_dir: tokenizer.json not found at {} \
                 (verify MLOG_VISION_WEIGHTS_DIR layout per \
                 docs/research/naryad-212-weights-manifest.md)",
                tokenizer_path.display()
            ));
        }
        let inner = tokenizers::Tokenizer::from_file(&tokenizer_path).map_err(|e| {
            format!(
                "Tokenizer::from_dir: failed to load {}: {}",
                tokenizer_path.display(),
                e
            )
        })?;
        Ok(Self { inner })
    }

    /// Encode a text prompt to token IDs.
    ///
    /// Per diffusers `ZImagePipeline.encode_prompt`: NO chat template is
    /// applied for Z-Image-Turbo T2I — the raw text is encoded as-is.
    /// No BOS/EOS injection (Qwen3 model handles token IDs as given; the
    /// pipeline does not add special tokens for T2I prompts — verified by
    /// inspecting diffusers `pipelines/z_image/pipeline_z_image.py`).
    pub fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        let encoding = self
            .inner
            .encode(text, false)
            .map_err(|e| format!("Tokenizer::encode: failed to encode: {}", e))?;
        Ok(encoding.get_ids().to_vec())
    }

    /// Number of tokens in the encoding (convenience).
    pub fn count(&self, text: &str) -> Result<usize, String> {
        Ok(self.encode(text)?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without a real tokenizer.json, we can only test the missing-file path.
    /// This is the CI-safe test — no real weights required.
    #[test]
    fn from_dir_missing_file_errors() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let result = Tokenizer::from_dir(tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("tokenizer.json not found"),
            "err should mention missing file: {}",
            err
        );
    }
}
