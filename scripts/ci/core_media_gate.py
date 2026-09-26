#!/usr/bin/env python3
"""№472 (gh#693): the physical core→media ban — the checked-in CI fact.

Decision 2-B (gate gh#680): the media isolation is the FIRST step of the
crate split (the physical split is 0.27+). The language is "core + std,
everything else — libraries"; core may not grow media dependencies
silently.

The gate scans the CORE files (the language: parser, ast, compiler,
bytecode, the interpreter tree, the VM, the pool, the audit and semantic
lanes) for references to the MEDIA MODULES (`crate::vision`, `crate::video`,
`crate::voice`, `crate::media`) and enforces the boundary:

  - the ALLOWED surface is the handle/registry tier only — the opaque
    handle types, the registry/store types and the media-METADATA
    helpers (MediaKind::from_slug, parse_sensitivity,
    KNOWN_VISION_MODELS) that the compiler and the semantic lane need
    for the compile-time validation of the media DECLS;
  - everything else (the inference stacks, the encoders, the pipelines,
    the VAEs, the weights machinery — the heavy tier behind the
    vision/video/voice/candle features) is a BAN: a core file importing
    it fails CI.

  - additionally, no core file may reference the heavy media
    DEPENDENCIES (candle, tokenizers, image, ffmpeg...) directly —
    those stay behind the feature gates inside the media modules.

Usage:
  core_media_gate.py            → exit 0 green / 1 violation
  core_media_gate.py --list     → the core→media references found
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

CORE = [
    "src/ast.rs",
    "src/compiler.rs",
    "src/bytecode.rs",
    "src/vm.rs",
    "src/vm_pool.rs",
    "src/audit.rs",
    "src/semantic.rs",
] + [str(p.relative_to(ROOT)) for p in sorted((ROOT / "src" / "parser").glob("**/*.rs"))] + [
    str(p.relative_to(ROOT)) for p in sorted((ROOT / "src" / "interpreter").glob("**/*.rs"))
]

MEDIA_MODS = ("vision", "video", "voice", "media")
REF_RE = re.compile(
    r"\bcrate::(vision|video|voice|media)::([A-Za-z_][A-Za-z0-9_]*)"
)

# The handle/registry/metadata tier — the ONLY surface core may touch
# (the naryad: «медиа-путь — только через реестр хэндлов/бэкендов»).
ALLOWED = {
    "vision": {
        "VisionRegistry",
        "SharedVisionRegistry",
        "VisionId",
        "VisionDecl",
        "KNOWN_VISION_MODELS",
    },
    "video": {
        "VideoId",
        "VideoRegistry",
    },
    "voice": {
        "VoiceId",
        "AudioId",
        "VoiceRegistry",
    },
    "media": {
        "MediaStore",
        "MediaHandle",
        "MediaKind",
        "parse_sensitivity",
    },
}

# The heavy media dependencies must never appear in core directly.
HEAVY_DEP_RE = re.compile(
    r"\b(candle_core|candle_nn|tokenizers|image::|ffmpeg|zip::)"
)


def strip_comments(text):
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return text


def main():
    args = sys.argv[1:]
    violations = []
    refs = []
    for rel in CORE:
        path = ROOT / rel
        if not path.exists():
            continue
        code = strip_comments(path.read_text(encoding="utf-8"))
        for m in REF_RE.finditer(code):
            mod, sym = m.group(1), m.group(2)
            refs.append((rel, mod, sym))
            if sym not in ALLOWED[mod]:
                violations.append(
                    f"{rel}: crate::{mod}::{sym} — outside the handle/registry "
                    f"allowlist (the heavy media tier must stay behind its "
                    f"feature and outside core; №472)"
                )
        for m in HEAVY_DEP_RE.finditer(code):
            violations.append(
                f"{rel}: a heavy media dependency reference `{m.group(1)}` — "
                f"core must not reference the media dependency surface (№472)"
            )
    if "--list" in args:
        for rel, mod, sym in sorted(refs):
            print(f"{rel}: crate::{mod}::{sym}"
                  f"{'  [allowed]' if sym in ALLOWED[mod] else '  [VIOLATION]'}")
        print(f"\ncore->media references: {len(refs)}, "
              f"violations: {len(violations)}")
    else:
        print(f"core->media references: {len(refs)} "
              f"(the handle/registry tier only)")
    if violations:
        print("\n".join(f"::error::{v}" for v in violations))
        print(f"::error::the core→media ban violated: {len(violations)} "
              f"reference(s) outside the allowlist (№472, decision 2-B)")
        sys.exit(1)
    print("core→media gate: OK (the media path goes through the "
          "handle/registry tier only)")


if __name__ == "__main__":
    main()
