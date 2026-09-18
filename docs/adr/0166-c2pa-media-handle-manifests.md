# ADR-0166: The C2PA contour of media handles — read/write manifests and the generation guarantee

**Status:** Accepted
**Date:** 2026-09-16
**Naryad:** №337 (issue #464, Волна 2 · Фаза 2)
**Depends on:** №320 (C2PA Art 50 slice, ADR-0152 — the `synthetic` field, the conservative read, the sidecar posture), №331 (media handles + store, ADR-0162 — the sanctioned byte sink), №332 (perception AST + origin chain, ADR-0164 — `kind: generation` binds), №241 (sidecar manifests — the format contract)
**Blocks:** Волна 2 Go/No-Go item 6 («Generation-лифты обязаны ставить synthetic: true»)

## 1. Context

№320 gave vision ARTIFACTS a provenance manifest (Art. 50 marking, `synthetic` with the conservative serde default `true`). №331 gave media HANDLES an opaque store with one sanctioned byte sink (`media_save`); №332 bound handles to declared origins (`kind: camera | file | generation`). The contour still has two holes: (1) a handle egresses through `media_save` with NO manifest at all — bytes leave without provenance, breaking the №241 sidecar continuity at the media sink; (2) the generation marking of media handles is nowhere — neither on the entry nor at the egress — so "generated" vs "captured" is invisible at the store layer, and the marking lives only in the runtime export path (the issue: "не только runtime-помечание").

## 2. Decision

### 2.1 The store entry carries its manifest facts

`MediaEntry` gains `synthetic: bool` (default `false` at insert — captured/stored bytes are NOT synthetic by default; a false negative here would lie in the opposite direction) and `bytes_sha256: String` (computed ONCE at insert from the plaintext bytes — hashing sealed payloads never decrypts anything a second time and `media_manifest` keeps its no-materialization promise). The entry-level manifest facts are the SSOT the egress sidecar is built from; no per-egress recomputation.

### 2.2 Write manifests — the egress sidecar (№241 continuity)

`media_save(handle, path)` now writes BOTH the bytes AND the sidecar `<path>.manifest.json` (the №241 naming, the same JSON form vision_export uses). The sidecar is a `MediaManifest` record:

```
{ "kind": "<slug>", "origin": "<declared origin name | empty>",
  "conf": "<declared sensitivity>", "bytes_sha256": "<hex>",
  "synthetic": true|false, "timestamp": "<RFC3339 UTC>" }
```

- The sidecar write is part of the SINK: a failed sidecar write is a loud error — a manifest-less media egress cannot happen through `media_save`.
- The returned value stays the path (the №331 contract is unchanged; existing `.expected` outputs are untouched).
- The `synthetic` field follows the №320 vocabulary with the same serde discipline on the READ side (`#[serde(default = "default_synthetic")]` → `true`): an old/hand-written manifest without the field describes SYNTHETIC content (unknown ⇒ marked, conservative read — the №320 posture).

### 2.3 The generation guarantee — compile-verified lift-only binding

A handle bound to a `kind: generation` origin is a GENERATION LIFT; it MUST be marked `synthetic: true`:

- **Runtime (the marking itself):** `media_bind_origin` (the ProvBind lowering) resolves the declared origin's kind; `kind == "generation"` FORCES `entry.synthetic = true` on the bound entry. There is NO API to un-mark: `bind_origin` has no synthetic parameter to lie about, and no builtin writes the field. Every legal generation bind is a FRESH `media_store_*` construction (the №332 rule already refuses binds over non-constructions) — so every legal generation handle is marked BY CONSTRUCTION.
- **Compile time (the guarantee):** the origin-chain pass (semantic.rs, mirrored into `audit_category_a`) names the contract on generation binds over non-constructions: the refusal message states that a generation lift must be a fresh `media_store_*` construction because the bind sets `synthetic: true` — a bind over an existing handle would either skip the marking (a generation lift without `synthetic: true` — the exact hole this ADR closes) or falsify it (captured bytes marked synthetic). The red/green corpus pins both the compile refusal and the runtime marking.
- **Egress:** a generation-bound handle's `media_save` sidecar carries `synthetic: true` — mechanically, because the entry's flag is `true` and the sidecar is built from the entry. The manifest-less direction is closed by §2.2 (the sidecar write is part of the sink).

### 2.4 Read manifests — provenance without materialization

- `media_manifest(handle)` — the in-program provenance read over the store: `Struct { kind, origin, conf, synthetic, bytes_sha256, refs, sealed }`. NO bytes leave the store (the hash is the entry-level fact from §2.1). State-carrying: interpreter/VM interception (the media_meta discipline).
- `media_manifest_read(path)` — the sidecar READ path (sandboxed): parses `<...>.manifest.json` and returns the same struct shape. Missing/empty/corrupt manifests are LOUD errors (the №320 `sidecar_read_report` posture — provenance you cannot read is refused, never defaulted silently); a manifest without `synthetic` reads `true` (conservative, §2.2).

### 2.5 Rejected alternatives

- **Recomputing the byte hash at egress**: rejected — duplicates work, and hashing at egress would force sealed-entry decryption on the sink path; the insert-time hash is the entry's identity.
- **A `synthetic` argument on `media_store_*`**: rejected — a programmer-supplied flag is a lie surface; the marking is derived from the BIND kind (declared, compile-validated origin), not from call-site prose.
- **Making the sidecar optional / an opt-out builtin**: rejected — the №320 Art. 50 posture is marking by default; a manifest-less egress must be impossible, not switchable.
- **Extending `VisionManifest` to describe media handles**: rejected — the vision manifest describes a generation event (model/seed/prompt); a media handle's manifest describes the STORE ENTRY (origin/conf/bytes identity). They share the sidecar vocabulary (`synthetic`, `timestamp`) and the file-naming continuity, not the schema.

## 3. Consequences

- Registry 440→442 (`media_manifest`, `media_manifest_read`, category `media`); №316 SSOT classification: `media_manifest` Source/Public/Pure (store metadata, no egress), `media_manifest_read` Source/Internal/Pure (sandboxed file ingest of a manifest record).
- `scripts/gen_classification.py`: `media` and `registry` join RISKY_CATEGORIES (their manual rows were destroyed by an earlier regeneration — the OVERRIDES sync in №336 plus the risky marking prevents a recurrence).
- Old sidecars remain readable (conservative `synthetic` default); vision sidecars and media sidecars coexist by file name without schema collision (different writers, same continuity rule).
- The origin-chain and the manifest now carry the SAME facts (origin name, conf, synthetic) — the §7.4 chain and the C2PA record cannot disagree.

## 4. Verification

- `tests/naryad_337_c2pa_handles.rs`: sidecar written by `media_save` (captured → `synthetic: false`, hash matches bytes, origin named); generation bind → sidecar `synthetic: true` (both backends); `media_manifest` reads provenance without materialization; `media_manifest_read` roundtrip + loud corrupt/empty refusals + the conservative `true` read of an old manifest without the field; compile red: generation binds over non-constructions are refused with the synthetic-contract message; no-stubs discipline.
- `examples/w1_c2pa_egress.mlog`: capture → egress → read-back (false), generation → egress → read-back (true) — both manifests on both backends.
