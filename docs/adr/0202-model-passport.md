# ADR-0202: ModelPassport — the Passport of a Model as a Typed Artifact (the License-Typed Model Layer, the 8D-1 Subphase Record)

- Status: **Accepted** (the owner decision of 2026-10-28 — the plan
  revision R38 accepted in full, C4 "License-Typed Model Layer" is part
  of the accepted Phase 8 line; the wave-71 directive issued the
  subphase's naryads the same day; the wave acceptance fixes the ADR
  status as Accepted)
- Date: 2026-10-29
- Deciders: the OWNER (the R38 revision — C4 with ModelPassport as the
  first code subphase; the wave-71 dispatch gh#1645)
- Naryad: №837 (issue #1639, Wave 71 — W-A; ADR before the first code
  of the subphase — the ADR-0201 §2.4 rule)
- Source: the strategy plan §23 (the office canon,
  `language_strategy_plan_v2.md`, revision R38); the R46 GET-only
  fact-check over live main `d40aa28`
- Related: [ADR-0201](0201-phase8-verifiable-compliance-trust.md) (the
  Phase 8 through-line — this is the first subphase record of its
  booking), [ADR-0163](0163-backend-registry.md) (the license classes
  and the SHA-pin contract this passport inherits verbatim),
  [ADR-0164](0164-perception-ast.md) (the origin chain — the passport's
  origin link reads the same lineage), [ADR-0197](0197-vm-single-executor.md)
  (the VM lane is not touched here), №828 (the G5 digest-pinning
  pattern — the fail-closed hygiene template), №838–№842 (the code
  naryads that land on top of this type)

## §1. Context — the model facts are smeared across two holders

ADR-0163 (№333) introduced the license classes `Osi` / `NonOsi` /
`Restrictive` as **claims with named bases**: every backend-registry
entry carries `license` + `license_note`, unverified licenses are
restrictive by default-deny, and the SHA-pin contract binds every
weights reference to an expected hash. The posture is sound, but the
facts about ONE model live in TWO holders and nowhere as one object:

- `src/backends.rs` — `BackendEntry` / `WeightsFile` / `WeightsSource`:
  the registry (class, `license_note`, SHA-pin, the per-file weights
  manifest of №334).
- `src/vision/weights.rs` — `WeightsManifest` / `WeightsManifestEntry`:
  the SHA-256 manifest discipline for local weights directories; no
  license field at all (verified: `grep licen` — 0 hits in the file).

And three facts a consumer needs do not exist anywhere: the **freshness**
of the record (when was this model last verified against its source),
the **revocation** state (is this passport still trusted), and a single
**identity** a compiler/audit lane could read instead of grepping two
files. A passport as a type does not exist on main (`model_passport` —
0 code hits; the word surfaces only in examples/docs, including the
leak-example n24 of ADR-0161 and ADR-0216's prose).

The G5 digest-pinning (№828, merged `694e0a2`) supplies the mechanics
template: a digest over a deterministic representation, re-pin only by
an explicit act, **fail-closed** on mismatch — proven on the MCP
tool-schema line and ready to reuse.

## §2. Decision — the passport is a type, and it is the single source of truth

1. **`ModelPassport` is one typed object per model.** Mandatory fields
   (every one required — a passport without any of them does not
   construct, loud `Result` error):
   - `identity` — the registry identifier (the weights id, the same key
     the №333 gate matches);
   - `digest` — the SHA-256 pin this passport vouches for (the ADR-0163
     contract: hex, lowercase; entries without an artifact pin carry the
     record digest of the passport's own fields — tamper-evidence
     without inventing a hash, the "never a fabricated hash" rule);
   - `license` — the class `Osi` / `NonOsi` / `Restrictive` (the
     ADR-0163 enum reused verbatim — no new taxonomy);
   - `license_note` — the license and its classification basis (the
     ADR-0163 one-line rule);
   - `version` / `source` — the model version and the origin source
     (the HF repo id or the in-tree statement);
   - `verified_at` — the **verification date** (the freshness anchor);
   - `origin` — the origin-chain link (the Phase-2-semantics lineage —
     ADR-0164's provenance posture applied to model records).
2. **The passport is the single source of truth about the model for
   compilation and audit.** Consumers (the profile rule of №839, the
   fact machines of №841, the hygiene of №842) read the passport — they
   do not re-derive license facts from files. The static passport table
   is keyed 1:1 to the registry (arity tested both directions); the
   runtime weights manifest reads the same passport through the same
   key instead of carrying a second copy.
3. **The profile binding (mechanics — №839).** A program may reference
   a `NonOsi`/`Restrictive` backend only when the program's profile
   explicitly permits that license class. Absence of the permit = only
   `Osi` is allowed. The refusal is loud, before run, with a dedicated
   error code. The default-deny posture (unverified → restrictive,
   `MODEL_WEIGHTS_UNSAFE`) is not weakened and not re-defined — the
   named permit sits on top of it, never instead of it.
4. **The hygiene rides the passport (mechanics — №842, the №828
   pattern).** Freshness (version + verification date), fail-closed
   `passport digest mismatch`, re-pin only by an explicit update (new
   digest + new verification date in one act), revocation as a
   consumption refusal. The passport is the object all of this operates
   on — without the type there is nothing to pin, nothing to revoke,
   nothing to date.
5. **Serialization out of scope here.** №838 lands the internal type
   and the 1:1 migration with zero behavior growth; passport files and
   release-manifest serialization are future work outside the 8D-1
   slice.

## §3. What this record does NOT decide

- The profile-rule mechanics (the option spelling, the site set, the
  error code): №839 fixes them and names the choice in its report.
- The freshness threshold and the revocation flow: №842 fixes them
  (the wave mode constant) and the report names the choice.
- The legal fine-reading of specific licenses: OUT — the ADR-0163
  boundary is preserved verbatim (classes and named bases here; no
  license texts, no legal analysis in the tree).
- The floor targets of the 0.35 machine facts (LICENSED_FLOOR,
  PASSPORT_FRESH): the owner fixes them at the subphase issuance (the
  wave dispatch's acceptance section).
- The market sections of the plan (§16.0-7): not published — the
  ADR-0201 §5 sanitization boundary applies to this record too.
