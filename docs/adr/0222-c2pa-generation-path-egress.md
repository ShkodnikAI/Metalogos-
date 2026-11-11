# ADR-0222: The C2PA contour completion — the generation-path egress manifest

**Status:** Accepted (2026-11-11 — the executed wave acceptance: В95 closed completed 2/2, диспатч gh#1902; исполнение — №943, PR #1904 squash 9040425c)
**Date:** 2026-11-10
**Naryad:** №942 (issue #1900, Волна 95 — W-Q, №Q1)
**Depends on:** №320 (ADR-0152 — the `synthetic` vocabulary, the conservative read, the sidecar posture), №337 (ADR-0166 — the media-handle contour, the entry-level manifest facts, the loud sidecar failure), №241 (the sidecar format contract — naming and the pretty JSON form), №279 (the `tts_generate` contract — synthesis without delivery, the returned path), №475 (the fs_gate facade), ADR-0207 §5 (the provenance-layer honest limits), ADR-0125 (the honest `"unspecified"` marker precedent)
**Blocks:** план v2 §13.4 2.7 («полноценный C2PA-контур хэндлов: read/write манифестов на входе/выходе, `synthetic: true` на generation-пути») — the last open hole of the line

## 1. Context

The storage side of the C2PA contour is built: №320 gave vision artifacts the `synthetic` marking with the conservative read; №332 bound media handles to declared origins; №337 gave media store entries their manifest facts (`MediaEntry.synthetic` + `bytes_sha256`, computed once at insert) and made `media_save` a provenance-bearing sink — the egress sidecar `<path>.manifest.json` is part of the sink, and «a failed sidecar write is a loud error — a manifest-less media egress cannot happen through `media_save`» (ADR-0166 §2.2). `vision_export` ships the same sidecar continuity (№241): bytes, then the manifest, with the sidecar failure refused loudly.

One media-generation egress remains outside the contour: `builtin_tts_generate` (`src/builtins/llm.rs:469–510`, naryad №279) synthesizes audio (OpenAI-compatible `/audio/speech`), writes `tts_{ts}.mp3` into the file sandbox through the fs_gate facade, and returns the path — **without any manifest**. The machine fact (verified on live main `81f6f88a`): `grep -c manifest src/builtins/llm.rs` = 0. The three-way comparison is exact: `vision_export` carries the sidecar and refuses an artifact without a manifest loudly (`src/builtins/vision.rs:427–431`), `media_save` carries the sidecar as part of the sink (ADR-0166 §2.2), `tts_generate` — nothing. Generated audio leaves the sandbox with no provenance record: the exact hole the plan's §13.4 2.7 names («`synthetic: true` на generation-пути, расширяет мини-слайс №320»).

The sequencing: the owner's §7.4 answer of 2026-10-05 fixed plan v2 §13.4 2.7 as the follow-on benefit «extending the №320 mini-slice; no C2PA work in the first line» of the bridge; the first line completed 09.11 (№938/№939 — steps 2/3 of 3), the §7.5 class-fact closed in волна 94, and the owner's directive of 10.11 sequenced this line (§16.0-2). This ADR fixes the form BEFORE the code (N1): the implementation lands in the sister naryad №943 and executes this document verbatim.

## 2. Decision

### 2.1 The sidecar form — the №706 base + the generation-event fields

`tts_generate` writes the sidecar `tts_{ts}.mp3.manifest.json` (the №241 naming — `<path>.manifest.json`, the same continuity `vision_export` and `media_save` run) next to the audio bytes. The record is the fixed 6-field `MediaManifest` base (the №706 shape) EXTENDED by the generation-event fields `provider` / `model` / `voice`:

```
{ "kind": "tts_generate", "origin": "", "conf": "unspecified",
  "bytes_sha256": "<hex>", "synthetic": true,
  "timestamp": "<RFC3339 UTC>",
  "provider": "openai", "model": "tts-1", "voice": "<voice>" }
```

- **`kind: "tts_generate"`** — the record describes the GENERATION EVENT, not a store entry: the slug names the producing path (the VisionManifest precedent — ADR-0166 §2.5 rejected folding the two schemas together and kept «separate schemas, shared dictionary continuity»: the base 6 fields carry the same vocabulary, the event fields are additive).
- **`origin: ""`** — generation has no declared origin to bind (the №332 origin chain does not apply: no handle exists at synthesis time; the empty string is the same marker `media_save` writes for pre-№332 entries).
- **`synthetic: true` BY CONSTRUCTION** — the only writer of this sidecar is the generation path, so every record it writes is marked true mechanically, not by call-site prose. The conservative read default (an absent field ⇒ `true`, №320/ADR-0152) stays untouched for foreign readers of OLD manifests; this ADR adds no new read-side default.
- **`bytes_sha256`** — the SHA-256 of the plaintext audio bytes held IN MEMORY, computed BEFORE any write. The ADR-0166 §2.1 posture transfers verbatim: the hash is the byte identity, and hashing here never decrypts anything a second time — there is no store ciphering on this path at all (the bytes never enter the media store; `tts_generate` is synthesis without delivery and without a handle).
- **`conf: "unspecified"`** — the ADR-0125 honest-marker precedent (the `VisionManifest.policy` R5 policy-relax): the confidentiality of the source TEXT is governed by the label lattice and the №325 sink clearance, not by the manifest; the manifest records ORIGIN. A value can be «perfectly tainted and perfectly provenanced at once» (ADR-0207 §5 verbatim) — the marker keeps the two axes honestly separate instead of fabricating a sensitivity class the layer does not know.
- **`timestamp`** — wall-clock, RFC 3339 UTC (the №241 form; not pinned in tests, as in both sibling manifests).
- **`provider` / `model` / `voice`** — the actual call arguments as the builtin resolved them (the defaults `"openai"` / `"tts-1"` included), the same fact class VisionManifest records as `model_id`/`seed` for the vision generation event.

The base 6 fields stay byte-compatible with the №706 fixed form: `media_manifest_read` reads this sidecar as a `MediaManifest` (the additive fields are unknown to the struct and are ignored by the serde posture — `MediaManifest` carries no `deny_unknown_fields`); the compatibility is pinned by a test in №943, and if it ever breaks, the honest fallback is the separate-schema reading THIS ADR already fixes (the schemas share the vocabulary, not the contract).

### 2.2 The write order — hash, audio, sidecar; the loud failure is the posture

The order is: hash (in-memory) → audio write → sidecar write. Both siblings run exactly this order (`vision_export`: bytes, then sidecar; `media_save`: bytes, then sidecar), and the posture is one posture: **the sidecar write is part of the SINK — a failed sidecar write is a loud error** (`tts_generate(): …`), verbatim the ADR-0166 §2.2 wording: a manifest-less media egress cannot happen through `tts_generate`. The egress is the returned path: the loud refusal means the program never receives the path, so no successful call can ship audio without its manifest. The failure window between the audio write and the sidecar write can leave the partial artifact on disk — the honest residue both siblings already carry, bounded by the window and named by the program-invisible generated filename (№475: the name is program-invisible, the egress stays gated).

- The returned value stays the path (the №279 contract is unchanged; no `.expected` output moves).
- The sidecar write goes through the same fs_gate №475 facade (the №252 write semantics: sandbox-resolve, symlink-safe, the hard write-deny and the deny-list stay); the gate is not weakened for the derived file — it sits next to the gated main file, the same mechanics `media_save` runs for its sidecar.
- No opt-out, no optional flag: the №320 Art. 50 posture is marking by default; a manifest-less generation egress must be impossible, not switchable (ADR-0166 §2.5 verbatim).

### 2.3 The read side — no new reader

No new builtin, no registry growth: `media_manifest_read` (the №706 fixed form) is the read path for the base fields; the generation-event fields are additive metadata a foreign reader takes from the sidecar document directly (the pretty JSON is the №241 form — provenance you cannot read is provenance you do not have). The spec table does not grow; the №493 builtin surface is untouched.

## 3. Honest boundaries (verbatim)

- **`tts_send` is OUTSIDE the contour.** The delivery egress (Telegram sendVoice/sendAudio) is governed by the №325 sink clearance and the Ledger; a delivery-side provenance record is a separate decision at the owner's sequencing, not this ADR's scope.
- **VideoSegment consumer-path — the owner's gate** (ADR-0182 §7.3: «отдельное решение владельца»); nothing here touches it.
- **ProvenanceMap media-carriers — the ADR-0207 §5 explicit future transaction.** This ADR does NOT extend the wave-77 registry; the storage contour (ADR-0152/ADR-0166) stays the media-domain SSOT, and no second manifest format is written over media bytes — the generation sidecar extends the SAME №241/№706 vocabulary the store already speaks.
- **No C2PA-SDK conformance claims.** The sidecar is the №241 JSON posture, not a binary C2PA Manifest: COSE signatures and the JUMBF container are unvalidated — the ADR-0152 D5 boundary verbatim. The word "C2PA" in the line's name names the contour, not a certification.

## 4. Consequences

- `tts_generate` becomes a provenance-bearing egress — the third media-generation egress and the last one without a manifest; with №943 the §13.4 2.7 line is CLOSED (read/write manifests on handle in/out — built by №320/№332/№337; `synthetic: true` on the generation path — by construction here).
- No registry rows, no new builtins, no grammar: the writer is a crate-internal helper beside the store's own sidecar form (the media domain stays the SSOT of the sidecar vocabulary, ADR-0207 §5); the №515 bounds hold — no new runtime state.
- The honest markers travel: `origin: ""` and `conf: "unspecified"` are statements of what the layer does NOT know, not fabricated facts.
- The mechanical syncs: ADR index 209→210, REFERENCE head ADR count, the generated `docs/metrics.md` row, llms.txt №1–№942, CHANGELOG [Unreleased].
- Status at landing: **Proposed** (the ADR-0207/wave-77 precedent; the Proposed→Accepted pass is a separate wave — the В86 precedent).

## 5. Verification (the №943 roster)

- Golden: `tts_generate` against the mock base (`METALOGOS_TTS_BASE_URL`, no real keys — the №279 infrastructure) writes the sidecar next to the file; the record carries `kind: "tts_generate"`, `origin: ""`, `conf: "unspecified"`, `synthetic: true`, `provider`/`model`/`voice` reflecting the call, and `bytes_sha256` equal to the hash of the bytes actually on disk.
- Negative: the sidecar path unavailable → the loud `tts_generate(): …` refusal; the manifest-less egress posture verified по механике §2.2.
- Compatibility: `media_manifest_read` over the tts sidecar returns the 6-field base with the additive fields ignored (the test pins the serde posture; a break flips to the separate-schema honesty this ADR already states).
- TW/VM parity: both backends run the same builtin; the crosscheck corpus exercises both.
- NO DRIFT (ci domain: the grammar, the builtins table and the string goldens untouched; the returned path and the №279 contract unchanged).
