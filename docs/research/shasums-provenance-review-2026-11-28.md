# The SHA256SUMS provenance review — the artifact chain, the platform gap, the В2.1 plan

**Волна В109 — В2.0 (наряд №1019, issue #2118), the date 2026-11-28, base main `7a28e6cf` (№1017).**
The gate for the SHA256SUMS cross-platform extension: per the never-cut §5 (ADR-0224 §4, the honesty
words), the extension is built ONLY on top of a recorded review of the checksum chain. This document
is that review. Zero product code, zero workflow changes — the implementation is the В2.1 naryad,
built from the conclusions here.

## 1. Scope and method

The checksum surface of the repository, inventoried from the working tree at main `7a28e6cf`:

- `.github/workflows/release.yml` — the release lane (the only surface attached to a GitHub release);
- `.github/workflows/artifact-checksums.yml` — the №1011 lane (the per-run SHA256SUMS + MANIFEST over
  workflow artifacts);
- `.github/workflows/edge-cross-platform.yml` — the five-platform native build lane (the source of
  every non-x86_64 binary the project has);
- `docs/adr/0218-reproducible-build-signed-registry.md` — the signed-registry posture;
- `docs/adr/0224-edge-deepening-verifier.md` §4 — the honesty words that constrain this review.

Method: every claim below carries a `file:line` citation from that tree; no numbers are invented, no
platform is declared working by inference, and every decision that belongs to the owner is named as
an owner-decision point, not taken (ADR-0224 §4.3: the budgets are owner-set; the executor measures).

## 2. The chain as it stands — three surfaces

**2.1 The release surface (x86_64 only).** `release.yml:176` builds the release SHA256SUMS from
exactly three files — `mlog-linux-x86_64`, `mlog-linux-x86_64.cdx.json`, `BUILD-INFO.txt` — and
`release.yml:235` uploads `dist/*` to the release. The granularity is right: the hashes cover the
inner binaries and the SBOM, not a container. But the coverage is one platform out of five. The
publication itself is already owner-gated by the lane's own note (`release.yml:232`: «the
publication is the OWNER's act after the Sunday stand verdict», the ADR-0189 amendment §5.1) — the
gap below is about WHAT is hashable at that moment, not about who presses the button.

**2.2 The artifact surface (the №1011 lane).** `artifact-checksums.yml` (dispatch-only, non-blocking
by construction, the edge-profile precedent) lists all fresh workflow artifacts
(`artifact-checksums.yml:34`, the `expired == false` filter), downloads each zip, and hashes the
CONTAINER: `artifact-checksums.yml:49` writes `SHA256SUMS` with one line per
`<safe>_<id>.zip`, while `artifact-checksums.yml:50` writes `MANIFEST.tsv` with
`artifact_id, name, size_bytes, created_at, head_sha, run_id`. The provenance of the RECORD is good —
head_sha and run_id tie every hash to the commit and the run that produced the artifact. The output
is published as its own `artifact-checksums` artifact: a per-run snapshot, itself subject to the same
expiry mechanics it filters on.

**2.3 The cross-platform build surface.** `edge-cross-platform.yml` builds natively on five runners
(darwin-arm64, darwin-x86_64, windows-x86_64, linux-arm64, linux-x86_64 — the matrix at
`edge-cross-platform.yml:30-34`), each platform ITS OWN RUN (the lane's header cites ADR-0224 §4
directly: «each platform gets ITS OWN RUN»), and uploads per-platform artifacts
`edge-cross-<name>` (`edge-cross-platform.yml:59`). No checksums are computed in this lane. This is
where every non-x86_64 binary lives, and none of them reaches a hashable release surface.

## 3. The gaps (findings)

**G1 — the platform gap.** A consumer of any non-x86_64 binary has no release-surface checksum to
verify against. The binaries exist as edge-cross artifacts, but (a) an artifact zip is not a release
asset — it is not what a release consumer downloads, and (b) even the №1011 lane's hash of that zip
would be a hash of the container, not of the binary (G2). Four of five platforms are effectively
hash-less at the surface where verification happens.

**G2 — container vs binary.** The №1011 lane hashes zip containers
(`artifact-checksums.yml:49`). That answers «is this archive byte-intact since the run» — a
provenance-of-the-record question. It does NOT answer «is this extracted binary the one the run
produced» — a provenance-of-the-artifact question, which is what a checksum consumer actually asks.
Both questions are legitimate; today only the first has any answer, and only for artifacts, never
for release assets.

**G3 — platform identification by name only.** `MANIFEST.tsv` (`artifact-checksums.yml:50`) carries
no platform column. The platform is inferable only from the artifact NAME convention
(`edge-cross-<name>`, `bench-*`, …). A rename or a new lane silently breaks the inference. The
MANIFEST already records `head_sha` and `run_id`; the platform is the missing column of the same
class of provenance facts.

**G4 — no signatures.** Every SHA256SUMS in the chain is integrity-only. Authenticity (who vouches
for the hash table) has no mechanism: ADR-0218's signed-registry posture is not applied to either
the release or the artifact lane. This is an owner-decision point (§4.3), not a gap the executor
fills by default: signing introduces key custody, a trust root and an operational burden — none of
which the executor may invent.

**G5 — time-reproducibility.** The `expired == false` filter (`artifact-checksums.yml:34`) means the
hashable set shrinks as artifacts expire. A past MANIFEST cannot be re-derived later; the record is
a snapshot of a window, not a registry. ADR-0218 is the contrast point: a registry is durable by
design. The №1011 lane did not promise durability — but the В2.1 plan must name which posture it is
building, snapshot or registry, before extending it.

## 4. The В2.1 proposal (a plan, not an implementation)

**P1 — per-platform release checksums, from each platform's own run.** The release lane gains
per-platform SHA256SUMS lines built from the edge-cross artifacts of the SAME commit: the binary is
extracted from each platform's artifact and hashed at binary granularity (the release.yml:176
convention). The never-cut rule applies verbatim: a platform whose run artifact is absent for that
commit gets NO hash line and is named «нет прогона» — no hash is inferred, no hash is copied across
commits. linux-x86_64 keeps its existing inline hash; the other four attach from their own runs.

**P2 — the №1011 lane: platform column + inner-binary hashes.** `MANIFEST.tsv` gains a platform
column, derived from the artifact name and VERIFIED, not trusted: a binfmt probe (`file` output) on
the inner binary confirms the platform claim before the column is written. Alongside the container
hash, the inner binary's hash is recorded (the G2 answer), with the container hash kept for the
record-integrity question. Name-verified-by-probe closes G3 without inventing a new naming scheme.

**P3 — signing stays an owner-decision point.** G4 is recorded, not resolved: if the owner wants
signatures (ADR-0218 posture), В2.1 implements signing ON TOP of P1/P2 hashes with a custody plan
the owner names; without that word, the chain stays integrity-only, and the review says so
explicitly rather than quietly shipping unsigned as if it were signed.

**P4 — snapshot vs registry named before building.** В2.1 states in its own header which posture it
implements: the per-run snapshot (the №1011 semantics, cheap, expires) or the durable registry (the
ADR-0218 semantics, needs storage and lifecycle decisions that are owner-set). No default is chosen
here for them.

## 5. The honest words (per ADR-0224 §4)

1. This review invents no numbers: every fact cites `file:line` from main `7a28e6cf`.
2. No platform is declared working: the four non-x86_64 platforms stay exactly what the lanes say
   they are — own-run artifacts without a hashable release surface.
3. The owner decisions (signing; snapshot-vs-registry) are named, not taken.
4. The review itself is a record, machine-landed through the same gates as every naryad — the chain
   it reviews is the chain that verifies it.

**The В2.1 naryad builds on P1+P2 by default, with P3/P4 held at the named owner-decision points.**
