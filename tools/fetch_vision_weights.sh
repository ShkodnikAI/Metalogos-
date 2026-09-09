#!/usr/bin/env bash
# tools/fetch_vision_weights.sh — manifest-driven fetcher for Z-Image-Turbo weights.
#
# Naryad №237 (Vision R3.7). The file list is read from the SHA-256 tables of
# docs/research/naryad-212-weights-manifest.md — the manifest is the SSOT for
# the layout; this script owns no second copy of it.
#
# All files (including text_encoder/) are downloaded from the
# Tongyi-MAI/Z-Image-Turbo repo resolve endpoints — verified 2026-09-09 via
# the HF models API: text_encoder/{config.json=726B, model.safetensors.index
# .json=32819B} are byte-size identical to the Qwen3-4B files the manifest
# expects and are bundled in the Turbo repo itself.
#
# Checksum discipline (naryad №237 Block 1 + Block 2.1):
#   - reference sha256 per file = manifest table value when filled (real run)
#     else HF LFS oid (lfs.sha256 in models API metadata) when the file is
#     LFS-backed; small non-LFS files (json/txt) have no pre-download sha256
#     reference anywhere — they are verified by size and post-download sha
#     recording, loudly.
#   - existing file + sha match        -> loud SKIP (idempotent re-runs)
#   - existing file + sha mismatch     -> resume via curl -L -C -
#   - post-download sha mismatch       -> loud refusal, file is NOT consumed
#   - any network failure              -> loud non-zero exit, no skip-and-go
#
# Dependencies: bash, curl, sha256sum, wc, du, find, grep, sed (coreutils).
# Weights NEVER go into git (naryad §3.2); only their sha256/bytes do.

set -euo pipefail

REPO_BASE="https://huggingface.co/Tongyi-MAI/Z-Image-Turbo/resolve/main"
API_URL="https://huggingface.co/api/models/Tongyi-MAI/Z-Image-Turbo?blobs=true"
MANIFEST_REL="docs/research/naryad-212-weights-manifest.md"

die() { echo "FATAL: $*" >&2; exit 1; }
warn() { echo "LOUD:  $*" >&2; }

usage() {
  cat >&2 <<'EOF'
USAGE:
  MLOG_VISION_WEIGHTS_DIR=/abs/path tools/fetch_vision_weights.sh [--dry-run] [--only SUBDIR]

  MLOG_VISION_WEIGHTS_DIR  absolute target directory for the weights tree
                           (mandatory; nothing is downloaded without it)
  --dry-run                print the plan (URL -> path, SKIP/download) and exit;
                           no network, no filesystem writes
  --only SUBDIR            restrict to one component dir:
                           text_encoder | transformer | vae | tokenizer

  File list source (SSOT): the SHA-256 tables of
    docs/research/naryad-212-weights-manifest.md
EOF
  exit 2
}

DRY_RUN=0
ONLY=""
ONLY_PENDING=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --only) [[ $# -ge 2 ]] || usage; ONLY="$2"; shift 2 ;;
    --only=*) ONLY="${1#--only=}"; shift ;;
    *) usage ;;
  esac
done
case "$ONLY" in
  ""|text_encoder|transformer|vae|tokenizer) ;;
  *) die "--only accepts: text_encoder | transformer | vae | tokenizer (got: $ONLY)" ;;
esac

# ── Mandatory target dir ───────────────────────────────────────────
if [[ -z "${MLOG_VISION_WEIGHTS_DIR:-}" ]]; then
  warn "MLOG_VISION_WEIGHTS_DIR is not set — refusing to guess a download location."
  usage
fi
TARGET_DIR="${MLOG_VISION_WEIGHTS_DIR}"
case "$TARGET_DIR" in
  /*) ;;
  *) warn "MLOG_VISION_WEIGHTS_DIR must be an ABSOLUTE path (got: $TARGET_DIR)"; usage ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="${SCRIPT_DIR}/../${MANIFEST_REL}"
[[ -f "$MANIFEST" ]] || die "manifest not found: $MANIFEST (SSOT required, naryad №237)"

# ── Parse the manifest SHA tables (SSOT for the file list) ────────
# Table rows look like: | `text_encoder/config.json` | _TODO_ | 726 |
FILES=()
while IFS= read -r row; do
  path="$(printf '%s' "$row" | sed -E 's/^\| *`([^`]+)` *\|.*/\1/')"
  [[ "$path" == */* ]] || continue   # header/noise guard: real rows are subdir/name
  FILES+=("$path")
done < <(grep -E '^\| *`[^`]+` *\|' "$MANIFEST" || true)
[[ ${#FILES[@]} -gt 0 ]] || die "no file rows parsed from manifest tables: $MANIFEST"

if [[ -n "$ONLY" ]]; then
  FILTERED=()
  for f in "${FILES[@]}"; do [[ "$f" == "$ONLY"/* ]] && FILTERED+=("$f"); done
  [[ ${#FILTERED[@]} -gt 0 ]] || die "--only $ONLY matched 0 manifest rows"
  FILES=("${FILTERED[@]}")
fi

# ── Reference metadata from HF (fetch mode only; dry-run is offline) ─
declare -A API_SIZE=() API_SHA=()
fetch_api_metadata() {
  local tmp; tmp="$(mktemp)"
  if ! curl -fsSL --retry 2 --max-time 60 "$API_URL" -o "$tmp"; then
    rm -f "$tmp"
    die "HF models API unreachable ($API_URL) — without it LFS sha256 references
     cannot be resolved; refusing to download unverified files (naryad №237 Block 2.1)."
  fi
  # one sibling object per line for safe per-file pairing
  local records; records="$(sed 's/},{/}\n{/g' "$tmp")"
  rm -f "$tmp"
  local f rec sha
  for f in "${FILES[@]}"; do
    rec="$(printf '%s\n' "$records" | grep -F "\"rfilename\":\"$f\"" || true)"
    [[ -n "$rec" ]] || die "file '$f' not found in HF model metadata — manifest/repo drift; refusing."
    sha="$(printf '%s' "$rec" | sed -nE 's/.*"lfs":\{"sha256":"([0-9a-f]{64})".*/\1/p' || true)"
    API_SIZE[$f]="$(printf '%s' "$rec" | sed -nE 's/^.*"rfilename":"[^"]*"[^{]*"size":([0-9]+).*/\1/p')"
    if [[ -n "$sha" ]]; then API_SHA[$f]="$sha"; fi
  done
}

manifest_sha() {  # manifest table sha, if it is a real 64-hex digest
  local row sha
  row="$(grep -E "^\| *\`$1\` *\|" "$MANIFEST" || true)"
  [[ -n "$row" ]] || return 0
  sha="$(printf '%s' "$row" | sed -E 's/^\| *`[^`]+` *\| *([^ |]+) *.*/\1/')"
  [[ "$sha" =~ ^[0-9a-f]{64}$ ]] && printf '%s' "$sha"
  return 0
}

file_sha() { sha256sum "$1" | cut -d' ' -f1; }
file_bytes() { wc -c < "$1" | tr -d ' '; }

print_manifest_line() {  # path sha bytes — ready to paste into the tables
  printf '| %s | %s | %s |\n' "$1" "$2" "$3"
}

# ── Plan ───────────────────────────────────────────────────────────
echo "== fetch_vision_weights: ${#FILES[@]} file(s) from Tongyi-MAI/Z-Image-Turbo"
echo "== target: $TARGET_DIR$([[ -n "$ONLY" ]] && printf ' (only: %s)' "$ONLY")"
if [[ $DRY_RUN -eq 1 ]]; then
  echo "== mode: DRY-RUN (offline plan, nothing will be downloaded)"
  for f in "${FILES[@]}"; do
    t="${TARGET_DIR}/${f}"
    msha="$(manifest_sha "$f")"
    if [[ -f "$t" ]]; then
      if [[ -n "$msha" ]]; then
        [[ "$(file_sha "$t")" == "$msha" ]] && st="SKIP (sha matches manifest)" \
          || st="RESUME (sha differs from manifest)"
      else
        st="RESUME (exists; no manifest sha yet — verify post-download)"
      fi
    else
      st="DOWNLOAD (missing)"
    fi
    printf '  [%s] %s\n        %s\n' "$st" "$f" "$REPO_BASE/$f"
  done
  echo "== dry-run complete: no writes performed"
  exit 0
fi

fetch_api_metadata

FAILURES=0
for f in "${FILES[@]}"; do
  t="${TARGET_DIR}/${f}"
  ref_sha="$(manifest_sha "$f")"
  api_sha="${API_SHA[$f]:-}"
  api_size="${API_SIZE[$f]:-}"
  if [[ -z "$ref_sha" && -n "$api_sha" ]]; then
    ref_sha="$api_sha"
    ref_note="HF LFS oid"
  elif [[ -n "$ref_sha" ]]; then
    ref_note="manifest"
  else
    ref_note="none (non-LFS small file)"
  fi

  mkdir -p "$(dirname "$t")"

  if [[ -f "$t" ]]; then
    cur_sha="$(file_sha "$t")"
    if [[ -n "$ref_sha" && "$cur_sha" == "$ref_sha" ]]; then
      echo "SKIP ($ref_note sha verified): $f"
      print_manifest_line "$f" "$cur_sha" "$(file_bytes "$t")"
      continue
    fi
    if [[ -z "$ref_sha" && -n "$api_size" && "$(file_bytes "$t")" == "$api_size" ]]; then
      warn "$f exists, size matches HF ($api_size B) but NO sha256 reference exists
       for non-LFS files pre-download. Treated as complete UNVERIFIED — record
       its sha256 (printed below) into the manifest tables now."
      print_manifest_line "$f" "$cur_sha" "$(file_bytes "$t")"
      continue
    fi
    warn "$f exists but sha256 differs from reference ($ref_note) — resuming download."
  fi

  echo "FETCH: $f"
  if ! curl -fL -C - --retry 3 --retry-delay 2 -o "$t" "$REPO_BASE/$f"; then
    warn "curl failed on $f. If the file is already fully downloaded but has no
     verifiable sha reference, delete it or record its sha manually; otherwise
     fix the network and re-run. NO skip-and-go (naryad №237 Block 1.1)."
    FAILURES=$((FAILURES + 1))
    continue
  fi

  got_sha="$(file_sha "$t")"
  got_bytes="$(file_bytes "$t")"
  if [[ -n "$ref_sha" && "$got_sha" != "$ref_sha" ]]; then
    warn "POST-DOWNLOAD REFUSAL: $f sha256=$got_sha != reference ($ref_note)=$ref_sha.
     The file will NOT be consumed (naryad №237 Block 2.1). Delete it and re-run."
    FAILURES=$((FAILURES + 1))
    continue
  fi
  if [[ -z "$ref_sha" ]]; then
    warn "$f downloaded; no reference sha existed — record the line below in the manifest."
  fi
  print_manifest_line "$f" "$got_sha" "$got_bytes"
done

if [[ $FAILURES -gt 0 ]]; then
  die "$FAILURES file(s) failed checksum/network discipline — see LOUD notes above."
fi

echo "== total size:"
du -sb "$TARGET_DIR"
echo "== done. Paste the printed manifest lines into $MANIFEST_REL (replace _TODO_)."
