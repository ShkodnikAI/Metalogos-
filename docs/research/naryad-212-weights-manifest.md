# Наряд №212 — Weights Manifest (template)

**Purpose:** SHA-256 manifest for all weight and tokenizer files used by the R3 e2e pipeline.
**Status:** Template — SHA-256 values are filled in by the executor at weight-download time (manually, per §0.2 of the naryad spec). This file lives in the repo; the actual weights do NOT.

## Weights directory layout

Expected at `$MLOG_VISION_WEIGHTS_DIR`:

```
{weights_dir}/
├── text_encoder/
│   ├── model-00001-of-00003.safetensors     (~2.7 GB)
│   ├── model-00002-of-00003.safetensors     (~2.7 GB)
│   ├── model-00003-of-00003.safetensors     (~2.1 GB)
│   ├── model.safetensors.index.json          (32819 bytes)
│   └── config.json                           (726 bytes)
├── transformer/
│   ├── diffusion_pytorch_model-00001-of-00003.safetensors   (~8 GB)
│   ├── diffusion_pytorch_model-00002-of-00003.safetensors   (~8 GB)
│   ├── diffusion_pytorch_model-00003-of-00003.safetensors   (~7 GB)
│   ├── diffusion_pytorch_model.safetensors.index.json       (48969 bytes)
│   └── config.json                                          (473 bytes)
├── vae/
│   ├── diffusion_pytorch_model.safetensors   (~167 MB)
│   └── config.json                          (805 bytes)
└── tokenizer/
    ├── tokenizer.json                       (~11 MB)
    ├── vocab.json                           (~2.7 MB)
    ├── merges.txt                           (~1.6 MB)
    └── tokenizer_config.json                 (~600 bytes)
```

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

| file | sha256 | bytes |
|------|--------|-------|
| `tokenizer/tokenizer.json` | _TODO_ | ~11 MB |
| `tokenizer/vocab.json` | _TODO_ | ~2.7 MB |
| `tokenizer/merges.txt` | _TODO_ | ~1.6 MB |
| `tokenizer/tokenizer_config.json` | _TODO_ | ~600 |

## Verification policy

`src/vision/weights.rs::load_safetensors_sharded` will read this manifest (if present in `$MLOG_VISION_WEIGHTS_DIR/manifest.md` or a sibling JSON file — implementation TBD in Block 1) and verify each loaded shard's SHA-256 against the manifest. Mismatch → loud error, no silent fallback.

## Download instructions (executor)

```bash
# Requires: pip install huggingface-hub
huggingface-cli download Tongyi-MAI/Z-Image-Turbo \
  --local-dir $MLOG_VISION_WEIGHTS_DIR \
  --local-dir-use-symlinks False

# Then restructure to match the layout above (HF downloads to flat dir):
mkdir -p $MLOG_VISION_WEIGHTS_DIR/{text_encoder,transformer,vae,tokenizer}
mv $MLOG_VISION_WEIGHTS_DIR/text_encoder/* $MLOG_VISION_WEIGHTS_DIR/text_encoder/ 2>/dev/null || true
# ... etc.

# Compute SHAs and update this file.
```

Auto-download is **R5 territory** (ADR-0125) — not in this naryad.
