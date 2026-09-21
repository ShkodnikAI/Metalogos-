# Action Ledger — Operator Runbook

**Audience:** the operator who runs a Metalogos deployment (the agent office, a
long-lived `mlog serve` process) and is responsible for the integrity of its
Action Ledger — the signed Ed25519 record chain every irreversible action
journals itself into (ADR-0167; naryads №393/№415/№418; the №412 grant
verification).

**The one-line model:** the ledger is **tamper-EVIDENT, not tamper-proof**
(ADR-0167 §6–7). Verifying a chain proves it was not *altered*; proving it was
not *replaced* requires an anchor published OUTSIDE the machine the chain
lives on. This runbook covers the key lifecycle, the export, the out-of-band
anchor, the periodic verification (via the native cron, 0.21.0) and the
reaction protocol.

---

## 1. Key lifecycle

| Step | Command / surface | Notes |
|---|---|---|
| Identity | env `METALOGOS_LEDGER_KEY` (64 hex chars) | Set BEFORE the first serve boot. A wrong-length value is loudly refused with a fresh-key warning (`[LEDGER] WARNING …`) — do not ignore it; a fresh key orphans the previous chain's signatures. |
| Rotation | `ledger_rotate()` (builtin, in-program) | The rotation record is signed by the still-active key; after it, new records use the new key. The CLI verifier follows the rotation seam (`distinct_keys` > 1 is normal after a rotation). |
| Snapshot | `ledger_snapshot()` (builtin) | Pins the head at a point in time; the archive anchor builds on it. |

The key is an identity, not a secret to rotate casually: every rotation is a
seam the verifier must traverse, and the anchor protocol below must re-publish
after each one.

## 2. Export

```mlog
let path = ledger_export("data/ledger/chain_latest.jsonl")   // JSONL + in-toto sidecar
```

Export writes the verifiable chain WITHOUT stopping the runtime. The export is
a classified sink (egress): the path is literal, sandboxed, and the write is
audited. Export at least:

- after every rotation or archive junction;
- before every host restart / deploy;
- on every soak/dogfood cycle (the №395 dogfood cadence).

## 3. The out-of-band anchor (the wholesale-rewrite catch)

The structural verifier proves INTERNAL consistency: seq continuity,
prev-hash linkage, per-record signatures, key continuity. A self-consistent
REWRITE (the whole chain rebuilt under a fresh key) passes it. The anchor
catches exactly that:

```bash
# On the machine (or in the trusted CI job):
mlog ledger verify data/ledger/chain_latest.jsonl
#   → read head_hash + distinct_keys from the --json verdict
mlog ledger verify --json data/ledger/chain_latest.jsonl | jq '.head_hash, .distinct_keys'

# Publish BOTH out-of-band (a second host, a git repo the runtime cannot
# write to, printed and stored off-box). Then verify WITH the anchor:
mlog ledger verify data/ledger/chain_latest.jsonl \
  --expect-head <published_head_hash> --expect-key <published_key_id>
# exit 0 = ok, 1 = fault
```

Cadence: publish the anchor after every export you intend to trust; verify
against the last published anchor on every periodic check. An EMPTY chain
verifies vacuously as `ok` without an anchor (nothing in it contradicts) —
with `--expect-head` pinned, an empty chain FAILS loudly, which is precisely
the wholesale-deletion catch (pinned by the №415 test suite).

## 4. Periodic verification via the native cron (0.21.0)

The №418 cron (windows fire at most once per matched minute, per-job IANA
timezone, catch-up policy, optional fixed payload) is the natural runner for
the periodic check. Register a job whose handler verifies and branches on the
STRUCT verdict — never on message prose (ADR-0131):

```mlog
pattern LedgerCheck(data: String) -> String {
  let v = ledger_verify(data)            // data = the chain path (the payload)
  if v.ok {
    return "ledger-ok,records=" + to_string(v.records)
  }
  // loud, structured, actionable — the reaction protocol (§5) starts here
  print("[LEDGER_FAULT] record=" + to_string(v.error_record) + " reason=" + v.error_reason)
  return "ledger-fault"
}
// at bootstrap (the №419 office pattern):
cron_add("0 * * * *", "LedgerCheck", "Europe/Moscow", "run_once", "data/ledger/chain_latest.jsonl")
```

Runnable end-to-end (chain build → export → verify → branch):
[`examples/w7_ledger_cron_verify.mlog`](../examples/w7_ledger_cron_verify.mlog)
(golden-tested; its `.expected` pins the verify-ok branch with a refused
second use journaled as a deny event).

The machine-readable external surface — for a verifier OUTSIDE the runtime:

```bash
mlog ledger verify --json data/ledger/chain_latest.jsonl   # exit 0 = ok, 1 = fault
```

**Known context boundary (observed 0.21.0, filed on issue #583):** the
cron-dispatch context does not bind the program's `db {}` block. A check
handler that also needs the database should forward to a route (the office
pattern) or keep the check db-free — `ledger_verify(path)` is read-only and
db-free by design.

## 5. Reaction protocol on `ok = false`

1. **Stop the destructive path first.** Destructive SQL already refuses
   without a grant (`IRREVERSIBLE_NO_GRANT`, fail-closed); a ledger fault
   means the TRUST trail is broken — deny further granted operations at the
   operator level (rotate the key only as part of recovery, not to "reset"
   the fault).
2. **Alert loudly** with the structured fields: `error_record` (1-based
   position) and `error_reason` — they name the first contradicting record
   and the why (signature, linkage, seq gap, key discontinuity).
3. **Freeze the state**: copy the current chain AND the last out-of-band
   anchor before anything else touches the file.
4. **Recover from the last good export/archive** (§2–§3), re-anchor, and
   record the incident itself — the recovery IS a new action trail.

## 6. Honest boundaries

- The hook is **read-only** and **anchor-less by default**: internal
  consistency only, unless `--expect-head` / `--expect-key` are supplied.
- The EMPTY ledger verifies vacuously as `ok` without an anchor (documented
  debatable case); pinned, it fails loudly.
- A missing file under the builtin is a soft `ok=false` "cannot read"
  verdict (the №254 read contract); a sandbox escape stays a loud
  `[SANDBOX_VIOLATION]`.
- Tamper-EVIDENT ≠ tamper-PROOF: an attacker with the key AND the anchor
  channel wins — that is why §3 publishes out-of-band.

## 7. Sources

ADR-0167 (§3.3 key identity, §3.4 side-effect journaling, §6–7 anchor model),
naryads №393 (ledger v1), №415 (the `ledger_verify` hook: builtin / CLI
`--json` / library verdict), №418 (the native cron: dedup, catch-up, TZ,
payload), №412 (the grant verification harness), ADR-0157 (the in-toto
profile), the №395 dogfood (gh#489 — the operating experience).
