# Action Provenance Ledger — Public Draft (profile over in-toto / W3C PROV)

**Status:** Public draft — circulated for external review (liaison, not an announcement)
**Date:** 2026-09-18
**Editor:** METALOGOS project (ShkodnikAI)
**Repository:** https://github.com/ShkodnikAI/Metalogos-
**Native spec:** [ADR-0167 — Action Ledger v1](../adr/0167-action-ledger-v1.md) · [ADR-0157 — PROV/in-toto alignment](../adr/0157-ledger-profile-prov-intoto.md)
**Implementation:** `src/ledger.rs`, `mlog ledger verify|archive`, `ledger_export*` builtins
**Worked corpus:** a real 20-record agent-office chain (§8) and a 10 000-record CI golden (§9)

---

## 1. Executive summary — a profile, not a new format

AI agents increasingly perform consequential actions: running SQL, calling
LLM providers, pushing to version control, sending messages. Projects that
govern those actions need an **action provenance trail**: who did what, when,
against which scope, and how we can later PROVE the trail was not altered.

The Metalogos Action Ledger v1 is such a trail: an append-only journal of
agent actions where every record is hash-linked to its predecessor and
individually signed with Ed25519, with explicit key-rotation and snapshot
records for long-lived deployments.

This document positions the format honestly:

- **It is a profile over existing standards, not a new format.** The
  in-toto profile (§4) emits stock ITE-5 statements with a fixed
  `predicateType`; the PROV profile (§5) is a linear activity lineage in
  PROV-JSON with one declared custom namespace. Nothing in the profiles
  requires the consumer to trust Metalogos-specific code to READ the trail.
- **It is executable and verified today**: the chain rules, the signatures,
  and the rotation seams are checked by an external verifier (`mlog ledger
  verify`) that has no runtime dependency on the Metalogos interpreter.
- **It is exercised by a real corpus**, not by toy fixtures: §8 shows
  records from an actual agent-office run (grant issuance, granted
  destructive SQL, a runtime deny, key rotation, snapshot); §9 documents a
  10 000-record golden that must sign and externally verify in under 10 s
  on CI.

We are publishing this draft to ask the in-toto/PROV/supply-chain communities
whether the profile is on the right side of their conventions — see §10
(the liaison request) and §11 (responses).

## 2. The record model (native JSONL)

One JSON object per line; the chain order is the stream order. Every field
is metadata or a hash — **payloads never enter the journal** (§6).

| Field | Type | Meaning |
|---|---|---|
| `seq` | u64 | 1-based, gap-free monotonic sequence |
| `ts` | u64 | unix seconds at the write (integer-only journal) |
| `kind` | enum | `action` \| `key_rotation` \| `snapshot` |
| `actor` | string | the acting scope (program, tool call, human identity) |
| `action` | string | the action verb (`grant.issued`, `grant.used`, `irreversible.db_execute`, `deny.<REASON>`, `session.create`, `ledger.rotate`, …) |
| `scope` | string | the guarded resource the action touched |
| `args_hash` | hex64 | SHA-256 over the action's detail tuple — the content binding; the preimage never leaves the process |
| `prev_hash` | hex64 | the previous record's `hash` (genesis: 64 zeros) |
| `key_id` / `pubkey` | hex | the signer that signed THIS record |
| `hash` | hex64 | SHA-256 over the canonical body (§2.1) |
| `sig` | hex | Ed25519 signature over the canonical body |

Rotation records additionally carry `new_key_id`/`new_pubkey`; snapshot
records carry a `seq` anchor for archival truncation.

### 2.1 Canonical body and chain rules

The canonical body is the JSON serialization of every field EXCEPT `hash`
and `sig`, with a fixed key order — `hash = SHA-256(canonical_body)`, and
`sig = Ed25519_sign(signer_key, canonical_body)`. The chain rules checked
by the external verifier (ADR-0167 §3.2):

1. `seq` continuity from 1, no gaps;
2. `prev_hash` linkage — each record commits to its predecessor;
3. `hash` recomputation over the canonical body;
4. per-record signature validity under the record's `pubkey`;
5. **signer continuity** — a rotation record is signed by the still-active
   key, names `new_key_id`/`new_pubkey`, and the following records must be
   signed by the new key (a mid-chain signer substitution is a loud
   verification failure);
6. snapshot anchoring — an archive truncation (`mlog ledger archive`) must
   cut at a snapshot record and re-verify the truncated chain.

## 3. What writes the journal (no "also log it" calls)

The journal is written as a SIDE EFFECT of the governed action paths
themselves — there is no separate "log this" call the program can forget:

- grant lifecycle: `grant.issued / granted / used / revoked` (inside the
  grant ledger's event recorder);
- runtime deny events: `deny.<REASON>` — written BEFORE handler selection,
  so a handled denial is still journaled;
- successful irreversible actions: `irreversible.db_execute` (post-success,
  post-consumption);
- HTTP session lifecycle: `session.create / destroy` (server feature);
- operator events: `ledger.rotate`, `ledger.snapshot`.

## 4. The in-toto profile (ITE-5 statement per record)

`ledger_export_intoto(path)` emits one ITE-5 statement per record (FILE
EGRESS — the builtin is classified `Sink` by the language's own №316
classification, which is itself journaled in the deny taxonomy):

```json
{
  "_type": "https://in-toto.io/Statement/v0.1",
  "subject": [{ "name": "metalogos:action:<action>@<seq>",
                "digest": { "sha256": "<args_hash>" } }],
  "predicateType": "https://metalogos.dev/attestations/action-ledger/v1",
  "predicate": {
    "seq": 2, "ts": 1789691215, "kind": "action",
    "actor": "db_execute_with_grant: DELETE FROM notes WHERE user = 'alice'",
    "action": "grant.used", "scope": "db:delete:notes",
    "prevHash": "a3621af2…", "recordHash": "ba3b571f…",
    "keyId": "ad99dc420e7b0bac", "signature": "f71a314b…"
  }
}
```

Design rules:

- `_type`, `subject`, `predicateType`, `predicate` are the ITE-5 required
  keys; **every native field survives** into `predicate` (nothing dropped,
  nothing invented).
- The subject digest is `args_hash` — the same content binding the native
  verifier checks. in-toto consumers get the exact bytes-binding without
  ever seeing the action's argument preimage.
- The chain travels in `predicate.prevHash`/`recordHash`: a consumer can
  verify linkage without any cryptography, and upgrade to per-record
  signature verification with the anchored public key.
- `predicateType` is versioned by URI suffix (`/v1`); a future incompatible
  change bumps the suffix rather than mutating field meanings.
- Rotation records export with `kind: "key_rotation"` and carry
  `predicate.newPubkey`.

## 5. The PROV-JSON profile (linear activity lineage)

| Native field | PROV construct |
|---|---|
| record | `prov:Activity` (`metalogos:action:<seq>`) |
| `action` | `prov:label` |
| `ts` | `prov:startedAtTime` (unix seconds; ISO-8601 left to the consumer) |
| `actor` | `prov:Agent` (`metalogos:agent:<actor>`), `prov:wasAssociatedWith` |
| `scope` / `args_hash` / `kind` | `metalogos:scope` / `metalogos:argsHash` / `metalogos:kind` |
| `prev_hash` → `hash` | `prov:wasInformedBy` edge to the previous activity |
| `key_id` / `pubkey` | `metalogos:signer` attribute |

Custom namespace declared per PROV-JSON conventions:
`"prefix": {"metalogos": "https://metalogos.dev/ns/ledger/v1#"}`. This is
deliberately a profile over PROV, not a conformance claim: what the chain
actually IS is a linear activity lineage — no delegation graphs, no bundles.

## 6. Threat boundary — what the ledger proves and what it does not

Proves (against an adversary that can alter files or replay fragments):

- **No record was added, removed, reordered, or altered** without breaking
  `seq`/`prev_hash`/`hash`/`sig` — each of the four is checked
  independently, and the golden test enumerates single-byte flips at every
  field position (12 tamper tests in `tests/naryad_393_ledger.rs`).
- **Who signed**: per-record Ed25519 under the anchored key; rotation seams
  are explicit — a mid-chain key substitution is a verification failure.
- **The content of an action's arguments via commitment**: `args_hash`
  binds the record to the exact detail tuple without leaking it.

Does NOT prove (the honest boundary):

- **Tamper-evident, not tamper-proof.** An adversary who controls the host
  AND all out-of-band anchors can rewrite the journal under a fresh key.
  The design's answer is out-of-band anchoring: pin the head `hash` and the
  active `pubkey` externally (an operator file, a monitor, an appendix of
  another system) — `mlog ledger verify --expect-head … --expect-key …`
  fails loudly on a rewritten chain. Post-host-compromise write integrity
  is out of scope, as it must be for any self-contained log.
- **The args preimage is not stored** — the journal proves the action
  happened and binds to its detail tuple; it does not by itself preserve
  the tuple's content for later audit (that is a retention policy choice).
- **Confidentiality**: metadata + hashes only. The payload never enters the
  journal, so the journal cannot leak it — but it also cannot answer
  questions that need the payload.
- **The journal is append-only by discipline** (SQLite insert path,
  no update/delete surface in the runtime) — not by hardware.

## 7. Positioning against adjacent standards

- **in-toto / ITE-5** (the profile this draft pairs with): in-toto
  attests software SUPPLY-CHAIN steps (builds, packaging). Our claim:
  agentic ACTIONS are a legitimate attestation domain — the predicate
  vocabulary differs, the statement shape and verification model do not.
  Question for the community: is a fixed, versioned `predicateType` with a
  native-field-preserving `predicate` the accepted way to introduce a
  domain profile (§10, Q1)?
- **SLSA**: SLSA levels describe build-platform guarantees over in-toto
  attestations. The Action Ledger is orthogonal — it does not attest build
  integrity; it attests runtime agent actions. A deployment can emit both
  (SLSA for the artifact, this ledger for what the agent did with it).
- **CycloneDX**: models software COMPONENTS (SBOM), VEX, and some service
  relationships. There is no CycloneDX document type for a per-action
  signed trail of an agent's runtime decisions; mapping one onto SBOM
  entries would lose the per-action chain semantics (prev-hash, rotation).
  We chose in-toto as the primary profile for this reason.
- **OWASP (LLM/agentic guidance)**: the OWASP GenAI security guidance calls
  for audit trails of agent actions and least-privilege scoping. The ledger
  is exactly that trail, with the grant algebra (ADR-0155) supplying the
  least-privilege mechanics; the `deny.<REASON>` records are journaled
  even when a handler degrades the refusal, so "the agent tried and was
  denied" is auditable.

## 8. Worked corpus — a real 20-record agent-office chain

The `w2_ledger` example is a real program: an N(1) quota grant meters ONE
destructive DELETE; the second granted call refuses (`GRANT_EXHAUSTED`) and
the `on_deny(db)` handler degrades it; the flow rotates the key, takes a
snapshot, and exports. Chain composition: **18 action records + 1
key_rotation + 1 snapshot**; `mlog ledger verify` passes on the export.

Representative records, verbatim from the run (synthetic dogfood data —
test keys, no real identities):

Record 1 — grant issued (genesis; `prev_hash` = 64 zeros):

```json
{"seq":1,"ts":1789691215,"kind":"action","actor":"program",
 "action":"grant.issued","scope":"db:delete:notes",
 "args_hash":"d2b40bb0ff1b703fcc7b7495078e05b34ce1b2cb4a2795d60cea48effb74427e",
 "prev_hash":"0000000000000000000000000000000000000000000000000000000000000000",
 "key_id":"ad99dc420e7b0bac",
 "pubkey":"ef3792a5477e0654ba7bdf873ecbf8ba07f3c62efd19c4aeb1e3eb83912a5f3c",
 "hash":"a3621af20b467de80cdfc748bef4c10dca0142030672ff9a387cc354f26f4558",
 "sig":"fdbe1c54c1d8767335445662f307fbebc590b3d82804b72e340105676b70ca3d…"}
```

Record 2 — the granted destructive action consumed the grant (note the
`actor`: the SQL statement itself is the actor's detail; its hash binds it):

```json
{"seq":2,"ts":1789691215,"kind":"action",
 "actor":"db_execute_with_grant: DELETE FROM notes WHERE user = 'alice'",
 "action":"grant.used","scope":"db:delete:notes",
 "args_hash":"fc9a006628e7f37ea0652244fb4a8b391cfccc58b65d58a07ff303a9e8d8a117",
 "prev_hash":"a3621af20b467de80cdfc748bef4c10dca0142030672ff9a387cc354f26f4558",
 "key_id":"ad99dc420e7b0bac","pubkey":"ef3792a5477e0654ba7bdf873ecbf8ba07f3c62efd19c4aeb1e3eb83912a5f3c",
 "hash":"ba3b571f34ce1dc69310f004b229bcdd1ae07fe0fdb23444835ee2d4b0888787",
 "sig":"f71a314bb008ad35f96d860243348fdb23c007dc6f25b1cd003c8b930d60…"}
```

Record 4 — a runtime deny is journaled BEFORE the deny handler runs (a
handled refusal is still evidence):

```json
{"seq":4,"kind":"action","action":"deny.IRREVERSIBLE_NO_GRANT",
 "scope":"db:delete:notes", …}
```

Records 19-20 — rotation + snapshot close the chain (the rotation is signed
by the STILL-ACTIVE key and names the taking-over key; the snapshot pins the
head for archival):

```json
{"seq":19,"kind":"key_rotation","action":"ledger.rotate","new_key_id":"…","new_pubkey":"…", …}
{"seq":20,"kind":"snapshot","action":"ledger.snapshot","…"}
```

## 9. Scale evidence — the 10 000-record golden

CI runs a blocking job that builds a 10 000-record chain, exports it, and
verifies the export with the external verifier (no interpreter involved) —
the whole cycle must stay under 10 seconds
(`ledger-golden (blocking)` in `.github/workflows/ci.yml`). This is the
scale claim we make: per-record signing does not preclude thousand-record
daily volumes.

## 10. Liaison request (text for the community threads)

> **Subject: Action provenance for AI agents — a profile over in-toto / PROV; review requested**
>
> Hello — we maintain METALOGOS, a small language/runtime for agent
> scenarios where actions (SQL, LLM calls, VCS, messaging) are gated by
> grants and labels. We built an **Action Provenance Ledger**: an
> append-only journal where every agent action is a record hash-linked to
> its predecessor and signed with Ed25519, with explicit key-rotation and
> snapshot records. We are publishing a **public draft** of the format and
> of two profiles over existing standards — in-toto ITE-5 statements (fixed
> `predicateType`, all native fields preserved in `predicate`, the subject
> digest is the action's args commitment) and a PROV-JSON linear activity
> lineage with one declared custom namespace.
>
> We would genuinely value review from people who know these formats better
> than we do. Concrete questions:
>
> **Q1.** For domain-specific attestations, is a fixed, versioned
> `predicateType` URI with a native-field-preserving `predicate` the
> accepted way to introduce a domain profile — or do you expect new
> attestation domains to iterate through some standardization process
> first?
>
> **Q2.** Hash-linked record CHAINS (each record commits to the previous
> one; explicit key-rotation records carry signer continuity) are central
> to our trust model but have no in-toto equivalent. Is expressing the
> chain inside `predicate` (prevHash/recordHash) the right place, or is
> there prior art for chain-native attestations we should adopt?
>
> **Q3.** PROV-JSON: our document is a linear activity lineage
> (`prov:wasInformedBy` edges) with a custom `metalogos:` prefix — is that
> within PROV-JSON conventions, or would reviewers expect a different
> construction for signer/agent identity?
>
> Draft (record model, mappings, threat boundary, worked corpus):
> `docs/research/action-provenance-ledger-draft.md` in the repository above.
> We are a small project — corrections and pointers to prior art are worth
> more to us than endorsements.

**Target channels (for the owner's dispatch):** in-toto community
(in-toto/in-toto GitHub — Discussions/Issues; the ITE process), SLSA
community (GitHub / Slack), the W3C PROV Community Group, CycloneDX TC
(for the §7 positioning), OWASP GenAI/agentic security channels. **Sending
is deliberately NOT part of this naryad** — publication happens by the
owner's explicit instruction (per the 2026-09-17 directive on external
communication); the letter above is the payload.

## 11. Responses

*(Section opened with the draft. No external responses yet — the liaison
letter (§10) is prepared but unsent pending the owner's dispatch. Internal
review notes from the repository's own protocol (ADR-0157 §4, §5) are the
baseline: the in-toto profile preserves every native field and invents
nothing; the PROV profile claims alignment, not conformance. Each external
comment will be recorded here with its resolution — accepted / rejected
with rationale, per the naryad protocol.)*

## 12. References

- ADR-0167 — Action Ledger v1 (native record model, chain rules, verifier)
- ADR-0157 — Ledger profile: PROV/in-toto alignment (field mappings)
- ADR-0168 — MCP server transports (adjacent: tool-surface policy)
- ADR-0155 — Grant algebra (what the ledger journals: grants, quotas)
- in-toto attestation spec (ITE-5), https://in-toto.io
- W3C PROV-JSON, https://www.w3.org/Submission/prov-json/
- SLSA v1.0, https://slsa.dev
- CycloneDX, https://cyclonedx.org
- C2PA 2.4 profile precedent in this repository: ADR-0166 (№320/№337)
