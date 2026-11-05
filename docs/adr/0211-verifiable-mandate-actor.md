# ADR-0211: The Verifiable Mandate Actor — a Program Cannot Approve Itself (the 8B-2 Continuation; ADR Before the Code)

- Status: **Proposed** — the owner's variant choice is the acceptance
  gate (№901, issue #1795: the code, the tests and the attestation-map
  row land AFTER the owner picks the variant; the ADR-0201 §2.4 rule —
  ADR before the code)
- Date: 2026-11-05
- Deciders: the OWNER (the N4 decision of 2026-11-04 — issue #1795; the
  variant choice closes this ADR as Accepted and opens the code naryad)
- Naryad: №901 (issue #1795; the ADR booking 0211–0215 — the first free
  number)
- Source: the №861 typed-approval surface (`src/builtins/approval.rs` —
  `approval_issue`/`approval_revoke`; the gate in
  `src/approval.rs::authorize_overseen_call`); ADR-0205 §5 (the honest
  «not a signature» record); the APPROVAL_COVERAGE machine (№862,
  gate_036, the 10000 bp only-up floor)
- Depends on: ADR-0205 (the typed mandate carrier), ADR-0203 §2.7 (the
  capability layer), ADR-0186 §4.5 (the publication/owner-act posture —
  the trust anchor stays OUTSIDE the machine), №335 (the consent-ledger
  process-local store)
- Blocks: the APPROVAL_COVERAGE verified mode (the gate_036 floor keeps
  its 10000 bp only when the counted mandates are actor-verified)

## 1. Context — the fact (checked, live main)

`approval_issue(actor, target, ttl_seconds)` is a normal builtin in the
registry (the append-only block, the `spec!("approval_issue", 3, …)`
row). Its `actor` argument is any non-empty String — the runtime
records the mandate with the actor name THE PROGRAM PASSED. A `.mlog`
program can therefore issue a mandate that names any human, over any
target it can spell, and the fail-closed approval gate
(`authorize_overseen_call`) will honor it — the gate verifies the
window, the target digest and the revocation state, but NOT the ORIGIN
of the decision.

ADR-0205 §5 records this honestly: the typed mandate is a CARRIER of
the consent decision, not a signature — nothing in the №861 line
verifies that a human was involved. The APPROVAL_COVERAGE machine fact
(gate_036; the plan §23 floor 10000 bp) counts the human-oversight
roster carriers covered by the composed gate — it does not and cannot
distinguish «a human approved» from «a program wrote a plausible
name». On the live tree the roster is `cron_run` (1 carrier) and the
coverage reads 10000 bp (1/1) — with the actor being whatever the
program typed.

The exposure: any program that can call `approval_issue` can grant
itself the human-oversight consent for `cron_run` (and for every
future carrier the HUMAN_OVERSIGHT registry transaction opens). The
oversight layer exists; its trust anchor does not. This ADR closes
that hole at the trust-anchor level and leaves the carrier, the gate
and the Ledger surface untouched.

## 2. The variants (the owner's choice menu)

The three variants below are the issue #1795 menu, compared over five
dimensions: the trust anchor, what the program can still do, the
runtime surface change, the verified-mode path for `cron_run`, and the
honest limits.

### 2.1 Variant (а) — the mandate SIGNED by the owner's key, the runtime verifies

The human signs the mandate OUTSIDE the program (an Ed25519 key held
in the host environment — never in `.mlog`, never in the repo); the
mandate carries the signature; the runtime verifies it at gate time
(the verify is a pure function over the typed carrier — the
`actor_verified=true` class is born signed).

- Trust anchor: the owner's key (a real cryptographic anchor; the
  runtime trusts the key, not a string).
- The program can still: call `approval_issue` — but its mandates are
  unsigned and (per §3, the common mechanics) never counted.
- Runtime surface: a signature-verify step in the approval store/gate
  + a new typed field on the carrier (`signature`, `actor_verified`) +
  the Ed25519 dependency in the runtime. The verification code is
  security-critical surface in-process.
- The cron_run verified path: the signed mandate is loaded from the
  host, the gate verifies the signature, the carrier runs.
- Honest limits: the key management is a new owner burden (generation,
  rotation, revocation); the runtime carries crypto code (a dependency
  + an attack surface that today it does not have); the signature
  proves WHO approved, not THAT the approval was informed — the same
  is true of every variant.

### 2.2 Variant (б) — the mandate is created ONLY outside the program (RECOMMENDED)

The issuance moves OUT of the program's reach: a host-side CLI
(`mlog approve …`) or an HTTP endpoint of the host with a secret; the
program can only READ the mandate store (the read side already exists
— the gate reads the store at call time). The `approval_issue`
builtin either retires from the counting path or stays for
local/development use with the never-counted marking (see §3 — the
two mechanics compose).

- Trust anchor: the HOST BOUNDARY — the same posture as the
  publication authority (ADR-0186 §4.5: the machine never publishes;
  here: the machine never approves). No new crypto in the runtime; no
  new dependencies.
- The program can still: read mandates, compute over them, PASS them
  around — but it cannot create a counted one. The `cron_run` verified
  mode reads host-issued mandates only.
- Runtime surface: minimal — a flag on the store entries
  (`actor_verified`) set by the ISSUANCE PATH, not by the program; the
  CLI/endpoint is host tooling (outside the `.mlog` reach by
  construction). The gate adds one check: the counted class requires
  `actor_verified=true`.
- The cron_run verified path: the owner runs `mlog approve
  carrier:cron_run --actor <name> --ttl …` on the host (or POSTs to
  the endpoint with the secret); the mandate lands in the store
  verified; the gate counts it.
- Honest limits: the host secret/CLI is the trust anchor — whoever
  holds the host holds the approval (the same boundary the publication
  authority already trusts); the multi-host/remote contour (a mandate
  issued on machine A honored on machine B) needs a transferable proof
  — that is variant (а)'s layer, RIDING on (б) when the contour
  appears, not before.

### 2.3 Variant (в) — the marking only: `actor_verified=false` never counts

The builtin stays as is; every program-issued mandate is marked
`actor_verified=false`; the APPROVAL_COVERAGE machine counts ONLY the
verified class.

- Trust anchor: none by itself — the marking is bookkeeping; the
  verified class is EMPTY until an issuance path outside the program
  exists (i.e. (а) or (б) lands). As a STANDALONE choice it reds the
  gate_036 floor (the coverage drops to 0/N against the 10000 bp
  only-up floor) and leaves no legal path back to a counted mandate.
- As the TRANSITIONAL mechanic inside (б): exactly right — the builtin
  keeps working for development/local flows, its mandates are honestly
  marked never-counted, and the coverage counts the host-issued class
  only. The №898-form honest posture: the number is checked, not
  declared.
- Runtime surface: the smallest (one field, one filter in the machine).
- The cron_run verified path: none, until (б)'s CLI/endpoint lands.

### 2.4 The comparison

| Dimension | (а) signed mandate | (б) out-of-program issuance | (в) marking only |
|---|---|---|---|
| Trust anchor | the owner's Ed25519 key | the host boundary | none (bookkeeping) |
| A program can self-approve | no (unsigned never counts) | no (no path to a counted mandate) | no — but no counted mandate exists at all |
| New runtime surface | crypto verify + dependency | a store flag + a host CLI/endpoint | a flag + a filter |
| New owner burden | key management | the approve ritual per mandate | none |
| The gate_036 floor after landing | stays 10000 bp (the signed class) | stays 10000 bp (the host-issued class) | RED until (а)/(б) |
| The multi-host future | native | needs (а) riding on top | n/a |
| The honest-limit family | crypto in-process | the host is the boundary (the ADR-0186 posture) | the empty verified class |

## 3. Recommendation

**Adopt (б) — the out-of-program issuance — with (в)'s marking as the
implementation mechanic.** The reasons, in the ADR-0186 §4.5 lineage:

1. **The trust anchor stays outside the machine.** The owner's
   decision (approve) and the owner's act (publish) get the same
   posture: the machine prepares and verifies, the human acts outside
   the program's reach. No new crypto enters the runtime, no new
   dependency, no new in-process security-critical surface — the
   smallest trusted-computing-base delta of the three.
2. **The fail-closed direction is structural.** A program cannot forge
   its way to a counted mandate because the counted class is born
   outside its reach — not because a check it can influence says no.
   The (а) verification is strong too, but it moves the anchor into
   the process the program runs in; (б) never lets the question arise.
3. **(в) alone is a dead end** (an empty verified class reds the
   floor), but as the marking mechanic it gives the migration its
   honesty: the program-issued mandates keep existing (the development
   flows do not break), they are marked never-counted, and
   APPROVAL_COVERAGE counts only what a human outside the program
   issued. The coverage number becomes a checked fact about HUMANS,
   not about strings.
4. **(а) remains the strengthening lane**, not the starting point: the
   signature layer rides on (б)'s verified store when the
   multi-host/remote contour demands a transferable proof (the first
   gov/bank-pilot class decision — the C7-trigger posture of
   ADR-0218 §5; the trigger, not a schedule).

The contract the code naryad implements (after the owner's choice):
the store entries carry `actor_verified`; the builtin-issued mandates
are always `false`; the counted class (the APPROVAL_COVERAGE
numerator, the gate_036 floor) is the verified class only; the host
CLI `mlog approve <target> --actor <name> [--ttl S]` issues verified
mandates (the endpoint form rides the host contour decision); the
test mutation — `approval_issue` from `.mlog` does NOT pass the
`cron_run` gate in the verified mode (the «Сделано, когда» form of
issue #1795); the attestation map gains the «actor verification» row.

## 4. Consequences

- The APPROVAL_COVERAGE fact becomes a HUMAN-coverage fact: the
  numerator counts host-verified mandates; a program can no longer
  inflate it (the mutation test pins the negative).
- The `approval_issue` builtin keeps its signature (no BREAKING row —
  the existing programs keep running; their mandates simply never
  count toward the oversight coverage — the honest downgrade of an
  inflated number is the point, not a regression).
- The host gains one tool (`mlog approve`) and one ritual (the owner
  approves the human-oversight carriers explicitly). The consent
  ledger's process-local posture (№335) is unchanged; the persistence
  of the verified store across restarts is the host contour's
  decision, recorded when the code naryad lands.
- The gate_036 floor semantics tighten: the 10000 bp only-up floor is
  read over the verified class once the code lands; the landing PR
  carries the floor re-pin if the roster arithmetic changes shape
  (the «the floor follows the fact» form).

## 5. The honest limits

- The host boundary is the trust anchor: a compromised host issues
  verified mandates in the owner's name. This ADR does not defend the
  host — it removes the PROGRAM from the trust path (the same boundary
  the publication authority already accepts).
- The signature variant (а) adds nothing against a compromised host
  either (the host holds the key); its value is the TRANSFERABLE proof
  (the multi-host contour), not a stronger local anchor.
- The approval decision's INFORMEDNESS (did the human understand what
  they approved) is outside every variant's reach — the typed carrier
  records THAT a decision was made, by whom, over what, until when;
  the №861 §5 record stands.
- The verified store's persistence and the endpoint's auth contour are
  the code naryad's surface (recorded there, not here — the ADR pins
  the trust anchor and the contract, the implementation pins the
  mechanics).
