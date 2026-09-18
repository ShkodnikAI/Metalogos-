# Naryad #237 — Real-Weights Runbook (Vision R3.7)

**Purpose:** a step-by-step protocol for the real-weights run (the env-gated tests of #212) on
the owner's machine. The code is GO-ready (#236, PR #234, CI 15/15); this document makes
the run = a single session of commands, with no improvisation.
**Status:** the run is **PARKED** (owner decision 2026-09-09 — no hardware in the
delivery environment: 9.2 GB of disk out of the required ≥ 40 GB). The owner's
assumption "everything is fine" = a conditional GO on the code; an assumption ≠ verification —
verification is done ONLY by this run.
**Rule §3.8:** every number below is filled in from the output of a real command. The
"REQUIRES REAL RUN" slot is filled only by a real run; the run did not happen —
the slot stays empty. Fabricating numbers = naryad failure.

---

## 0. What the run decides (Go/No-Go)

The criteria are verbatim from `docs/research/naryad-212-go-no-go.md` (Block 5.2
of the #212 naryad specification):

- Latency on CPU;
- Necessity of GPU;
- Quality acceptability (subjective — requires actual generated image).

If the env-gated run produces a recognizable image at an acceptable latency —
**Go for R4** (vision grammar + dispatch, #213). If the image is corrupted or
the latency is unacceptable — **No-Go** + a targeted fix-forward PR per the R3
simplification list from go-no-go ("R3 architecture status"). The decision is made by
the coordinator (the go-no-go header), not the executor.

## 1. Pre-run (every item mandatory)

| # | Check | Command / criterion |
|---|----------|--------------------|
| 1.1 | Free disk ≥ 40 GB on the target FS | `df -h $MLOG_VISION_WEIGHTS_DIR` — the weights are actually 32 848 304 654 B ≈ 32.85 GB (16 files, verified via the HF API 2026-09-09; the older go-no-go estimate "~24.6 GB" is an underestimate from the same source) + PNG + headroom |
| 1.2 | RAM ≥ 64 GB (F32 policy) | dtype policy: `naryad-212-wedge-e2e-facts.md` §8 — F32: transformer 22.93 GB → ~46 GB RAM + Qwen3 7.49 GB → ~15 GB + VAE ~0.33 GB + activations ~30 GB → **~62 GB peak**. The BF16 path (~32 GB peak) — **R4+ territory, do NOT improvise in this run** |
| 1.3 | The repository on a green commit ≥ the #237 base | `git fetch origin main --force && git reset --hard origin/main`; CI 15/15 on that sha |
| 1.4 | The weights directory set as an absolute path | `export MLOG_VISION_WEIGHTS_DIR=/abs/path` (the large FS from step 1.1) |
| 1.5 | The build is intact | `cargo build --workspace --features vision` — success |

Time expectation: a CPU run is slow — minutes to tens of minutes per forward
pass (facts §8); budget hours for the full session (download + 3 tests +
the determinism run).

## 2. Weights download (fetch under checksum discipline)

```bash
export MLOG_VISION_WEIGHTS_DIR=/abs/path/to/weights

tools/fetch_vision_weights.sh --dry-run   # plan: URL → path, SKIP/download; offline
tools/fetch_vision_weights.sh             # download ~32.85 GB; resume (curl -L -C -)
tools/fetch_vision_weights.sh             # idempotency: EVERY file → SKIP (sha-verified)
```

Script discipline (implements Block 2.1 of the manifest): reference = the manifest SHA
(if filled) → otherwise the HF LFS oid; a mismatch → a loud failure, the file is not
consumed; a network failure → a loud non-zero exit, "skipped it and moved on" does not
exist.

After the download:

1. Paste the printed `| file | sha256 | bytes |` rows into the tables
   `docs/research/naryad-212-weights-manifest.md` (replacing the remaining `_TODO_`).
2. Commit ONLY the manifest: weights in git are forbidden (§3.2). Check:
   `git status` shows not a single `.safetensors`;
   `git ls-tree -r HEAD --name-only | grep -ciE 'safetensors|\.ckpt$|\.pth$|\.gguf$'` = 0.

## 3. Running the env-gated tests (exact commands from #212)

```bash
export MLOG_VISION_OUT=/abs/path/to/out    # optional; default target/
cargo test --features vision --test naryad_212_wedge_e2e -- --nocapture 2>&1 | tee n237_run1.log
```

Without `MLOG_VISION_WEIGHTS_DIR` these tests loudly SKIP; with the directory set —
all three must execute:

- `text_encoder_real_weights_forward` — a real Qwen3-4B forward (3 shards, 8 044 982 000 B ≈ 8.05 GB) → `[seq, 2560]`;
- `vae_real_weights_decode_fixed_latent` — a real VAE decode (167 MB) → PNG 1024×1024 (`n212_vae_fixed_latent.png`);
- `clinical_e2e_first_image` — the full wedge: prompt → tokens → Qwen3 → DiT 8 forward → VAE → PNG 1024×1024 (`first_image.png`, seed 21200, prompt "a red apple on a wooden table, studio light").

> **LOUD note (a deviation, recorded by the verifier 2026-09-09).**
> The #237 naryad wording of Block 3.1 "+ one generation from .mlog" in
> R3.7/R4.1 is infeasible: the vision builtins (`vision_generate` etc., `src/builtins/registry.rs:548–553`)
> are stubs; .mlog generation is R4.2 territory (dispatch). The generation equivalent
> before R4.2 is the env-gated test `clinical_e2e_first_image` (the full wedge:
> prompt → tokens → Qwen3 → DiT → VAE → PNG).
>
> Implemented in #240 (PR #238): the env-gated .mlog test
> `mlog_vision_generate_export_e2e` (`tests/naryad_240_vision_mlog_e2e.rs`); command:
> `cargo test --workspace --features vision --no-fail-fast --test naryad_240_vision_mlog_e2e -- --nocapture`.

The CI-visible tiny goldens (VAE `85ef6a87…`, DiT `860c85b311905f6c23b90a4e9e3192928027a24bf3e4a00a08096336abad4b3c`; n231: `e686167b…` — pre-rebuild architecture) are unchanged in this run
— their greenness is already in CI; if they suddenly turn red — STOP, record the environment,
touch no pins (§3.2).

## 3.1. Edit-e2e run (Naryad #243, R6.2 — same session)

The edit wedge (in-context editing) goes in THIS SAME session after step 3 — the
run remains a single command with no improvisation (Block 4.2 of #243):

```bash
cargo test --features vision --test naryad_240_vision_mlog_e2e -- --nocapture 2>&1 | tee n243_edit_run1.log
sha256sum $MLOG_VISION_OUT/naryad_243_mlog_first_edit.png   # record it
```

The test `mlog_vision_edit_export_e2e`: generate (seed 42, prompt "a red apple…")
→ `vision_edit(v, "make the apple green, keep everything else unchanged")` →
export (`naryad_243_mlog_first_edit.png` + sidecar). What is checked:

- **Dims contract:** the output 1024×1024 = the source (resize forbidden — Block 1.4);
  an incompatible source = a loud Err; in e2e the dimensions are multiples of the factor 8.
- **Inheritance:** the sidecar of the edited artifact — `model_id`
  z-image-turbo, `policy` safe, `seed` 42 (inherited from the source),
  `prompt_sha256` = the SHA-256 of the edit prompt, `png_sha256` = the final
  watermarked PNG, `timestamp` fresh (Block 2.3).
- **Watermark:** LSB detection of the MLGV magic + model-hash32.
- **VAE encoder from the file header (a MANDATORY cross-check step, Block 1.1):**
  the run log carries a loud note from `VaeEncoder::from_weights`; the arithmetic
  of the manifest (244 = 138 decoder + 106 encoder) predicts the ABSENCE
  of `quant_conv` in the file → the expected note is "no quant_conv tensors …
  138 + 106 = 244". If `quant_conv.*` is actually present — the note changes
  to pin consumption; if the list of non-decoder keys diverges from
  the generator (`vae_expected_encoder_keys`) — the test fails LOUDLY with
  the list of what is missing: enter the file fact into the manifest doc, the generator fix
  — as a separate loud fix-forward (improvising on a live run is forbidden).
- **Edit-path determinism:** a re-run of the same command → the SHA
  of `naryad_243_mlog_first_edit.png` bit-for-bit (the inherited seed 42,
  deterministic encode in mode — Block 2.3).

The edit golden is NOT pinned in this run (a pin = a separate loud decision;
run acceptance — by the §0 go-no-go criteria: recognizability + latency).

## 3.2. lora-e2e run (Naryad #244, R6.3 — same session)

Loading a LoRA adapter and generating with it. **The adapter file is placed by the
machine owner** (pinning a LoRA file into the repo is forbidden by §3 of #244): put a
safetensors LoRA adapter (Z-Image attention targets — `layers.N.attention.to_q/to_k/to_v/
to_out.0`, diffusers-PEFT `lora_A/lora_B` or ComfyUI `lora_down/lora_up`
+ an optional `alpha`) under weights_dir, e.g.:

```bash
mkdir -p "$MLOG_VISION_WEIGHTS_DIR/lora"
cp /path/to/my_adapter.safetensors "$MLOG_VISION_WEIGHTS_DIR/lora/"
```

A separate env-gated test is NOT added to CI (an owner debt — a CI step for
the env-gated `naryad_240_vision_mlog_e2e`, round 5; the run is manual, per
the templates of §3). The control wedge (writing an `.mlog` program: a `db` declaration →
`vision_lora_load("my-adapter", "lora/my_adapter.safetensors")` →
`vision_lora_generate("poster", "prompt", "my-adapter")` → export) —
manually via `mlog` in the same session. What is cross-checked against the file header
(the #243 quant_conv template):

- **Target keys from the header:** the loud output of `vision_lora_load`
  ("adapter 'my-adapter' validated — N target(s), rank R, alpha …, scale …");
  the actual list of targets in the file must be a subset of
  the `zimage_expected_keys` attention projections — other keys = a loud Err with
  the list (this is validation, not silent dropping).
- **Dtype:** non-F32 adapter tensors are upcast LOUDLY — record
  the actual dtype of the file into the report (for a future format decision).
- **The composite in the manifest:** the sidecar of the exported artifact —
  `model_sha256 = sha256("{base}\nlora:my-adapter:{lora_sha256}")`,
  where `base` — the weights-tree fingerprint (or "unpinned" — the composite is honest
  over the marker), `lora_sha256` = the SHA of the adapter bytes; `model_id` = the base
  (z-image-turbo); the watermark = the base model (the adapter is a delta).
- **Integrity:** a repeat call of `vision_lora_generate` after a manual
  UPDATE of bytes in `vision_lora_adapters` must fail loudly
  ("integrity failure") — the pin is verified on EVERY resolve from the DB.
- **Signature:** the 7 manifest fields, the policy from the decl, the MLGV watermark + the hash
  of the base model.
- **lora-path determinism:** two runs of the same .mlog program → the SHA
  of the final PNG bit-for-bit (the seed from the decl, a deterministic
  sorted merge order of the targets).

The lora golden is NOT pinned in this run (a pin = a separate loud decision;
acceptance — by the §0 criteria: recognizability + latency + the adapter's effect
on the output compared to generate without the adapter).

## 4. Recording the result (filling the "REQUIRES REAL RUN" slots)

The slots — in `docs/research/naryad-212-go-no-go.md`, the section "Verbatim DoD entries".
Fill verbatim from the output and the file system:

| Slot | Source |
|------|----------|
| PNG path | the test's stdout (`clinical_e2e_first_image: PNG path=…`) |
| PNG SHA-256 | `sha256sum $MLOG_VISION_OUT/first_image.png` |
| PNG size | `wc -c $MLOG_VISION_OUT/first_image.png` |
| Timings (tokenize / encode / sampler / decode) | the test's stderr (`clinical_e2e: tokenize: … / encode … / sampler … / decode …`) — copy verbatim |
| Determinism (2 runs bit-exact) | a second run of the same test + comparing the PNG SHA |
| Hardware | `nproc`; `free -g`; `uname -r`; GPU presence (the CPU path — none) |

The determinism run:

```bash
cargo test --features vision --test naryad_212_wedge_e2e clinical_e2e_first_image -- --nocapture 2>&1 | tee n237_run2.log
sha256sum $MLOG_VISION_OUT/first_image.png   # must match run1 bit-for-bit
```

If the run did not reach some slot — the slot stays empty (`<REQUIRES
REAL RUN>`), with a loud note on which step it stopped at and why.

## 5. Subjective quality gate

Open `first_image.png`. Answer honestly: is the image recognizable (a red
apple on a wooden table)? Classification: noise / structure without an object /
a recognizable object. This is the input to the "Quality acceptability" criterion — the
coordinator decides, the executor records the observation.

## 6. Invariants during the run

- `src/**` and `tests/**` are not touched (the code is GO-ready per #236). A discovered
  defect → a separate fix-forward naryad, not an ad-hoc edit in the middle of the run.
- The ignore formula (truth-up #237 Block 2.3 — "96/0" is not reproducible):
  `git grep -c '#\[ignore' HEAD -- src tests` = **129** on the base `1f26f41`/`396b1df`
  (125 tests + 4 src); the invariant = "enter N as the actual number, the delta to the base = 0".
- Weights are not in git: `git ls-tree -r HEAD --name-only | grep -ciE 'safetensors|\.ckpt$|\.pth$|\.gguf$'` = 0.
- Goldens are not re-pinned: VAE `85ef6a879d58…`, DiT `860c85b311905f6c23b90a4e9e3192928027a24bf3e4a00a08096336abad4b3c` (n231: `e686167b2e82…` — pre-rebuild architecture) — test constants.

## 7. Report

The run is issued as a SEPARATE report by the machine owner (per this runbook), not
as commits of naryad #237 (§3.5): the filled go-no-go slots + PNG metadata +
timings + the coordinator's Go/No-Go conclusion. Naryad #237 delivers the tooling and
the protocol; the run is the next unit of work.
