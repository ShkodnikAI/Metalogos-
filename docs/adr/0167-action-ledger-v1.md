# ADR-0167: Action Ledger v1 — signed append-only journal of actions

**Status:** Accepted
**Date:** 2026-09-17
**Naryad:** #393 (issue #487; dispatch #491, wave 3)
**Pillar:** cross-cutting (security/ledger); consumes the grant ledger events (naryad #390, ADR-0155), the DenyEvent vocabulary (naryad #392), the consent-ledger persistence template (`src/consent.rs`)
**Consumers:** naryads #395 (dogfood run under grants + ledger trail), #396 (public Action Provenance Ledger draft + liaison request)

## 1. Context

Today's audit trails in Metalogos are real but fragmented: the static audit report (`mlog audit`), the consent ledger (`src/consent.rs`, process-local SQLite), the grant ledger events (`grant_events`, naryad #390) and the DenyEvent stream (naryad #392) each record their own slice. None of them is **externally verifiable**: an exported JSON of the consent ledger can be edited after the fact with nothing in the file detecting the edit. The grant ledger comment states it verbatim: *"Signing (prev-hash + Ed25519, in-toto/PROV profile) is naryad #393: it upgrades the INTEGRITY of this ledger, not its semantics."* (`src/grants.rs:22-23`).

The wave-3 acceptance (dispatch #491) requires a *signed* action trail: an external party must be able to take the exported journal file and verify that (a) no record was modified, (b) no record was removed, (c) records were appended in exactly this order, and (d) the chain terminates at a specific head that the exporter can publish separately (out-of-band anchor).

`docs/adr/0157-ledger-profile-prov-intoto.md` is the reserved booking for the in-toto/PROV export profile — this ADR defines the chain structure; ADR-0157 (filled in the same naryad) defines the interoperability mapping.

## 2. Decision Drivers

1. **Tamper-evidence, not tamper-proofing.** An append-only file on the same host cannot survive host compromise (an attacker with write access can rewrite everything and re-sign under a fresh key). The honest boundary — required by the naryad — is: integrity holds *given an externally anchored head hash / signer key*; a full rewrite is detectable only against that anchor. §7 states this explicitly.
2. **The write must be a side effect of the action.** A separate "log this action" call the caller can forget is the failure mode the naryad names. Grant lifecycle transitions, runtime deny events, successful irreversible actions and session lifecycle events are therefore wired *inside* the action paths (`grants.rs::record_event`, `fire_on_deny` TW + VM, `invoke_db_execute_with_grant` post-success, session create/destroy).
3. **The verifier must not need the Metalogos runtime.** Verification reads a JSONL file and performs hashing + Ed25519 signature checks — a CLI subcommand (`mlog ledger verify`) with no interpreter, no database, no network.
4. **Dependency discipline (FEATURE_INTAKE §5).** One new top-level crate: `ed25519-dalek` (2.x). Hashing uses the already-present `sha2`, hex encoding the already-present `hex`, key generation the already-present `rand`. Warning threshold is 2 new crates per version, hard limit 5 — one is within budget (the ADR-0132 Decision Drivers precedent).
5. **Fail-open of the *action*, fail-loud of the *journal*.** A ledger write failure inside an action path must never flip the action's outcome (a granted DELETE must not silently become a refusal because the journal is unavailable) — but the failure must be loud on stderr every time it happens. Explicit ledger builtins (`ledger_export`) are ordinary Result-returning surfaces: they refuse loudly.
6. **Confidentiality.** Records carry action *metadata* (actor, action word, scope, hash of arguments), never argument values. `args_hash` is a SHA-256 over the detail string, so secret payloads do not enter the journal.

## 3. Decision

### 3.1 Record format

One record = one JSON line (JSONL, UTF-8, LF-terminated). Field order below is the canonical serialization order (serde struct order; `serde_json` preserves it):

| Field | Type | Meaning |
|---|---|---|
| `seq` | u64 | 0-based position; continuity is enforced by the verifier |
| `ts` | u64 | Unix epoch seconds at append time |
| `kind` | string | `"action"` \| `"key_rotation"` \| `"snapshot"` |
| `actor` | string | Best-effort attribution (grant issuer, `"runtime"`, user id, …) |
| `action` | string | The action word: `grant.issued`, `deny.IRREVERSIBLE_NO_GRANT`, `irreversible.db_execute`, `session.create`, … |
| `scope` | string | Scope/limitation of the action (grant scope, sink class, …) |
| `args_hash` | string | SHA-256 hex over the action's detail tuple (never raw arguments) |
| `prev_hash` | string | `hash` of record `seq-1`; genesis uses 64 zeros |
| `new_pubkey` | string | key_rotation only: the Ed25519 public key taking over |
| `key_id` | string | 16 hex chars of SHA-256 over the signer's public key |
| `pubkey` | string | Signer's Ed25519 public key (hex) — self-describing chain |
| `hash` | string | SHA-256 hex over the canonical body (§3.2) |
| `sig` | string | Ed25519 signature (hex) over the record `hash` bytes |

### 3.2 Chain and signatures

- **Body** = the record without `hash` and `sig` (all other fields, including `new_pubkey`, participate). `hash = SHA-256(canonical body JSON)`.
- **Every record is signed.** `sig = Ed25519_sign(signing_key, hash_bytes)`. Signing every record (instead of signing only the head) makes each record independently verifiable and gives rotation a clean seam; a "head signature" is then simply the last record's signature, and the externally publishable head is the last record's `hash`.
- **Signer continuity.** The active key is the genesis record's `pubkey`; a `key_rotation` record must itself be signed by the *currently active* key and names the next key in `new_pubkey`; every subsequent record must be signed by that key. A mid-chain key substitution without a rotation record is a verification failure.
- **Genesis.** `seq = 0`, `prev_hash = "000…0"` (64 zeros).
- **Snapshot.** A `kind = "snapshot"` record is an ordinary chained record that pins the head at its position. Archival (`mlog ledger archive --at <seq>`) truncates everything *before* the snapshot; the resulting file starts at an anchored record (the snapshot's own `prev_hash` stays an opaque anchor — the pre-history it references is deliberately out of the archive). Verifier rule: a file must start at `seq = 0` (genesis) **or** at a `snapshot` record (anchored start).

### 3.3 Runtime store and keys

- Persistence template: the consent ledger — process-local SQLite table `action_ledger` (INSERT-only; no UPDATE/DELETE code path exists), behind `OnceLock<Mutex<…>>`.
- The signing key is generated from OS randomness at first use (or loaded from the `METALOGOS_LEDGER_KEY` hex-seed environment variable when a reproducible chain is wanted — a test/CI affordance, loud if malformed). `ledger_rotate()` generates a fresh key in-process, appends the rotation record signed by the old key and switches the active key. Key material never leaves the process except as public keys.

### 3.4 Integration — the side-effect surface

| Event source | Trigger point | Action word |
|---|---|---|
| Grant lifecycle (issue/subgrant/consume/revoked/use) | `src/grants.rs::record_event` after the `grant_events` INSERT | `grant.<kind>` |
| Runtime deny events (TW + VM) | `fire_on_deny` / `vm_fire_on_deny` before handler selection | `deny.<REASON>` |
| Successful irreversible action | `invoke_db_execute_with_grant` after SQL success | `irreversible.db_execute` |
| HTTP session lifecycle | `create_session_db` / session destroy (feature `server`) | `session.create` / `session.destroy` |

All four are calls *inside* the action's own code path — the caller cannot forget them. All four are best-effort (§2 driver 5).

### 3.5 Language surface (builtins)

| Builtin | Role | Meaning |
|---|---|---|
| `ledger_count()` | Pure | In-process record count (read — not egress) |
| `ledger_head()` | Pure | Current head hash (designed to be published out-of-band) |
| `ledger_export(path)` | **Sink** | FILE EGRESS: the verifiable JSONL chain to a sandboxed path |
| `ledger_export_intoto(path)` | **Sink** | FILE EGRESS: the in-toto Statement profile (ADR-0157) |
| `ledger_rotate()` | Lift | Append a key-rotation record; returns the new `key_id` |
| `ledger_snapshot()` | Lift | Append a snapshot record; returns the snapshot record hash |

Export of the journal is FILE EGRESS — classified Sink, exactly the `consent_ledger_export` precedent (№335).

### 3.6 External verifier (CLI)

- `mlog ledger verify <file> [--expect-head <hash>] [--expect-key <hex>]` — full structural verification: seq continuity, chain links, hash recomputation, `key_id` recomputation, signature validity, signer continuity across rotations, snapshot anchoring; the optional flags pin the out-of-band anchors (§7).
- `mlog ledger archive <file> <out> --at <seq>` — produce the anchored truncation at a snapshot record; the output is verified before it is written.
- Exit codes: 0 valid, 1 invalid (with the loud reason), 2 usage.
- Golden: a 10 000-record chain signs and externally verifies in **< 10 s** (release-mode CI evidence).

### 3.7 in-toto / PROV profile

The field mapping to in-toto Statements and PROV-JSON is specified in ADR-0157 (filled by this naryad). One in-toto Statement per record, JSONL stream.

## 4. Alternatives considered

1. **Sign only the head.** Cheaper, but a verifier then needs the whole chain re-hashed to check any single middle record, and rotation has no natural seam. Per-record Ed25519 signing costs ~tens of µs each (release) — irrelevant against the 10 s golden budget.
2. **Merkle tree over batches.** Stronger compaction properties, but no consumer in the wave needs sublinear proofs; a linear hash chain is simpler, the verifier is ~200 lines, and in-toto/PROV consumers map 1:1 onto linear records.
3. **Reuse `grant_events` with added signature columns.** Entangles the grant *state machine* with the *integrity mechanism* and makes the deny/session/irreversible populations second-class. A dedicated `action_ledger` keeps semantics (ADR-0155) and integrity (this ADR) orthogonal — the grant ledger comment promises exactly this split.
4. **Merkle-tree/CT-log external service.** A new trust root and network dependency — rejected by the no-new-trust-root posture (ADR-0155 Decision Drivers 6).

## 5. Consequences

- Every wave-3 action class leaves a signed, externally verifiable trail; the dogfood run (№395) can publish its head hash and a third party can verify the full trail.
- The journal is process-local by default (the consent-ledger template); a multi-process persistent store (surviving restarts, shared file locking) is *not* in v1 and stays an open item — honest limitation, does not block the external-verification contract.
- The verifier is a pure file reader: it can be re-implemented by any third party from §3.1-3.2 (this is the №396 prerequisite).
- `grep todo!|unimplemented!|SKELETON` over the diff stays 0 (№16.0-D).

## 6. Security analysis

- **What the ledger proves:** that the record sequence was produced by a holder of the anchored signing key(s), in this order, unmodified since export (given the anchor). Any single-byte modification of any covered field, any deletion, any reordering, any key substitution without a rotation record — all are verification failures (tamper tests in `tests/naryad_393_ledger.rs` enumerate single-byte flips).
- **What it does not prove:** that the *content* of an action record was truthful (garbage in, signed garbage out), that the signer key was not exfiltrated before signing, or integrity of an export whose head/key was never anchored out-of-band (a full rewrite under a fresh key is self-consistent — §2 driver 1).
- **Confidentiality:** only metadata and hashes are journaled; the args-hash preimage never leaves the process. The journal itself is not a secret sink, but its *export to a file* is FILE EGRESS and is classified as such (§3.5).
- **Availability:** ledger unavailability degrades to loud stderr warnings (§2 driver 5) — the journal is an audit asset, not a gate; it never becomes a denial-of-service lever against the actions themselves.

## 7. Threat model — the honest boundary

The ledger is **tamper-evident, not tamper-proof**. Concretely:

| Attacker capability | Detected? | Why |
|---|---|---|
| Edit any field of any exported record | yes | hash recomputation + signature |
| Delete a record / reorder records | yes | seq continuity + prev_hash links |
| Forge a signature | yes | Ed25519 unforgeability under the anchored key |
| Swap in a fresh key and re-sign everything | yes **only with** `--expect-key`/`--expect-head` anchor; no, without | self-consistent file, no external anchor in it |
| Compromise the host *after* export and anchor publication | yes (export file integrity) | anchor + verification |
| Compromise the host *while* records are being written | **no** | out of scope by the naryad's honest boundary: the signer and the store live on the host |

This matches the naryad's requirement verbatim: *"threat-граница (что ledger доказывает и что нет — честная граница: целостность после компрометации хоста не доказывается)"*.

## 8. Compliance mapping

| Naryad criterion (§"Сделано, когда") | Where |
|---|---|
| (а) ADR Accepted, implementation matches | this document; `src/ledger.rs` |
| (б) 10k-record chain signs + externally verifies < 10 s | `golden_10k_chain_signs_and_verifies_under_10s` (release, CI step) |
| (в) tamper tests red on forgery | single-byte flip / deletion / reorder / key-substitution tests |
| (г) grant/deny/session events land automatically | integration tests over `grants.rs`, `fire_on_deny` (TW + VM), session hooks |
| (д) REFERENCE + threat-model synchronized | REFERENCE §Action Ledger, `docs/threat-model.md` ledger rows |
| (е) blocking CI green, 0 stubs | PR CI run |
