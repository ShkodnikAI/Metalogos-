# ADR-0218: Reproducible Build & Signed Package Registry — the Registry Package Path as a Bit-for-Bit Verifiable Artifact (the 8E-2 / C7+C12 Subphase Record)

- Status: **Proposed** (the ACCEPTED path is the wave-82 acceptance —
  the dispatch gh#1817 issues the subphase's naryads the same week; the
  wave acceptance fixes the ADR status as Accepted)
- Date: 2026-11-05
- Deciders: the OWNER (the R38 train — 0.45 = 8E-2 (C7+C12,
  REPRO_BITFORBIT 10000 bp; C7 strictly by trigger — the first
  gov/bank pilot); the wave-82 dispatch gh#1817; the «Волна 81
  исполнена, заводи следующую» directive of 2026-11-05)
- Naryad: №909 (issue #1811, Wave 82 — W-K; ADR before the first code of
  the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 Phase 8 MASTER-PLAN (the office canon,
  `language_strategy_plan_v2.md`, revision R38 — the C7+C12 line); the
  R60 GET-only fact-check over live main `8bcee429` (2026-11-05)
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — 0.45 is this record's train slot),
  [ADR-0019](0019-mlogpkg.md) (the mlogpkg charter — the registry this
  record signs), [ADR-0167](0167-action-ledger-v1.md) (the Ledger/DSSE
  contour — adjacent, NOT duplicated: the CLI is not the runtime
  executor), №828 (the digest-pinning form — the pin-the-digest
  contract reused verbatim), №475 (the mlogpkg trust domain — the CLI's
  refusals stay CLI errors), №618 (the kamerton repro corpus — prior
  art), R38(8) (the C7 trigger — verbatim in §2.4)

## §1. Context — the registry is anonymous; the toolchain repro is trigger-gated

Live main `8bcee429` carries a package manager with NO integrity chain:
`mlogpkg` (ADR-0019) serves `init/add/build/info/audit` over the local
registry `~/.mlog/registry/<pkg-name>/`, the lockfile is deterministically
sorted (the №198 test lives), the advisory DB is an honest manual file
(№198, the header states the limits) — and `pack`/`sign`/`verify`/
`keygen` are ZERO hits. A tampered package in the registry builds
silently: there is no digest contract in `mlog.toml [dependencies]`, no
signature anywhere, nothing to verify.

The release side (C7's live face) is stronger: `.github/workflows/
release.yml` attaches the binary, a CycloneDX SBOM built for exactly
that binary, SLSA build-provenance and SBOM attestations signed through
GitHub's Sigstore keyless integration («no human-held signing key»,
release.yml:14), and SHA256SUMS. The kamerton cycle (№618) proved the
SHA256-re-run form on a real release (v0.29.0) — the verification form
is not hypothetical.

The R38 train defines the 0.45 subphase fact as REPRO_BITFORBIT 10000 bp
and pins the frame (R38(8), verbatim): C7 — жёстко по триггеру (первый
гос/банк пилот; Nix не открывать раньше). The honest resolution of the
tension: the fact counts what IS verifiable today and every day in CI —
the registry package path and the release digest chain — and NOT the
bit-for-bit toolchain build, which stays behind the trigger. This record
fixes that definition before any code lands (the №891/№903 form: the ADR
fixes the definition, the naryad №912 implements the machine).

## §2. Decision — the fact is the verified share of an explicit reproduction roster

**REPRO_BITFORBIT := (the verified roster carriers / all roster
carriers) × 10000 bp.**

1. **A carrier** is a live, CI-checked chain «re-derive → compare
   byte-for-byte → refuse on divergence»: the pack-twice byte identity
   of a package's canonical digest document, the lockfile determinism
   (the live №198 test), the digest-verify tamper refusals, and the
   release-workflow wiring literals (the SHA256SUMS recompute, the SBOM
   for-exactly-that-binary binding, the Sigstore keyless attestation).
   The roster is the explicit registry `scripts/ci/gate_043_goals.txt`
   (the №862/№894 form): each row carries (i) the quoted literal from
   the live wiring source and (ii) the alive composition site — the
   count is CHECKED, never declared.
2. **The signature covers the digest** (the №828 digest-pinning form):
   `mlogpkg pack` builds the canonical digest document of the package
   tree (JSON, sorted keys, LF, no timestamps); the package digest is
   sha256 of its bytes; `sign` is HMAC-SHA256 over the 32 digest bytes
   (hmac 0.13 + sha2 0.11 are already in the workspace graph — Cargo.lock
   grows by ZERO entries); `verify` re-derives the document from the
   tree, compares byte-for-byte, and recomputes the MAC in constant
   time. The trust anchor of this phase is the explicit verification-key
   pin in `mlog.toml [registry]`; PKI / Web-of-Trust is out of phase
   (§5).
3. **The contour boundary:** the registry refusals are mlogpkg CLI
   errors (the №475 trust domain — argv/config-driven, the same class
   as the compiler's import loader). They are NOT new language refusal
   classes; the frozen №385 vocabulary does not grow; the language
   surface — grammar, builtin registry, golden outputs — is a zero
   diff.
4. **The C7 trigger contract (R38(8), verbatim):** «C7 — жёстко по
   триггеру (первый гос/банк пилот; Nix не открывать раньше)». The
   bit-for-bit toolchain lane (and Nix) opens ONLY at the first
   gov/bank pilot. Before the trigger this wave builds nothing on the
   toolchain path; the fact's scope is the registry package path plus
   the release digest chain. A future widening of the roster (a
   toolchain carrier) can only RAISE the denominator together with the
   verified numerator — the floor never falls (№841 form).

## §3. The parents are consumed by reference — what this record does NOT redefine

- The mlogpkg trust domain and its CLI surface (№475/ADR-0019) —
  extended by new subcommands; `init/add/build/info/audit` and their
  tests are untouched (the 1:1 migration discipline).
- The digest-pinning form (№828, `compliance_pack.rs`) — reused as the
  pin-the-digest dependency contract; the G5 row-digest lineage is
  acknowledged, not redefined.
- The release workflow — consumed as wiring literals (checked strings
  in the live source), not re-executed: CI does not download release
  assets; the kamerton precedent (№618) remains the owner-forwarded
  external verification form.
- The machine-fact discipline — the established pattern (№735: the
  fact rides with its checker; №841/№862/№894: the roster + tamper
  traps + the only-up floor; the double `--check` determinism).
- The Ledger/DSSE contour (ADR-0167) — the forensic journal of ACTIONS
  inside the language runtime; the registry path journals nothing into
  the Ledger (a CLI is not the runtime executor). Tamper events on the
  package path are loud CLI refusals, not Ledger rows; a future
  runtime-side consumption of signed packages would ride the Ledger —
  out of this subphase.

## §4. The consumption contracts — what №910–№914 land

1. **№910 — the signed package kernel.** `mlogpkg pack` (the canonical
   digest document; re-pack of the same tree is byte-identical),
   `mlogpkg keygen` (32 bytes from /dev/urandom, hex, 0o600), `mlogpkg
   sign --key` (HMAC over the digest; the signature field is excluded
   from the hashed document — a signature never signs itself), `mlogpkg
   verify --key` (fail-closed: tree↔document byte comparison, digest↔
   signature constant-time recomputation; every divergence names
   itself). The negative controls: a flipped source byte, a foreign
   key, a stripped signature, a manifest edit — each is a named
   refusal.
2. **№911 — the coverage rule.** The dependency contract grows the
   digest pin: `mlog.toml [dependencies] pkg = { version = "…",
   digest = "<64 hex>" }` (the №828 form). `mlogpkg build` resolves,
   then for EVERY dependency re-derives the digest document from the
   registry tree, compares byte-for-byte with the pin, verifies the
   signature when the tree carries one and a `verify_key` is pinned,
   consults the advisory DB (№198) strictly AFTER the integrity pass,
   and only then writes the lock and runs the semantic check. The
   happy path journals nothing; every refusal fires BEFORE the effect
   (the №874/№845 form).
3. **№912 — the machine fact.** `repro_bitforbit()` in gen_metrics.py
   + gate_043 (the floor 10000 bp — the wave's dispatch constant, the
   only-up floor; the tamper traps in the №906 form, the honest
   Trap-37 strict-drop shape where a per-row drop is mathematically
   unreachable; the GENERATED rows in `docs/metrics.md` and
   `docs/PLAN-SUMMARY.md`; the double `--check` run deterministic).
4. **№913 — the C7 trigger-readiness report.**
   `docs/audits/repro-readiness-2026-11.md` (the №907 form): the roster
   table with the machine verdicts, the release chain audited from the
   workflow source, the trigger contract verbatim, the honest limits
   (§5); the limitations/threat-model sync.
5. **№914 — the ride.** «The floor follows the fact»: the full
   surface-gate verification on the wave merge chain; NO DRIFT expected
   (the wave touches the mlogpkg crate, the metrics machine and docs —
   zero registry rows grow; the counters read the REGISTRY, not the
   module tree).

## §5. Honest limits — what REPRO_BITFORBIT is NOT

- **Not the bit-for-bit toolchain build.** That line stays trigger-
  gated (R38(8)); the runner images drift, the provider weights
  deprecate (honest-limits №588), and no CI run re-builds the
  toolchain twice. Reading the 10000 bp as «the build is reproducible»
  would be a lie; the №913 report states the scope in the first
  section.
- **Not authenticity.** The pinned-digest / pinned-key model is an
  integrity chain, not a PKI trust web: whoever pins the digest vouches
  for it. A compromised producer machine that pins a tampered digest
  defeats the pin; the defense is the pinned VERIFICATION key + the
  signature, and the honest threat-model row says exactly this.
- **Not tuned to green.** The roster never shrinks to pass: a removed
  carrier, a renamed wiring literal, or a deleted composition site
  lowers the share below the floor — the gate working, not a pass.
  The corpus is not adjusted to make a run green (the №476 preamble
  rule in its registry face).
- **Not the external audit.** No external institutional audit is
  procured by this wave; the kamerton form (№618) stays the
  owner-forwarded external channel.

## §6. The migration discipline — 1:1, byte-identical surfaces

The wave touches the mlogpkg crate (new subcommands + the build-order
contract), the metrics machine, and docs. The grammar, the builtin
registry, the golden outputs, the language runtime modules —
byte-identical (the №687 counter reports zero surface growth; the №914
ride verifies no drift). The existing mlogpkg subcommands and their
tests run unmodified; `Cargo.lock` grows by zero entries (hmac/sha2/hex
already in the workspace graph — the supply-chain surface does not
move).

## §7. Acceptance

- ADR-0218 Proposed → Accepted at the wave-82 acceptance (the dispatch
  gh#1817 records the verdict).
- The gate 0.45: `gen_metrics --check` green with REPRO_BITFORBIT =
  10000 bp on the wave merge chain; the tamper traps alive; the double
  `--check` deterministic.
- The pack-twice byte identity and the full negative-control set green
  on both the unit and the integration path; `init/add/build/info/
  audit` untouched (the №198 suite green without edits).
- The C7 trigger-readiness report present with regenerable numbers
  (every number is regenerable by the machine — no prose numbers); the
  limitations/threat-model rows synced with §5 verbatim.
