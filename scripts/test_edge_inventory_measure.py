#!/usr/bin/env python3
"""№1004 (gh#2067, волна В106 — W2): the edge_inventory_measure self-test —
the honest platform label (a CI-runner run only when the Actions env says
so), the median math (the №998 bench convention) and the E-form artifact
shape (the required keys, the sha256-verified flag, the W3 budgets line — №1008: the gates live, the owner's verbatim carried).
Stdlib unittest — no new dependencies;
run: python3 -I scripts/test_edge_inventory_measure.py"""
import importlib.util
import os
import unittest
from pathlib import Path
from unittest import mock

R = Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location("edge_inventory_measure", R / "scripts/edge_inventory_measure.py")
em = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(em)

SOURCE = {"kind": "release", "tag": "v0.40.1",
          "created_at": "2026-11-21T20:25:47Z",
       "target_commitish": "245fe99610"}


class HonestLabel(unittest.TestCase):
    def test_no_actions_env_means_not_a_runner(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            lab = em.honest_label()
        self.assertEqual(lab["honest_label"], "not a CI runner (this machine)")
        self.assertIsNone(lab["run_url"])

    def test_actions_env_means_ci_runner_run(self):
        env = {"GITHUB_RUN_ID": "45393451354", "GITHUB_REPOSITORY": "ShkodnikAI/Metalogos-",
               "GITHUB_SERVER_URL": "https://github.com", "RUNNER_NAME": "Ubuntu-24",
               "ImageOS": "ubuntu24"}
        with mock.patch.dict(os.environ, env, clear=True):
            lab = em.honest_label()
        self.assertEqual(lab["honest_label"], "CI-runner run")
        self.assertIn("/actions/runs/45393451354", lab["run_url"])
        self.assertEqual(lab["image_os"], "ubuntu24")


class Median(unittest.TestCase):
    def test_median_odd_and_unsorted(self):
        self.assertEqual(em.median([0.005, 0.003, 0.003, 0.003, 0.003]), 0.003)
        self.assertEqual(em.median([0.02, 0.001]), 0.02)


class BuildArtifact(unittest.TestCase):
    def _art(self, label_env=None, rss_kib=25 * 1024, source=None):
        if source is None:
            source = SOURCE
        if label_env is None:
            ctx = mock.patch.dict(os.environ, {}, clear=True)
        else:
            ctx = mock.patch.dict(os.environ, label_env, clear=True)
        with ctx:
            return em.build_artifact(source, "mlog-linux-x86_64", "96d2db1b",
                                     "verified-against-the-release-SHA256SUMS" if source["kind"] == "release"
                                     else "computed-from-the-built-file (a local build; no SHA256SUMS check)",
                                     37318352,
                                     "rustc 1.100.0\n", [0.003, 0.003, 0.003, 0.003, 0.005],
                                     rss_kib, 5)

    def test_required_keys_and_e_form(self):
        art = self._art()
        for key in ("artifact", "form", "source", "asset", "sha256",
                    "sha256_source", "build_info_verbatim", "measurement",
                    "platform", "budgets"):
            self.assertIn(key, art)
        self.assertEqual(art["artifact"], "edge-profile")
        self.assertIn("only from this recorded run", art["form"])
        self.assertEqual(art["source"]["kind"], "release")
        self.assertEqual(art["sha256_source"], "verified-against-the-release-SHA256SUMS")
        self.assertEqual(art["asset"]["size_bytes"], 37318352)

    def test_measurement_shape(self):
        m = self._art()["measurement"]
        self.assertEqual(m["startup_help_median_s"], 0.003)
        self.assertEqual(m["runs"], 5)
        self.assertEqual(len(m["startup_help_runs_s"]), 5)
        self.assertEqual(m["peak_rss_children_max_mib"], 25.0)

    def test_rss_none_on_os_without_getrusage(self):
        m = self._art(rss_kib=None)["measurement"]
        self.assertIsNone(m["peak_rss_children_max_mib"])

    def test_local_build_source_is_named_not_silent(self):
        art = self._art(source={"kind": "local-build"})
        self.assertEqual(art["source"]["kind"], "local-build")
        self.assertIn("no SHA256SUMS check", art["sha256_source"])
        self.assertEqual(art["asset"]["name"], "mlog-linux-x86_64")

    def test_w3_budgets_line_is_carried(self):
        art = self._art()
        self.assertIn("W3 RESOLVED", art["budgets"])
        self.assertIn("я доверяю твоим рекомендациям", art["budgets"])
        self.assertIn("PASS", art["budgets"])
        self.assertIn("N/A", art["budgets"])  # the fixture passes workload=None



    def test_platform_label_propagates(self):
        self.assertEqual(self._art()["platform"]["honest_label"],
                         "not a CI runner (this machine)")
        env = {"GITHUB_RUN_ID": "1"}
        self.assertEqual(self._art(env)["platform"]["honest_label"], "CI-runner run")


class BudgetVerdicts(unittest.TestCase):
    """№1008 (gh#2080): the W3 gate verdicts — PASS/FAIL/N-A, numbers only."""

    def test_size_fail_and_others_pass(self):
        v = dict(em.budget_verdicts(46_000_000, 0.003, 25.0, None))
        self.assertTrue(all("FAIL" in g for g in v if "size_bytes" in g))
        self.assertTrue(all("PASS" in g for g in v if "cli_floor_ms" in g))

    def test_na_where_no_number(self):
        v = em.budget_verdicts(37_318_352, 0.003, None,
                               {"status": "pin-mismatch"})
        oks = [ok for _, ok in v]
        self.assertIn(None, oks)
        self.assertTrue(all(ok is not False for ok in oks))

    def test_workload_over_budget_fails(self):
        v = em.budget_verdicts(37_318_352, 0.003, 25.0,
                               {"status": "verified", "median_s": 0.02})
        self.assertTrue(any(ok is False for _, ok in v))



class WorkloadPins(unittest.TestCase):
    """№1005 (gh#2071, В106 — W2+): the §3.4 workload pin — the drift
    tripwire: the pinned files must match byte-for-byte on every lane."""

    def test_pins_match_repo_files(self):
        self.assertTrue(em.workload_pins_ok(),
                        "the §3.4 workload pins drifted from the repo files")

    def test_workload_matches_trailing_newline_only(self):
        self.assertTrue(em.workload_matches(open(em.WORKLOAD_EXPECTED[0]).read() + "\n"))
        self.assertFalse(em.workload_matches("phase 0: extract,load\n"))

    def test_artifact_carries_workload_block(self):
        wl = {"program": em.WORKLOAD_PROGRAM[0], "status": "verified",
              "median_s": 0.007, "runs_s": [0.007] * 5, "runs": 5}
        with mock.patch.dict(os.environ, {}, clear=True):
            art = em.build_artifact(SOURCE, "mlog-linux-x86_64", "96d2db1b",
                                    "verified-against-the-release-SHA256SUMS",
                                    37318352, "rustc 1.100.0\n",
                                    [0.003] * 5, 25 * 1024, 5, wl)
        self.assertEqual(art["measurement"]["workload"]["status"], "verified")
        self.assertEqual(art["measurement"]["workload"]["median_s"], 0.007)
        self.assertIn("gh#2071", art["naryad"])


if __name__ == "__main__":
    unittest.main()
