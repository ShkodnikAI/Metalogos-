# ADR-0164: Perception AST — HandleSource/Lift/Sink/ProvBind and the origin chain

**Status:** Accepted
**Date:** 2026-09-16
**Naryad:** №332 (issue #459, Волна 2 · Фаза 2)
**Depends on:** №331 (media handles + store, ADR-0162), №325 (sink clearance, ADR-0161), №322 (label lattice, ADR-0154), №241 (provenance manifests), №294 (PARKED boundary discipline)
**Blocks:** №337 (full C2PA contour of handles), №335 (consent sources — the consent axis of origin labels)

## 1. Context

The №331 store made media handles opaque and sealed at rest, but a handle's PROVENANCE was unexpressed at the language level: `media_store_image(bytes, "public")` mints a handle from nowhere, and nothing connects a handle to the declared source of its bytes. The spec's origin-chain rule (§7.4) — "a handle without origin is not constructed" — is not enforceable, and the §5.3 kitchen-camera scenario (a private camera whose frames must not materialize) has no static end-to-end expression. Existing provenance infrastructure (№241 `VisionManifest`, sidecar manifests) covers generated media only.

## 2. Decision

### 2.1 The perception syntax (grammar: +7 rules, 316 → 323)

```
origin kitchen_cam { kind: camera, media: image, label: private }
let frame = source kitchen_cam                          // HandleSource
let img   = from gen media_store_image(bytes, "public") // ProvBind over a Lift
```

- `origin <name> { kind: camera|file|generation, media: <MediaKind>, label: public|consented|private, path: "..." }` — the DECLARED SOURCE of perception handles. `kind: file` requires `path` (the sandboxed capture source); `kind: camera` is static-chain-valid but the capture itself is a loud PARKED boundary at runtime (no capture hardware in this environment, №294 class); `kind: generation` is the kind of bind-constructed handles (Lift+ProvBind). `label: poisoned` is NOT constructible by declaration — quarantine comes only from the taint machinery (ADR-0154 §2.1).
- `source <origin>` (HandleSource) — produce a handle FROM a declared origin; legal ONLY as a binding initializer (`let h = source o`) or in `return` position (a capture pattern may hand its handle to the next pipeline stage). Unknown origin names are loud.
- `from <origin> <construction>` (ProvBind) — bind the provenance of a NEWLY constructed handle to a declared origin; the inner expression must be a handle construction (`media_store_*` — the Lift). A bind joins the declared origin conf into the entry label (re-sealing at rest when a public entry becomes non-public — the №331 store contract tracks the STRONGEST label).
- The Sink node of §7.4 is `media_save` (№331's sanctioned materialization sink) — no new sink builtin; the Sink clearance is the №325 gate, which now sees origin labels through the data flow.

### 2.2 The origin chain rule: `ORIGIN_REQUIRED` (Category-A, always Error)

A media handle without origin is NOT constructed. Static enforcement in `semantic.rs` (`media_origin_violations`), wired into BOTH compile paths (`check_program` + `audit_category_a`/`audit_program` — the №331 opacity posture, no profile downgrades):

- a bare `media_store_*(...)` binding → `ORIGIN_REQUIRED` ("a handle without origin is not constructed (§7.4); wrap it: `from <origin> media_store_*(...)`");
- `source`/`from` in illegal positions (nested in another expression, over a non-construction) → loud;
- direct `media_source_capture`/`media_bind_origin` calls → loud (the dispatch builtins are lowered forms only);
- bindings and one-step aliases are tracked (the opacity walker's media_vars model), so the chain survives `let alias = img`.

The construction-site pass is mirrored by `ORIGIN_DECL_INVALID` (Category-A Error): declared-origin shape and vocabulary errors (unknown kind/media/label words, unknown fields, missing required fields, `kind: file` without `path`) are loud on EVERY compile path — a mis-declared provenance source is a provenance lie.

### 2.3 Labels flow from origins

`source <origin>` carries the origin's declared conf into the flow (integrity Trusted, consent empty until №335); `from <origin> <construction>` carries the JOIN of the origin label and the construction's data-flow label (conservative — the strongest axis wins). The №325 sink clearance consumes these labels unchanged: the §5.3 kitchen camera (`label: private` → `source kitchen_cam` → `media_save(...)`) is denied AT COMPILE TIME with `SECRET_LEAK` naming the sink, the container and the carried label — a static deny with an explainable reason, never a runtime fall and never a silent pass.

### 2.4 Runtime lowering

HandleSource lowers to `media_source_capture(origin)`; ProvBind lowers to `media_bind_origin(origin, handle)` — state-carrying builtins intercepted by interpreter and VM BEFORE the generic registry fallback (the №331 dispatch model; registry stubs are loud on live paths). `media_source_capture` for `kind: file` reads the sandboxed path (loud on missing files) and inserts into the store with the declared label; for `kind: camera` it is the loud PARKED boundary. `media_bind_origin` binds the entry's origin and joins the declared conf (re-sealing on a public → non-public transition). `media_meta` now exposes `origin` — the bound provenance name, observable WITHOUT materializing bytes (empty string for unbound entries).

## 3. Alternatives considered

- **A dedicated Lift AST node** (mirroring HandleSource) — rejected: the Lift already exists as the `media_store_*` builtin family with a typed, classified contract (№316 `lift` role); a parallel node would duplicate lowering and classification. ProvBind over the existing construction is the minimal honest bind.
- **A dedicated Sink AST node** — rejected for the same reason: `media_save` IS the sanctioned sink with its №325 gate and runtime backstop; a second sink form would create two egress vocabularies.
- **Runtime-only provenance tracking** — rejected: the origin rule is a type-of-construction invariant; runtime denial arrives after bytes already entered the store, and the §5.3 scenario demands a static deny with the node/rule/sink named.
- **Trusting the `sensitivity` argument alone** — rejected: the string argument speaks about at-rest sealing, not about WHERE the bytes came from; provenance is a declaration, not a parameter guess.

## 4. Consequences

- №331's corpus was adapted to the origin chain (the rule is the language evolution this naryad mandates): every `media_store_*` construction is now origin-bound; the w1_handle_opaque fixture constructs under a bind and its error contract is unchanged.
- №337 (C2PA) reads the bound origin to seed manifest provenance; №335 (consent) extends the origin label's consent axis with grant/revoke cascades.
- Handles returned from patterns (Return position) carry their origin binding only as far as the store entry does — cross-pattern provenance composition is №337's scope.
- Grammar rule count 316 → 323; builtins 430 → 432 (media_source_capture, media_bind_origin); media_meta gains the `origin` field.

## 5. Verification

- `tests/naryad_332_origin_chain.rs` (21): origin vocabulary/shape red/green (unknown kind/media/label/field, missing fields, file-without-path), HandleSource red/green (unknown origin, illegal position, file capture end-to-end on TW AND VM, camera PARKED), ProvBind red/green (bare Lift, bind over non-construction, unknown bind origin, direct builtin calls), provenance observable via media_meta + re-seal parity, the §5.3 kitchen-camera static deny (fixture + inline, alias propagation), public origins flow through the №325 gate, and the `w1_origin_chain` example passes on both backends.
- `examples/w1_origin_chain.mlog` + `.expected` — the passing origin-bound path (golden + crosscheck contracts).
- `examples/w1_kitchen_camera.mlog` + `.error` — the §5.3 static deny contract: `[SECRET_LEAK] sink clearance violated: argument 0 of media_save in pattern KitchenCam carries label 'private, trusted'; sinks require public`.
- No stubs: grep `todo!`/`unimplemented!`/`SKELETON` — 0.
