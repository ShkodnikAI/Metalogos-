# ADR-0120: Opening text generation — amends `ADR-0117` §3 by explicit owner decision

**Status:** Accepted
**Date:** 2026-09-06
**Naryad:** #193 (blocks on this ADR)
**Pillar:** `Reflex` (new capability, not a new pillar — same pillar)
**Amends:** `ADR-0117` §3 ("why free-form generation is out of scope,
not future work"). That section is **superseded**, not deleted —
kept in the document as the historical record of the boundary and
the exact condition under which it would be revisited ("requires
its own, separate owner decision"). That condition has now been met.

## Context

`ADR-0117` drew a structural line: `Reflex`/`reflex_seq` classify a
closed label set or a whole sequence into one label — never generate
open-ended text. Naryads №183–192 built (and integration-verified)
real attention, GQA, RoPE, `RmsNorm`/`SwiGLU`, multi-block stacking
with correct gradient flow through the whole stack, and real
`candle`-autograd training. The owner has now explicitly authorized
crossing the generation boundary this specific infrastructure was
built up to, but deliberately not across.

This is not a reopening of a settled question absent new information
— it is the exact scenario `ADR-0117` named as its own exit
condition.

## What generation genuinely requires that classification did not

Three real, load-bearing gaps — not incremental additions to existing
code:

1. **Vocabulary output, not a closed label list.** `reflex_seq`'s
   classification head projects to `labels.len()` outputs (naryad
   №185: 3-10 classes typically). Generation projects to a real
   vocabulary (thousands to tens of thousands of token IDs) at every
   position. This is a different final-layer shape, not a bigger
   version of the same one.
2. **Autoregressive decoding loop with KV-cache.** Classification
   runs the forward pass once per input. Generation runs it once per
   *output token*, reusing cached key/value tensors from prior steps
   — without a cache, generating N tokens costs O(N²) instead of
   O(N). `candle-transformers`' real Llama reference (naryad №176's
   documented template) already implements this pattern — it is the
   template for this naryad too, not a new invention.
3. **Sampling, not argmax.** Classification takes the single highest-
   confidence label. Generation needs temperature/top-k/top-p
   sampling to avoid deterministic, repetitive output — a real
   design surface `Reflex`'s classification path never needed.

## Decision

A **new declaration form**, `reflex_gen`, not a silent extension of
`reflex_seq`. `reflex_seq`'s existing behavior (mean-pooling to one
label, naryads №185–192's entire test suite) is **not touched** —
same discipline as naryad №182/188/190 applied to `Layer`/`Attention`.

```mlog
reflex_gen TinyStoryteller {
  input: embedding(64)
  vocab_size: 4096
  layers: [transformer_block(4, 64, 256), transformer_block(4, 64, 256)]
  seed: 42
}
```

`reflex_generate(model, prompt_tokens, max_tokens, temperature)` —
new builtin, KV-cache-backed autoregressive loop, following the exact
structure of `candle-transformers`' Llama generation loop (naryad
№176's reference), not a from-scratch design.

**Tokenization is explicitly out of scope for this ADR.** `reflex_gen`
operates on already-tokenized integer sequences (`Vec<u32>` token
IDs) — mapping raw text to/from tokens (BPE or similar) is a separate,
follow-up decision, not bundled into opening generation itself. This
mirrors naryad №120's own discipline of not silently growing scope
inside one decision.

## Consequences

- `ADR-0117`'s classification path (`reflex`, `reflex_seq`) is
  provably unaffected — a new declaration keyword, not a modified
  one.
- The pillar now has two genuinely distinct capabilities sharing the
  same underlying architecture blocks (attention/GQA/transformer
  stack, naryads №183–192): closed-set classification and open-ended
  generation. Both are `Reflex`, not two pillars.
- Tokenization remains a named, deferred gap — `reflex_generate`
  without a paired tokenizer decision only accepts pre-tokenized
  integer input, an explicit, documented limitation, not a silent
  one.
- Image/video generation (asked about separately, same conversation)
  is **not** addressed by this ADR — genuinely different tensor
  shapes (2D/3D, not 1D sequences), a different training paradigm
  (diffusion, not autoregression), and an additional subsystem
  (VAE latent compression) that nothing in naryads №177–192 provides.
  `candle-transformers` does ship `stable_diffusion`/`wuerstchen`
  reference implementations (verified before writing this ADR), so
  the same "follow a real reference" principle would apply if that
  door is opened later — but that is explicitly a separate, future
  owner decision, not implied by this one.
