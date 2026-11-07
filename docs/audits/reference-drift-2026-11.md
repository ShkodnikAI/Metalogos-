# The reader-surface drift sweep — the roster verdicts of the reader surfaces (the wave-87 W-R closing artifact)

**Naryad №929 (gh#1864, wave 87 — W-R, №R2, P2, [audit][docs][process]),
strictly after №928** — it verifies the №928 pin on the wave merge
commit.
The closing artifact of the honest-reference-tail wave: the roster
verdict of every reader surface of the repository, each verdict
regenerable by a machine command or readable verbatim in a named live
source — no prose verdicts (the №907/№925 form: методика → ростер →
граница → урок, here + the верификационная секция).

**Report date:** 2026-11-07. **The tree:** the wave-87 merge commit
`abedee052` (№928 landed: the REFERENCE.md head re-synced and pinned).
**The find that opened the wave:** the fact-check R67 (2026-11-07,
main `921464a`) caught the REFERENCE.md head silently drifted for six
weeks while the CI stayed 57/57 green — the rest of the sweep below
was taken on the same R67 pass and re-taken here on the merge commit.

## 1. Methodology — machines, not prose

Every verdict below is one of:

- **A machine count** — a one-line command over the live tree (the
  command is printed with the verdict; every count reproduces on the
  wave merge commit);
- **A test pin** — `cargo test --test reference_consistency` (the
  №270 §4/§6 body gates + the three №928 head asserts) and the №763
  agent-index gate (the AGENTS↔CLAUDE byte pin);
- **A verbatim quote** of a named live source (the README badge row,
  the llms.txt lines, the CHANGELOG sections).

No verdict comes from prose memory or from a screenshot of an earlier
audit. The counting conventions are the machines' own: a builtin is a
named `spec!("name")` row in `src/builtins/registry.rs` (deduplicated
— the same convention the №270 §4 gate and the §6 generated index
read); an ADR is a `docs/adr/*.md` file minus the `README.md` index
itself; a surface verdict is GREEN only when its number matches its
generating machine.

## 2. The roster verdicts — every surface, one line, its machine

The reproducible commands (each prints the machine number; each
reproduces on the wave merge commit):

```bash
# (a) the head Version line:
grep -m1 "Version:" REFERENCE.md
# (b) the named spec! registry count (the §4-gate convention):
grep -oE 'spec!\(\s*"[^"]+"' src/builtins/registry.rs | sort -u | wc -l
# (c) the ADR file count (minus the README index):
ls docs/adr/*.md | grep -v README | wc -l
# (d) the README badge row:
sed -n '9,12p' README.md
# (e) the llms.txt fact lines:
grep -n "530 builtins\|209 ADR\|naryads №1-№\|530+ lines\|257 working" llms.txt
# (f) the grammar line count:
wc -l src/grammar.pest
# (g) the examples count:
ls examples/*.mlog | wc -l
# (h) the ADR main-table row count (the 3-column index table):
sed -n '/^|---|-------|--------|/,$p' docs/adr/README.md | grep -cE '^\| *[0-9]'
# (i) the AGENTS↔CLAUDE byte pin:
cmp AGENTS.md CLAUDE.md && echo BYTE-IDENTICAL
# (j) the spec surface:
wc -l docs/spec/tools.md
# (k) the pin battery (the №270 body + the №928 head + the №809 tier):
cargo test --test reference_consistency
```

| # | Surface | Verdict | Command | The machine number |
|---|---|---|---|---|
| 1 | REFERENCE.md head (Version) | **RED → fixed by №928 (this wave), now GREEN + pinned** | (a) | `> **Version:** 0.39.0` == `Cargo.toml [workspace.package]` |
| 2 | REFERENCE.md head (builtins) | **GREEN (pinned)** | (b) | **530** == the head's `530 builtins` |
| 3 | REFERENCE.md head (ADR files) | **GREEN (pinned)** | (c) | **209** == the head's `209 ADR files` |
| 4 | README badges | **GREEN** | (d) | `rust-1.93.1` (the honest MSRV floor, Cargo.toml) / `v0.39.0` (== Cargo.toml) / `license-MIT%2FApache--2.0` (== LICENSES) |
| 5 | README CI-badge caption («50 blocking jobs») | **GREEN** | the check-runs API on `abedee052` | 56 runs = 49 named `(blocking)` + `build` (always-blocking) = **50** + 1 class-conditional (№604 honest-boundary, blocking on the security class) + 5 non-blocking |
| 6 | llms.txt (builtins) | **GREEN** | (b), (e) | `530 builtins` (llms.txt line 15) == 530 |
| 7 | llms.txt (ADR) | **GREEN** | (c), (e) | `209 ADR` (llms.txt line 33) == 209 |
| 8 | llms.txt (the naryad range) | **GREEN on the wave merge commit** | (e) | `№1-№929` (bumped by this naryad) == the CHANGELOG's max naryad №929 |
| 9 | llms.txt (grammar floor) | **GREEN — a conservative floor** | (f) | **676** ≥ the claimed `530+ lines` (llms.txt line 18; a truthful under-claim, not a drift — noted, no fix: the report creates no floors) |
| 10 | llms.txt (examples) | **GREEN** | (g) | **257** == the claimed `257 working .mlog files` |
| 11 | docs/adr/README.md (the index ↔ files bijection) | **GREEN** | (h), (c) | **209** main-table rows == **209** files, zero diff in both directions |
| 12 | docs/adr/README.md (the status sync, the №926 class) | **GREEN** | the status first-word diff (file `**Status:**` vs the index cell) | 156/156 parseable pairs agree, **0** mismatches; `Proposed` = exactly {0139 (the spike draft), 0187 (the owner gate), 0211 (the owner's variant choice)} — all three Proposed in BOTH sources (0187/0211 carry the list-form `- Status: **Proposed**`); the №926-touched ADRs (0205/0206/0207/0209/0210/0217–0221) read `Accepted**` on both sides (0205 spot-checked verbatim) |
| 13 | AGENTS.md ↔ CLAUDE.md | **GREEN** | (i) | `BYTE-IDENTICAL` (the №763 agent-index pin; the CLAUDE.md bridge copy) |
| 14 | docs/spec/tools.md | **GREEN** | (j) | 118 lines, the normative tools spec — the №645-protocol surface; 0 mutations this wave (no grammar/builtin change), the norm set intact |
| 15 | CHANGELOG.md | **GREEN** | `sed -n '1,12p' CHANGELOG.md` | `[Unreleased]` carries the №928 row (+ the №929 row lands with this naryad); `[0.39.0] - 2026-11-09` — the amended-train section, the waves 76–86 verbatim (the №646 form, the §13.13 «Вошедшие подфазы» block) |

The R67 pre-fix snapshot, for the record: the surface №1 read
`Version: 0.20.0` / `439 builtins` / `156 ADR files (149 accepted +
7 reserved)` against the machine's 0.39.0 / 530 / 209 — RED; surfaces
№2–№15 were already GREEN on `921464a` (the R67 take), re-taken here.

## 3. The verification section — the №928 pin on the wave merge commit

The pin №928 (three head asserts in `tests/reference_consistency.rs`)
runs on the merge commit `abedee052`:

- **Positive:** `cargo test --test reference_consistency` →
  `test result: ok. 7 passed; 0 failed` — the three №270 body pins +
  the three №928 head asserts + the №809 tier pin, all green; the head
  numbers (0.39.0 / 530 / 209) equal the machine numbers of the roster
  rows №1–№3.
- **Negative control (reproduced on the merge commit, 2026-11-07,
  stable 1.99.1):** the head ADR count inverted 209 → 208 →
  `reference_head_adr_count_matches_files` **FAILED** —
  `the REFERENCE.md head ADR count (208) does not match docs/adr/*.md
  minus the README index (209) — the head drifted (the R67 lesson)` —
  `test result: FAILED. 6 passed; 1 failed`; restored →
  `test result: ok. 7 passed; 0 failed`. (The №928 report's own
  negative control inverted the builtins count 530 → 529 — the two
  controls between them cover the two count-assert families; the
  Version assert was exercised by the same mechanical path.)
- **The drift class is closed:** any of the three head numbers moving
  off its machine now turns a named test red on the merge gate — the
  six-week silent drift cannot recur in this surface.

## 4. The lesson — the «шапка вне пина» class

The №270 gate (the reference-consistency harness) pinned the §4
coverage (every builtin mentioned) and the §6 generated markers — the
BODY of the reference. The HEAD of the same file carried four more
numbers (Version / synced-date / builtins / ADR files) and **no test
read them**: the head drifted 2026-09-16 → 2026-11-07 (six weeks,
`0.20.0/439/156` against `0.39.0/530/209`) while the CI stayed 57/57
green. The reader who opened REFERENCE.md first (its purpose) read a
six-weeks-wrong version line; nothing in the machine ever noticed.

The lesson is the №735 class («a fact lives together with its
checker») applied to reader surfaces: **a number in a file a human
reads is either machine-generated, or machine-pinned, or it is a lie
on a timer.** The fix shape (№928): the head asserts read the same
machines the body gate reads — Cargo.toml's `[workspace.package]`,
the registry's named `spec!` set, the `docs/adr/` file count — so the
head and the body now fail together, loudly, on the same gate.

## 5. The boundary — what this report does NOT do

- The report **creates no new floors** and **edits no canon**: the
  conservative grammar floor (`530+`, machine 676) is recorded in the
  roster, not rewritten — a floor move is a naryad of its own.
- The PLAN-SUMMARY / REALITY wave-record surfaces are the №789
  generator track (the weekly publisher PR, the C4 decision gh#1473)
  — **out of this roster**, per the issue's Ф3.
- `docs/limitations.md` / `docs/threat-model.md` need no sync: the
  wave carries **0 mutations of the language surface** (no grammar,
  no builtins, no golden change) — the only code-adjacent change is
  the test pin itself.
- The newcomer pool (gh#1353, gh#1469) and the UPSTREAM-BLOCKED
  gh#1544 (rustyline — reopenable only per the issue's condition) are
  not touched, per the dispatch.
- The owner gates stand untouched: ADR-0187 (`owner_fixed: false`)
  and ADR-0211 (the (а)/(б)/(в) variant choice) remain Proposed —
  they are honest-outside-the-repo acts (ADR-0221 §5.5 class), not
  drift to fix.

## 6. The sync landed in this naryad

- `docs/audits/reference-drift-2026-11.md` — this report (the №907
  form).
- `llms.txt` — the naryad range bumped №1–№929 (the §6.79 lesson).
- `CHANGELOG.md` — the `[Unreleased]` report row.
- NO DRIFT (docs domain): the grammar, the builtins, the golden files
  — untouched; the §4/§6 №270 pins and the №809 tier pin — untouched
  and green; the floors — not moved.
