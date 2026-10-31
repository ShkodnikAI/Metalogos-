#!/usr/bin/env python3
"""Generate the metrics block for README.md (Naryad #460, audit 25.09 §7.1).

The problem the naryad closed: README carried hand-written, contradictory
numbers ("~59 000 LOC" vs "src/ ~93 000 LOC" vs the real thing) that drifted
on every merge. The fix: the numbers are GENERATED from the repository state
by this script and embedded into README.md between explicit markers — the
same posture as `gen_reference.py` (the curated text stays hand-written; the
counts are computed).

№804 (gh#1551, the C2 positioning): the FULL table lives in
`docs/metrics.md` (one honest place for the numbers); the README front
page carries THREE rows max — the version, the leak-suite negatives/
positives, the CI link. The same mirrors apply: the full table is
recomputed independently in `tests/readme_consistency.rs`.

Mirrors the independent recomputation in `tests/readme_consistency.rs`:
the test derives the same numbers from the same sources and compares them
against the claims the README makes. If the generated block and the test
ever disagree, CI is red — by construction.

Run from the repo root:
    python3 scripts/gen_metrics.py [--check]

    --check   exit 1 if regeneration would change the file (CI-style gate)
"""

import datetime as _dt
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
README = REPO / "README.md"
METRICS_DOC = REPO / "docs" / "metrics.md"
PLAN_SUMMARY = REPO / "docs" / "PLAN-SUMMARY.md"
BACKENDS_RS = REPO / "src" / "backends.rs"
MODEL_PASSPORT_RS = REPO / "src" / "model_passport.rs"
GATE_033_GOALS = REPO / "scripts" / "ci" / "gate_033_goals.txt"
# №846 (wave 72 — W-B): the 8B-1 machine fact — GRANT_COVERAGE with its
# own only-up floor file. The floor VALUE 10000 bp = 100.00% is the plan
# §23 number (the owner revision R38, 2026-10-28) — the checker pins it
# as the OWNER constant: the gate file can RAISE the floor, never lower
# it silently (the lowering attempt is the loud refusal below — the
# №776 transaction discipline).
GATE_034_GOALS = REPO / "scripts" / "ci" / "gate_034_goals.txt"
GRANT_COVERAGE_FLOOR_OWNER_BP = 10000
CAPABILITY_RS = REPO / "src" / "capability.rs"
BUILTINS_CLASSIFICATION_RS = REPO / "src" / "builtins_classification.rs"
MCP_SERVER_RS = REPO / "metalogos-server" / "src" / "mcp_server.rs"

# №857 (wave 74 — W-C): the 8F-1 machine fact — SEMA_EXTERNAL with its
# own only-up floor file. The floor VALUE 1 is the plan §23 number (the
# owner revision R38 fixed the 0.37 gate: SEMA_EXTERNAL ≥ 1). The
# definition lives in ADR-0204 §5: the number of EXTERNALLY consumable
# artifacts of the Portable Refusal Algebra that EXIST AND ARE LIVE on
# main — each counted ONLY on a real existence check plus a live wiring
# in the blocking CI workflow (no self-declaration; the tamper traps
# below prove the counter catches placeholders). The checker pins the
# owner constant: the gate file can RAISE the floor, never lower it
# silently (the №846 discipline, verbatim).
GATE_035_GOALS = REPO / "scripts" / "ci" / "gate_035_goals.txt"
SEMA_EXTERNAL_FLOOR_OWNER = 1
CI_YML = REPO / ".github" / "workflows" / "ci.yml"
POLICY_ACTION_YML = REPO / "action.yml"
FACADE_TOML = REPO / "refusal-wasm" / "Cargo.toml"
SPEC_ALGEBRA_PAGE = REPO / "docs" / "spec" / "refusal-algebra.md"
SPEC_README = REPO / "docs" / "spec" / "README.md"
CONFORMANCE_DIR = REPO / "tests" / "conformance"

# №841 (gh#1643, wave 71 — W-A): the 8D-1 machine facts — the wave's
# freshness window, the PYTHON TWIN of the №842 Rust constant
# (src/model_passport.rs::PASSPORT_FRESHNESS_DAYS). The two must move
# together — the tamper-test pins the twin relationship by value.
PASSPORT_FRESHNESS_DAYS = 180

BEGIN = "<!-- BEGIN GENERATED METRICS (scripts/gen_metrics.py — do not edit inside) -->"
END = "<!-- END GENERATED METRICS -->"

# №566 (gh#928; the audit 02.10 §7.2 W-4): the SECOND generated target —
# the machine facts the public digest (docs/PLAN-SUMMARY.md) quotes as
# CURRENT. The audit's finding: the digest carried a hand-written
# "now 2110 bp" while the fact was 3195 bp — the grantors' digest lagged
# the machine metrics. The fix mirrors the №460 posture: ONE generator,
# the numbers between explicit markers, the narrative stays hand-written
# (the №566 boundary: the digest's prose is the publisher's voice).
MD_BEGIN = "<!-- BEGIN GENERATED METRICS (scripts/gen_metrics.py — do not edit inside) -->"
MD_END = "<!-- END GENERATED METRICS -->"
PS_BEGIN = "<!-- BEGIN GENERATED NUMBERS (scripts/gen_metrics.py — do not edit inside) -->"
PS_END = "<!-- END GENERATED NUMBERS -->"


def count_total_builtins() -> int:
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    return len(re.findall(r"spec!\(", content))


def count_builtin_modules() -> int:
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    string_re = re.compile(r'"([^"]+)"')
    layer_re = re.compile(r'=>\s*"[^"]+"')
    # №467: the typed rows end with the signature string —
    # `spec!(..., handler, "Type")` — strip it so the last remaining
    # string is the category again (the type is not a module).
    type_re = re.compile(r'(;\s*[A-Za-z_0-9]+),\s*"[A-Za-z][A-Za-z0-9<>{}:,_|]*"(?:\s*,\s*"(?:stable|experimental|parked)")?\s*\)')
    categories = set()
    for line in content.splitlines():
        if "spec!(" not in line:
            continue
        code = line.split("//")[0]
        clean = layer_re.sub("", code)
        clean = type_re.sub(r"\1)", clean)
        strings = string_re.findall(clean)
        if strings:
            categories.add(strings[-1])
    return len(categories)


def count_typed_signatures() -> tuple[int, int, int]:
    """№467/№560: the general and PRECISE typed-signature counts over
    BUILTIN_REGISTRY (the same regex contract as
    scripts/ci/type_signature_share.py; precise = the bare scalar shapes
    String/Float/Bool/Unit)."""
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    spec_re = re.compile(r'spec!\("([a-z_0-9]+)"')
    typed_re = re.compile(
        r'spec!\("([a-z_0-9]+)",[^\n;]*;\s*[A-Za-z_0-9]+\s*,\s*"(?!stable"|experimental"|parked")([A-Za-z][A-Za-z0-9<>{}:,_|]*)"(?:\s*,\s*"(?:stable|experimental|parked)")?\s*\)'
    )  # №796: the tier tail is tolerated AND the type capture must not BE the tier word
    precise_types = {"String", "Float", "Bool", "Unit"}
    total = spec_re.findall(content)
    typed = typed_re.findall(content)
    precise = sum(1 for _, t in typed if t in precise_types)
    return len(typed), len(total), precise


def count_stable_typed() -> tuple[int, int]:
    """№809 (ADR-0199): the typed count over the STABLE tier — the PRIMARY
    definition (the C3-a T3 owner decision). Same regex contract as
    scripts/ci/type_signature_share.py::stable_share — the tier is the
    №796 trailing literal, the LAST tier word on the spec! line."""
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    spec_re = re.compile(r'spec!\("([a-z_0-9]+)"')
    typed_re = re.compile(
        r'spec!\("([a-z_0-9]+)",[^\n;]*;\s*[A-Za-z_0-9]+\s*,\s*"(?!stable"|experimental"|parked")([A-Za-z][A-Za-z0-9<>{}:,_|]*)"(?:\s*,\s*"(?:stable|experimental|parked)")?\s*\)'
    )
    tier_re = re.compile(r'"(stable|experimental|parked)"')
    tiers = {}
    for line in content.splitlines():
        nm = spec_re.search(line)
        if not nm:
            continue
        tm = tier_re.findall(line)
        if tm:
            tiers[nm.group(1)] = tm[-1]
    typed = typed_re.findall(content)
    stable_total = sum(1 for name in spec_re.findall(content) if tiers.get(name) == "stable")
    stable_typed = sum(1 for name, _ in typed if tiers.get(name) == "stable")
    return stable_typed, stable_total


def count_svg_builtins() -> int:
    content = (REPO / "src" / "builtins" / "registry.rs").read_text(encoding="utf-8")
    re_svg = re.compile(r'spec!\("(svg_|chart_|diagram_|color_palette|template_render|html_render)')
    return sum(1 for line in content.splitlines() if re_svg.search(line))


def count_parser_rules() -> int:
    content = (REPO / "src" / "grammar.pest").read_text(encoding="utf-8")
    re_rule = re.compile(r"^[a-zA-Z_][a-zA-Z_0-9]*\s*=")
    return sum(1 for line in content.splitlines() if re_rule.match(line))


def count_adrs() -> int:
    adr = REPO / "docs" / "adr"
    return sum(
        1
        for p in adr.iterdir()
        if p.suffix == ".md" and p.name != "README.md"
    )


def count_examples() -> int:
    examples = REPO / "examples"
    return sum(
        1
        for p in examples.iterdir()
        if p.is_file() and p.suffix == ".mlog"
    )


def file_kb(rel: str) -> int:
    return (REPO / rel).stat().st_size // 1024


def cargo_version() -> str:
    content = (REPO / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'^version\s*=\s*"([^"]+)"', content, re.M)
    return m.group(1)


def leak_counts() -> tuple[int, int]:
    """№804: the leak-suite showcase numbers — the negative scenarios are
    the .error files (one per refused program), the positives are the
    ok_*.mlog programs that run clean. The BLOCKING runner
    (tests/run_leak_suite.rs) is the gate; these counts are its census."""
    leak_dir = REPO / "examples" / "leak"
    neg = sum(1 for p in leak_dir.iterdir() if p.suffix == ".error")
    pos = sum(1 for p in leak_dir.iterdir() if p.is_file() and p.suffix == ".mlog" and p.name.startswith("ok_"))
    return neg, pos


# ── №841 (gh#1643, wave 71 — W-A): the 8D-1 machine facts ────────────
#
# LICENSED_FLOOR (bps, only-up): the share of the backend-registry
# entries whose license class is EXPLICIT and whose license_note is
# filled (the ADR-0163 terminology, verbatim). The data source is the
# SSOT file (src/backends.rs) parsed the same way the other counters
# parse their sources — a record without the explicit class or with an
# empty note LOWERS the floor (the tamper-test proves the checker
# catches it).
#
# PASSPORT_FRESH (bps, only-up): the share of the №838 passports whose
# verification date is fresh — the №842 rule (the age within
# PASSPORT_FRESHNESS_DAYS; the boundary inclusive; an unparseable date
# is NOT fresh — fail-closed). The data source is the PASSPORT_META
# table of src/model_passport.rs (the const names resolved).
#
# Both ride --check (the checkist: the floors of gate_033_goals.txt,
# only-up) and --tamper-test (the RED harness — the checker must catch
# the fabricated records). The text follows ADR-0202's terminology.


def _registry_license_rows(content: str) -> list[tuple[str | None, str | None]]:
    """The BackendEntry rows of the registry SSOT: (class word, note)."""
    try:
        block = content[content.index("pub const BACKEND_REGISTRY"):]
        block = block[: block.index("\n];")]
    except ValueError:
        return []
    rows: list[tuple[str | None, str | None]] = []
    for chunk in block.split("BackendEntry {")[1:]:
        lic = re.search(r"license:\s*LicenseClass::(\w+)", chunk)
        note = re.search(r'license_note:\s*"((?:[^"\\]|\\.)*)"', chunk, re.S)
        rows.append((lic.group(1) if lic else None, note.group(1) if note else None))
    return rows


def licensed_floor_bp(content: str | None = None) -> int:
    """LICENSED_FLOOR — the explicit-class + filled-note share, bps."""
    if content is None:
        content = BACKENDS_RS.read_text(encoding="utf-8")
    rows = _registry_license_rows(content)
    if not rows:
        return 0
    licensed = sum(
        1
        for lic, note in rows
        if lic in ("Osi", "NonOsi", "Restrictive") and note and note.strip()
    )
    return licensed * 10000 // len(rows)


def _passport_verified_dates(content: str) -> list[str | None]:
    """The verified_at values of the PASSPORT_META table (the const
    names resolved; a missing/unresolvable value = None — fail-closed)."""
    try:
        block = content[content.index("const PASSPORT_META"):]
        block = block[: block.index("\n];")]
    except ValueError:
        return []
    consts = dict(re.findall(r'const ([A-Z_]+): &str = "([^"]+)"', content))
    dates: list[str | None] = []
    for chunk in block.split("PassportMeta {")[1:]:
        m = re.search(r'verified_at:\s*(?:"([^"]+)"|([A-Z_]+))', chunk)
        if not m:
            dates.append(None)
        elif m.group(1) is not None:
            dates.append(m.group(1))
        else:
            dates.append(consts.get(m.group(2)))
    return dates


def _parse_iso_date(value: str | None) -> _dt.date | None:
    if not value:
        return None
    try:
        return _dt.date.fromisoformat(value)
    except ValueError:
        return None


def passport_fresh_bp(
    today: _dt.date | None = None, content: str | None = None
) -> int:
    """PASSPORT_FRESH — the fresh-verification-date share, bps (the
    №842 rule: the age within PASSPORT_FRESHNESS_DAYS, inclusive)."""
    if content is None:
        content = MODEL_PASSPORT_RS.read_text(encoding="utf-8")
    today = today or _dt.date.today()
    rows = _passport_verified_dates(content)
    if not rows:
        return 0
    fresh = 0
    for raw in rows:
        date = _parse_iso_date(raw)
        if date is None:
            continue
        age = (today - date).days
        if 0 <= age <= PASSPORT_FRESHNESS_DAYS:
            fresh += 1
    return fresh * 10000 // len(rows)


def load_floors(path: Path = GATE_033_GOALS) -> dict[str, int]:
    """The floor file: `goal_<name> <value>` lines (the only-up floors)."""
    floors: dict[str, int] = {}
    if not path.exists():
        return floors
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) == 2 and parts[0].startswith("goal_"):
            try:
                floors[parts[0]] = int(parts[1])
            except ValueError:
                pass
    return floors


# ── №846 (wave 72 — W-B, ADR-0203 §2.7): the GRANT_COVERAGE machine ────

def _irreversible_carriers(content: str | None = None) -> list[str]:
    """The classification SSOT (the machine source): every builtin the
    static map marks Reversibility::Irreversible. The parse covers the
    committed array ONLY (the test module after #[cfg(test)] is cut —
    the test asserts are not carriers)."""
    if content is None:
        content = BUILTINS_CLASSIFICATION_RS.read_text(encoding="utf-8")
    try:
        block = content[content.index("pub static BUILTIN_CLASSES"):]
        block = block[: block.index("#[cfg(test)]")]
    except ValueError:
        return []
    names: list[str] = []
    for chunk in block.split("BuiltClassEntry {")[1:]:
        name = re.search(r'name:\s*"([^"]+)"', chunk)
        if not name:
            continue
        if "Reversibility::Irreversible" in chunk:
            names.append(name.group(1))
    return names


def _capability_rosters(content: str | None = None) -> dict[str, list[str]]:
    """The disposition rosters of the capability layer (№845): the three
    named consts, parsed from src/capability.rs — the machine source the
    Rust totality test pins against the classification."""
    if content is None:
        content = CAPABILITY_RS.read_text(encoding="utf-8")
    rosters: dict[str, list[str]] = {}
    for const in ("GRANT_ALWAYS", "PROFILE_PERMIT_SHARP", "BASELINE_STANDARD"):
        try:
            start = content.index(f"pub const {const}")
            block = content[start:]
            block = block[: block.index("];")]
            rosters[const] = re.findall(r'"([a-z0-9_]+)"', block)
        except ValueError:
            rosters[const] = []
    return rosters


def _mcp_gateway_decision_site(content: str | None = None) -> bool:
    """The MCP class unit: the gateway's capability DECISION SITE — the
    covering-grant lookup over the `mcp:<tool>` action scope in the tool
    dispatch path (№801). The exposed tools are program-declared (the
    policy compiler computes `irreversible` from the SAME classification),
    so the machine-checkable unit is the decision site itself: if the
    gateway stops demanding the grant decision, the class is uncovered
    and the coverage drops (that is the gate working)."""
    if content is None:
        content = MCP_SERVER_RS.read_text(encoding="utf-8")
    return 'lookup_active_covering(&action_scope)' in content and '"mcp:{}"' in content


def grant_coverage_bp(
    carriers: list[str] | None = None,
    rosters: dict[str, list[str]] | None = None,
    mcp_decision_site: bool | None = None,
) -> int:
    """GRANT_COVERAGE — the share of the irreversible carriers whose
    rights decision passes the capability layer, bps.

    Numerator: the classified-Irreversible builtins sitting in ONE of the
    №845 disposition rosters (GrantAlways / ProfilePermit-sharp / the
    named standard baseline) + the MCP irreversible-tool class when its
    gateway decision site is in place. Denominator: ALL classified
    Irreversible carriers + the MCP class. Both sides come from the
    machine sources (the classification SSOT, the capability rosters, the
    gateway source) — never a hand list. After №845 the coverage is 100%
    by construction; a NEW carrier added without its disposition drops
    the number and reddens the CI (the gate).
    """
    if carriers is None:
        carriers = _irreversible_carriers()
    if rosters is None:
        rosters = _capability_rosters()
    if mcp_decision_site is None:
        mcp_decision_site = _mcp_gateway_decision_site()
    covered_roster = set(rosters.get("GRANT_ALWAYS", [])) | set(
        rosters.get("PROFILE_PERMIT_SHARP", [])
    ) | set(rosters.get("BASELINE_STANDARD", []))
    language_total = len(carriers)
    language_covered = sum(1 for c in carriers if c in covered_roster)
    total = language_total + 1  # + the MCP irreversible-tool class
    covered = language_covered + (1 if mcp_decision_site else 0)
    if total == 0:
        return 0
    return covered * 10000 // total


# ── №857 (wave 74 — W-C, ADR-0204 §5): the SEMA_EXTERNAL machine ───────

def _sra_conformance_pairs(directory: Path | None = None) -> list[str]:
    """The topic-7 conformance pairs (the S-RA-*.mlog stems) — the spec
    page counts ONLY with its checked-in pairs (ADR-0204 §5/§7)."""
    directory = directory or CONFORMANCE_DIR
    if not directory.exists():
        return []
    return sorted(
        p.stem for p in directory.iterdir() if p.suffix == ".mlog" and p.stem.startswith("sra_")
    )


def sema_external(
    ci_content: str | None = None,
    facade_toml: str | None = None,
    action_yml: str | None = None,
    spec_page: str | None = None,
    spec_readme: str | None = None,
    sra_pairs: list[str] | None = None,
) -> dict:
    """SEMA_EXTERNAL — the number of externally consumable artifacts of
    the Portable Refusal Algebra that EXIST AND ARE LIVE on main
    (ADR-0204 §5).

    Candidates (the three legs of the external artifact, R38):
      (а) the WASM facade: the crate exists AND the ci.yml build job is
          active AND the host smoke-consumer step is wired;
      (б) the reusable Action: action.yml exists AND the ci.yml
          dogfood job is wired (the living external consumer);
      (в) the spec page: docs/spec/refusal-algebra.md exists AND the
          Topics table references it AND the S-RA conformance pairs are
          checked in (№858).

    Every candidate counts ONLY when BOTH sides hold — the artifact AND
    the live blocking-CI wiring. All inputs are parameterized so the
    tamper traps can prove the counter catches placeholders, deletions
    and definition substitutions without touching the real tree.
    """
    if ci_content is None:
        ci_content = CI_YML.read_text(encoding="utf-8")
    if facade_toml is None:
        facade_toml = FACADE_TOML.read_text(encoding="utf-8") if FACADE_TOML.exists() else None
    if action_yml is None:
        action_yml = POLICY_ACTION_YML.read_text(encoding="utf-8") if POLICY_ACTION_YML.exists() else None
    if spec_page is None:
        spec_page = SPEC_ALGEBRA_PAGE.read_text(encoding="utf-8") if SPEC_ALGEBRA_PAGE.exists() else None
    if spec_readme is None:
        spec_readme = SPEC_README.read_text(encoding="utf-8") if SPEC_README.exists() else None
    if sra_pairs is None:
        sra_pairs = _sra_conformance_pairs()
    counted: list[str] = []
    skipped: list[str] = []
    # (а) the WASM facade — the crate + the live build job + the smoke.
    if (
        facade_toml is not None
        and "wasm-facade" in ci_content
        and "wasm32-unknown-unknown" in ci_content
        and "refusal-wasm/smoke.mjs" in ci_content
    ):
        counted.append("wasm-facade")
    else:
        skipped.append("wasm-facade (needs the crate + the live ci.yml build job + the smoke step)")
    # (б) the reusable Action — action.yml + the live dogfood job.
    if (
        action_yml is not None
        and "policy-check" in action_yml
        and "policy-check-dogfood" in ci_content
        and "uses: ./" in ci_content
    ):
        counted.append("policy-check action")
    else:
        skipped.append("policy-check action (needs action.yml + the live ci.yml dogfood job)")
    # (в) the spec page — the topic + the Topics-table row + the pairs,
    # RUN LIVE by the blocking conformance job (the pairs without a live
    # runner are a static file, not a proof).
    if (
        spec_page is not None
        and spec_readme is not None
        and "refusal-algebra.md" in spec_readme
        and sra_pairs
        and "conformance (blocking)" in ci_content
    ):
        counted.append("spec page")
    else:
        skipped.append(
            "spec page (needs docs/spec/refusal-algebra.md + the Topics row "
            "+ the S-RA pairs + the live conformance (blocking) runner)"
        )
    return {"count": len(counted), "counted": counted, "skipped": skipped}


def load_floors_strict(path: Path) -> dict[str, int]:
    """The №846 strict loader (fail-closed, the negative control (а)):
    a missing/empty file is a loud refusal, and EVERY non-comment line
    must parse as `goal_<name> <int>` — a corrupted floor file can never
    silently shrink to zero gates (the №776 discipline)."""
    if not path.exists():
        raise ValueError(f"{path.name}: the floor file is missing (the only-up gates must exist)")
    floors: dict[str, int] = {}
    for lineno, raw in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) != 2 or not parts[0].startswith("goal_"):
            raise ValueError(
                f"{path.name}:{lineno}: malformed floor line '{raw}' "
                "(expected 'goal_<name> <value-bps>')"
            )
        try:
            floors[parts[0]] = int(parts[1])
        except ValueError:
            raise ValueError(
                f"{path.name}:{lineno}: non-numeric floor value '{parts[1]}'"
            ) from None
    if not floors:
        raise ValueError(f"{path.name}: no floors (the empty gate file is a refusal)")
    return floors


def tamper_test() -> int:
    """The RED harness (№841): the checkers MUST catch the fabricated
    records. Every trap asserts the floor DROPS under the tamper —
    a checker that stays silent under a stub record is a liar checker
    (the exit 1), and CI fails."""
    good_backends = BACKENDS_RS.read_text(encoding="utf-8")
    good_passports = MODEL_PASSPORT_RS.read_text(encoding="utf-8")
    full_lic = licensed_floor_bp(good_backends)
    full_pass = passport_fresh_bp(content=good_passports)

    # Trap 1: the empty license_note lowers LICENSED_FLOOR.
    stub_note = good_backends.replace(
        'license_note: "Chatterbox (Resemble AI) — MIT (osi)"', 'license_note: ""', 1
    )
    if stub_note == good_backends:
        print("tamper-test: TRAP FAILED — the note anchor drifted (the fixture is stale)", file=sys.stderr)
        return 1
    if licensed_floor_bp(stub_note) >= full_lic:
        print("tamper-test: TRAP FAILED — the empty license_note did NOT lower LICENSED_FLOOR", file=sys.stderr)
        return 1

    # Trap 2: a fabricated unknown class word lowers LICENSED_FLOOR.
    stub_class = good_backends.replace("license: LicenseClass::Osi,", "license: LicenseClass::Unknown,", 1)
    if stub_class == good_backends:
        print("tamper-test: TRAP FAILED — the class anchor drifted", file=sys.stderr)
        return 1
    if licensed_floor_bp(stub_class) >= full_lic:
        print("tamper-test: TRAP FAILED — the unknown class did NOT lower LICENSED_FLOOR", file=sys.stderr)
        return 1

    # Trap 3: an aged verification date lowers PASSPORT_FRESH (the №842
    # window). The aged record is fabricated IN MEMORY (the real tree
    # stays untouched).
    aged = good_passports.replace(
        'const PIN_VERIFIED_AT: &str = "2026-09-16";',
        'const PIN_VERIFIED_AT: &str = "2020-01-01";',
        1,
    )
    if aged == good_passports:
        print("tamper-test: TRAP FAILED — the PIN_VERIFIED_AT anchor drifted", file=sys.stderr)
        return 1
    if passport_fresh_bp(content=aged) >= full_pass:
        print("tamper-test: TRAP FAILED — the aged date did NOT lower PASSPORT_FRESH", file=sys.stderr)
        return 1

    # Trap 4: an unparseable date is NOT fresh (fail-closed).
    broken = good_passports.replace(
        'const PIN_VERIFIED_AT: &str = "2026-09-16";',
        'const PIN_VERIFIED_AT: &str = "not-a-date";',
        1,
    )
    if passport_fresh_bp(content=broken) >= full_pass:
        print("tamper-test: TRAP FAILED — the unparseable date did NOT lower PASSPORT_FRESH", file=sys.stderr)
        return 1

    # ── №846 traps: the coverage machine catches the tamper ───────────
    good_capability = CAPABILITY_RS.read_text(encoding="utf-8")
    full_cov = grant_coverage_bp()
    if full_cov != GRANT_COVERAGE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the coverage is not at the owner floor "
            f"({full_cov} bp != {GRANT_COVERAGE_FLOOR_OWNER_BP} bp): the rosters drifted "
            "from the classification (fix the disposition, never the floor)",
            file=sys.stderr,
        )
        return 1

    # Trap 5: a carrier REMOVED from its roster lowers GRANT_COVERAGE —
    # the silent-roster-edit class (the forged authority would run).
    stub_roster = good_capability.replace('"memory_forget_cascade"', '""', 1)
    if stub_roster == good_capability:
        print("tamper-test: TRAP FAILED — the GRANT_ALWAYS roster anchor drifted", file=sys.stderr)
        return 1
    if grant_coverage_bp(rosters=_capability_rosters(stub_roster)) >= full_cov:
        print(
            "tamper-test: TRAP FAILED — the un-rostered carrier did NOT lower GRANT_COVERAGE",
            file=sys.stderr,
        )
        return 1

    # Trap 6: the gateway decision site removed lowers GRANT_COVERAGE —
    # the MCP class is uncovered (the silent bypass class).
    if grant_coverage_bp(mcp_decision_site=False) >= full_cov:
        print(
            "tamper-test: TRAP FAILED — the missing gateway decision site did NOT lower GRANT_COVERAGE",
            file=sys.stderr,
        )
        return 1

    # Trap 7 (the negative control (б)): the gate file LOWERED below the
    # owner constant is a loud refusal — the floor is only-up (plan §23).
    strict = load_floors_strict(GATE_034_GOALS)
    value = strict.get("goal_grant_coverage_bp")
    if value is None or value < GRANT_COVERAGE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the gate_034 floor is below the owner constant "
            f"(plan §23, revision R38: {GRANT_COVERAGE_FLOOR_OWNER_BP} bp) — the lowering is refused",
            file=sys.stderr,
        )
        return 1

    # ── №857 traps: the SEMA_EXTERNAL machine catches the tamper ──────
    good_ci = CI_YML.read_text(encoding="utf-8")
    full_sema = sema_external()
    if full_sema["count"] < SEMA_EXTERNAL_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — SEMA_EXTERNAL is below the owner floor on the "
            f"clean tree ({full_sema['count']} < {SEMA_EXTERNAL_FLOOR_OWNER}): the wave "
            "artifacts drifted (fix the artifacts, never the floor)",
            file=sys.stderr,
        )
        return 1

    # Trap 8 (the placeholder class): the facade file WITHOUT the live
    # ci.yml wiring is NOT counted — a file is not an adoption.
    no_wasm_job = "\n".join(
        line for line in good_ci.splitlines() if "wasm-facade" not in line
    )
    if no_wasm_job == good_ci:
        print("tamper-test: TRAP FAILED — the ci.yml wasm-facade anchor drifted", file=sys.stderr)
        return 1
    if sema_external(ci_content=no_wasm_job)["count"] >= full_sema["count"]:
        print(
            "tamper-test: TRAP FAILED — the placeholder facade (no live ci.yml job) "
            "was COUNTED — the self-declaration hole is open",
            file=sys.stderr,
        )
        return 1

    # Trap 9 (the deletion class): an action.yml stripped of the
    # policy-check surface (the deleted/gutted file) lowers the count —
    # the gate reddens while the floor stands. (The empty string IS the
    # deleted-file simulation: None would mean "load the real file".)
    if sema_external(action_yml="")["count"] >= full_sema["count"]:
        print(
            "tamper-test: TRAP FAILED — the stripped action.yml did NOT lower SEMA_EXTERNAL",
            file=sys.stderr,
        )
        return 1

    # Trap 10 (the definition-substitution control): a counter that
    # counted by file EXISTENCE alone would report a nonzero number
    # with all artifacts present but NO live CI wiring — the real one
    # reports 0 (the existence check is actually wired, not declared).
    if sema_external(ci_content="")["count"] != 0:
        print(
            "tamper-test: TRAP FAILED — the counter answered without the live-CI "
            "existence check (the definition was substituted)",
            file=sys.stderr,
        )
        return 1

    # Trap 11 (the negative control (б), the №846 form): the gate_035
    # floor LOWERED below the owner constant is a loud refusal.
    sema_strict = load_floors_strict(GATE_035_GOALS)
    sema_value = sema_strict.get("goal_sema_external_owner")
    if sema_value is None or sema_value < SEMA_EXTERNAL_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — the gate_035 floor is below the owner constant "
            f"(plan §23, revision R38: {SEMA_EXTERNAL_FLOOR_OWNER}) — the lowering is refused",
            file=sys.stderr,
        )
        return 1

    # The twin pin: the Python window equals the №842 Rust constant.
    rust = (REPO / "src" / "model_passport.rs").read_text(encoding="utf-8")
    m = re.search(r"PASSPORT_FRESHNESS_DAYS: i64 = (\d+)", rust)
    if not m or int(m.group(1)) != PASSPORT_FRESHNESS_DAYS:
        print(
            "tamper-test: TRAP FAILED — the freshness-window twins drifted "
            f"(python={PASSPORT_FRESHNESS_DAYS}, rust={m.group(1) if m else 'absent'})",
            file=sys.stderr,
        )
        return 1

    print(
        "tamper-test: OK — the checkers catch the fabricated records "
        f"(LICENSED_FLOOR {full_lic} bp, PASSPORT_FRESH {full_pass} bp; the traps lowered both)"
    )
    return 0


def full_block() -> str:
    """№804: the FULL generated table — docs/metrics.md (the one honest
    place for every machine number; the README front page links here)."""
    total = count_total_builtins()
    modules = count_builtin_modules()
    svg = count_svg_builtins()
    rules = count_parser_rules()
    adrs = count_adrs()
    examples = count_examples()
    version = cargo_version()
    changelog_kb = file_kb("CHANGELOG.md")
    reference_kb = file_kb("REFERENCE.md")
    typed, total_fns, precise = count_typed_signatures()
    typed_bp = (typed * 10000) // total_fns if total_fns else 0
    precise_bp = (precise * 10000) // total_fns if total_fns else 0
    stable_typed, stable_total = count_stable_typed()
    stable_bp = (stable_typed * 10000) // stable_total if stable_total else 0
    neg, pos = leak_counts()
    return (
        f"| Metric | Value (generated — do not hand-edit) |\n"
        f"| ------ | ------------------------------------- |\n"
        f"| Version | {version} |\n"
        f"| Leak Suite | {neg} negatives / {pos} positives (examples/leak/ — the BLOCKING gate) |\n"
        f"| Built-in Functions | {total} functions across {modules} modules |\n"
        f"| Typed Signatures | {typed}/{total_fns} ({typed_bp / 100:.2f}%) — precise {precise}/{total_fns} ({precise_bp / 100:.2f}%) |\n"
        f"| Typed Signatures (stable tier — the PRIMARY definition, №809/ADR-0199) | {stable_typed}/{stable_total} ({stable_bp / 100:.2f}%) — the full-registry figure beside: {typed}/{total_fns} ({typed_bp / 100:.2f}%) |\n"
        f"| SVG/Graphics | {svg} builtins, hand-rolled in pure Rust |\n"
        f"| Grammar | {rules} rules |\n"
        f"| Architecture Decisions | {adrs} ADRs |\n"
        f"| Example Programs | {examples} .mlog programs |\n"
        f"| Reference | REFERENCE.md (~{reference_kb} KB) — 100% registry coverage |\n"
        f"| Changelog | CHANGELOG.md (~{changelog_kb} KB) — every wave documented |\n"
        f"| LICENSED_FLOOR (the 8D-1 machine fact, №841) | {licensed_floor_bp()} bp — the license class explicit + the license_note filled over the backend registry (ADR-0163/ADR-0202 terms) |\n"
        f"| PASSPORT_FRESH (the 8D-1 machine fact, №841) | {passport_fresh_bp()} bp — the passports fresh within the {PASSPORT_FRESHNESS_DAYS}-day verification window (the №842 rule) |\n"
        f"| GRANT_COVERAGE (the 8B-1 machine fact, №846) | {grant_coverage_bp()} bp — the irreversible carriers whose rights decision passes the capability layer (ADR-0203 §2.7; the gate_034 floor) |\n"
        f"| SEMA_EXTERNAL (the 8F-1 machine fact, №857) | {sema_external()['count']} — the live external-consumption artifacts of the Portable Refusal Algebra (ADR-0204 §5; the gate_035 only-up floor ≥ 1): {', '.join(sema_external()['counted']) or 'none yet'} |\n"
    )


def front_block() -> str:
    """№804: the README front-page rows — THREE max (the version, the
    leak-suite negatives/positives, the CI badge row is a badge, not a
    table row, so the third line is the pointer to the full table)."""
    version = cargo_version()
    neg, pos = leak_counts()
    return (
        f"| Metric | Value (generated — do not hand-edit) |\n"
        f"| ------ | ------------------------------------- |\n"
        f"| Version | {version} |\n"
        f"| Leak Suite | {neg} negatives / {pos} positives (the BLOCKING gate) |\n"
        f"| Full metrics | [docs/metrics.md](docs/metrics.md) — the generated table |\n"
    )


def block() -> str:
    # №804: the README block IS the front block (≤3 rows); the full table
    # moved to docs/metrics.md. The legacy name kept for the call sites.
    return front_block()
    total = count_total_builtins()
    modules = count_builtin_modules()
    svg = count_svg_builtins()
    rules = count_parser_rules()
    adrs = count_adrs()
    examples = count_examples()
    version = cargo_version()
    changelog_kb = file_kb("CHANGELOG.md")
    reference_kb = file_kb("REFERENCE.md")
    typed, total_fns, precise = count_typed_signatures()
    typed_bp = (typed * 10000) // total_fns if total_fns else 0
    precise_bp = (precise * 10000) // total_fns if total_fns else 0
    return (
        f"| Metric | Value (generated — do not hand-edit) |\n"
        f"| ------ | ------------------------------------- |\n"
        f"| Version | {version} |\n"
        f"| Built-in Functions | {total} functions across {modules} modules |\n"
        f"| Typed Signatures | {typed}/{total_fns} ({typed_bp / 100:.2f}%) — precise {precise}/{total_fns} ({precise_bp / 100:.2f}%) |\n"
        f"| SVG/Graphics | {svg} builtins, hand-rolled in pure Rust |\n"
        f"| Grammar | {rules} rules |\n"
        f"| Architecture Decisions | {adrs} ADRs |\n"
        f"| Example Programs | {examples} .mlog programs |\n"
        f"| Reference | REFERENCE.md (~{reference_kb} KB) — 100% registry coverage |\n"
        f"| Changelog | CHANGELOG.md (~{changelog_kb} KB) — every wave documented |\n"
    )


def plan_summary_block() -> str:
    """№566: the digest's CURRENT machine facts — the same sources as the
    README's Typed Signatures row (one SSOT; the two can never disagree
    because one process computes both)."""
    typed, total_fns, precise = count_typed_signatures()
    typed_bp = (typed * 10000) // total_fns if total_fns else 0
    precise_bp = (precise * 10000) // total_fns if total_fns else 0
    return (
        f"| Machine fact (generated — do not hand-edit) | Value |\n"
        f"| --- | --- |\n"
        f"| Typed-signature floor — the 0.28-gate line (ADR-0179) "
        f"| {typed_bp} bp — {typed}/{total_fns} = {typed_bp / 100:.2f}% "
        f"(precise {precise}/{total_fns} = {precise_bp / 100:.2f}%) |\n"
        f"| BUILTIN_REGISTRY rows | {total_fns} |\n"
        f"| LICENSED_FLOOR — the 8D-1 machine fact (№841) | {licensed_floor_bp()} bp |\n"
        f"| PASSPORT_FRESH — the 8D-1 machine fact (№841) | {passport_fresh_bp()} bp |\n"
        f"| GRANT_COVERAGE — the 8B-1 machine fact (№846) | {grant_coverage_bp()} bp |\n"
        f"| SEMA_EXTERNAL — the 8F-1 machine fact (№857) | {sema_external()['count']} |\n"
    )


def upsert_block(path: Path, begin: str, end: str, generated: str, label: str) -> tuple[bool, str]:
    """Replace (or verify, in --check mode) the block between the markers.
    Returns (ok, current_block) — ok=False means stale in check mode."""
    text = path.read_text(encoding="utf-8")
    if begin not in text or end not in text:
        print(f"ERROR: {path.name} is missing the {label} markers", file=sys.stderr)
        sys.exit(2)
    start = text.index(begin) + len(begin)
    end_pos = text.index(end, start)
    current = text[start:end_pos]
    fresh = "\n" + generated
    return current == fresh, current


def main() -> int:
    check = "--check" in sys.argv
    stale = []
    ok, current = upsert_block(README, BEGIN, END, block(), "metrics")
    if check:
        if ok:
            print("OK: README.md generated metrics block is up to date")
        else:
            stale.append("README.md")
    else:
        if not ok:
            text = README.read_text(encoding="utf-8")
            start = text.index(BEGIN) + len(BEGIN)
            end_pos = text.index(END, start)
            README.write_text(text[:start] + "\n" + block() + text[end_pos:], encoding="utf-8")
            print("README.md metrics block regenerated")

    # №804: the full generated table lives in docs/metrics.md.
    if not METRICS_DOC.exists():
        print("ERROR: docs/metrics.md is missing (the №804 full-metrics place)", file=sys.stderr)
        return 2
    ok, current = upsert_block(METRICS_DOC, MD_BEGIN, MD_END, full_block(), "metrics-doc")
    if check:
        if ok:
            print("OK: docs/metrics.md generated table is up to date")
        else:
            stale.append("docs/metrics.md")
    else:
        if not ok:
            text = METRICS_DOC.read_text(encoding="utf-8")
            start = text.index(MD_BEGIN) + len(MD_BEGIN)
            end_pos = text.index(MD_END, start)
            METRICS_DOC.write_text(
                text[:start] + "\n" + full_block() + text[end_pos:], encoding="utf-8"
            )
            print("docs/metrics.md table regenerated")

    if not PLAN_SUMMARY.exists():
        print("ERROR: docs/PLAN-SUMMARY.md is missing", file=sys.stderr)
        return 2
    ok, current = upsert_block(PLAN_SUMMARY, PS_BEGIN, PS_END, plan_summary_block(), "numbers")
    if check:
        if ok:
            print("OK: PLAN-SUMMARY.md generated numbers block is up to date")
        else:
            stale.append("docs/PLAN-SUMMARY.md")
    else:
        if not ok:
            text = PLAN_SUMMARY.read_text(encoding="utf-8")
            start = text.index(PS_BEGIN) + len(PS_BEGIN)
            end_pos = text.index(PS_END, start)
            PLAN_SUMMARY.write_text(
                text[:start] + "\n" + plan_summary_block() + text[end_pos:], encoding="utf-8"
            )
            print("docs/PLAN-SUMMARY.md numbers block regenerated")

    if check and stale:
        print(
            "STALE: generated blocks do not match the repository: "
            + ", ".join(stale)
            + " — run scripts/gen_metrics.py",
            file=sys.stderr,
        )
        return 1

    # №841: the floors checkist (the only-up gates of gate_033_goals.txt)
    # ride --check — a floor breach is a CI failure exactly like a stale
    # block. The target floors for the 0.35 gate are the owner's decision
    # at the subphase issuance; the START floors = the values at the №841
    # merge commit.
    if check:
        floors = load_floors()
        if not floors:
            print(
                "ERROR: scripts/ci/gate_033_goals.txt is missing or empty "
                "(the №841 floors must exist — the only-up gates)",
                file=sys.stderr,
            )
            return 2
        current = {
            "goal_licensed_floor_bp": licensed_floor_bp(),
            "goal_passport_floor_bp": passport_fresh_bp(),
            "goal_passport_fresh_bp": passport_fresh_bp(),
        }
        breached = []
        for name, floor in sorted(floors.items()):
            value = current.get(name)
            if value is None:
                breached.append(f"{name}: the counter is missing")
            elif value < floor:
                breached.append(f"{name}: current {value} bp < floor {floor} bp (only-up)")
        if breached:
            print(
                "FLOOR BREACH: the №841 machine facts dropped below the only-up floors: "
                + "; ".join(breached)
                + " — re-pin the passports explicitly (№842) or fix the records",
                file=sys.stderr,
            )
            return 1
        print(
            "OK: the №841 floors hold — LICENSED_FLOOR "
            f"{current['goal_licensed_floor_bp']} bp, PASSPORT_FRESH {current['goal_passport_fresh_bp']} bp"
        )

    # №846: the GRANT_COVERAGE floor twin (the only-up gate_034) — the
    # STRICT loader (fail-closed on a missing/empty/corrupted file — the
    # negative control (а)) and the owner-pinned lowering refusal (the
    # negative control (б): the plan §23 floor can rise, never fall).
    if check:
        try:
            cov_floors = load_floors_strict(GATE_034_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        floor = cov_floors.get("goal_grant_coverage_bp")
        if floor is None:
            print(
                "ERROR: gate_034_goals.txt carries no goal_grant_coverage_bp floor",
                file=sys.stderr,
            )
            return 2
        if floor < GRANT_COVERAGE_FLOOR_OWNER_BP:
            print(
                "FLOOR LOWERING REFUSED: gate_034 "
                f"({floor} bp) is below the owner constant "
                f"({GRANT_COVERAGE_FLOOR_OWNER_BP} bp — plan §23, revision R38); "
                "the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        coverage = grant_coverage_bp()
        if coverage < floor:
            print(
                f"FLOOR BREACH: GRANT_COVERAGE {coverage} bp < the gate_034 floor {floor} bp "
                "(only-up) — an irreversible carrier landed WITHOUT its capability "
                "disposition (add it to GRANT_ALWAYS / PROFILE_PERMIT_SHARP / "
                "BASELINE_STANDARD in src/capability.rs — never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        print(
            f"OK: the №846 floor holds — GRANT_COVERAGE {coverage} bp "
            f"(gate_034 floor {floor} bp)"
        )

    # №857: the SEMA_EXTERNAL floor twin (the only-up gate_035) — the
    # STRICT loader (fail-closed on a missing/empty/corrupted file) and
    # the owner-pinned lowering refusal (the plan §23 floor can rise,
    # never fall). The report carries the honest composition — WHAT
    # exactly is counted (the №468 form): the counted artifacts, and
    # the candidates still missing their live wiring.
    if check:
        try:
            sema_floors = load_floors_strict(GATE_035_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        sema_floor = sema_floors.get("goal_sema_external_owner")
        if sema_floor is None:
            print(
                "ERROR: gate_035_goals.txt carries no goal_sema_external_owner floor",
                file=sys.stderr,
            )
            return 2
        if sema_floor < SEMA_EXTERNAL_FLOOR_OWNER:
            print(
                "FLOOR LOWERING REFUSED: gate_035 "
                f"({sema_floor}) is below the owner constant "
                f"({SEMA_EXTERNAL_FLOOR_OWNER} — plan §23, revision R38); "
                "the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        sema = sema_external()
        if sema["count"] < sema_floor:
            print(
                f"FLOOR BREACH: SEMA_EXTERNAL {sema['count']} < the gate_035 floor {sema_floor} "
                "(only-up) — a live external-consumption artifact of the Portable "
                "Refusal Algebra disappeared (restore the crate/action/spec page "
                "or its blocking-CI wiring — never a silent floor edit)",
                file=sys.stderr,
            )
            return 1
        print(
            f"OK: the №857 floor holds — SEMA_EXTERNAL {sema['count']} "
            f"(gate_035 floor {sema_floor}, only-up) — counted: "
            f"{', '.join(sema['counted']) or 'none'}; not counted yet: "
            f"{'; '.join(sema['skipped']) or 'none'}"
        )

    # №841: the RED harness (the tamper traps) — an explicit mode, run by
    # CI beside the --check (the same posture as the other gates'
    # --tamper-test modes).
    if "--tamper-test" in sys.argv:
        return tamper_test()

    return 0


if __name__ == "__main__":
    sys.exit(main())
