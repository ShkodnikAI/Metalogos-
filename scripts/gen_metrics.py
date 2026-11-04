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

# №862 (wave 75 — W-D): the 8B-2 machine fact — APPROVAL_COVERAGE with
# its own only-up floor file. The floor VALUE 10000 bp = 100.00% is the
# plan §23 number (the owner revision R38 fixed the 0.38 gate: C11,
# APPROVAL_COVERAGE 10000 bp). The definition lives in ADR-0205 §4: the
# share of the human-oversight carriers covered by the fail-closed
# approval gate (№861) — counted over the LIVE registry (the roster is
# the explicit registry transaction in src/approval.rs; the gate's
# composition site in capability.rs is the wiring the counter demands —
# the fact rides with its checker, №735). The checker pins the owner
# constant: the gate file can RAISE the floor, never lower it silently
# (the №846 discipline, verbatim).
GATE_036_GOALS = REPO / "scripts" / "ci" / "gate_036_goals.txt"
# №869 (wave 76 — W-E): the 8A-1 machine-fact floor record (the
# gate_036 form — the fact and its --check twin).
GATE_037_GOALS = REPO / "scripts" / "ci" / "gate_037_goals.txt"
# The compiled compliance package (the №868 compiler output) — the
# ATTEST_MAP machine source.
PACKAGE_0206 = REPO / "docs" / "compliance" / "attestation-map-v1.txt"
APPROVAL_COVERAGE_FLOOR_OWNER_BP = 10000

# №869 (wave 76 — W-E): the 8A-1 machine fact — ATTEST_MAP with its
# gate_037 floor (the №857/№862 form: the fact rides with its checker,
# the floor only-up, the tamper traps live). The definition lives in
# ADR-0206 §4: the count of the LIVE attestation rows in the compiled
# compliance package (docs/compliance/attestation-map-v1.txt — the №868
# compiler output), counted from the RAW rows STRUCTURALLY (the №565
# lesson: reproducibility is structural — never from a summary header).
# The floor is the wave's start floor (the №841 form: the start floors
# are pinned by the naryad; raising the floor is an explicit owner
# transaction — lowering is refused).
ATTEST_MAP_FLOOR_OWNER = 30

# №875 (wave 77 — W-F): the 8C/C3 machine fact — PROVENANCE_SHARE with
# its own only-up floor file. The floor VALUE 10000 bp = 100.00% is the
# wave-77 dispatch constant (gh#1729: the 0.40 gate, PROVENANCE_SHARE
# 10000 bp). The definition lives in ADR-0207 §4: the share of the
# registered I/O carriers whose boundary provenance record is LIVE-wired
# — counted over the EXPLICIT registry (PROVENANCE_CARRIERS in
# src/provenance.rs is the machine source, the №862 roster form), each
# carrier counted ONLY on a live boundary call in its implementation
# file (crate::provenance::ingest/gate_emit with the carrier's own name
# — no self-declaration; the tamper traps below prove the counter
# catches the removed wiring and the forged roster). The egress gate's
# composition site (`pub fn gate_emit` in src/provenance.rs) is the
# wiring the counter demands — if it is removed, NO egress carrier is
# gated and the share reads 0 (the gate working, not a pass). The
# checker pins the owner constant: the gate file can RAISE the floor,
# never lower it silently (the №846 discipline, verbatim).
GATE_038_GOALS = REPO / "scripts" / "ci" / "gate_038_goals.txt"
PROVENANCE_SHARE_FLOOR_OWNER_BP = 10000
PROVENANCE_RS = REPO / "src" / "provenance.rs"
BUILTINS_IO_RS = REPO / "src" / "builtins" / "io.rs"
BUILTINS_HTTP_RS = REPO / "src" / "builtins" / "http.rs"

# №881 (wave 78 — W-G): the 8E-1/C5 machine fact — OFFLINE_SUITE with
# its own only-up floor file. The floor VALUE 8000 bp is the wave-78
# dispatch constant (gh#1745: the 0.41 gate, OFFLINE_SUITE ≥ 8000 bp).
# The definition lives in ADR-0208 §2.5: the share, in basis points, of
# the REGISTERED LLM-surface checks that execute WITHOUT the external
# network (the local provider, the deterministic mock fault seam №385,
# the loopback stub servers — the №819/n812 fixture form). The machine
# source is the EXPLICIT registry (scripts/ci/gate_039_goals.txt — the
# №862 roster form): a row counts ONLY when BOTH hold — (1) the row
# carries offline=yes, and (2) the check file EXISTS (tests/<name>.rs
# or metalogos-server/tests/<name>.rs — the count is CHECKED, not
# declared; a forged row is uncovered); an empty registry is 0 bp (a
# vacuous class is a gate failure, never a pass). The checker pins the
# owner constant: the gate file can RAISE the floor, never lower it
# silently (the №846 discipline, verbatim).
GATE_039_GOALS = REPO / "scripts" / "ci" / "gate_039_goals.txt"
OFFLINE_SUITE_FLOOR_OWNER_BP = 8000

# №888 (wave 79 — W-H): the 8D-2/C10 machine fact — JURISDICTION_MATRIX
# with its own only-up floor file. The floor VALUE 3 is the wave-79
# dispatch constant (gh#1764: the 0.42 gate, JURISDICTION_MATRIX ≥ 3).
# The definition lives in ADR-0209 §4 (the №886 seed): the number of
# DISTINCT jurisdictions in the compiled jurisdiction matrix that carry
# ≥ 1 live rule — the machine source is the EXPLICIT registry
# (scripts/ci/gate_040_goals.txt — the №862 roster form): a row counts
# ONLY when the jurisdiction code is PRESENT in the LIVE seed matrix
# source (src/jurisdiction.rs — the issue("<code>" call; the count is
# CHECKED, not declared; a forged row is uncovered); an empty registry
# is 0 (a vacuous class is a gate failure, never a pass). The checker
# pins the owner constant: the gate file can RAISE the floor, never
# lower it silently (the №846 discipline, verbatim).
GATE_040_GOALS = REPO / "scripts" / "ci" / "gate_040_goals.txt"
JURISDICTION_MATRIX_FLOOR_OWNER = 3
JURISDICTION_RS = REPO / "src" / "jurisdiction.rs"

# №894 (wave 80 — W-I): the 8G/C8 machine fact — REPLAY_MATCH with its
# own only-up floor file. The floor VALUE 10000 bp is the wave-80
# dispatch constant (gh#1779: the 0.43 gate, REPLAY_MATCH ≥ 10000 bp).
# The definition lives in ADR-0210 §3-4 (the №892/№893 kernel): the
# share (bp) of the registered decision carriers whose decision trace
# is WRITTEN through the live Ledger ACTION contour AND verified by the
# replay_gate REPLAY_MATCH contract — the machine source is the
# EXPLICIT registry (scripts/ci/gate_041_goals.txt — the №862 roster
# form): a row counts ONLY when the decision name is PRESENT in the
# LIVE e2e wiring source (tests/naryad_893_replay_coverage.rs — the
# quoted decision literal; the count is CHECKED, not declared; a forged
# row is uncovered) AND the gate composition site (pub fn replay_gate —
# the №874 form) is alive in src/decision_trace.rs: the composition
# removed → NOTHING is gated → the share reads 0 (the gate working,
# never a pass). An empty registry is 0 bp (a vacuous class is a gate
# failure, never a pass). The checker pins the owner constant: the
# gate file can RAISE the floor, never lower it silently (the №846
# discipline, verbatim).
GATE_041_GOALS = REPO / "scripts" / "ci" / "gate_041_goals.txt"
REPLAY_MATCH_FLOOR_OWNER_BP = 10000
DECISION_TRACE_RS = REPO / "src" / "decision_trace.rs"
REPLAY_E2E_RS = REPO / "tests" / "naryad_893_replay_coverage.rs"

# №906 (wave 81 — W-J): the 8F-2/C9 machine fact — TAINT_SATURATION with
# its own only-up floor file. The floor VALUE 10⁹ is the wave-81 dispatch
# constant (gh#1803: the 0.44 gate, TAINT_SATURATION >= 10^9). The
# definition lives in ADR-0217 §2 (the №903 record): the EXACT count of
# distinct taint-relevant programs the wave-81 generator can emit — an
# integer product over LIVE anchors, never declared (the №894 form: the
# count is CHECKED, not declared). Every factor is read from the live
# tree with a precise anchor:
#   T — the scenario templates of the №904 roster (the quoted region in
#       tests/naryad_465_diff_fuzzer.rs — the №862 quoted-region form);
#   L — the live label lanes (the PRESENCE of the three lane modules —
#       a removed lane module zeroes the factor);
#   S — the live sources (the roster's distinct source fields, each
#       cross-checked against the live label/classification/audit files);
#   K — the live sinks (SINK_FUNCTIONS in src/audit.rs — the №565 list —
#       plus the sql entry points SQL_ARG_POSITIONS in
#       src/builtins_classification.rs — the №831 SSOT table);
#   Z — the live sanitizer (the escape_html classification entry,
#       src/builtins_classification.rs — the :142 anchor);
#   F, B — the generator's statement forms (the FORM_* roster) and the
#       statement budget (the budget constant), both in the live fuzzer
#       source (the №766/№465 file).
#   Space = ScenarioVariants × SequenceSpace;
#   ScenarioVariants = T × L × S × K × Z;
#   SequenceSpace = Σ_{k=0..B} F^k (python int — no overflow).
GATE_042_GOALS = REPO / "scripts" / "ci" / "gate_042_goals.txt"
TAINT_SATURATION_FLOOR_OWNER = 1_000_000_000
TAINT_FUZZER_RS = REPO / "tests" / "naryad_465_diff_fuzzer.rs"
TAINT_LANE_MODULES = (
    REPO / "src" / "html_label.rs",
    REPO / "src" / "sql_label.rs",
    REPO / "src" / "secret_label.rs",
)
TAINT_SOURCE_ANCHORS = (
    REPO / "src" / "html_label.rs",
    REPO / "src" / "sql_label.rs",
    REPO / "src" / "secret_label.rs",
    REPO / "src" / "builtins_classification.rs",
    REPO / "src" / "audit.rs",
)
AUDIT_RS = REPO / "src" / "audit.rs"
CLASSIFICATION_RS = REPO / "src" / "builtins_classification.rs"

APPROVAL_RS = REPO / "src" / "approval.rs"
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
    silently shrink to zero gates (the №776 discipline).

    №881 (wave 78 — W-G): the gate_039 goals file carries a SECOND data
    class — the registry rows (`check <name> offline=yes|no`, the №862
    roster form, machine-read by _offline_suite_registry). The loader
    SKIPS those rows: they are roster data, never floors; every other
    non-comment line must still parse as a goal_ floor or the file is
    refused (the strictness for the floor fields is unchanged).

    №888 (wave 79 — W-H): the same skip covers the gate_040 registry
    rows (`jurisdiction <code>`, machine-read by _jurisdiction_registry)
    — roster data, never floors (the wave-79 dispatch form)."""
    if not path.exists():
        raise ValueError(f"{path.name}: the floor file is missing (the only-up gates must exist)")
    floors: dict[str, int] = {}
    for lineno, raw in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("check "):
            # The №881 registry row — roster data, not a floor.
            continue
        if line.startswith("jurisdiction "):
            # The №888 registry row — roster data, not a floor.
            continue
        if line.startswith("decision "):
            # The №894 registry row — roster data, not a floor.
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


# ── №862 (wave 75 — W-D, ADR-0205 §4): the APPROVAL_COVERAGE machine ──

def _human_oversight_roster(content: str | None = None) -> list[str]:
    """The wave-75 human-oversight roster (№861): the EXPLICIT registry
    transaction in src/approval.rs — the machine source the Rust
    totality test pins against the classification (a name here that is
    not a classified Irreversible carrier reddens the Rust test AND
    lowers this coverage — no silent shelves)."""
    if content is None:
        content = APPROVAL_RS.read_text(encoding="utf-8")
    try:
        start = content.index("pub const HUMAN_OVERSIGHT")
        block = content[start:]
        block = block[: block.index("];")]
        return re.findall(r'"([a-z0-9_]+)"', block)
    except ValueError:
        return []


def _approval_gate_composition_site(content: str | None = None) -> bool:
    """The gate's composition site (№861): the approval decision is
    composed INSIDE capability::authorize_call — the ONE decision
    surface both backends dispatch through (TW invoke/QualifiedCall/
    FnCall + VM call_builtin). If the composition is removed, NO roster
    carrier is gated — the coverage reads 0 (the gate working, not a
    pass)."""
    if content is None:
        content = CAPABILITY_RS.read_text(encoding="utf-8")
    return "crate::approval::authorize_overseen_call(name)" in content


def approval_coverage_bp(
    roster: list[str] | None = None,
    classification: list[str] | None = None,
    composition_site: bool | None = None,
) -> int:
    """APPROVAL_COVERAGE — the share of the human-oversight carriers
    covered by the fail-closed approval gate, bps (№862, ADR-0205 §4).

    Numerator: the roster names that are CLASSIFIED Irreversible
    carriers, counted only when the gate's composition site is in
    place. Denominator: the whole roster. An empty roster is 0 bp (a
    vacuous class is a gate failure, never a pass); a roster name
    outside the classification is UNCOVERED (a forged name lowers the
    coverage — the machine source is the classification, never the
    roster alone)."""
    if roster is None:
        roster = _human_oversight_roster()
    if classification is None:
        classification = _irreversible_carriers()
    if composition_site is None:
        composition_site = _approval_gate_composition_site()
    total = len(roster)
    if total == 0 or not composition_site:
        return 0
    classified = set(classification)
    covered = sum(1 for name in roster if name in classified)
    return covered * 10000 // total


# ── №869 (wave 76 — W-E, ADR-0206 §4): the ATTEST_MAP machine ─────────

_ROW_STATUSES = ("live", "expired", "revoked")
_ROW_REQUIRED_FIELDS = (
    "claim=",
    "subject=",
    "subject_digest=",
    "evidence=",
    "verifier=",
    "window=",
    "actor=",
)


def _package_raw_rows(package_text: str | None = None) -> list[dict]:
    """The structural parse of the v1 package rows (the №868 line
    contract): a line counts as a row ONLY when it carries a 64-hex
    row digest, a known status and ALL seven key=value fields — a
    corrupted or summary line can never inflate the fact (the №565
    lesson: the reproducibility is structural)."""
    if package_text is None:
        if not PACKAGE_0206.exists():
            return []
        package_text = PACKAGE_0206.read_text(encoding="utf-8")
    rows: list[dict] = []
    for line in package_text.splitlines():
        if not line.startswith("row "):
            continue
        parts = line.split(" ")
        if len(parts) < 3 + len(_ROW_REQUIRED_FIELDS):
            continue
        digest, status = parts[1], parts[2]
        if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
            continue
        if status not in _ROW_STATUSES:
            continue
        body_parts = parts[3:]
        if not all(
            any(p.startswith(field) for p in body_parts)
            for field in _ROW_REQUIRED_FIELDS
        ):
            continue
        rows.append({"digest": digest, "status": status})
    return rows


def attest_map_live(package_text: str | None = None) -> int:
    """ATTEST_MAP — the number of LIVE attestation rows in the compiled
    compliance package (№869, ADR-0206 §4). Revoked rows stay in the
    package for the audit trail but attest nothing; expired rows are
    past their freshness window — neither counts. The count is over the
    RAW rows structurally parsed, never over a summary header."""
    return sum(1 for r in _package_raw_rows(package_text) if r["status"] == "live")


# ── №875 (wave 77 — W-F, ADR-0207 §4): the PROVENANCE_SHARE machine ────

def _provenance_registry(content: str | None = None) -> list[str]:
    """The wave-77 carrier registry (№873): the EXPLICIT registry
    transaction in src/provenance.rs — the machine source the Rust
    totality test pins against the classification (a name here that is
    not a classified Source/Sink carrier reddens the Rust test AND
    lowers this share — no silent shelves)."""
    if content is None:
        content = PROVENANCE_RS.read_text(encoding="utf-8")
    try:
        start = content.index("pub const PROVENANCE_CARRIERS")
        block = content[start:]
        block = block[: block.index("];")]
        return re.findall(r'name:\s*"([a-z0-9_]+)"', block)
    except ValueError:
        return []


def _classified_carrier_names(content: str | None = None) -> set[str]:
    """The classification SSOT (the machine source): every builtin the
    static map marks Role::Source or Role::Sink — the two I/O roles of
    the shared layer (the ingest sources, the egress sinks). The parse
    covers the committed array ONLY (the test module after
    #[cfg(test)] is cut — the test asserts are not carriers)."""
    if content is None:
        content = BUILTINS_CLASSIFICATION_RS.read_text(encoding="utf-8")
    try:
        block = content[content.index("pub static BUILTIN_CLASSES"):]
        block = block[: block.index("#[cfg(test)]")]
    except ValueError:
        return set()
    names: set[str] = set()
    for chunk in block.split("BuiltClassEntry {")[1:]:
        name = re.search(r'name:\s*"([^"]+)"', chunk)
        if not name:
            continue
        if "Role::Source" in chunk or "Role::Sink" in chunk:
            names.add(name.group(1))
    return names


def _provenance_gate_composition_site(content: str | None = None) -> bool:
    """The egress gate's composition site (№874): the fail-closed gate
    (`pub fn gate_emit`) lives in src/provenance.rs — the ONE surface
    the egress carriers dispatch through. If the composition is
    removed, NO egress carrier is gated — the share reads 0 (the gate
    working, not a pass)."""
    if content is None:
        content = PROVENANCE_RS.read_text(encoding="utf-8")
    return "pub fn gate_emit(carrier: &str" in content


def _provenance_boundary_wired(name: str, io_text: str, http_text: str) -> bool:
    """The LIVE wiring evidence of one carrier (the №862 form — the
    composition site fixed): the carrier's OWN boundary call with its
    OWN name in its implementation file. read_file/http_get record
    through `ingest`; the egress carriers record through the
    fail-closed `gate_emit`. A removed or renamed call un-wires the
    carrier — the share drops (the tamper traps prove the counter
    catches it)."""
    ingest_call = f'crate::provenance::ingest("{name}"'
    emit_call = f'crate::provenance::gate_emit("{name}"'
    return (
        ingest_call in io_text
        or ingest_call in http_text
        or emit_call in io_text
        or emit_call in http_text
    )


def provenance_share_bp(
    registry: list[str] | None = None,
    classification: set[str] | None = None,
    io_text: str | None = None,
    http_text: str | None = None,
    composition_site: bool | None = None,
) -> int:
    """PROVENANCE_SHARE — the share of the registered I/O carriers
    whose boundary provenance record is LIVE-wired, bps (№875, ADR-0207
    §4). Numerator: the registry names that are CLASSIFIED Source/Sink
    carriers AND carry the live boundary call in their implementation
    file. Denominator: the whole registry. An empty registry is 0 bp (a
    vacuous class is a gate failure, never a pass); a forged registry
    name (not classified) is UNCOVERED — the machine source is the
    classification, never the registry alone; a missing composition
    site zeroes the share (the gate working, not a pass)."""
    if registry is None:
        registry = _provenance_registry()
    if classification is None:
        classification = _classified_carrier_names()
    if io_text is None:
        io_text = BUILTINS_IO_RS.read_text(encoding="utf-8")
    if http_text is None:
        http_text = BUILTINS_HTTP_RS.read_text(encoding="utf-8")
    if composition_site is None:
        composition_site = _provenance_gate_composition_site()
    total = len(registry)
    if total == 0 or not composition_site:
        return 0
    covered = sum(
        1
        for name in registry
        if name in classification and _provenance_boundary_wired(name, io_text, http_text)
    )
    return covered * 10000 // total


# ── №881 (wave 78 — W-G, ADR-0208 §2.5): the OFFLINE_SUITE machine ────

def _offline_suite_registry(content: str | None = None) -> list[tuple[str, bool]]:
    """The wave-78 LLM-surface check registry (№881): the EXPLICIT
    roster in scripts/ci/gate_039_goals.txt — the machine source. Each
    row is `check <name> offline=yes|no`; the counter counts a row ONLY
    on the offline=yes field AND the check file's EXISTENCE (see
    _offline_check_exists — the count is checked, not declared)."""
    if content is None:
        content = GATE_039_GOALS.read_text(encoding="utf-8")
    rows: list[tuple[str, bool]] = []
    for line in content.splitlines():
        stripped = line.strip()
        if not stripped.startswith("check "):
            continue
        parts = stripped.split()
        if len(parts) != 3 or parts[2] not in ("offline=yes", "offline=no"):
            continue
        rows.append((parts[1], parts[2] == "offline=yes"))
    return rows


def _offline_check_exists(name: str, repo: Path | None = None) -> bool:
    """The existence half of the machine source: a registered check
    must BE a real test file — tests/<name>.rs or
    metalogos-server/tests/<name>.rs. A forged row (a name with no
    file behind it) is UNCOVERED — the declaration alone never counts
    (the №862 no-self-declaration form)."""
    base = repo if repo is not None else REPO
    return (base / "tests" / f"{name}.rs").is_file() or (
        base / "metalogos-server" / "tests" / f"{name}.rs"
    ).is_file()


def offline_suite_bp(
    registry: list[tuple[str, bool]] | None = None,
    repo: Path | None = None,
) -> int:
    """OFFLINE_SUITE — the share, in basis points, of the registered
    LLM-surface checks that execute WITHOUT the external network (№881,
    ADR-0208 §2.5). Numerator: the rows that carry offline=yes AND
    whose check file exists. Denominator: the whole registry. An empty
    registry is 0 bp (a vacuous class is a gate failure, never a
    pass); a forged row (offline=yes with no file) is UNCOVERED — the
    machine source is the filesystem, never the declaration alone."""
    if registry is None:
        registry = _offline_suite_registry()
    total = len(registry)
    if total == 0:
        return 0
    covered = sum(1 for name, offline in registry if offline and _offline_check_exists(name, repo))
    return covered * 10000 // total


# ── №888 (wave 79 — W-H, ADR-0209 §4): the JURISDICTION_MATRIX machine ──

def _jurisdiction_registry(content: str | None = None) -> list[str]:
    """The wave-79 jurisdiction registry (№888): the EXPLICIT roster in
    scripts/ci/gate_040_goals.txt — the machine source. Each row is
    `jurisdiction <code>`; the counter counts a row ONLY on the code's
    PRESENCE in the live seed matrix source (see
    _jurisdiction_in_source — the count is checked, not declared)."""
    if content is None:
        content = GATE_040_GOALS.read_text(encoding="utf-8")
    codes: list[str] = []
    for line in content.splitlines():
        stripped = line.strip()
        if not stripped.startswith("jurisdiction "):
            continue
        parts = stripped.split()
        if len(parts) != 2:
            continue
        codes.append(parts[1])
    return codes


def _jurisdiction_in_source(code: str, source: str | None = None) -> bool:
    """The presence half of the machine source: a registered jurisdiction
    must BE a real seed row — a quoted code inside the LIVE seed_matrix
    constructor (src/jurisdiction.rs, the function body between the
    `pub fn seed_matrix` anchor and the test module). The seed calls are
    rustfmt-normalized (the arguments reflow across lines), so the check
    pins the QUOTED CODE inside the seed region — not a call shape. A
    forged row (a code with no seed rule behind it) is UNCOVERED — the
    declaration alone never counts (the №862 no-self-declaration form)."""
    if source is None:
        source = JURISDICTION_RS.read_text(encoding="utf-8")
    start = source.find("pub fn seed_matrix")
    if start < 0:
        return False
    end = source.find("#[cfg(test)]", start)
    body = source[start : end if end > start else len(source)]
    return f'"{code}"' in body


def jurisdiction_matrix_count(
    registry: list[str] | None = None,
    source: str | None = None,
) -> int:
    """JURISDICTION_MATRIX — the number of registered jurisdictions that
    carry ≥ 1 live rule in the compiled matrix (№888, ADR-0209 §4). A
    row counts ONLY on its code's presence in the live seed source. An
    empty registry is 0 (a vacuous class is a gate failure, never a
    pass); a forged row (a code with no seed rule) is UNCOVERED — the
    machine source is the code, never the declaration alone."""
    if registry is None:
        registry = _jurisdiction_registry()
    return sum(1 for code in registry if _jurisdiction_in_source(code, source))


# ── №894 (wave 80 — W-I, ADR-0210 §3-4): the REPLAY_MATCH machine ──────

def _replay_registry(content: str | None = None) -> list[str]:
    """The wave-80 decision-carrier registry (№894): the EXPLICIT roster
    in scripts/ci/gate_041_goals.txt — the machine source. Each row is
    `decision <name> wired=yes|no`; the counter counts a row ONLY on
    the decision name's PRESENCE in the live e2e wiring source (see
    _replay_carrier_wired — the count is checked, not declared)."""
    if content is None:
        content = GATE_041_GOALS.read_text(encoding="utf-8")
    names: list[str] = []
    for line in content.splitlines():
        stripped = line.strip()
        if not stripped.startswith("decision "):
            continue
        parts = stripped.split()
        if len(parts) != 3 or not parts[2].startswith("wired="):
            continue
        names.append(parts[1])
    return names


def _replay_composition_site(content: str | None = None) -> bool:
    """The coverage gate's composition site (№893, the №874 form): the
    fail-closed gate (`pub fn replay_gate`) lives in src/decision_trace.rs
    — the ONE surface the traced decisions reconcile through. If the
    composition is removed, NO decision is gated — the share reads 0
    (the gate working, not a pass)."""
    if content is None:
        content = DECISION_TRACE_RS.read_text(encoding="utf-8")
    return "pub fn replay_gate" in content


def _replay_carrier_wired(name: str, e2e_text: str | None = None) -> bool:
    """The LIVE wiring evidence of one decision carrier (the №862 form —
    the composition site fixed): the decision's OWN name as a quoted
    literal in the LIVE e2e wiring source, where the carrier's run is
    traced (the trace row built from the road's real output) AND the
    trace journals through the live Ledger ACTION contour AND the gate
    reconciles the claim (record + record_trace + replay_gate — the
    №893 e2e). A removed or renamed wiring un-wires the carrier — the
    share drops (the tamper traps prove the counter catches it)."""
    if e2e_text is None:
        e2e_text = REPLAY_E2E_RS.read_text(encoding="utf-8")
    return f'"{name}"' in e2e_text


def replay_match_bp(
    registry: list[str] | None = None,
    e2e_text: str | None = None,
    composition_site: bool | None = None,
) -> int:
    """REPLAY_MATCH — the share of the registered decision carriers
    whose decision trace is written AND verified by the replay_gate
    REPLAY_MATCH contract, bps (№894, ADR-0210 §3-4). Numerator: the
    registry names that carry the live wiring evidence in the e2e
    source. Denominator: the whole registry. An empty registry is 0 bp
    (a vacuous class is a gate failure, never a pass); a forged
    registry name (no wiring behind it) is UNCOVERED — the machine
    source is the wiring, never the registry alone; a missing
    composition site zeroes the share (the gate working, not a pass)."""
    if registry is None:
        registry = _replay_registry()
    if composition_site is None:
        composition_site = _replay_composition_site()
    total = len(registry)
    if total == 0 or not composition_site:
        return 0
    covered = sum(1 for name in registry if _replay_carrier_wired(name, e2e_text))
    return covered * 10000 // total


# ── №906 (wave 81 — W-J, ADR-0217 §2): the TAINT_SATURATION machine ─────

def _taint_roster_rows(fuzzer_text: str | None = None) -> list[tuple[str, ...]]:
    """The №904 scenario roster (the №862 quoted-region form): the rows
    between the `№904 taint roster BEGIN/END` markers in the LIVE fuzzer
    source. The scan is TUPLE-based (a `(` followed — after any
    whitespace, rustfmt may wrap the row — by a quote, closed at depth
    0), every block must carry exactly 5 string fields; a lost marker
    or a malformed block is a loud fail-closed panic — the count cannot
    silently shrink."""
    if fuzzer_text is None:
        fuzzer_text = TAINT_FUZZER_RS.read_text(encoding="utf-8")
    begin = fuzzer_text.find("№904 taint roster BEGIN")
    if begin < 0:
        raise ValueError(
            "the roster marker BEGIN is lost from "
            f"{TAINT_FUZZER_RS.name} — the machine source is unparsable"
        )
    rel_end = fuzzer_text[begin:].find("№904 taint roster END")
    if rel_end < 0:
        raise ValueError(
            "the roster marker END is lost from " f"{TAINT_FUZZER_RS.name}"
        )
    section = fuzzer_text[begin : begin + rel_end]
    # ALL scanning runs in BYTE coordinates (the markers' region carries
    # multi-byte №/─ characters — mixing byte indices with str indices
    # desynchronizes the block boundaries; bytes only, decode at the end).
    b = section.encode("utf-8")
    rows: list[tuple[str, ...]] = []
    i = 0
    while i < len(b):
        first = i + 1
        while first < len(b) and chr(b[first]).isspace():
            first += 1
        if b[i] == 0x28 and first < len(b) and b[first] == 0x22:
            depth = 1
            j = i + 1
            while j < len(b) and depth > 0:
                if b[j] == 0x28:
                    depth += 1
                elif b[j] == 0x29:
                    depth -= 1
                j += 1
            block = b[i + 1 : j - 1].decode("utf-8")
            fields = re.findall(r'"([^"]*)"', block)
            if len(fields) != 5:
                raise ValueError(
                    "a roster row must carry exactly 5 fields (id, lane, "
                    f"source, sink, expect), got {len(fields)}: {block!r}"
                )
            rows.append(tuple(fields))
            i = j
        else:
            i += 1
    return rows


def _taint_lanes_alive() -> int:
    """The live label lanes (ADR-0217 §2): L = the three lane modules
    PRESENT — a removed lane module zeroes the factor (fail-closed:
    a partially removed lane surface is not a smaller number, it is
    zero — the lane semantics the generator rides on are gone)."""
    return 3 if all(p.exists() for p in TAINT_LANE_MODULES) else 0


def _taint_sources_alive(rows: list[tuple[str, ...]] | None = None) -> int:
    """The live sources (ADR-0217 §2): S = the roster's DISTINCT source
    fields, each cross-checked against the live label/classification/
    audit files — a source the engine no longer names is not a source
    (the factor drops to 0: a generator over dead names is a vacuum)."""
    if rows is None:
        rows = _taint_roster_rows()
    if not rows:
        return 0
    anchor_text = " ".join(
        p.read_text(encoding="utf-8") for p in TAINT_SOURCE_ANCHORS if p.exists()
    )
    sources = {row[2] for row in rows}
    if any(s not in anchor_text for s in sources):
        return 0
    return len(sources)


def _taint_sinks_alive(
    audit_text: str | None = None, class_text: str | None = None
) -> int:
    """The live sinks (ADR-0217 §2): K = |SINK_FUNCTIONS| (the №565 list
    in src/audit.rs) + |SQL_ARG_POSITIONS| (the №831 SSOT table in
    src/builtins_classification.rs) — both parsed from the LIVE static
    initializers, so a removed sink name lowers K (the tamper trap (в))."""
    if audit_text is None:
        audit_text = AUDIT_RS.read_text(encoding="utf-8")
    if class_text is None:
        class_text = CLASSIFICATION_RS.read_text(encoding="utf-8")
    m = re.search(
        r"const SINK_FUNCTIONS:\s*&\[&str\]\s*=\s*&\[(.*?)\];",
        audit_text,
        re.S,
    )
    if not m:
        return 0
    sinks = re.findall(r'"([^"]+)"', m.group(1))
    m = re.search(
        r"static SQL_ARG_POSITIONS:\s*&\[\(&str,\s*usize\)\]\s*=\s*&\[(.*?)\];",
        class_text,
        re.S,
    )
    if not m:
        return 0
    sql_points = re.findall(r'\("([^"]+)",\s*\d+\)', m.group(1))
    return len(sinks) + len(sql_points)


def _taint_sanitizer_alive(class_text: str | None = None) -> int:
    """The live sanitizer (ADR-0217 §2): Z = 1 while the escape_html
    classification entry stands (src/builtins_classification.rs — the
    :142 anchor, the Sanitized stamp per audit.rs), else 0."""
    if class_text is None:
        class_text = CLASSIFICATION_RS.read_text(encoding="utf-8")
    entry = re.search(
        r'BuiltClassEntry \{ name: "escape_html",[^}]*rationale: '
        r'"[^"]*[Ss]anitiz',
        class_text,
    )
    return 1 if entry else 0


def _taint_generator_fb(fuzzer_text: str | None = None) -> tuple[int, int]:
    """The generator's statement forms and budget (ADR-0217 §2): F = the
    FORM_* roster size, B = the statement budget constant — both read
    from the LIVE fuzzer source (the №766/№465 file)."""
    if fuzzer_text is None:
        fuzzer_text = TAINT_FUZZER_RS.read_text(encoding="utf-8")
    forms = re.findall(r"^const FORM_[A-Z_]+: u8", fuzzer_text, re.M)
    m = re.search(r"let mut budget = (\d+)u32", fuzzer_text)
    if not forms or not m:
        return 0, 0
    return len(forms), int(m.group(1))


def taint_saturation(
    roster: list[tuple[str, ...]] | None = None,
    lanes: int | None = None,
    sinks: int | None = None,
    sanitizers: int | None = None,
    forms: int | None = None,
    budget: int | None = None,
) -> int:
    """TAINT_SATURATION — the exact count of distinct taint-relevant
    programs the wave-81 generator can emit, an integer product over
    LIVE anchors (№906, ADR-0217 §2):

        Space = ScenarioVariants × SequenceSpace
        ScenarioVariants = T × L × S × K × Z
        SequenceSpace = Σ_{k=0..B} F^k

    Every factor is READ from the live tree (the anchors in the module
    docstring) — the count is CHECKED, not declared. A dead anchor
    zeroes the space (a removed roster region, a removed lane module,
    a removed sanitizer entry, a dead generator roster) — the vacuous
    class is a gate failure, never a pass; python int — no overflow."""
    rows = _taint_roster_rows() if roster is None else roster
    t_variants = len(rows)
    l_lanes = _taint_lanes_alive() if lanes is None else lanes
    s_sources = _taint_sources_alive(rows)
    k_sinks = _taint_sinks_alive() if sinks is None else sinks
    z_sanitizers = _taint_sanitizer_alive() if sanitizers is None else sanitizers
    f_forms, b_budget = _taint_generator_fb()
    if forms is not None:
        f_forms = forms
    if budget is not None:
        b_budget = budget
    if 0 in (t_variants, l_lanes, s_sources, k_sinks, z_sanitizers, f_forms, b_budget):
        return 0
    scenario_variants = t_variants * l_lanes * s_sources * k_sinks * z_sanitizers
    sequence_space = sum(f_forms**k for k in range(b_budget + 1))
    return scenario_variants * sequence_space


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

    # ── №862 traps: the APPROVAL_COVERAGE machine catches the tamper ──
    good_approval = APPROVAL_RS.read_text(encoding="utf-8")
    full_appr = approval_coverage_bp()
    if full_appr != APPROVAL_COVERAGE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the approval coverage is not at the owner floor "
            f"({full_appr} bp != {APPROVAL_COVERAGE_FLOOR_OWNER_BP} bp): the roster or "
            "the gate composition drifted (fix the gate, never the floor)",
            file=sys.stderr,
        )
        return 1

    # Trap 12 (the placeholder/roster-substitution class): a roster name
    # replaced with a FORGED name (not in the classification) lowers the
    # coverage — the machine source is the classification, never the
    # roster alone.
    stub_appr = good_approval.replace('"cron_run"', '"cron_run_fake"', 1)
    if stub_appr == good_approval:
        print(
            "tamper-test: TRAP FAILED — the HUMAN_OVERSIGHT roster anchor drifted",
            file=sys.stderr,
        )
        return 1
    if approval_coverage_bp(roster=_human_oversight_roster(stub_appr)) >= full_appr:
        print(
            "tamper-test: TRAP FAILED — the forged roster name did NOT lower APPROVAL_COVERAGE",
            file=sys.stderr,
        )
        return 1

    # Trap 13 (the deletion class): the gate's composition site removed
    # zeroes the coverage — the class is uncovered (the silent-bypass
    # class of №846 Trap 6).
    if approval_coverage_bp(composition_site=False) != 0:
        print(
            "tamper-test: TRAP FAILED — the removed composition site did NOT zero APPROVAL_COVERAGE",
            file=sys.stderr,
        )
        return 1

    # Trap 14 (the definition-substitution control): a counter that
    # counted by roster EXISTENCE alone would stay at the floor with an
    # EMPTY classification — the real one reads the classification and
    # reports 0 (the fact is checked, not declared).
    if approval_coverage_bp(classification=[]) != 0:
        print(
            "tamper-test: TRAP FAILED — the counter answered without the classification "
            "check (the definition was substituted)",
            file=sys.stderr,
        )
        return 1

    # Trap 15 (the negative control (б), the №846/№857 form): the
    # gate_036 floor LOWERED below the owner constant is a loud refusal
    # (the floor is only-up — plan §23, revision R38).
    appr_strict = load_floors_strict(GATE_036_GOALS)
    appr_value = appr_strict.get("goal_approval_coverage_bp")
    if appr_value is None or appr_value < APPROVAL_COVERAGE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the gate_036 floor is below the owner constant "
            f"(plan §23, revision R38: {APPROVAL_COVERAGE_FLOOR_OWNER_BP} bp) — "
            "the lowering is refused",
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

    # ── №869 traps: the ATTEST_MAP machine catches the tamper ─────────
    full_att = attest_map_live()
    if full_att < ATTEST_MAP_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — ATTEST_MAP is below the owner floor on the "
            f"clean tree ({full_att} < {ATTEST_MAP_FLOOR_OWNER}): the compliance "
            "package drifted (re-attest with the №868 compiler, never the floor)",
            file=sys.stderr,
        )
        return 1

    # Trap 16 (the summary-substitution class, the №565 lesson): a
    # package whose header CLAIMS a row count but carries NO raw row
    # lines counts 0 — the fact reads the raw rows structurally, never
    # the summary.
    fake_summary = (
        "# METALOGOS compliance package v1\n"
        "# package_digest: " + "a" * 64 + "\n"
        "# rows_total: 999\n"
        "# rows_live: 999\n"
    )
    if attest_map_live(fake_summary) != 0:
        print(
            "tamper-test: TRAP FAILED — the summary-only package was COUNTED "
            "(the fact read the header, not the raw rows)",
            file=sys.stderr,
        )
        return 1

    # Trap 17 (the deletion class): an empty / row-less package counts 0.
    if attest_map_live("") != 0:
        print(
            "tamper-test: TRAP FAILED — the empty package did NOT count 0",
            file=sys.stderr,
        )
        return 1

    # Trap 18 (the definition-substitution class): a counter counting
    # ALL rows (revoked/expired included) would overcount the fact —
    # the real one counts LIVE rows only (a revoked row attests
    # nothing, the ADR-0206 §2.2 append-only semantics).
    sample = (
        "# package v1\n"
        "row " + "a" * 64 + " live claim=c subject=s subject_digest=" + "b" * 64
        + " evidence=artifact:x@" + "c" * 64 + " verifier=v window=0-0 actor=a\n"
        "row " + "d" * 64 + " revoked claim=c2 subject=s2 subject_digest=" + "e" * 64
        + " evidence=artifact:y@" + "f" * 64 + " verifier=v window=0-0 actor=a\n"
        "row " + "1" * 64 + " expired claim=c3 subject=s3 subject_digest=" + "2" * 64
        + " evidence=artifact:z@" + "3" * 64 + " verifier=v window=0-9 actor=a\n"
    )
    if attest_map_live(sample) != 1:
        print(
            "tamper-test: TRAP FAILED — the counter did not distinguish the LIVE "
            "rows from the revoked/expired ones (the definition was substituted)",
            file=sys.stderr,
        )
        return 1

    # Trap 19 (the corrupted-row class): a row line missing a required
    # field is NOT a row — it cannot inflate the fact.
    corrupted = "row " + "a" * 64 + " live claim=c subject=s\n"
    if attest_map_live(corrupted) != 0:
        print(
            "tamper-test: TRAP FAILED — the corrupted row (missing fields) was COUNTED",
            file=sys.stderr,
        )
        return 1

    # Trap 20 (the negative control (б), the №846/№857 form): the
    # gate_037 floor LOWERED below the owner constant is a loud refusal
    # (the floor is only-up — the wave start floor).
    attest_strict = load_floors_strict(GATE_037_GOALS)
    attest_value = attest_strict.get("goal_attest_map")
    if attest_value is None or attest_value < ATTEST_MAP_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — the gate_037 floor is below the owner constant "
            f"(the wave-76 start floor: {ATTEST_MAP_FLOOR_OWNER}) — the lowering is refused",
            file=sys.stderr,
        )
        return 1

    # ── №875 traps: the PROVENANCE_SHARE machine catches the tamper ───
    full_prov = provenance_share_bp()
    if full_prov != PROVENANCE_SHARE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the provenance share is not at the owner floor on the "
            f"clean tree ({full_prov} bp != {PROVENANCE_SHARE_FLOOR_OWNER_BP} bp): the registry "
            "or the boundary wiring drifted (fix the wiring, never the floor)",
            file=sys.stderr,
        )
        return 1

    # Trap 21 (the roster-substitution class, the №862 Trap 12 form): a
    # registry name replaced with a FORGED name (not in the
    # classification) lowers the share — the machine source is the
    # classification, never the registry alone.
    stub_registry = [
        name if name != "print" else "print_forged" for name in _provenance_registry()
    ]
    if provenance_share_bp(registry=stub_registry) >= full_prov:
        print(
            "tamper-test: TRAP FAILED — the forged registry name did NOT lower PROVENANCE_SHARE",
            file=sys.stderr,
        )
        return 1

    # Trap 22 (the deletion class): a boundary call removed from the
    # carrier's implementation un-wires the carrier — the share drops
    # (the silent-bypass class of №846 Trap 5/№862 Trap 13).
    good_io = BUILTINS_IO_RS.read_text(encoding="utf-8")
    stub_io = good_io.replace(
        'crate::provenance::gate_emit("write_file"',
        'crate::provenance::MISSING_GATE("write_file"',
        1,
    )
    if stub_io == good_io:
        print(
            "tamper-test: TRAP FAILED — the write_file wiring anchor drifted (the fixture is stale)",
            file=sys.stderr,
        )
        return 1
    if provenance_share_bp(io_text=stub_io) >= full_prov:
        print(
            "tamper-test: TRAP FAILED — the removed write_file wiring did NOT lower PROVENANCE_SHARE",
            file=sys.stderr,
        )
        return 1

    # Trap 23 (the definition-substitution control, the №857 Trap 10
    # form): a counter counting by registry EXISTENCE alone would stay
    # at the floor with the gate's composition site removed — the real
    # one reads the composition and reports 0 (the fact is checked,
    # not declared).
    if provenance_share_bp(composition_site=False) != 0:
        print(
            "tamper-test: TRAP FAILED — the counter answered without the gate composition "
            "site (the definition was substituted)",
            file=sys.stderr,
        )
        return 1

    # Trap 24 (the negative control (б), the №846/№857 form): the
    # gate_038 floor LOWERED below the owner constant is a loud refusal
    # (the floor is only-up — the wave-77 dispatch constant).
    prov_strict = load_floors_strict(GATE_038_GOALS)
    prov_value = prov_strict.get("goal_provenance_share_bp")
    if prov_value is None or prov_value < PROVENANCE_SHARE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the gate_038 floor is below the owner constant "
            f"(the wave-77 dispatch constant: {PROVENANCE_SHARE_FLOOR_OWNER_BP} bp) — "
            "the lowering is refused",
            file=sys.stderr,
        )
        return 1

    # ── №881 traps: the OFFLINE_SUITE machine catches the tamper ────
    full_off = offline_suite_bp()
    if full_off < OFFLINE_SUITE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the offline suite is below the owner floor on the "
            f"clean tree ({full_off} bp < {OFFLINE_SUITE_FLOOR_OWNER_BP} bp): the registry or "
            "the check files drifted (fix the roster, never the floor)",
            file=sys.stderr,
        )
        return 1

    # Trap 25 (the field-flip class): a registry row flipped offline=yes
    # → no LOWERS the share — the counter must READ the field (a counter
    # counting rows alone would stay at the floor).
    good_registry = _offline_suite_registry()
    stub_registry = [
        (name, False) if name == "naryad_880_failover_coverage" else (name, offline)
        for name, offline in good_registry
    ]
    if offline_suite_bp(registry=stub_registry) >= full_off:
        print(
            "tamper-test: TRAP FAILED — the offline=yes→no flip did NOT lower OFFLINE_SUITE",
            file=sys.stderr,
        )
        return 1

    # Trap 26 (the forged-row class, the №862 Trap 12 form): a row whose
    # name has NO check file behind it is UNCOVERED — the machine source
    # is the filesystem, never the declaration alone.
    forged_registry = good_registry + [("n999_forged_llm_check", True)]
    if offline_suite_bp(registry=forged_registry) >= full_off:
        print(
            "tamper-test: TRAP FAILED — the forged registry row (no check file) did NOT "
            "lower OFFLINE_SUITE",
            file=sys.stderr,
        )
        return 1

    # Trap 27 (the vacuous-registry control, the №875 Trap 23 form): an
    # empty registry reads 0 bp — a vacuous class is a gate failure,
    # never a pass.
    if offline_suite_bp(registry=[]) != 0:
        print(
            "tamper-test: TRAP FAILED — the empty registry must read 0 bp (the vacuous "
            "class is a failure, never a pass)",
            file=sys.stderr,
        )
        return 1

    # Trap 28 (the negative control (б), the №846/№857/№875 form): the
    # gate_039 floor LOWERED below the owner constant is a loud refusal
    # (the floor is only-up — the wave-78 dispatch constant).
    off_strict = load_floors_strict(GATE_039_GOALS)
    off_value = off_strict.get("goal_offline_suite_bp")
    if off_value is None or off_value < OFFLINE_SUITE_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the gate_039 floor is below the owner constant "
            f"(the wave-78 dispatch constant: {OFFLINE_SUITE_FLOOR_OWNER_BP} bp) — "
            "the lowering is refused",
            file=sys.stderr,
        )
        return 1

    # ── №888 traps: the JURISDICTION_MATRIX machine catches the tamper ──
    jur_registry = _jurisdiction_registry()
    full_jur = jurisdiction_matrix_count()

    # Trap 29 (the clean-tree assertion, the №1306 form): the seed on
    # the clean tree carries the owner-constant jurisdictions.
    if full_jur < JURISDICTION_MATRIX_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — the clean tree carries fewer live "
            f"jurisdictions ({full_jur}) than the owner constant "
            f"({JURISDICTION_MATRIX_FLOOR_OWNER}): the seed matrix or the "
            "registry drifted",
            file=sys.stderr,
        )
        return 1

    # Trap 30 (the vacuous-registry control, the №875 Trap 27 form): an
    # empty registry reads 0 — a vacuous class is a gate failure, never
    # a pass.
    if jurisdiction_matrix_count(registry=[]) != 0:
        print(
            "tamper-test: TRAP FAILED — the empty registry must read 0 (the vacuous "
            "class is a failure, never a pass)",
            file=sys.stderr,
        )
        return 1

    # Trap 31 (the forged row, the №862 no-self-declaration form): a
    # registry row whose jurisdiction has NO seed rule behind it is
    # UNCOVERED — the count does NOT rise on the forgery.
    forged = jur_registry + ["XX"]
    if jurisdiction_matrix_count(registry=forged) != full_jur:
        print(
            "tamper-test: TRAP FAILED — a forged jurisdiction row (no seed rule "
            "behind it) RAISED the count — the declaration counted, not the code",
            file=sys.stderr,
        )
        return 1

    # Trap 32 (the row-loss control): removing a live jurisdiction from
    # the registry LOWERS the count — the machine follows the source.
    if len(jur_registry) > 0:
        shrunk = jur_registry[:-1]
        if jurisdiction_matrix_count(registry=shrunk) >= full_jur:
            print(
                "tamper-test: TRAP FAILED — dropping a registered jurisdiction did NOT "
                "lower JURISDICTION_MATRIX",
                file=sys.stderr,
            )
            return 1

    # Trap 33 (the negative control (б), the №846 form): the gate_040
    # floor LOWERED below the owner constant is a loud refusal (the
    # floor is only-up — the wave-79 dispatch constant).
    jur_strict = load_floors_strict(GATE_040_GOALS)
    jur_value = jur_strict.get("goal_jurisdiction_matrix")
    if jur_value is None or jur_value < JURISDICTION_MATRIX_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — the gate_040 floor is below the owner constant "
            f"(the wave-79 dispatch constant: {JURISDICTION_MATRIX_FLOOR_OWNER}) — "
            "the lowering is refused",
            file=sys.stderr,
        )
        return 1

    # ── №894 traps: the REPLAY_MATCH machine catches the tamper ──
    rep_registry = _replay_registry()
    full_rep = replay_match_bp()

    # Trap 34 (the clean-tree assertion, the №1306 form): the clean tree
    # carries the owner-constant share — every registered carrier is
    # live-wired and the composition site stands.
    if full_rep < REPLAY_MATCH_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the clean tree reads REPLAY_MATCH "
            f"{full_rep} bp below the owner constant "
            f"({REPLAY_MATCH_FLOOR_OWNER_BP} bp): the e2e wiring or the "
            "gate composition drifted",
            file=sys.stderr,
        )
        return 1

    # Trap 35 (the vacuous-registry control, the №875 Trap 27 form): an
    # empty registry reads 0 — a vacuous class is a gate failure, never
    # a pass.
    if replay_match_bp(registry=[]) != 0:
        print(
            "tamper-test: TRAP FAILED — the empty registry must read 0 (the vacuous "
            "class is a failure, never a pass)",
            file=sys.stderr,
        )
        return 1

    # Trap 36 (the forged row, the №862 no-self-declaration form): a
    # registry row whose decision has NO live wiring behind it is
    # UNCOVERED — the forgery cannot FABRICATE coverage: the share
    # never rises on it (the forged row only dilutes the denominator —
    # the honest drop the --check floor breach refuses loudly).
    forged_rep = rep_registry + ["never_traced_decision"]
    if replay_match_bp(registry=forged_rep) > full_rep:
        print(
            "tamper-test: TRAP FAILED — a forged decision row (no live wiring "
            "behind it) RAISED the share — the declaration counted, not the code",
            file=sys.stderr,
        )
        return 1

    # Trap 37 (the row-loss control, the share form): dropping a live
    # carrier from the registry LOWERS the number of live-wired
    # carriers — the machine follows the source (the bp share of an
    # all-wired registry stays 10000 by arithmetic; the honest
    # observable is the covered-carrier COUNT, the mass the floor
    # protects).
    if len(rep_registry) > 0:
        shrunk_rep = rep_registry[:-1]
        full_covered = sum(1 for n in rep_registry if _replay_carrier_wired(n))
        shrunk_covered = sum(1 for n in shrunk_rep if _replay_carrier_wired(n))
        if shrunk_covered >= full_covered:
            print(
                "tamper-test: TRAP FAILED — dropping a registered decision carrier "
                "did NOT lower the live-wired carrier count",
                file=sys.stderr,
            )
            return 1

    # Trap 38 (the composition-site control, the №874 form): the gate
    # composition removed → NOTHING is gated → the share reads 0 (the
    # gate working, not a pass).
    if replay_match_bp(composition_site=False) != 0:
        print(
            "tamper-test: TRAP FAILED — the removed replay_gate composition must "
            "zero the share (the gate working, not a pass)",
            file=sys.stderr,
        )
        return 1

    # Trap 39 (the negative control (б), the №846 form): the gate_041
    # floor LOWERED below the owner constant is a loud refusal (the
    # floor is only-up — the wave-80 dispatch constant).
    rep_strict = load_floors_strict(GATE_041_GOALS)
    rep_value = rep_strict.get("goal_replay_match_bp")
    if rep_value is None or rep_value < REPLAY_MATCH_FLOOR_OWNER_BP:
        print(
            "tamper-test: TRAP FAILED — the gate_041 floor is below the owner constant "
            f"(the wave-80 dispatch constant: {REPLAY_MATCH_FLOOR_OWNER_BP} bp) — "
            "the lowering is refused",
            file=sys.stderr,
        )
        return 1

    # ── №906 traps: the TAINT_SATURATION machine catches the tamper ──
    # (the наряд-трэпы (а)–(д): the roster row loss, the dead anchor
    # region, the removed lane/sink, the lowered floor, the vacuum).
    full_taint = taint_saturation()

    # Trap 40 (the clean-tree assertion, the №1306 form): the clean tree
    # carries a space at or above the owner constant (the wave-81
    # dispatch floor: 10^9).
    if full_taint < TAINT_SATURATION_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — the clean tree reads TAINT_SATURATION "
            f"{full_taint} below the owner constant "
            f"({TAINT_SATURATION_FLOOR_OWNER}): a live anchor or the "
            "generator's reach drifted",
            file=sys.stderr,
        )
        return 1

    # Trap 41 (the vacuum control, the наряд-трэп (д), the №875 Trap 27
    # form): an EMPTY roster reads 0 — a vacuous class is a gate
    # failure, never a pass (the №621 lesson).
    if taint_saturation(roster=[]) != 0:
        print(
            "tamper-test: TRAP FAILED — the empty roster must read 0 (the vacuous "
            "class is a failure, never a pass)",
            file=sys.stderr,
        )
        return 1

    # Trap 42 (the row-loss control, the наряд-трэп (а), the №894 Trap 37
    # form): dropping a live scenario from the roster LOWERS the space —
    # the machine follows the source. (The honest observable is the
    # strict drop, not a floor breach: the live space is ~10^23, orders
    # above the 10^9 floor — a single lost scenario cannot breach the
    # floor arithmetically; the machine still MUST move with the source.)
    if len(_taint_roster_rows()) > 0:
        shrunk_roster = _taint_roster_rows()[:-1]
        if taint_saturation(roster=shrunk_roster) >= full_taint:
            print(
                "tamper-test: TRAP FAILED — dropping a roster scenario did NOT "
                "lower the saturation space (the declaration counted, not the code)",
                file=sys.stderr,
            )
            return 1

    # Trap 43 (the dead-anchor control, the наряд-трэп (б)): the roster
    # region removed (the markers gone) is a LOUD failure — the machine
    # refuses to read a dead source, it never silently passes.
    try:
        _taint_roster_rows("no markers here")
        print(
            "tamper-test: TRAP FAILED — the removed roster region must be a loud "
            "failure, not a silent read",
            file=sys.stderr,
        )
        return 1
    except ValueError:
        pass

    # Trap 44 (the lane/sink removal, the наряд-трэп (в)): a removed lane
    # module zeroes the L factor; a removed sink name lowers K — both
    # move the machine (the lane-zero is the ADR's fail-closed form).
    if taint_saturation(lanes=0) != 0:
        print(
            "tamper-test: TRAP FAILED — a removed lane module must ZERO the space "
            "(the lane semantics are gone — the ADR-0217 fail-closed form)",
            file=sys.stderr,
        )
        return 1
    if taint_saturation(sinks=_taint_sinks_alive() - 1) >= full_taint:
        print(
            "tamper-test: TRAP FAILED — a removed sink name did NOT lower the "
            "saturation space",
            file=sys.stderr,
        )
        return 1

    # Trap 45 (the sanitizer removal): the escape_html entry gone → the
    # clean-twin half of the roster is unsanitizable → the space reads 0
    # (the Z factor is the ADR's fail-closed form).
    if taint_saturation(sanitizers=0) != 0:
        print(
            "tamper-test: TRAP FAILED — a removed sanitizer entry must ZERO the "
            "space (the ADR-0217 fail-closed form)",
            file=sys.stderr,
        )
        return 1

    # Trap 46 (the negative control, the наряд-трэп (г), the №846 form):
    # the gate_042 floor LOWERED below the owner constant is a loud
    # refusal (the floor is only-up — the wave-81 dispatch constant 10^9).
    rep42_strict = load_floors_strict(GATE_042_GOALS)
    taint_floor = rep42_strict.get("goal_taint_saturation_min")
    if taint_floor is None or taint_floor < TAINT_SATURATION_FLOOR_OWNER:
        print(
            "tamper-test: TRAP FAILED — the gate_042 floor is below the owner "
            "constant (the wave-81 dispatch constant: "
            f"{TAINT_SATURATION_FLOOR_OWNER}) — the lowering is refused",
            file=sys.stderr,
        )
        return 1

    print(
        "tamper-test: OK — the checkers catch the fabricated records "
        f"(LICENSED_FLOOR {full_lic} bp, PASSPORT_FRESH {full_pass} bp, "
        f"APPROVAL_COVERAGE {full_appr} bp, ATTEST_MAP {full_att} live rows, "
        f"PROVENANCE_SHARE {full_prov} bp, OFFLINE_SUITE {full_off} bp, "
        f"JURISDICTION_MATRIX {full_jur}, REPLAY_MATCH {full_rep} bp, "
        f"TAINT_SATURATION {full_taint}; "
        "the traps lowered all)"
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
        f"| PROVENANCE_SHARE (the 8C/C3 machine fact, №875) | {provenance_share_bp()} bp — the registered I/O carriers whose boundary provenance record is LIVE-wired (ADR-0207 §4; the gate_038 floor 10000 bp, only-up) |\n"
        f"| OFFLINE_SUITE (the 8E-1/C5 machine fact, №881) | {offline_suite_bp()} bp — the registered LLM-surface checks that execute without the external network (ADR-0208 §2.5; the gate_039 floor 8000 bp, only-up) |\n"
        f"| JURISDICTION_MATRIX (the 8D-2/C10 machine fact, №888) | {jurisdiction_matrix_count()} — the registered jurisdictions with ≥ 1 live rule in the seed matrix (ADR-0209 §4; the gate_040 floor 3, only-up) |\n"
        f"| REPLAY_MATCH (the 8G/C8 machine fact, №894) | {replay_match_bp()} bp — the registered decision carriers whose trace is written through the Ledger ACTION contour and verified by the replay_gate REPLAY_MATCH contract (ADR-0210 §3-4; the gate_041 floor 10000 bp, only-up) |\n"
        f"| TAINT_SATURATION (the 8F-2/C9 machine fact, №906) | {taint_saturation()} — the exact reachable taint-program space of the wave-81 generator, the integer product over the live anchors (ADR-0217 §2; the gate_042 floor 10^9, only-up) |\n"
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
        f"| APPROVAL_COVERAGE — the 8B-2 machine fact (№862) | {approval_coverage_bp()} bp |\n"
        f"| ATTEST_MAP — the 8A-1 machine fact (№869) | {attest_map_live()} live attestation rows |\n"
        f"| PROVENANCE_SHARE — the 8C/C3 machine fact (№875) | {provenance_share_bp()} bp |\n"
        f"| OFFLINE_SUITE — the 8E-1/C5 machine fact (№881) | {offline_suite_bp()} bp |\n"
        f"| JURISDICTION_MATRIX — the 8D-2/C10 machine fact (№888) | {jurisdiction_matrix_count()} jurisdictions |\n"
        f"| REPLAY_MATCH — the 8G/C8 machine fact (№894) | {replay_match_bp()} bp |\n"
        f"| TAINT_SATURATION — the 8F-2/C9 machine fact (№906) | {taint_saturation()} |\n"
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

    # №862: the APPROVAL_COVERAGE floor twin (the only-up gate_036) —
    # the STRICT loader (fail-closed on a missing/empty/corrupted file)
    # and the owner-pinned lowering refusal (the plan §23 floor can
    # rise, never fall). The report names the roster and the gate
    # composition state (the №468 honest form).
    if check:
        try:
            appr_floors = load_floors_strict(GATE_036_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        appr_floor = appr_floors.get("goal_approval_coverage_bp")
        if appr_floor is None:
            print(
                "ERROR: gate_036_goals.txt carries no goal_approval_coverage_bp floor",
                file=sys.stderr,
            )
            return 2
        if appr_floor < APPROVAL_COVERAGE_FLOOR_OWNER_BP:
            print(
                "FLOOR LOWERING REFUSED: gate_036 "
                f"({appr_floor} bp) is below the owner constant "
                f"({APPROVAL_COVERAGE_FLOOR_OWNER_BP} bp — plan §23, revision R38); "
                "the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        appr_cov = approval_coverage_bp()
        if appr_cov < appr_floor:
            print(
                f"FLOOR BREACH: APPROVAL_COVERAGE {appr_cov} bp < the gate_036 floor {appr_floor} bp "
                "(only-up) — a human-oversight carrier landed WITHOUT the approval gate "
                "(keep the roster total over the classification and the composition "
                "site in capability.rs — never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        print(
            f"OK: the №862 floor holds — APPROVAL_COVERAGE {appr_cov} bp "
            f"(gate_036 floor {appr_floor} bp, only-up) — roster: "
            f"{', '.join(_human_oversight_roster()) or 'none'}; "
            f"composition site: {'present' if _approval_gate_composition_site() else 'MISSING'}"
        )

    # №869: the ATTEST_MAP floor twin (the only-up gate_037) — the
    # STRICT loader (fail-closed on a missing/empty/corrupted file) and
    # the owner-pinned lowering refusal (the wave-76 start floor can
    # rise, never fall). The fact counts the LIVE attestation rows in
    # the compiled package STRUCTURALLY (the №565 lesson) — a summary
    # header, a corrupted line or a revoked row cannot inflate it.
    if check:
        try:
            attest_floors = load_floors_strict(GATE_037_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        attest_floor = attest_floors.get("goal_attest_map")
        if attest_floor is None:
            print(
                "ERROR: gate_037_goals.txt carries no goal_attest_map floor",
                file=sys.stderr,
            )
            return 2
        if attest_floor < ATTEST_MAP_FLOOR_OWNER:
            print(
                "FLOOR LOWERING REFUSED: gate_037 "
                f"({attest_floor}) is below the owner constant "
                f"({ATTEST_MAP_FLOOR_OWNER} — the wave-76 start floor); "
                "the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        attest_live = attest_map_live()
        if attest_live < attest_floor:
            print(
                f"FLOOR BREACH: ATTEST_MAP {attest_live} live rows < the gate_037 floor "
                f"{attest_floor} (only-up) — the compliance package lost live rows "
                "(re-attest with the №868 compiler (METALOGOS_GEN_COMPLIANCE=1 "
                "cargo test --test naryad_868_package) — never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        print(
            f"OK: the №869 floor holds — ATTEST_MAP {attest_live} live rows "
            f"(gate_037 floor {attest_floor}, only-up) — source: "
            f"{PACKAGE_0206.relative_to(REPO)} (the raw rows, structurally counted)"
        )

    # №875: the PROVENANCE_SHARE floor twin (the only-up gate_038) —
    # the STRICT loader (fail-closed on a missing/empty/corrupted file)
    # and the owner-pinned lowering refusal (the wave-77 dispatch
    # constant can rise, never fall). The fact counts the registered
    # carriers whose boundary record is LIVE-wired (the №862 roster
    # form — the registry is the machine source, the composition site
    # fixed, the wiring per carrier).
    if check:
        try:
            prov_floors = load_floors_strict(GATE_038_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        prov_floor = prov_floors.get("goal_provenance_share_bp")
        if prov_floor is None:
            print(
                "ERROR: gate_038_goals.txt carries no goal_provenance_share_bp floor",
                file=sys.stderr,
            )
            return 2
        if prov_floor < PROVENANCE_SHARE_FLOOR_OWNER_BP:
            print(
                "FLOOR LOWERING REFUSED: gate_038 "
                f"({prov_floor} bp) is below the owner constant "
                f"({PROVENANCE_SHARE_FLOOR_OWNER_BP} bp — the wave-77 dispatch constant, "
                "gh#1729); the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        prov_share = provenance_share_bp()
        if prov_share < prov_floor:
            print(
                f"FLOOR BREACH: PROVENANCE_SHARE {prov_share} bp < the gate_038 floor "
                f"{prov_floor} bp (only-up) — a registered carrier lost its live boundary "
                "record (restore the crate::provenance::ingest/gate_emit call with the "
                "carrier's own name in its implementation — never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        print(
            f"OK: the №875 floor holds — PROVENANCE_SHARE {prov_share} bp "
            f"(gate_038 floor {prov_floor} bp, only-up) — registry: "
            f"{', '.join(_provenance_registry()) or 'none'}; gate composition site: "
            f"{'present' if _provenance_gate_composition_site() else 'MISSING'}"
        )

    # №881: the OFFLINE_SUITE floor twin (the only-up gate_039) — the
    # STRICT loader (fail-closed on a missing/empty/corrupted file) and
    # the owner-pinned lowering refusal (the wave-78 dispatch constant
    # can rise, never fall). The fact counts the registered LLM-surface
    # checks that execute without the external network (ADR-0208 §2.5)
    # — a row counts ONLY on offline=yes AND the check file's existence
    # (the count is checked, not declared; the №862 roster form).
    if check:
        try:
            off_floors = load_floors_strict(GATE_039_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        off_floor = off_floors.get("goal_offline_suite_bp")
        if off_floor is None:
            print(
                "ERROR: gate_039_goals.txt carries no goal_offline_suite_bp floor",
                file=sys.stderr,
            )
            return 2
        if off_floor < OFFLINE_SUITE_FLOOR_OWNER_BP:
            print(
                "FLOOR LOWERING REFUSED: gate_039 "
                f"({off_floor} bp) is below the owner constant "
                f"({OFFLINE_SUITE_FLOOR_OWNER_BP} bp — the wave-78 dispatch constant, "
                "gh#1745); the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        off_suite = offline_suite_bp()
        if off_suite < off_floor:
            print(
                f"FLOOR BREACH: OFFLINE_SUITE {off_suite} bp < the gate_039 floor "
                f"{off_floor} bp (only-up) — a registered LLM-surface check went offline=no "
                "or its file vanished (fix the roster honestly — register the check with "
                "its TRUE offline posture, or restore the file; never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        off_rows = _offline_suite_registry()
        print(
            f"OK: the №881 floor holds — OFFLINE_SUITE {off_suite} bp "
            f"(gate_039 floor {off_floor} bp, only-up) — registry: "
            f"{len(off_rows)} registered LLM-surface checks "
            f"({sum(1 for _, ok in off_rows if ok)} offline=yes, "
            f"{sum(1 for _, ok in off_rows if not ok)} offline=no)"
        )

    # №888: the JURISDICTION_MATRIX floor twin (the only-up gate_040) —
    # the STRICT loader (fail-closed on a missing/empty/corrupted file)
    # and the owner-pinned lowering refusal (the wave-79 dispatch
    # constant can rise, never fall). The fact counts the registered
    # jurisdictions that carry ≥ 1 live rule in the seed matrix
    # (ADR-0209 §4) — a row counts ONLY on the code's presence in the
    # LIVE src/jurisdiction.rs (the count is checked, not declared; the
    # №862 roster form).
    if check:
        try:
            jur_floors = load_floors_strict(GATE_040_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        jur_floor = jur_floors.get("goal_jurisdiction_matrix")
        if jur_floor is None:
            print(
                "ERROR: gate_040_goals.txt carries no goal_jurisdiction_matrix floor",
                file=sys.stderr,
            )
            return 2
        if jur_floor < JURISDICTION_MATRIX_FLOOR_OWNER:
            print(
                "FLOOR LOWERING REFUSED: gate_040 "
                f"({jur_floor}) is below the owner constant "
                f"({JURISDICTION_MATRIX_FLOOR_OWNER} — the wave-79 dispatch constant, "
                "gh#1764); the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        jur_count = jurisdiction_matrix_count()
        if jur_count < jur_floor:
            print(
                f"FLOOR BREACH: JURISDICTION_MATRIX {jur_count} < the gate_040 floor "
                f"{jur_floor} (only-up) — a registered jurisdiction lost its live seed "
                "rule (restore the seed row honestly — never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        jur_rows = _jurisdiction_registry()
        print(
            f"OK: the №888 floor holds — JURISDICTION_MATRIX {jur_count} "
            f"(gate_040 floor {jur_floor}, only-up) — registry: "
            f"{len(jur_rows)} registered jurisdictions "
            f"({jur_count} live in src/jurisdiction.rs)"
        )

    # №894: the gate_041 only-up floor over the REPLAY_MATCH share (the
    # gate_040 form): the owner-pinned wave-80 dispatch constant can
    # rise, never fall; the live share must hold the floor — a
    # de-wired carrier (the e2e literal renamed/removed, the gate
    # composition deleted) breaches it loudly.
    if check:
        try:
            rep_floors = load_floors_strict(GATE_041_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        rep_floor = rep_floors.get("goal_replay_match_bp")
        if rep_floor is None:
            print(
                "ERROR: gate_041_goals.txt carries no goal_replay_match_bp floor",
                file=sys.stderr,
            )
            return 2
        if rep_floor < REPLAY_MATCH_FLOOR_OWNER_BP:
            print(
                "FLOOR LOWERING REFUSED: gate_041 "
                f"({rep_floor} bp) is below the owner constant "
                f"({REPLAY_MATCH_FLOOR_OWNER_BP} bp — the wave-80 dispatch constant, "
                "gh#1779); the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        rep_share = replay_match_bp()
        if rep_share < rep_floor:
            print(
                f"FLOOR BREACH: REPLAY_MATCH {rep_share} bp < the gate_041 floor "
                f"{rep_floor} bp (only-up) — a registered decision carrier lost its "
                "live trace wiring or the replay_gate composition site is gone "
                "(restore the wiring honestly — never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        rep_rows = _replay_registry()
        print(
            f"OK: the №894 floor holds — REPLAY_MATCH {rep_share} bp "
            f"(gate_041 floor {rep_floor}, only-up) — registry: "
            f"{len(rep_rows)} registered decision carriers "
            f"({rep_share // 100} live-wired %)"
        )

    # №906: the gate_042 only-up floor over the TAINT_SATURATION space
    # (the gate_040/041 form): the owner-pinned wave-81 dispatch constant
    # (10^9) can rise, never fall; the live product must hold the floor —
    # a dead anchor (the roster region, a lane module, the sanitizer
    # entry, a sink name) breaches it loudly.
    if check:
        try:
            taint_floors = load_floors_strict(GATE_042_GOALS)
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        taint_floor = taint_floors.get("goal_taint_saturation_min")
        if taint_floor is None:
            print(
                "ERROR: gate_042_goals.txt carries no goal_taint_saturation_min floor",
                file=sys.stderr,
            )
            return 2
        if taint_floor < TAINT_SATURATION_FLOOR_OWNER:
            print(
                "FLOOR LOWERING REFUSED: gate_042 "
                f"({taint_floor}) is below the owner constant "
                f"({TAINT_SATURATION_FLOOR_OWNER} — the wave-81 dispatch constant, "
                "gh#1803); the floor is only-up — the change is an explicit owner transaction",
                file=sys.stderr,
            )
            return 1
        taint_space = taint_saturation()
        if taint_space < taint_floor:
            print(
                f"FLOOR BREACH: TAINT_SATURATION {taint_space} < the gate_042 floor "
                f"{taint_floor} (only-up) — a live anchor of the saturation product "
                "died (the roster region, a lane module, the sanitizer entry, a "
                "sink name) or the generator's reach collapsed "
                "(restore the anchor honestly — never a silent gate edit)",
                file=sys.stderr,
            )
            return 1
        t_roster = _taint_roster_rows()
        print(
            f"OK: the №906 floor holds — TAINT_SATURATION {taint_space} "
            f"(gate_042 floor {taint_floor}, only-up) — roster: "
            f"{len(t_roster)} scenario rows, "
            f"{len({row[2] for row in t_roster})} live sources"
        )

    # №841: the RED harness (the tamper traps) — an explicit mode, run by
    # CI beside the --check (the same posture as the other gates'
    # --tamper-test modes).
    if "--tamper-test" in sys.argv:
        return tamper_test()

    return 0


if __name__ == "__main__":
    sys.exit(main())
