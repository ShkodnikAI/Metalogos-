# ADR-0157: Ledger profile — PROV/in-toto alignment

**Status:** Accepted (fills the reserved booking of 2026-09-14)
**Date:** 2026-09-17
**Naryad:** #393 (issue #487; the booking was made by naryad #319 — plan v2 §19 originally mapped this slot to naryad #343, renumbered by wave 3)
**Pillar:** cross-cutting (security/ledger); depends on the Action Ledger v1 record structure (ADR-0167)
**Consumers:** naryad #396 (public Action Provenance Ledger draft, external liaison)

## 1. Context

ADR-0167 defines the native Action Ledger record (prev-hash chain + per-record Ed25519 signatures). For external ecosystems the journal must interoperate with the two provenance standards the naryad names — **in-toto attestations** (ITE-5 statement layout) and **W3C PROV-JSON** — without pretending Metalogos implements their full toolchains. This ADR fixes the honest, lossless-where-possible field mapping implemented by `ledger_export_intoto(path)` (the in-toto profile; FILE EGRESS, classified Sink). One JSON object per line (JSONL stream): the chain order is the stream order.

## 2. in-toto Statement mapping (per record)

```json
{
  "_type": "https://in-toto.io/Statement/v0.1",
  "subject": [{ "name": "metalogos:action:<action>@<seq>", "digest": { "sha256": "<args_hash>" } }],
  "predicateType": "https://metalogos.dev/attestations/action-ledger/v1",
  "predicate": {
    "seq": <u64>, "ts": <u64>, "kind": "<kind>",
    "actor": "<actor>", "action": "<action>", "scope": "<scope>",
    "prevHash": "<prev_hash>", "recordHash": "<hash>",
    "keyId": "<key_id>", "signature": "<sig hex>"
  }
}
```

Rules:

- `_type`, `subject`, `predicateType`, `predicate` are the ITE-5 required keys; every native record field survives (nothing is dropped, nothing is invented).
- The subject digest is `args_hash` — the SHA-256 of the action's detail tuple (ADR-0167 §3.1). in-toto consumers see the same content-binding the native verifier checks; the preimage never leaves the process.
- `prevHash` + `recordHash` carry the chain: a consumer can check record linkage without Ed25519 at all, and with the signer's public key it can verify every signature (the public keys ride in the native export; the in-toto profile references them by `keyId`).
- `predicateType` is a fixed URI — versioned by the `/v1` suffix; a future incompatible profile bumps the suffix, never mutates field meanings.
- Rotation and snapshot records export with their `kind` in `predicate.kind`; the rotation's `new_pubkey` rides as `predicate.newPubkey` (present only on rotation records).

## 3. PROV-JSON mapping (per record)

| Native field | PROV construct |
|---|---|
| record | `prov:Activity` (id `metalogos:action:<seq>`) |
| `action` | `prov:label` of the activity |
| `ts` | `prov:startedAtTime` (unix seconds; ISO-8601 conversion left to the consumer — the journal stores integers only) |
| `actor` | `prov:Agent` (`metalogos:agent:<actor>`), linked by `prov:wasAssociatedWith` |
| `scope` | custom attribute `metalogos:scope` on the activity |
| `args_hash` | custom attribute `metalogos:argsHash` |
| `prev_hash` → `hash` | `prov:wasInformedBy` edge to the previous activity (the chain as a provenance lineage) |
| signer `key_id`/`pubkey` | `metalogos:signer` attribute (Agent identity; verification stays out-of-band) |
| `kind` | custom attribute `metalogos:kind` |

The custom `metalogos:` namespace is declared in the document (`"prefix": {"metalogos": "https://metalogos.dev/ns/ledger/v1#"}`) per PROV-JSON conventions. The mapping is deliberately a *profile over* PROV, not a claim of full PROV conformance: no provenance bundles, no delegation graphs — a linear activity lineage is what the chain actually is.

## 4. Interop boundary (honest)

- The profile exports **statements about actions**; it does not emit in-toto layouts, layout verification, or full PROV bundles. Consumers that can ingest ITE-5 statements (e.g. attest-verification pipelines) can verify per-record signatures with the anchored public key; the chain rules (seq/prev-hash continuity, signer continuity) are ADR-0167 §3.2 and are checked by `mlog ledger verify` on the native export.
- Nothing in this profile weakens the native chain: `ledger_export_intoto` is a *view* over the same records; the native JSONL export remains the verification format.

## 5. Consequences

- External parties (the №396 draft, the liaison request) consume the trail with stock in-toto/PROV tooling to the extent mapped above.
- The verifier story stays single-sourced: the native JSONL is the chain of record; the profiles are projections.
- `grep todo!|unimplemented!|SKELETON` over the diff stays 0 (№16.0-D).
