<!-- №865 (issue #1707, P0): the PREPARED migration-section texts for the
     PUBLISHED release notes of v0.36.0 and v0.38.0. The release notes are
     GENERATED from CHANGELOG.md verbatim (№497: the notes and the
     changelog can never disagree) — the retroactive `### Changed`
     BREAKING rows landed in the CHANGELOG in the №865 PR, so the
     published bodies (which predate the rows) now lag the changelog.
     The right to edit a PUBLISHED release is the OWNER's (ADR-0186
     §4.5 — the machine never publishes); this file is the prepared
     text for that act. THE OWNER'S PROCEDURE (either form works):
       (a) replace the release body with
           `python3 scripts/ci/release_notes.py 0.36.0` /
           `python3 scripts/ci/release_notes.py 0.38.0` output — the
           regenerated body carries the BREAKING row verbatim and the
           notes-vs-changelog invariant restores by construction; or
       (b) paste the exact block below into the published body, right
           before the `### Added` part.
     AFTER the owner applies the edit to BOTH releases, delete this
     file (a stale draft is a lie — the №719.3 rule). -->

# v0.36.0 — the prepared Migration text

The block to insert before `### Added` in the published v0.36.0 body
(verbatim = the CHANGELOG [0.36.0] `### Changed` row):

```markdown
### Changed

- **BREAKING (№865 — the migration-honesty verdict 2026-11-01; the rule
  is №845, ADR-0203 §2.4):** the sharp set refuses BEFORE the run —
  12 carriers (`exec`, `exec_argv`, `delete_file`, `git_push`,
  `http_post`, `http_post_multipart`, `send_message`, `send_photo`,
  `send_document`, `smtp_send`, `smtp_send_html`, `tts_send`) fail
  with `IRREVERSIBLE_NO_GRANT` without one of the THREE legal paths:
  (1) a covering capability grant (issue with scope `builtin:<name>`
  — the strict path, TTL/quota metered at the gate), (2) `profile
  capability { allow_carrier: "<name>" }` (the minimal path),
  (3) (exec/exec_argv) the host permit — `METALOGOS_ALLOW_EXEC=1` in
  the process context, `METALOGOS_SERVE_ALLOW_EXEC=1` in a
  serve-route body. `profile legacy` does NOT bypass the rule. A
  0.34-era program calling any sharp carrier refuses at the first
  call on 0.36.0. The migration recipe:
  [docs/upgrade.md](docs/upgrade.md) (§ "0.34 → 0.38"). The `### Added`
  row below stays verbatim: its "existing programs byte-unchanged"
  clause refers to the №844 grant-carrier migration (the typed
  Capability view over the same ledger), NOT to the №845 coverage
  rule — read the two rows together (the rule shipped in the Added
  prose without the BREAKING mark; that omission is what this row
  fixes).
```

# v0.38.0 — the prepared Migration text

The block to insert before `### Added` in the published v0.38.0 body
(verbatim = the CHANGELOG [0.38.0] `### Changed` row):

```markdown
### Changed

- **BREAKING (№865 — the migration-honesty verdict 2026-11-01; the rule
  is №861, ADR-0205 §2.6):** `cron_run` is a human-oversight carrier
  (`HUMAN_OVERSIGHT`, `src/approval.rs`): a call without a LIVE
  approval mandate refuses BEFORE the action with
  `IRREVERSIBLE_NO_GRANT` — record the human's decision with
  `approval_issue(actor, "carrier:cron_run", ttl_seconds)` (the
  mandate is bound to the CARRIER, not the call digest; it expires by
  ttl; `approval_revoke("carrier:cron_run")` withdraws it). A 0.34-era
  program that fires `cron_run` refuses at the first call on 0.38.0.
  The migration recipe: [docs/upgrade.md](docs/upgrade.md) (§
  "0.34 → 0.38", the cron_run step). The `### Added` row below stays
  verbatim — read the two rows together (the rule shipped in the Added
  prose without the BREAKING mark; that omission is what this row
  fixes).
```
