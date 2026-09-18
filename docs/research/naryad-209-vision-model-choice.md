# Naryad #209 — Reconnaissance: Vision pillar wedge choice (Z-Image-Turbo vs FLUX.2-klein)

> **Status:** Research report — input for ADR-0122/0123, not an architectural decision by itself.
> **Date:** 2026-09-07
> **Priority:** Research-only, not a single line of implementation in Metalogos.
> **Parent document:** Metalogos_Vision_Pillar_Plan.md (the Vision pillar plan,
> approved by the owner on 2026-09-07 with the directive "start implementation").

---

## Block 1 — Verified fact base (web search, 2026-09-07)

Facts confirmed by search at the time of writing; everything not included in this block
is marked in Block 3 as "fact-check before R2/R3":

| Fact | Source class |
|---|---|
| Z-Image-Turbo: 6B parameters, Apache-2.0, 8 NFE, VRAM profiles 16/8/6 GB (BF16/FP8/GGUF) | HF model card, repo |
| lightx2v/Z-Image-Turbo-Quantized — quantized weights exist publicly | HF |
| FLUX.2 [klein] 4B — Apache-2.0, Black Forest Labs, official HF publication | HF/BFL |
| Wan2.2-TI2V-5B — Apache-2.0, consumer GPU, candidate for the video phase | HF |
| candle has native precedents of the full "text encoder → denoiser → VAE" cycle: SD 1.5/2.1, SDXL, Turbo, Wuerstchen | candle-example-* in the candle repo |

Conclusion from the fact base: **the class of tasks "modern-DiT/flow model in Rust" has been walked before** (candle
SD precedents); the R2/R3 task is to repeat the cycle for the new DiT+flow generation, not to invent
inference from scratch.

## Block 2 — Wedge comparison by selection criteria

Criteria (order = weight): 1) weight license (Apache-first — the F5-TTS/Emilia lesson from
the voice twin of this plan); 2) quality/compute on consumer GPUs; 3) reuse of
already implemented nn blocks (`src/nn/`: attention/GQA/RmsNorm/SwiGLU/transformer_block,
KV-cache of naryad #193); 4) a reproducible quantization path; 5) edit capabilities.

| Criterion | **Z-Image-Turbo** ✅ | FLUX.2 [klein] | Qwen-Image (family) | SD3.5 Large |
|---|---|---|---|---|
| License | Apache-2.0 ✅ (verif.) | Apache-2.0 ✅ (verif.) | Apache-2.0 | Community (not Apache) |
| Size / VRAM | 6B; 16/8/6 GB ✅ (verif.) | 4B | 20B (2.0 ~7B) | 8B |
| NFE | **8** (distillation — the norm, not an optimization) | — | — | 28+ |
| Text encoder | LLM-class (our blocks) — composition is a fact-check item | its own, per-model | Qwen-VL-class | CLIP-class |
| Edit in checkpoint | fact-check (§Block 3) | native edit/multi-ref ✅ | edit family | via ecosystem |
| Quant ecosystem | lightx2v quant ✅ (verif.) | official FP8 path | growing | LoRA/ControlNet ecosystem |

Recommendation (to be formulated as a decision in ADR-0123): **the Z-Image-Turbo wedge**,
**FLUX.2-klein — fallback and second target for the edit-first scenario**. The decision goes through
the Go/No-Go gate after phase R3 (the full path "text encoder → 8-step flow → VAE → PNG").

## Block 3 — Fact-check blocking R2/R3 (does not block ADR-0123)

Mandatory verification checklist for the assignee with network access before R2 starts:

1. Architecture of the Z-Image text encoder: which LLM, parameters, weight loading rules
   (affects the R2 plan "encoder assembly = wiring of existing nn blocks").
2. Availability of edit weights in the Z-Image family (if absent — the edit scenario moves to the
   FLUX.2-klein path in phase R6, which changes the R6 priority numbering, but not R1–R5).
3. Exact text of Apache-2.0 on the Z-Image weight cards (match with the expectation, presence
   of additional terms).
4. Reference sampler code (Euler/Sway class) — which repository is considered the reference.
5. NFE profiles: confirmation of the 8-step mode and its quality on reference examples.

## Block 4 — Go/No-Go criteria (phase R3, naryad #212)

- The full path produces a correct image on a single consumer machine.
- Time: reasonable for 8 NFE + decode (the number is fixed in the R3 report, not promised in advance).
- Memory: fits the 16 GB BF16 profile or the 8/6 GB FP8/GGUF profiles.
- On failure of any item — pivot to FLUX.2-klein (the "acknowledge and pivot" precedent of
  ADR-0106/0107), not a quiet dragging-out of the phase.

## Block 5 — What is deliberately NOT included (non-scope, duplicates ADR-0122 for coherence)

1. Pretraining of base models — datacenter economics, pretraining corpora do not exist
   publicly; Metalogos' unit of value is the layer ON TOP of the weights.
2. NCII-class capability targets — contradict the security-by-design brand; instead of
   a ban taken on word — the mechanics of policy declarations and provenance gates (ADR-0125).
3. Video — a separate phase V after R6 (ADR-0126 reserved).
