# The M1 checklist — NLnet/Restack, milestone due 2026-11-03

Draft for the NLnet/Restack traction package (№577, the owner's directive
2026-10-04; the delivery-to-owner deadline is **2026-10-20**, M1 is
**2026-11-03**). This is the office-side reconciliation of what the M1
milestone needs against what already exists — the gaps are listed as gaps,
not papered over. The external wording and every submission remain the
OWNER's (№541; №549, issue #887).

## What M1 asks for (the standard NLnet milestone shape)

A milestone submission is a short, evidence-linked report to the NLnet
foundation (through the Restack platform where the grant is managed):
what shipped since the start, whether the promised milestones hold, and
the public evidence a reviewer can check without asking anyone. No
proprietary data, no projections — the same honesty discipline as this
package.

## The evidence matrix (as of 2026-10-05, main `7b10f84`)

| M1 need | Status | Evidence / where it lives |
|---|---|---|
| A public release the reviewer can install | **READY** — v0.28.0 published 2026-10-03 (tag `v0.28.0`, commit `3576887`) with the full supply-chain contract: binary, CycloneDX SBOM, BUILD-INFO, SHA256SUMS, two Sigstore attestations | the GitHub releases page; `docs/release-checklist.md` |
| A demo reproducible by the reviewer | **READY** — one command (`cargo build --bin mlog && bash scripts/demo_traction.sh`), three legs over public examples verbatim | `02-demo.md`; the script exits 0 on every leg |
| Honest, machine-generated metrics | **READY** — the one-pager refreshed 2026-10-05; every number traces to a checked-in generator | `01-metrics-one-pager.md` (the 2026-10-05 snapshot block) |
| A differentiator the reviewer remembers | **READY** — the code-intelligence pilot extract: the language's own machinery beats a generic code-intelligence tool on the tool's home ground, with the methodology and the limits stated | `03-code-intelligence-cbm.md` |
| Community-readiness signals | **READY (structural)** — the good-first-issue pool (9 reserved-for-newcomer issues, each with its verification path), CONTRIBUTING.md, the docs/book, the risk-review checklist | issues #951–#959; the repo entry points |
| The work plan (milestones, budget lines) matching what shipped | **READY** — waves 25–29 executed against the PLAN-SUMMARY; the typed-signature floor 3654 bp exceeds the 0.29 goal (3500) already | `docs/PLAN-SUMMARY.md`, `docs/REALITY.md` |
| The follow-up security release | **IN PREPARATION** — 0.28.1 (the wave-29 audit fixes №581/№582, the release carrier №583) is staged; the release-block discipline holds until the live tag | the wave-29 dispatch gh#1004; `docs/limitations.md` |

## The gaps (honest, with the owner in the loop)

1. **The 0.28.1 security release is not tagged yet.** The fixes are
   staged (№581/№582), the release carrier is №583; the tag and the
   Security announcement are the OWNER's act (№549). Ideal shape for M1:
   the announcement can name the security-discipline story (audit →
   honest boundaries → the release within days) — that story is stronger
   than the absence of any High.
2. **The community numbers are structural, not social.** Stars/forks/
   external contributors are near zero; the GFI pool is prepared but not
   yet picked up by a newcomer. The package says this honestly (no
   projections, §16.0-7); if the reviewer asks "who uses it", the honest
   answer is the dogfood: the FOSVED office runs ON the language.
3. **The CBM pilot logs live in a branch archive.** The extract
   (`03-…cbm.md`) is self-sufficient; if the reviewer asks for raw logs,
   the pilot branch needs to be made public or the measurement re-run on
   the live main — an owner decision.
4. **The wording to the foundation is the owner's.** The package
   provides the facts; the milestone message itself is written and
   submitted by the owner (№541).

## The owner's action list (everything left outside the executor's lane)

- [ ] Write and submit the M1 milestone message (deadline 2026-11-03;
      the package gives the facts — 01/02/03/04 here).
- [ ] Publish the v0.28.1 tag + the Security announcement (№583, after
      the fix-PRs merge; the earlier, the better for the M1 story).
- [ ] Decide the CBM pilot-branch visibility (public branch vs re-run on
      main) if raw logs are wanted in the submission.
- [ ] (Optional, the office's suggestion) pin 2–3 GFI issues as
      "started" before the submission — a reviewer sees a live entry
      path, not a prepared one.

## The dates

| Date | What |
|---|---|
| 2026-10-20 | the package delivery to the owner (this checklist, refreshed metrics, the CBM extract) |
| 2026-10-22 | the office verification pass: every number re-generated, every link re-resolved |
| 2026-11-03 | the M1 milestone submission (the owner) |
