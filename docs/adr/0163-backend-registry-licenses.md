# ADR-0163: Backend registry — classes, SHA-pin contract, license classes and the distribution gate

**Status:** Accepted
**Date:** 2026-09-16
**Naryad:** №333 (issue #460, Волна 2 · Фаза 2)
**Depends on:** №316 (classification SSOT — table format precedent), №325 (profile bridge precedent), №261/№130 (SSRF allowlist precedent), `MODEL_WEIGHTS_UNSAFE` (audit.rs — Category-A gate precedent)
**Blocks:** №334 (real backends — the SHA-pin path), №336 (degradation ladder)

## 1. Context

Wave 2 builds real STT/omni/vision-understanding backends (№334) on top of the Wave-1 lattice. Today the tree knows model NAMES (`KNOWN_VOICE_MODELS`, `KNOWN_VISION_MODELS`, plan §15 canon names) but there is no registry entity that says what a backend IS: its class, which weights artifact backs it, what hash pins those weights, and under which license they may be distributed. Distribution (shipping a program/image that pulls non-OSI weights) is currently ungoverned.

## 2. Decision

### 2.1 The registry: a static SSOT table (`src/backends.rs`)

`BACKEND_REGISTRY: &[BackendEntry]` — spec!-style static table, one entry per backend:

- `name` — the program-visible backend identifier;
- `class` — `STT | TTS | Omni | VisionUnderstanding | LLM` (the §7.6 MDL classes);
- `weights_id` — the weights artifact identifier a program references (also the gate's matching key);
- `pin` — `ShaPin::Pinned(&'static str)` (the expected SHA-256 of the weights artifact, verified at fetch/load time) or `ShaPin::PendingNo334` — an explicit TYPED boundary: the weights artifact is not vendored in-tree (real-weights runs are PARKED by hardware, №294), so there is NO hash to state. Fabricating a hex string would be a lie; a type-level variant is loud, total (exhaustive match), and grep-visible. №334 replaces every `PendingNo334` with a real pin as its SHA-pin path lands, and its contract REFUSES loading `PendingNo334` entries.
- `license` — `Osi | NonOsi | Restrictive` with a `license_note` naming the license and its basis.

Seed entries (the list the tree and plan §15 already name — reported loudly here, as the issue requires; legal fine-reading of specific licenses is explicitly OUT of scope, classes only): `chatterbox-multilingual-v3` (TTS, MIT → osi), `koko-ro-82m` (TTS, Apache-2.0 → osi), `z-image-turbo` (VisionUnderstanding, Apache-2.0 → osi), `molmoact2` (VisionUnderstanding, Apache-2.0 → osi), `wall-oss-0.5` (Omni, license NOT verified in-tree → **restrictive by default-deny** — the MODEL_WEIGHTS_UNSAFE allowlist posture: unverified = forbidden until a license record lands), `nemotron-3-nano-omni-30b-a3b` (Omni, NVIDIA Open Model License → **non-osi** — the MDL-3 test case).

### 2.2 The distribution gate: `BACKEND_LICENSE_DISTRIBUTION` (Category-A)

Programs reference backends by identifier; the gate matches identifiers at STRING-LITERAL positions (the `MODEL_WEIGHTS_UNSAFE` literal-URL precedent — you cannot use what you never name, and naming is always a literal somewhere) plus the `vision { model: … }` declaration field. For every matched entry with `license != Osi`:

- **default (distribution):** `Severity::Error`, check_id `BACKEND_LICENSE_DISTRIBUTION`, message names the weights id, the license class, and the license note — compile refusal;
- **explicitly permissive profile:** `Severity::Info` audit event (the loud bridge — usage is ALLOWED but never silent, mirroring `profile legacy`'s audit-event discipline).

Restrictive entries clear NOTHING in either mode beyond the Info event — wait, they do: the profile unlocks non-osi AND restrictive together (both are "not OSI-approved"; the distinction is informational, carried in the message).

### 2.3 The profile bridge: `profile licensing { backends: permissive_with_audit }`

Precedent: `profile legacy` (№325/ADR-0161) — a program-level declaration, validated loudly (unknown names/options are semantic errors), one per program per gate. `licensing` unlocks ONLY the license gate; the №325 sink clearance and every other Category-A check stay strict. The two profiles are independent flags (`ResolvedProfiles`), a program may declare both. Lifecycle note (ADR-0161 spirit): `licensing` is a bridge for develop/distribute-offline workflows — distribution images that ship non-OSI weights must carry the profile declaration in the shipped source, so the usage is auditable text, not a config default.

## 3. Alternatives considered

- **Fabricated SHA-256 pins** to satisfy "every entry carries a pin" literally — rejected: a made-up hash is worse than a typed pending marker; the DoD is honored by the SCHEMA (every entry carries a pin FIELD) plus the loud, greppable boundary.
- **Gating only builtin call-sites that take model args** — rejected: the current builtin surface barely takes model ids (vision decls do), and the surface will grow in №334–№336; literal-position matching is backend-agnostic and cannot be bypassed by renaming a variable.
- **A separate `distribution.toml` config** — rejected: profiles are per-PROGRAM source text (auditable, versioned with the code), the №325 precedent.

## 4. Consequences

- №334 (real backends) MUST: load only registry entries, verify `Pinned` hashes at fetch/load, refuse `PendingNo334`; new backends join the registry, never bypass it.
- №336 (degradation ladder) reads `class` for ladder ordering.
- The seed license classes are claims with named bases in `license_note`; a legal pass may reclassify an entry — that is a one-line table edit, loudly diffable.
- `backend_list()` exposes the registry to programs (metadata only) and to the REFERENCE.

## 5. Verification

- `tests/naryad_333_backends.rs`: registry schema (every entry: class + id + license + note; pin either real or PendingNo334), gate red (distribution refuses non-osi/restrictive with license named), green (osi entries pass), the `licensing` bridge downgrades to Info audit events, `w1_license_gate` fixture contract, `backend_list` builtin contract, no-stubs.
- `examples/w1_license_gate.mlog` + `.error` — the MDL-3 scenario: a Nemotron reference does not compile under the default profile.
