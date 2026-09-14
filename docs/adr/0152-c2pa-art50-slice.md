# ADR-0152: C2PA mini-slice — Art. 50 synthetic marking on egress (no clearance lattice)

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** №320 (issue #407, P0/security-provenance, Волна 0)
**Precedent:** ADR-0125 (vision provenance gates), №241 (VisionManifest + LSB watermark + sidecar), №300 / audit.rs:1607–1757 (Category-A gate template), ADR-0151 (the adjacent video provenance posture)
**External anchors:** EU AI Act Art. 50 (transparency of synthetic content — marking window closes **2026-12-02**), C2PA 2.4 spec (SEC-5)

## Context

The review decision D6-«а» (диспетч №408): the Art. 50 marking slice MUST NOT wait for the clearance lattice / label-checker (Фаза 1) — the deadline is fixed. The repo already has the provenance contour (№241): every generation path writes a `VisionManifest`, default export writes the sidecar `<path>.manifest.json`, raw export is an explicit unsigned opt-out (ADR-0125 D3). What is missing: the artifacts do not carry an explicit **synthetic** flag, and the unmarked egress path is not gated.

## Decision

### D1. `synthetic: true` on every generation manifest (back-compatible)

`VisionManifest` gains `synthetic: bool` (serde). Every generation constructor sets `synthetic: true`. **Deserialize default is `true`**: the only historical writer was the generation path, so an old sidecar without the field describes a synthetic artifact — the conservative read is the honest read (unknown ⇒ marked).

### D2. Static gate `MEDIA_SYNTHETIC_UNMARKED` (Category-A Error)

`vision_export_raw` call sites are statically visible attempted unmarked egresses: every locally generated artifact is synthetic by construction (generation is the only artifact writer), so a raw export of a registry artifact egresses synthetic content without its manifest. Per the audit.rs:1607–1757 template: `Severity::Error`, wired into `audit_category_a` (compile error via the №98 promotion) and `audit_program`. The №241 `VISION_UNSIGNED_EXPORT_RAW` advisory Warning stays (audit_program only) — the Warning records the opt-out intent, the Error enforces the Art. 50 marking.

### D3. Runtime backstop in `vision_export_raw` (amendment of ADR-0125 D3, scoped)

`vision_export_raw` refuses (loud `MEDIA_SYNTHETIC_UNMARKED` error):
- artifacts whose manifest says `synthetic: true` (the normal case — all local generation);
- manifest-less artifacts (unmarked ⇒ treated as synthetic, conservative default).

Raw egress remains possible only for artifacts explicitly marked `synthetic: false` — which no local generation path produces today; the flag exists for the future ingest contour (foreign non-synthetic media). This is a deliberate, scoped amendment of ADR-0125 D3 ("raw works on anything"): the Art. 50 window overrides the universal opt-out for synthetic content. The №241/№242 tests that exercised raw egress of manifest-less/synthetic artifacts are updated to the new contract with references to this ADR.

### D4. Sidecar continuity (№241) + loud read path

The sidecar writer is unchanged (pretty JSON next to the bytes — provenance you cannot read is provenance you cannot verify). The read path gains `provenance::sidecar_read_report`: it extracts the manifest (including `synthetic`) and reports a missing/corrupt manifest LOUDLY (Err, never a panic, never a silent default).

### D5. Honest boundary — COSE/JUMBF not validated (loud)

This slice does NOT implement C2PA COSE signature validation or the JUMBF container: the sidecar is plain JSON with the C2PA-shaped fields (claim generator, model, seed, prompt hash, timestamp, synthetic flag). The boundary is loud in code, REFERENCE, and here; the full handle contour (validated C2PA manifests, hardware-rooted signing) is №337 (Фаза 2). The "external validator" contract test validates the sidecar structure as JSON with the required fields — the same honesty as №241's sidecar tests.

### D6. Compliance statement

With this slice: every generation path marks `synthetic: true` (test-enumerated constructors); the only egress paths are the signed export (ships the manifest sidecar — Art. 50 marking present) and the raw export (statically gated Error + runtime refusal for synthetic). No generation → egress path remains unmarked. Deadline 2026-12-02: satisfied at the slice level; full C2PA conformance stays with №337.

## Consequences

- `VisionManifest` carries `synthetic` (serde-default true); all constructors updated (source-enumeration test).
- `MEDIA_SYNTHETIC_UNMARKED` appears in audit output and on the compile path for `vision_export_raw` call sites.
- №241/№242 raw-export tests updated to the amended contract (loud refusal for synthetic/manifest-less).
- REFERENCE `vision_export_raw` row documents the gate.
- Scoped to the vision contour; voice/video marking rides their signed-by-construction exports (№309) and the full cross-pillar contour is №337.
