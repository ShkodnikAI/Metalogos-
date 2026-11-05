# Repro-readiness report: the C7 trigger contract and the verified bit-for-bit roster (the 8E-2 subphase)

**Naryad №913 (gh#1815, wave 82 — W-K), class [security][docs].** The
«отчёт аудита» artifact of the reproducibility subphase (8E-2; C7+C12,
REPRO_BITFORBIT 10000 bp; C7 строго по триггеру — the R38 version
binding was replaced by the ADR-0189 amendment, №899).
Every number below is regenerable by the machine (`scripts/gen_metrics.py
--check`) or readable verbatim in the named live source — no prose
numbers.

**Report date:** 2026-11-05. **The tree:** main `090831e3` (the wave-82
merge chain). **The record:** ADR-0218 (the №909 subphase record — the
fact definition, the digest signature, the C7 trigger verbatim).

## 1. What REPRO_BITFORBIT counts — the machine verdict on this tree

```
OK: the №912 floor holds — REPRO_BITFORBIT 10000 bp (gate_043 floor
10000, only-up) — roster: 11 registered reproduction carriers
(100 live-wired %)
```

The roster (`scripts/ci/gate_043_goals.txt`, the №862/№894 form — the
count is CHECKED, not declared; a row counts only on its needle literal
in the named live source, and both composition sites —
`fn verify_dependency_integrity` and `fn build_digest_document` in
`mlogpkg/src/main.rs` — must stand):

| # | Carrier | The live source | What is verified bit-for-bit |
|---|---|---|---|
| 1 | pack_twice_byte_identity | `mlogpkg/tests/naryad_910_signed_packages.rs` | re-pack of the same tree → byte-identical digest document |
| 2 | sign_verify_happy | same | sign→verify green on the canonical document |
| 3 | digest_verify_source_tamper | same | a flipped source byte refuses, named |
| 4 | digest_verify_foreign_key | same | a foreign key refuses (`signature mismatch`) |
| 5 | lockfile_determinism | `mlogpkg/tests/naryad_198_lockfile_determinism.rs` | same mlog.toml → byte-identical mlogpkg.lock |
| 6 | pinned_happy_build | `mlogpkg/tests/naryad_911_integrity_gate.rs` | digest-pinned dependency builds green |
| 7 | tamper_before_semantic_check | same | the tampered registry refuses BEFORE the semantic check |
| 8 | advisory_after_integrity | same | the advisory consult rides strictly after the integrity pass |
| 9 | release_sha256sums_attach | `.github/workflows/release.yml` | the SHA256SUMS recompute step in the live workflow |
| 10 | release_sbom_for_that_binary | same | the CycloneDX SBOM for exactly the `mlog` binary (the binding check) |
| 11 | release_sigstore_keyless | same | the keyless Sigstore attestations (`actions/attest@v4.2.2`) |

## 2. The release digest chain as it stands — audited from the workflow source

`.github/workflows/release.yml` (the tagged-commit build, verified by
reading the live source — CI does not re-download release assets; the
external verification form is the owner-forwarded kamerton cycle, №618,
which re-ran SHA256 over the downloaded v0.29.0 assets):

1. **The build is locked:** `cargo build --release --locked -p
   metalogos-server --bin mlog` from the tagged commit
   (`actions/checkout` with `ref: tag`).
2. **The toolchain is recorded:** `BUILD-INFO.txt` carries the source
   commit, `rustc -Vv`, `cargo -V`, and the runner image.
3. **The SBOM describes exactly that binary:** `cargo cyclonedx
   --describe binaries`, then the workflow PICKS the SBOM whose
   `.metadata.component.name == "mlog"` and refuses otherwise («no
   generated SBOM describes the 'mlog' binary»); the smoke checks pin
   the format (CycloneDX 1.5), the non-empty component tree and a known
   direct dependency (`pest`).
4. **The checksums:** `sha256sum` over the binary, the SBOM and
   BUILD-INFO → `SHA256SUMS`, attached to the release; the digests are
   also published as check-run notices (cross-checkable without
   downloading the assets).
5. **The attestations:** `actions/attest@v4.2.2` twice — the SLSA build
   provenance and the SBOM attestation bound to the same subject —
   signed through GitHub's Sigstore keyless integration («no
   human-held signing key», release.yml:14); the verification command
   is printed into the step summary
   (`gh attestation verify mlog-linux-x86_64 --repo …`).

## 3. The C7 trigger contract (R38(8), verbatim)

> C7 — жёстко по триггеру (первый гос/банк пилот; Nix не открывать
> раньше).

- **What fires the trigger:** the first gov/bank pilot (a paying
  institutional deployment of the 8H-2 verticals class).
- **What gets built when it fires:** the bit-for-bit toolchain lane —
  the pinned toolchain/runner reproduction of the release binary (the
  Nix-shaped lane), widening the gate_043 roster with toolchain
  carriers.
- **What stays closed until then:** Nix is NOT opened; the toolchain
  lane is NOT built; no carrier of this report claims toolchain
  reproducibility. The 10000 bp of REPRO_BITFORBIT cover the REGISTRY
  PACKAGE PATH and the RELEASE DIGEST CHAIN only.

## 4. Honest limits (ADR-0218 §5, verbatim)

- REPRO_BITFORBIT is **not the bit-for-bit toolchain build.** Reading
  the 10000 bp as «the build is reproducible» would be a lie; the
  runner images drift and no CI run re-builds the toolchain twice.
- It is **not authenticity.** The pinned-digest / pinned-key model is
  an integrity chain, not a PKI trust web: whoever pins the digest
  vouches for it; a compromised producer machine that pins a tampered
  digest defeats the pin — the defense is the pinned VERIFICATION key +
  the signature (the `mlog.toml [registry] verify_key` contract, №911).
- It is **not tuned to green.** The roster never shrinks to pass: a
  removed carrier, a renamed wiring literal or a deleted composition
  site lowers the share below the floor — the gate working, not a pass.
- It is **not the external audit.** No external institutional audit is
  procured by this wave; the kamerton form (№618) stays the
  owner-forwarded external channel.

## 5. Prior art and the registry adjacency

- **The kamerton cycle (№618):** the external audit verified the v0.29.0
  assets by a fresh SHA256 re-run over the DOWNLOADED artifacts — the
  release digest chain's verification form, proven on a real release.
- **The deterministic lockfile (№198):** `mlogpkg.lock` sorted
  alphabetically — the same-mlog.toml → byte-identical-lock contract,
  carrier #5 of the roster.
- **The Forensic Ledger (ADR-0167) and the decision trace (ADR-0210):**
  the ACTION-contour journal of the language runtime — the registry CLI
  journals nothing there (the №475 trust domain); the two contours are
  adjacent, not duplicated.

## 6. The docs sync landed in this naryad

- `docs/limitations.md` — the package-registry integrity row (the
  pinned-key model's boundaries) + the C7 trigger row.
- `docs/threat-model.md` — the supply-chain row: what the signed
  registry already catches (the tampered package, the foreign key, the
  digest drift), what remains (the pinned-producer trust, the unix-only
  keygen entropy source).
