#!/usr/bin/env python3
"""№997 (gh#2053): the watchlist_check self-test — the scenario census,
the calibration table (the same Yellow/Orange/Red for every scenario,
№997 does not touch it) and the pure trigger functions, now over SEVEN
scenarios (WL-07 obra/superpowers added). Stdlib unittest — no new
dependencies; run: python3 -I scripts/test_watchlist_check.py"""
import importlib.util
import json
import unittest
from pathlib import Path

R = Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location("watchlist_check", R / "scripts/watchlist_check.py")
wc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(wc)


class ScenarioCensus(unittest.TestCase):
    def test_seven_scenarios_in_registry_order(self):
        ids = [sid for sid, _, _ in wc.SCENARIOS]
        self.assertEqual(ids, [f"WL-0{i}" for i in range(1, 8)])

    def test_wl07_is_obra_superpowers_and_not_a_crate(self):
        row = dict((sid, (full, crate)) for sid, full, crate in wc.SCENARIOS)
        self.assertIn("WL-07", row)
        full, crate = row["WL-07"]
        self.assertEqual(full, "obra/superpowers")
        self.assertFalse(crate, "crates.io N/A — not a crate")

    def test_registry_md_carries_wl07_in_full_schema(self):
        text = (R / "docs/watchlist/registry.md").read_text()
        self.assertIn("the seven-scenario registry", text)
        self.assertIn("### WL-07 — obra/superpowers", text)
        for section in ("**Signals:**", "**The adoption interest:**",
                        "**Horizons:**", "**Baseline 23.11:**"):
            self.assertIn(section, text.split("### WL-07")[1].split("### WL")[0])

    def test_baseline_json_carries_wl07_capture(self):
        b = json.loads((R / "docs/watchlist/baseline.json").read_text())
        row = b["repos"]["obra/superpowers"]
        self.assertEqual(row["stars"], 324691)
        self.assertEqual(row["latest_release"], "v7.0.2")
        self.assertIsNone(row["crates_downloads"])


class CalibrationTable(unittest.TestCase):
    def test_constants_untouched_by_997(self):
        self.assertEqual(wc.STARS_WEEK_YELLOW, 0.05)
        self.assertEqual(wc.STARS_WEEK_ORANGE, 0.15)
        self.assertEqual(wc.STARS_WEEK_RED, 0.25)
        self.assertEqual(wc.DOWNLOADS_YELLOW, 500)
        self.assertEqual(wc.PUSH_PAUSE_YELLOW_DAYS, 14)
        self.assertEqual(wc.PUSH_PAUSE_RED_DAYS, 45)

    def test_yellow_one_trigger(self):
        self.assertEqual(wc.level_of(["stars 1->2 (+100.0% >= +5%/week)"], None, False), "Yellow")

    def test_orange_two_triggers_or_big_stars(self):
        self.assertEqual(wc.level_of(["a", "b"], None, False), "Orange")
        self.assertEqual(wc.level_of([], 0.16, False), "Orange")

    def test_red_three_triggers_or_25_plus_release(self):
        self.assertEqual(wc.level_of(["a", "b", "c"], None, False), "Red")
        self.assertEqual(wc.level_of(["a"], 0.30, True), "Red")

    def test_quiet_week(self):
        self.assertEqual(wc.level_of([], None, False), "quiet")


class PureTriggers(unittest.TestCase):
    BASE = {"stars": 324691, "crates_downloads": None,
            "latest_release": "v7.0.2", "pushed_at": "2026-11-03T23:24:58Z"}

    def test_stars_growth_math(self):
        live = dict(self.BASE, stars=int(324691 * 1.10))
        g = wc.stars_growth(live, self.BASE)
        self.assertAlmostEqual(g, 0.10, places=6)
        # base=0 is the None path (no division by zero); live=0 is the
        # honest -100% regression, not a None.
        self.assertIsNone(wc.stars_growth(self.BASE, {"stars": 0}))
        self.assertAlmostEqual(wc.stars_growth({"stars": 0}, self.BASE), -1.0)

    def test_stars_trigger_fires_at_5_percent(self):
        live = dict(self.BASE, stars=int(324691 * 1.05) + 10)
        events = wc.week_events(live, self.BASE)
        self.assertTrue(any("stars" in e for e in events))

    def test_new_release_trigger(self):
        live = dict(self.BASE, latest_release="v7.1.0")
        events = wc.week_events(live, self.BASE)
        self.assertTrue(any("NEW release" in e for e in events))

    def test_push_pause_trigger_names_days(self):
        # The base's pushed_at is 2026-11-03 — every check date later than
        # 17.11 fires the pause trigger; the event carries the days.
        events = wc.week_events(self.BASE, self.BASE)
        pauses = [e for e in events if e.startswith("push pause")]
        self.assertTrue(pauses and int(pauses[0].split()[2]) >= 14)

    def test_downloads_trigger_only_for_crate_scenarios(self):
        base = dict(self.BASE, crates_downloads=1000)
        live = dict(self.BASE, crates_downloads=1501)
        events = wc.week_events(live, base)
        self.assertTrue(any("crates.io downloads" in e for e in events))
        none_live = dict(self.BASE, crates_downloads=None)
        self.assertFalse(any("crates.io" in e for e in wc.week_events(none_live, base)))


if __name__ == "__main__":
    unittest.main(verbosity=2)
