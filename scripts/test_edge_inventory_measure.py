#!/usr/bin/env python3
"""№1004 (gh#2067, волна В106 — W2): the edge_inventory_measure self-test —
the honest platform label (a CI-runner run only when the Actions env says
so), the median math (the №998 bench convention) and the E-form artifact
shape (the required keys, the sha256-verified flag, the no-budgets line).
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

REL = {"tag_name": "v0.40.1", "created_at": "2026-11-21T20:25:47Z",
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
    def _art(self, label_env=None):
        if label_env is None:
            ctx = mock.patch.dict(os.environ, {}, clear=True)
        else:
            ctx = mock.patch.dict(os.environ, label_env, clear=True)
        with ctx:
            return em.build_artifact(REL, "96d2db1b", 37318352,
                                     "rustc 1.100.0\n", [0.003, 0.003, 0.003, 0.003, 0.005],
                                     25 * 1024, 5)

    def test_required_keys_and_e_form(self):
        art = self._art()
        for key in ("artifact", "form", "release", "asset", "sha256_verified",
                    "build_info_verbatim", "measurement", "platform", "budgets"):
            self.assertIn(key, art)
        self.assertEqual(art["artifact"], "edge-profile")
        self.assertIn("only from this recorded run", art["form"])
        self.assertTrue(art["sha256_verified"])
        self.assertEqual(art["asset"]["size_bytes"], 37318352)

    def test_measurement_shape(self):
        m = self._art()["measurement"]
        self.assertEqual(m["startup_help_median_s"], 0.003)
        self.assertEqual(m["runs"], 5)
        self.assertEqual(len(m["startup_help_runs_s"]), 5)
        self.assertEqual(m["peak_rss_children_max_mib"], 25.0)

    def test_no_budgets_line_is_carried(self):
        art = self._art()
        self.assertIn("none", art["budgets"])
        self.assertIn("numbers measured only", art["budgets"])

    def test_platform_label_propagates(self):
        self.assertEqual(self._art()["platform"]["honest_label"],
                         "not a CI runner (this machine)")
        env = {"GITHUB_RUN_ID": "1"}
        self.assertEqual(self._art(env)["platform"]["honest_label"], "CI-runner run")


if __name__ == "__main__":
    unittest.main()
