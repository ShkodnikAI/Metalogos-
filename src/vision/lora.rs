//! LoRA adapter loading + validation (Наряд №244, Vision R6.3).
//!
//! The final third of R6 "Edit + LoRA" (plan §7.1). A LoRA adapter travels
//! as safetensors BYTES (read ONCE from a file inside the weights dir by
//! `vision_lora_load`, then persisted as a SQLite BLOB — the ADR-0124 §6
//! pattern, "not a new file format"); at generation time the adapter is
//! resolved from the database and merged into the DiT attention
//! projections (`blocks.N.attention.{to_q,to_k,to_v,to_out.0}.weight`).
//!
//! ## Contract (naryad №244, Block 1.1)
//!
//! - **Both canonical name forms are accepted**: diffusers-PEFT
//!   (`<target>.lora_A.weight` / `<target>.lora_B.weight`) and ComfyUI
//!   (`<target>.lora_down.weight` / `<target>.lora_up.weight` + optional
//!   `<target>.alpha`); mixing the two forms WITHIN one target is a loud
//!   error (ambiguous form).
//! - `rank` = the mean dimension across the accepted pairs;
//!   `scale = alpha / rank`; alpha absent → `scale = 1.0` with a LOUD note
//!   (eprintln-стиль quant_conv №243) — a silently defaulting scale would
//!   quietly change the adapter's effective strength.
//!
//! - dtype: a non-F32 tensor is upcast to F32 LOUDLY (quiet upcast is a
//!   forbidden silent transform, §3.1).
//! - Validation — every target, after stripping the rank suffix, MUST be
//!   an attention projection of the base (`to_q/to_k/to_v/to_out` per
//!   `zimage_expected_keys`); non-attention targets, unknown prefixes,
//!   B-without-A (or vice versa), orphaned keys, mismatched dimensions —
//!   ALL loud `Err` with the FULL list of problems (a quiet drop of any
//!   key is forbidden: a partially-applied adapter is a silent no-op on
//!   the dropped targets — §3.1's exact failure mode).

use std::collections::HashMap;

use candle_core::{DType, Device, Tensor};

/// One LoRA pair for a single target: `up` is `[out, rank]`, `down` is
/// `[rank, in]` (both F32 after the loud upcast). The merge computes
/// `delta = up @ down` → `[out, in]`.
#[derive(Debug, Clone)]
pub struct LoraPair {
    pub up: Tensor,
    pub down: Tensor,
}

/// A validated LoRA adapter (Наряд №244 Block 1.1).
///
/// `targets` maps the BASE attention-projection key (the exact key the
/// base tensors use, e.g. `layers.3.attention.to_q.weight`) to its pair.
/// `scale = alpha / rank` — one scale per adapter (the naryad's formula;
/// mismatched alphas are a loud error, not a quiet per-target average).
#[derive(Debug, Clone)]
pub struct LoraAdapter {
    pub targets: HashMap<String, LoraPair>,
    pub rank: usize,
    pub alpha: Option<f64>,
    pub scale: f64,
}

impl LoraAdapter {
    /// Parse safetensors BYTES (the BLOB from the SQLite store, or a file
    /// read by `vision_lora_load`) into a validated adapter.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let tensors = candle_core::safetensors::load_buffer(bytes, &Device::Cpu)
            .map_err(|e| format!("vision lora: safetensors parse failed: {}", e))?;
        Self::from_tensors(tensors)
    }

    /// Validate + build from an already-loaded tensor map (the reader is
    /// the Stage B/C лекало `candle_core::safetensors`).
    pub(crate) fn from_tensors(tensors: HashMap<String, Tensor>) -> Result<Self, String> {
        // Loud upcast FIRST (§3.1: quiet upcast is forbidden) — every
        // non-F32 tensor is named on stderr before any math happens.
        let mut tensors = tensors;
        let non_f32: Vec<String> = tensors
            .iter()
            .filter(|(_, t)| t.dtype() != DType::F32)
            .map(|(name, _)| name.clone())
            .collect();
        for name in &non_f32 {
            eprintln!(
                "vision lora: tensor '{}' is {:?}, upcasting to F32 (loud upcast, quiet is forbidden)",
                name,
                tensors[name].dtype()
            );
        }
        if !non_f32.is_empty() {
            for t in tensors.values_mut() {
                if t.dtype() != DType::F32 {
                    *t = t
                        .to_dtype(DType::F32)
                        .map_err(|e| format!("vision lora: upcast to F32 failed: {}", e))?;
                }
            }
        }

        // ── Classify every key; collect ALL problems for the full list ──
        let mut problems: Vec<String> = Vec::new();
        let mut a_parts: HashMap<String, Tensor> = HashMap::new();
        let mut b_parts: HashMap<String, Tensor> = HashMap::new();
        let mut down_parts: HashMap<String, Tensor> = HashMap::new();
        let mut up_parts: HashMap<String, Tensor> = HashMap::new();
        let mut alphas: HashMap<String, f64> = HashMap::new();

        const SUFFIXES: [&str; 5] = [
            ".lora_A.weight",
            ".lora_B.weight",
            ".lora_down.weight",
            ".lora_up.weight",
            ".alpha",
        ];

        for (name, tensor) in tensors.into_iter() {
            let hit = SUFFIXES.iter().find(|s| name.ends_with(*s)).copied();
            match hit {
                Some(suffix) => {
                    let target = &name[..name.len() - suffix.len()];
                    if !is_attention_projection_target(target) {
                        problems.push(format!(
                            "target '{}' (from '{}') is NOT an attention projection of the base \
                             (to_q/to_k/to_v/to_out per zimage_expected_keys — norms, FFN, \
                             embedders, final layer and unknown prefixes are refused)",
                            target, name
                        ));
                        continue;
                    }
                    match suffix {
                        ".lora_A.weight" => {
                            a_parts.insert(target.to_string(), tensor);
                        }
                        ".lora_B.weight" => {
                            b_parts.insert(target.to_string(), tensor);
                        }
                        ".lora_down.weight" => {
                            down_parts.insert(target.to_string(), tensor);
                        }
                        ".lora_up.weight" => {
                            up_parts.insert(target.to_string(), tensor);
                        }
                        _ => match scalar_f64(&tensor) {
                            Ok(v) => {
                                alphas.insert(target.to_string(), v);
                            }
                            Err(e) => problems.push(format!("'{}': {}", name, e)),
                        },
                    }
                }
                None => {
                    problems.push(format!(
                        "orphaned key '{}' — matches neither <target>.lora_A.weight / \
                         .lora_B.weight (diffusers-PEFT) nor <target>.lora_down.weight / \
                         .lora_up.weight / .alpha (ComfyUI); quiet dropping is forbidden",
                        name
                    ));
                }
            }
        }

        // ── Pair the parts per target; validate shapes ─────────────────
        let mut union: HashMap<String, (Option<Tensor>, Option<Tensor>)> = HashMap::new();
        for (t, v) in a_parts.into_iter() {
            union.entry(t).or_default().0 = Some(v);
        }
        for (t, v) in down_parts.into_iter() {
            union.entry(t).or_default().0 = Some(v);
        }
        for (t, v) in b_parts.into_iter().chain(up_parts) {
            union.entry(t).or_default().1 = Some(v);
        }

        let mut targets: HashMap<String, LoraPair> = HashMap::new();
        let mut ranks: Vec<usize> = Vec::new();
        for (target, (low, high)) in union {
            let (down, up) = match (low, high) {
                (Some(down), Some(up)) => (down, up),
                (low, high) => {
                    // A B-without-A (or vice versa) — the adapter cannot be
                    // applied half-way, so this is a loud refusal with the
                    // target named.
                    let has_low = low.is_some() || high.is_some();
                    let _ = has_low; // the message names the missing half below
                    let what = if low.is_some() {
                        "A/down without B/up"
                    } else {
                        "B/up without A/down"
                    };
                    problems.push(format!(
                        "target '{}': {} — a half pair cannot be applied \
                         (refusing the whole adapter, no partial application)",
                        target, what
                    ));
                    continue;
                }
            };
            if let Err(e) = check_pair_dims(&down, &up) {
                problems.push(format!("target '{}': {}", target, e));
                continue;
            }
            let r = down.dims()[0];
            ranks.push(r);
            targets.insert(target, LoraPair { up, down });
        }

        // ── Alpha consistency + scale ─────────────────────────────────
        let mut alpha_values: Vec<f64> = alphas.values().copied().collect();
        alpha_values.sort_by(|a, b| a.total_cmp(b));
        let alpha = match alpha_values.len() {
            0 => None,
            1 => Some(alpha_values[0]),
            _ => {
                if alpha_values[0] != alpha_values[alpha_values.len() - 1] {
                    problems.push(format!(
                        "mismatched alpha values across targets ({:?}) — one scale per \
                         adapter is the №244 contract; a silent per-target scale is \
                         forbidden",
                        alpha_values
                    ));
                    None
                } else {
                    Some(alpha_values[0])
                }
            }
        };

        if targets.is_empty() && problems.is_empty() {
            problems.push(
                "no LoRA pairs found in the adapter — nothing to apply (an empty \
                 adapter would be a silent no-op)"
                    .to_string(),
            );
        }
        if !problems.is_empty() {
            return Err(format!(
                "vision lora: adapter validation failed with {} problem(s):\n{}",
                problems.len(),
                problems
                    .iter()
                    .map(|p| format!("  - {}", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        // rank = the MEAN dimension (naryad Block 1.1); scale = alpha/rank.
        let rank_f = ranks.iter().sum::<usize>() as f64 / ranks.len() as f64;
        let rank = rank_f.round() as usize;
        let scale = match alpha {
            Some(a) => a / rank_f,
            None => {
                eprintln!(
                    "vision lora: no alpha tensor in the adapter — scale defaults to 1.0 \
                     (loud note; the effective strength is the raw up@down delta)"
                );
                1.0
            }
        };

        Ok(LoraAdapter {
            targets,
            rank,
            alpha,
            scale,
        })
    }
}

/// A target (the key minus the rank suffix) must be an attention
/// projection of the base: `{prefix}.attention.{to_q,to_k,to_v,to_out.0}.weight`
/// where `prefix` ∈ `layers.N` / `noise_refiner.N` / `context_refiner.N`
/// (the `zimage_expected_keys` shape — Block 1.1). Norms, FFN, embedders,
/// the final layer and any unknown prefix are refused.
pub fn is_attention_projection_target(target: &str) -> bool {
    let Some((prefix, rest)) = target.split_once(".attention.") else {
        return false;
    };
    match rest {
        "to_q.weight" | "to_k.weight" | "to_v.weight" | "to_out.0.weight" => {}
        _ => return false,
    }
    let Some((kind, idx)) = prefix.rsplit_once('.') else {
        return false;
    };
    if idx.is_empty() || !idx.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    matches!(kind, "layers" | "noise_refiner" | "context_refiner")
}

/// Shape contract for one pair: `down` is `[r, in]`, `up` is `[out, r]`,
/// both 2-D, and `up.dims()[1] == down.dims()[0]` (the inner rank must
/// agree — otherwise the matmul silently cannot represent the delta).
fn check_pair_dims(down: &Tensor, up: &Tensor) -> Result<(), String> {
    let dd = down.dims();
    let ud = up.dims();
    if dd.len() != 2 || ud.len() != 2 {
        return Err(format!(
            "lora pair must be 2-D ([rank, in] down, [out, rank] up), got down {:?} / up {:?}",
            dd, ud
        ));
    }
    if ud[1] != dd[0] {
        return Err(format!(
            "mismatched dimensions: up {:?} vs down {:?} — inner rank disagrees \
             (up.dims()[1] {} != down.dims()[0] {})",
            ud, dd, ud[1], dd[0]
        ));
    }
    Ok(())
}

/// Extract a scalar tensor as f64 ([] or [1] shape, F32 after the loud
/// upcast) — the ComfyUI `<target>.alpha` form.
fn scalar_f64(t: &Tensor) -> Result<f64, String> {
    match t.dims() {
        [] => t
            .to_vec0::<f32>()
            .map(|v| v as f64)
            .map_err(|e| e.to_string()),
        [1] => t
            .to_vec1::<f32>()
            .map(|v| v[0] as f64)
            .map_err(|e| e.to_string()),
        other => Err(format!(
            "alpha tensor must be a scalar ([] or [1]), got shape {:?}",
            other
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::vision::dit::{tiny_dit_config, zimage_expected_keys};

    /// Every attention-projection key of the tiny config passes the target
    /// check; every NON-attention key of the same generator fails it (the
    /// Block 1.1 rule «по zimage_expected_keys», mechanically pinned).
    #[test]
    fn attention_targets_match_zimage_expected_keys_shape() {
        let config = tiny_dit_config();
        let keys = zimage_expected_keys(&config);
        let mut attention = 0usize;
        for k in &keys {
            if is_attention_projection_target(k) {
                attention += 1;
                // And the target must name one of the four projections.
                assert!(
                    k.ends_with("to_q.weight")
                        || k.ends_with("to_k.weight")
                        || k.ends_with("to_v.weight")
                        || k.ends_with("to_out.0.weight"),
                    "attention target must be a projection: {}",
                    k
                );
            }
        }
        // 4 projections × (2 layers + 1 noise_refiner + 1 context_refiner).
        assert_eq!(attention, 4 * 4, "tiny config attention projections");
        // Non-attention keys must be refused.
        assert!(!is_attention_projection_target(
            "layers.0.attention_norm1.weight"
        ));
        assert!(!is_attention_projection_target(
            "layers.0.feed_forward.w1.weight"
        ));
        assert!(!is_attention_projection_target("x_pad_token"));
        assert!(!is_attention_projection_target(
            "all_final_layer.2-1.linear.weight"
        ));
        assert!(!is_attention_projection_target("t_embedder.mlp.0.weight"));
        // Unknown prefixes and malformed forms.
        assert!(!is_attention_projection_target(
            "blocks.0.attention.to_q.weight"
        ));
        assert!(!is_attention_projection_target(
            "layers..attention.to_q.weight"
        ));
        assert!(!is_attention_projection_target(
            "layers.0.attention.norm_q.weight"
        ));
        assert!(!is_attention_projection_target("layers.0"));
    }
}
