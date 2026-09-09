# Наряд №212 — Weights Manifest (template)

**Purpose:** SHA-256 manifest for all weight and tokenizer files used by the R3 e2e pipeline.
**Status:** Template — SHA-256 values are filled in by the executor at weight-download time (manually, per §0.2 of the naryad spec). This file lives in the repo; the actual weights do NOT.

## Weights directory layout

Expected at `$MLOG_VISION_WEIGHTS_DIR`. Sizes verified 2026-09-09 via the HF
models API (`/api/models/Tongyi-MAI/Z-Image-Turbo?blobs=true`) — the earlier
"~" approximations (notably text_encoder shard 3 and tokenizer_config.json)
were inaccurate and are replaced by real byte counts:

```
{weights_dir}/
├── text_encoder/
│   ├── model-00001-of-00003.safetensors     (3957900840 bytes ≈ 3.96 GB)
│   ├── model-00002-of-00003.safetensors     (3987450520 bytes ≈ 3.99 GB)
│   ├── model-00003-of-00003.safetensors     (99630640 bytes ≈ 99.6 MB)
│   ├── model.safetensors.index.json          (32819 bytes)
│   └── config.json                           (726 bytes)
├── transformer/
│   ├── diffusion_pytorch_model-00001-of-00003.safetensors   (9973693184 bytes ≈ 9.97 GB)
│   ├── diffusion_pytorch_model-00002-of-00003.safetensors   (9973714824 bytes ≈ 9.97 GB)
│   ├── diffusion_pytorch_model-00003-of-00003.safetensors   (4672282880 bytes ≈ 4.67 GB)
│   ├── diffusion_pytorch_model.safetensors.index.json       (48969 bytes)
│   └── config.json                                          (473 bytes)
├── vae/
│   ├── diffusion_pytorch_model.safetensors   (167666902 bytes ≈ 167.7 MB)
│   └── config.json                          (805 bytes)
└── tokenizer/
    ├── tokenizer.json                       (11422654 bytes ≈ 11.4 MB)
    ├── vocab.json                           (2776833 bytes ≈ 2.8 MB)
    ├── merges.txt                           (1671853 bytes ≈ 1.7 MB)
    └── tokenizer_config.json                 (9732 bytes)
```

Total, all 16 files: **32848304654 bytes ≈ 32.85 GB** (sum of the verified
sizes above). Free-disk requirement for the real-weights run stays **≥ 40 GB**
(weights + PNG output + working headroom); note the older go-no-go estimate
"~24.6 GB total" was an underestimate of the same source.

All 16 files — including `text_encoder/` — live in the single HF repo
`Tongyi-MAI/Z-Image-Turbo` (verified 2026-09-09: its `text_encoder/config.json`
= 726 B and `text_encoder/model.safetensors.index.json` = 32819 B are
byte-size identical to Qwen/Qwen3-4B's; the TE is bundled in the Turbo repo,
not a submodule download).

## How to verify against the source (Как сверять с источником)

- **HF LFS oid = SHA-256 of the file** (for LFS-backed files — all 7
  safetensors + `tokenizer.json`). It is visible in the model's files
  metadata: `https://huggingface.co/api/models/Tongyi-MAI/Z-Image-Turbo?blobs=true`
  → `siblings[].lfs.sha256` (equivalently `lfs.oid` on the tree endpoint).
- **Rule:** local `sha256sum` ≠ oid → **loud refusal, the file is not
  consumed**. This is enforced mechanically by
  `tools/fetch_vision_weights.sh` (naryad №237 Block 2.1): post-download
  mismatch → `POST-DOWNLOAD REFUSAL`, non-zero exit; existing-file match →
  loud `SKIP`.
- **Non-LFS small files** (`*.json`, `merges.txt`) carry no pre-download
  sha256 reference anywhere (their git blob oid is NOT a file digest). For
  these: verify size against the API metadata, record the post-download
  `sha256sum` into the tables below, and treat the recorded value as the
  reference from then on.
- Canonical fetch path: `tools/fetch_vision_weights.sh` (manifest-driven,
  `curl -L -C -` resume, per-file checksum discipline, `--dry-run` plan,
  `--only <subdir>` for component-scoped fetch).

## SHA-256 manifest (executor fills in)

To compute the SHAs (after downloading):

```bash
cd $MLOG_VISION_WEIGHTS_DIR
find . -type f \( -name "*.safetensors" -o -name "*.json" -o -name "*.txt" \) -print0 | \
  xargs -0 sha256sum | sort
```

### Text encoder (Qwen3-4B)

| file | sha256 | bytes |
|------|--------|-------|
| `text_encoder/model-00001-of-00003.safetensors` | _TODO_ | |
| `text_encoder/model-00002-of-00003.safetensors` | _TODO_ | |
| `text_encoder/model-00003-of-00003.safetensors` | _TODO_ | |
| `text_encoder/model.safetensors.index.json` | _TODO_ | 32819 |
| `text_encoder/config.json` | _TODO_ | 726 |

### Transformer (DiT)

| file | sha256 | bytes |
|------|--------|-------|
| `transformer/diffusion_pytorch_model-00001-of-00003.safetensors` | _TODO_ | |
| `transformer/diffusion_pytorch_model-00002-of-00003.safetensors` | _TODO_ | |
| `transformer/diffusion_pytorch_model-00003-of-00003.safetensors` | _TODO_ | |
| `transformer/diffusion_pytorch_model.safetensors.index.json` | _TODO_ | 48969 |
| `transformer/config.json` | _TODO_ | 473 |

### VAE (AutoencoderKL flux-dev-style)

| file | sha256 | bytes |
|------|--------|-------|
| `vae/diffusion_pytorch_model.safetensors` | _TODO_ | ~167 MB |
| `vae/config.json` | _TODO_ | 805 |

### Tokenizer (Qwen2Tokenizer)

The four tokenizer files are small (≈16 MB total) and were fetched in the
delivery environment on 2026-09-09 via `tools/fetch_vision_weights.sh
--only tokenizer` (twice — download run + SKIP re-run — identical values).
These rows are real `sha256sum`/`wc -c` output, not placeholders. The heavy
weights remain _TODO_ (naryad №237 Block 2.2, branch (б): no ≥40 GB machine
in the delivery environment — 9.2 GB free).

| file | sha256 | bytes |
|------|--------|-------|
| `tokenizer/tokenizer.json` | aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4 | 11422654 |
| `tokenizer/vocab.json` | ca10d7e9fb3ed18575dd1e277a2579c16d108e32f27439684afa0e10b1440910 | 2776833 |
| `tokenizer/merges.txt` | 8831e4f1a044471340f7c0a83d7bd71306a5b867e95fd870f74d0c5308a904d5 | 1671853 |
| `tokenizer/tokenizer_config.json` | d5d09f07b48c3086c508b30d1c9114bd1189145b74e982a265350c923acd8101 | 9732 |

## Verification policy

`src/vision/weights.rs::load_safetensors_sharded` will read this manifest (if present in `$MLOG_VISION_WEIGHTS_DIR/manifest.md` or a sibling JSON file — implementation TBD in Block 1) and verify each loaded shard's SHA-256 against the manifest. Mismatch → loud error, no silent fallback.

## Download instructions (executor)

**Canonical (naryad №237):** `tools/fetch_vision_weights.sh` — manifest-driven,
checksum-disciplined, downloads directly into the layout above (no restructure
step), resumable (`curl -L -C -`), idempotent (sha-verified SKIP on re-run):

```bash
export MLOG_VISION_WEIGHTS_DIR=/abs/path/to/weights
tools/fetch_vision_weights.sh --dry-run        # offline plan: URL -> path, SKIP/download
tools/fetch_vision_weights.sh                  # full fetch (~32.85 GB)
tools/fetch_vision_weights.sh --only tokenizer # component-scoped fetch (small, testable)
```

Paste the script's printed `| file | sha256 | bytes |` lines into the tables
below (replacing _TODO_) — every number must be real command output.

Historical manual path (superseded by the script, kept for reference):

```bash
# Requires: pip install huggingface-hub
huggingface-cli download Tongyi-MAI/Z-Image-Turbo \
  --local-dir $MLOG_VISION_WEIGHTS_DIR \
  --local-dir-use-symlinks False
# ... then restructure to match the layout above and compute SHAs manually.
```

Auto-download is **R5 territory** (ADR-0125) — not in this naryad.

## Tensor counts (fetched 2026-09-08 from HF index.json files)

| Component | Tensor count | Source |
|-----------|--------------|--------|
| DiT (transformer) | 521 | transformer/diffusion_pytorch_model.safetensors.index.json |
| VAE (decoder + encoder) | 244 | vae/diffusion_pytorch_model.safetensors (header parse) |
| VAE decoder-only | 138 | (subset of VAE, prefix `decoder.`) |
| TextEncoder (Qwen3-4B) | 398 | text_encoder/model.safetensors.index.json |

These counts are the expected constants for the loader tensor-coverage guard
(`check_tensor_coverage` in `src/vision/weights.rs`).
