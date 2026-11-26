#!/usr/bin/env python3
"""The self-test of scripts/recurring_bench_attest_verify.py (№1010,
gh#2085): the pure logic — the stats form, the verdict, the spawn-effect
check, the artifact shape, the pins — no binary needed (the same posture
as scripts/test_edge_inventory_measure.py). Run: python3 scripts/test_recurring_bench_attest_verify.py
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import recurring_bench_attest_verify as rb


def main() -> int:
    # 1. the stats form (the w9 table shape)
    times = [7.2, 6.4, 6.7, 6.8, 6.9, 7.6]
    assert rb.p95(times) == 7.6, rb.p95(times)
    assert rb.p95([1.0, 2.0]) == 2.0
    assert rb.p95([]) == 0.0
    import statistics
    assert abs(statistics.median(times) - 6.85) < 1e-9

    # 2. the spawn-effect check: exit 0 + ok:true; anything else is a mismatch
    assert rb.verify_spawn_ok(0, 'noise\n{"ok": true, "model": "AttestHead"}')
    assert not rb.verify_spawn_ok(1, '{"ok": true}')
    assert not rb.verify_spawn_ok(0, '{"ok": false}')
    assert not rb.verify_spawn_ok(0, "plain text, no json")
    assert not rb.verify_spawn_ok(0, '{"ok": "true"}')

    # 3. the verdict form
    assert rb.verdict_of(6.7, 20.0) == "PASS"
    assert rb.verdict_of(20.0, 20.0) == "PASS"
    assert rb.verdict_of(20.1, 20.0) == "FAIL"

    # 4. the pins hold on the current checkout
    assert rb.pins_ok(), "the pinned files drifted — fix the pins loudly, never silently"

    # 5. the artifact shape (a fake recorded run — the form, not numbers)
    art_probe = {
        "artifact": "recurring-bench-attest-verify",
        "form": "E (the numbers only from this recorded run)",
        "delegation_verbatim": rb.OWNER_VERBATIM_W3,
        "baseline": {"median_ms": rb.BASELINE_MS},
        "budget": {"median_ms": rb.BUDGET_MS},
        "verdict": "PASS",
    }
    assert art_probe["delegation_verbatim"] == (
        "я доверяю твоим рекомендациям, при том что все "
        "решения должны идти только на улучшения")
    assert rb.BASELINE_MS == 6.7 and rb.BUDGET_MS == 20.0

    # 6. the txt writer produces the human form with the verbatim quote
    art_probe_full = {
        "naryad": "№1010 (test)",
        "scenario": "test",
        "pins": {"program": {"path": "p", "sha256": "a" * 64},
                 "held_out": {"path": "h", "sha256": "b" * 64}},
        "binary_sha256": "c" * 64,
        "measurement": {"median_ms": 6.7, "min_ms": 6.4, "p95_ms": 7.2,
                        "max_ms": 7.6, "runs": 3, "times_ms": [6.4, 6.7, 7.6]},
        "baseline": {"median_ms": rb.BASELINE_MS, "source": "w9"},
        "budget": {"median_ms": rb.BUDGET_MS, "provenance": "agent"},
        "delegation_verbatim": rb.OWNER_VERBATIM_W3,
        "platform": {"honest_label": "test"},
        "verdict": "PASS",
    }
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        txt = str(Path(td) / "x.txt")
        rb.write_txt(txt, art_probe_full, "summary line")
        body = Path(txt).read_text(encoding="utf-8")
    assert "я доверяю твоим рекомендациям" in body
    assert "VERDICT: PASS" in body
    assert "6.7" in body

    print("self-test OK: the stats, the verdict, the effect check, the pins, the E-form")
    return 0


if __name__ == "__main__":
    sys.exit(main())
