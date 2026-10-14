# Feature Gates

The feature-flag surface of the `mlog` binary. Moved from the README root
by Naryad №676 (gh#1203, wave 41, item D2 — the front-door rework, the
#1191 precedent: deep reference content lives in `docs/`, the README
links instead of duplicating).

| Feature | Default | What it gates | The heavy tier |
| --- | --- | --- | --- |
| *(none — core)* | ✓ | the language: parser, compiler, the TW/VM backends, audit, semantic | — |
| `svg` / `chart` / `diagram` / `template` | on | the document/graphics family (pure Rust) | — |
| `llm` | on | the LLM client surface (HTTP, no local inference) | — |
| `server` | off | the HTTP/SSE serve stack (axum/tokio) | — |
| `candle` | off | the tensor backend for the generative stacks | candle-core/nn |
| `vision` | off | the image pillar: dit/vae/sampler/encoders (implies `candle`) | tokenizers, image |
| `voice` | off | the voice pillar: TTS/clone/design (implies `candle`) | — |
| `video` | off | the video pillar: T2V/I2V/interp/mux (implies `candle`) | — |
| `vec` | off | the vector contour (sqlite-vec) | — |
| `timesfm` | off | the off-process timesfm-2.5 rung of the `timeseries` ladder | off-process |

The media-path invariant: **core never imports the media processing** —
the interpreter/compiler/VM/audit touch only the handle/registry tier
(`MediaStore`, `MediaHandle`, `MediaKind`, `VisionRegistry`, `VisionId`,
`VideoId`, `VoiceId`, `AudioId`...), enforced by the `core-media-gate` CI
job (`scripts/ci/core_media_gate.py`). The core builds without media:
`cargo build --no-default-features` is a blocking CI job.
