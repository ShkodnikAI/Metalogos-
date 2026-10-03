#!/usr/bin/env python3
"""№559 (issue #920; the audit 02.10 M-2 systematic + §6.3): the blocking
registry-vs-corpus check.

Every call shape recorded in the corpus snapshot
(scripts/ci/office_call_shapes.txt) whose name is a REGISTRY BUILTIN must
be accepted by the registry's arity spec (the `check_builtin_arity`
contract, exported machine-read to scripts/ci/registry_arity.txt). A
change that turns a warning into an error (a spec hardening like the
№523 1..2 → 2 move, or an arity narrowing) fails here — against the
KNOWN corpus — before a user hits it (the respond_html lesson: the
corpus was not run, the user caught the regression).

Boundaries:
  - pairs whose name is NOT in the registry export are skipped — patterns,
    learnables and tool methods are the program's own surface, not the
    registry's contract;
  - the DYNAMIC-ARITY builtins (forget, render — the semantic.rs
    dynamic_arity list) are skipped: the registry spec cannot state their
    real contract, the builtin remains the loud runtime validator;
  - the snapshot carries NO code and NO query strings — only the
    name|argc|count triples (the №559 privacy boundary).

Exit 0 = every registry-bound corpus pair is accepted; exit 1 = at least
one divergence (each FAIL names the builtin and the arity).

--self-test runs the embedded fixtures (green, a registry-bound
divergence, a skip case, a dynamic-arity skip) and must print SELF-TEST OK.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(HERE, "office_call_shapes.txt")
REGISTRY = os.path.join(HERE, "registry_arity.txt")

# №559: the dynamic-arity names — the semantic.rs `dynamic_arity` list
# (`matches!(name.as_str(), "forget" | "render")`), mirrored here so the
# corpus check and the static check share ONE posture (the spec cannot
# state a non-contiguous or data-driven contract; the builtin validates
# loudly at runtime). Pinned by tests/naryad_559_registry_vs_corpus.rs.
DYNAMIC_ARITY = {"forget", "render"}


def load_arity(path: str = REGISTRY) -> dict[str, tuple[int, int | None]]:
    """name → (min, max) — max None = unbounded variadic (arity 0, no max)."""
    specs: dict[str, tuple[int, int | None]] = {}
    for lineno, line in enumerate(open(path, encoding="utf-8"), start=1):
        line = line.rstrip("\n")
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        parts = line.split("|")
        if len(parts) != 3:
            raise ValueError(f"{path}:{lineno}: expected 3 |-separated columns")
        name, amin, amax = parts[0].strip(), parts[1].strip(), parts[2].strip()
        specs[name] = (int(amin), None if amax == "*" else int(amax))
    return specs


def load_corpus(path: str = CORPUS) -> tuple[list[tuple[str, int, int]], list[tuple[str, int, str]]]:
    """((name, argc, count) triples, (name, argc, reason) skips).

    The snapshot carries two line kinds:
      name|argc|count          — an extracted corpus call shape;
      SKIP|name|argc|reason    — a hand-annotated expected refusal (a
                                 try-wrapped negative fixture: the call's
                                 refusal IS the tested outcome). SKIP lines
                                 are hand-added AFTER an extraction — a
                                 regeneration drops them, the validator's
                                 FAIL report is the mechanical list to
                                 re-annotate.
    """
    pairs = []
    skips = []
    for lineno, line in enumerate(open(path, encoding="utf-8"), start=1):
        line = line.rstrip("\n")
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line.startswith("SKIP|"):
            parts = line.split("|", 3)
            if len(parts) != 4:
                raise ValueError(
                    f"{path}:{lineno}: a SKIP line needs SKIP|name|argc|reason"
                )
            skips.append((parts[1].strip(), int(parts[2]), parts[3].strip()))
            continue
        parts = line.split("|")
        if len(parts) != 3:
            raise ValueError(f"{path}:{lineno}: expected 3 |-separated columns")
        pairs.append((parts[0].strip(), int(parts[1]), int(parts[2])))
    return pairs, skips


def check_corpus(corpus, specs) -> tuple[list[str], int, int]:
    """Returns (failures, checked_registry_bound, skipped)."""
    failures = []
    checked = 0
    skipped = 0
    for name, argc, _count in corpus:
        if name in DYNAMIC_ARITY:
            skipped += 1
            continue
        spec = specs.get(name)
        if spec is None:
            skipped += 1  # not a registry builtin — the program's own surface
            continue
        amin, amax = spec
        ok = argc >= amin and (amax is None or argc <= amax)
        if not ok:
            max_repr = "*" if amax is None else str(amax)
            failures.append(
                f"FAIL: '{name}' called with {argc} argument(s) — the registry "
                f"spec accepts {amin}..{max_repr}; a corpus call would break"
            )
        else:
            checked += 1
    return failures, checked, skipped


def self_test() -> int:
    ok = True

    def run_case(name, corpus, specs, expect_fail_substr=None):
        nonlocal ok
        failures, checked, skipped = check_corpus(corpus, specs)
        if expect_fail_substr is None:
            if failures:
                print(f"[{name}] expected green, got: {failures}")
                ok = False
            else:
                print(f"[{name}] green (checked={checked}, skipped={skipped})")
        else:
            if any(expect_fail_substr in f for f in failures):
                print(f"[{name}] red as expected: {failures[0]}")
            else:
                print(f"[{name}] expected a FAIL containing "
                      f"'{expect_fail_substr}', got: {failures}")
                ok = False

    specs = {"upper": (1, 1), "replace": (3, 3), "format": (1, None), "canary_insert": (1, 2)}
    run_case("green", [("upper", 1, 5), ("replace", 3, 2), ("format", 4, 1)], specs)
    run_case("arity-divergence", [("replace", 2, 1)], specs, "FAIL: 'replace' called with 2")
    run_case("not-a-builtin-skipped", [("shout", 1, 9)], specs)
    run_case("dynamic-arity-skipped", [("forget", 1, 3), ("render", 5, 2)], specs)
    run_case("range-accepts", [("canary_insert", 2, 1)], specs)
    run_case("range-refuses", [("canary_insert", 3, 1)], specs, "FAIL: 'canary_insert' called with 3")

    # the SKIP annotation round-trip: an inline snapshot with a hand-added
    # expected-refusal line parses into (pairs, skips) correctly
    import tempfile
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as tf:
        tf.write("upper|1|5\nSKIP|replace|1|a try-wrapped negative fixture\n")
        tmp = tf.name
    try:
        pairs, skips = load_corpus(tmp)
        if len(pairs) == 1 and pairs[0][0] == "upper" and len(skips) == 1 \
                and skips[0][0] == "replace" and skips[0][1] == 1 \
                and "negative fixture" in skips[0][2]:
            print("[skip-annotation-roundtrip] green")
        else:
            print(f"[skip-annotation-roundtrip] unexpected parse: {pairs}, {skips}")
            ok = False
    finally:
        os.unlink(tmp)

    if ok:
        print("SELF-TEST OK: green, arity-divergence, not-a-builtin-skipped, "
              "dynamic-arity-skipped, range-accepts, range-refuses, "
              "skip-annotation-roundtrip")
        return 0
    print("SELF-TEST FAILED")
    return 1


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()
    corpus = os.environ.get("N559_CORPUS", CORPUS)
    registry = os.environ.get("N559_REGISTRY", REGISTRY)
    for path in (corpus, registry):
        if not Path(path).is_file():
            print(f"error: required input missing: {path}", file=sys.stderr)
            return 2
    try:
        pairs, expected_skips = load_corpus(corpus)
        specs = load_arity(registry)
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    # the hand-annotated expected refusals suppress their pairs (a
    # try-wrapped negative fixture's refusal IS the tested outcome)
    skip_shapes = {(name, argc) for name, argc, _ in expected_skips}
    pairs = [p for p in pairs if (p[0], p[1]) not in skip_shapes]
    failures, checked, skipped = check_corpus(pairs, specs)
    for f in failures:
        print(f)
    for name, argc, reason in expected_skips:
        print(f"SKIP (expected refusal): {name} with {argc} arg(s) — {reason}")
    verdict = "OK" if not failures else f"FAIL ({len(failures)} divergence(s))"
    print(
        f"registry-vs-corpus: {verdict} — {len(pairs)} corpus pair(s), "
        f"{checked} registry-bound accepted, {skipped} skipped "
        f"(non-builtin or dynamic-arity), {len(expected_skips)} annotated "
        f"expected refusal(s)"
    )
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
