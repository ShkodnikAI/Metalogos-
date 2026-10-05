# ADR-0182: The media-handle family and the backend-registry interface — the domain-line reopening contour

**Status:** Proposed
**Date:** 2026-10-05
**Naryad:** №578 (issue #991, Волна 28 — the domain line's first naryad after the lift)
**Depends on:** ADR-0177 (the domain freeze — Lifted), ADR-0114 (the opaque-handle pattern), ADR-0154 (the label lattice), ADR-0162 (the unified media layer), ADR-0163 (the backend registry), ADR-0165 (the degradation ladder)
**Blocks:** the first implementation line of the reopened domain line (Wave 29; the owner's §7 answers are the gate)

## 1. Context

The domain freeze is lifted (ADR-0177 → Lifted 2026-09-27, gh#680) and the
owner's decision 3Б (2026-10-04, machine-recorded in the
`scripts/ci/gate_029_goals.txt` header, gh#979) opens Phase 2 with the first
line = media handles + the backend registry. This ADR is the CONTOUR for that
line: it closes the contour part of the Charter's Phase 8 debt (the production
LLM backend) and fixes what the first implementation wave builds.

**The honest stock-taking first.** The naryad's draft fact base enumerated the
media surface as pillar-scoped stores (`PDF_DOCS`, `VIDEO_REGISTRY`,
`VOICE_REGISTRY` ×2, `LLM_STREAM_REGISTRY` — the №515 inventory) and stated
"the label lattice is not yet landed". The tree holds more than that, and this
ADR builds on the landed work rather than re-planning it:

- the **unified media layer** landed with №331 (ADR-0162): `ImageId` /
  `AudioId` / `VideoFrameId` / `VideoSegmentId`, one `Value::Media(MediaHandle)`
  variant, the `MediaStore` (lazy materialization, refcount, AES-256-GCM
  at-rest sealing, sanctioned sinks) — wired in BOTH backends (interpreter
  interception + the VM's own store);
- the **backend registry** landed with №333 (ADR-0163): `BACKEND_REGISTRY`
  (17 entries), classes, SHA-pin contract, license classes, the
  `BACKEND_LICENSE_DISTRIBUTION` gate, the `licensing` profile;
- the **degradation ladder** landed with №336 (ADR-0165):
  `backend_select` + typed `Degraded(t)`;
- the **label lattice** landed with №322 (ADR-0154) and carries into the
  `Labeled(Box<Type>, Label)` carrier (№544, 0.28.1) — media handles already
  carry static labels through the №323 inference and hit the №325 sink gate.

Where this ADR corrects the naryad's draft, §6 says so loudly — the honest fix
is to record the delta, not to pretend a parallel track.

## 2. The gap this contour closes (the fact at 2026-10-05)

The two halves of the Phase-2 contour both exist — **they are not connected**:

**(a) Every media-taking backend accepts raw strings, not handles.** The input
of `vision_understand(image, …)`, `ocr_extract(image, …)`,
`stt_transcribe(audio, …)`, `omni_ask(prompt, media?, …)` and
`video_understand(segment, …)` is `Value::String` (see
`src/vision/understand.rs`, `src/vision/ocr.rs`, `src/voice/backend.rs`,
`src/video/understand.rs`). Media bytes can therefore reach backend calls
AROUND the store: no label join into the result, no sealing on the way in, no
refcount, no origin. The opaque-handle discipline (ADR-0114 — "the value the
language manipulates is a reference, not the payload") stops at the exact
boundary the Phase-8 debt is about: the backend input.

**(b) The registry has no capability axis.** `BACKEND_REGISTRY` says what a
backend IS (class, weights, pin, license) but not what it ACCEPTS. Nothing in
the table distinguishes an image-consuming entry from a text-only one, so
capability-driven selection is impossible and a mistyped media argument can
only fail at the builtin's own arity check — a runtime string error, not a
typed, selectable contract.

## 3. Decision

### 3.1 The handle family

The family is **ADR-0162's, kept** — no new types, no new `Value` variant:

- the language-visible names stay `Image` / `Audio` / `VideoFrame` /
  `VideoSegment` (`Value::type_name` of `Value::Media`);
- the backend-input line starts with the **three the backend surface names**
  (Image, Audio, VideoFrame — the naryad's three); `VideoSegment`'s own
  consumer path follows with video-understanding's next step if the owner
  keeps it in scope (§7.4);
- **opacity is completed, not invented:** the one raw-string hatch that exists
  (§2a) closes — backend media inputs accept
  `Value::Media(MediaHandle::Image(_))` and siblings;
- **lifecycle create → use → drop** uses the store's existing API unchanged:
  `media_store_*` (create, sealed per declared sensitivity) → the backend call
  (use — new) → `media_release` (drop). The honest `Copy`/refcount boundary of
  ADR-0162 §2.4 is untouched;
- **local security per the Voice precedent:** a backend call is PROCESSING,
  not byte egress — the handle's static label joins the call's result label
  through the existing №323 inference (a private image in → a private
  description out), consent-scope rides the same carrier, and the №325
  sink-clearance machinery keeps governing the real egress (`media_save`).
  The materialization a backend consumer needs reuses the store's sanctioned
  read path (the `media_save` mechanics — lazy, sealed entries decrypt only
  inside the sanctioned consumer); the at-rest rule is untouched.

**The lattice connection is landed, not parallel.** The naryad draft asked the
ADR to record "the lattice is not yet landed" and design handles "ready for
connection". The fact is the opposite: ADR-0154 + №544 landed the lattice and
the `Labeled` carrier, and ADR-0162 already flows handle labels through №323.
The contour therefore BUILDS ON the lattice; nothing parallel is needed. (The
correction is itself the honest-boundary record the naryad asked for.)

### 3.2 The backend-registry interface (the Phase-8 debt contour)

- **Registration stays static.** `BACKEND_REGISTRY` remains the spec!-style
  SSOT table (ADR-0163 §2.1) — one entry per backend, drift = failure. The
  plan-v2 "generalization of BUILTIN_REGISTRY" is read as the TABLE +
  CAPABILITY axis below, not as runtime registration.
- **Capability descriptor.** Each entry gains an `inputs: &[MediaKind]` field
  (empty = text-only): the machine answer to "what does this backend accept".
  It is the selection key for §3.2's fail-closed rule and the compile-time
  documentation of the media-input surface.
- **Fail-closed selection.** A selection that names a capability (a media
  input kind) refuses loudly when no registered entry satisfies it —
  degradation descends the ADR-0165 ladder, it never steps sideways into an
  unsatisfied capability; the loud-refusal discipline is the №263
  `STREAM_LIMIT_REACHED` precedent, not a silent fallback.
- **Bounds discipline (№515).** Any runtime state the interface grows
  (in-flight call tables, weight-load caches) joins
  `scripts/ci/registry_bounds_inventory.txt` with two-circuit bounds BEFORE
  landing — the ratchet already fails a new unbounded map, this line just
  refuses to trip it.
- **TW/VM parity.** The new input forms keep one body over the shared
  registry per backend builtin (ADR-0163's marshaling split); the parity
  contract (ADR-0156) covers the handle-input shapes — the crosscheck corpus
  grows handle-input cases alongside the string cases (a separate surface
  from №585's route-body lane; no generative-contour growth).

### 3.3 The first implementation line (the Wave-29 map)

1. **Image first** (the naryad's order): `vision_understand` and
   `ocr_extract` accept `Value::Media(MediaHandle::Image(_))` — materialize
   through the store, label-join into the result, capability descriptors
   filled for the VisionUnderstanding/Ocr entries. Pre-condition: THIS ADR
   merged.
2. **Audio second:** `stt_transcribe` and `omni_ask`'s media argument accept
   Audio handles.
3. **VideoFrame third:** `video_understand` accepts VideoFrame handles.
4. The fate of the existing String forms (overload / deprecation / refusal)
   is the owner's §7.2 answer — the migration text follows the №493
   multi-form builtin precedent either way; golden behavior of the string
   forms does not change by default.

The implementation naryads are NOT issued without the owner's §7 answers
(the Wave-29 gate — the naryad's own contract).

## 4. Alternatives considered

- **No bridge — handles stay store-only, backends stay string-fed.** Rejected:
  §2a is exactly the Phase-2 risk the plan names; the store's label/seal/
  refcount guarantees are void on the path that matters most (the model sees
  the bytes). Price of the bridge: five builtins' input forms + both-backend
  tests — small against the guarantee.
- **Dynamic registry (register backends from .mlog at runtime).** Rejected for
  the first line: the static table is the auditable SSOT the SHA-pin/license
  gates are built on; runtime registration re-opens the trust boundary those
  gates exist to hold. Price: none now; if an out-of-tree backend need ever
  appears, it gets its own ADR.
- **One `Value` variant per media kind (`Value::Image`, …).** Rejected: four
  exhaustive matches to touch everywhere for zero extra safety — ADR-0162's
  one-variant decision stands; the dispatch builtins already give the per-type
  static contract.
- **A byte-safe newtype input (`MediaBytes(String)`) instead of handles.**
  Rejected: it launders the raw string (the hatch with a nicer name), loses
  the store's sealing/refcount/label, and duplicates a lifecycle the store
  already holds.

## 5. Consequences

- The five backend builtins' input surface changes — a semantic-visible,
  both-backend change with a migration text (№493 discipline); the string
  forms' golden behavior is untouched by default.
- `MediaStore` gains a sanctioned read path for backend consumers (lazy,
  decryption inside the consumer only) — the at-rest sealing and the
  `media_save` egress gate are unchanged.
- `BACKEND_REGISTRY` entries grow the `inputs` field — a one-line schema
  change, drifted entries fail the existing table tests.
- The Charter Phase 8 debt: this closes its CONTOUR part. The production LLM
  backend itself (real weights, serving) is the follow-on line — §7.5 — and
  the class fact is honest today: `BackendClass::Llm` exists, the registry
  holds ZERO LLM entries.

## 6. Honest boundaries

- Three corrections to the naryad's draft fact base, recorded in §1: the
  unified media layer (№331), the backend registry (№333) and the label
  lattice (№322) are LANDED — the contour builds on them; "the lattice is
  not yet landed" is stale and is corrected here rather than re-planned
  around.
- The naryad's diff is this file + the ADR index row — no src edits, no
  builtin changes, no C2PA, no plan edits (the plan sync is office-side).
- This ADR lands as **Proposed**: §7 is the owner's gate; nothing in §3
  executes without it.

## 7. Open questions for the owner (the Wave-29 gate)

1. **Type names** — the language-visible names stay `Image` / `Audio` /
   `VideoFrame` (§3.1)? Or a namespaced form (`Media.Image`)?
2. **The String forms** of the five backend builtins — keep as overloads
   (№493 precedent), migrate with a deprecation warning, or refuse in 0.29
   with a migration text?
3. **The order** — Image → Audio → VideoFrame confirmed as the first line?
4. **C2PA slice** — out of this contour (plan v2 §13.4 2.7 as the follow-on
   benefit, extending the №320 mini-slice), confirmed?
5. **The production LLM backend** (the Phase-8 core) — which implementation
   first: real LLM-class registry entries behind the existing `llm` builtin
   surface, or a new serving backend line? A separate naryad line after the
   bridge lands; the owner sequences it.
