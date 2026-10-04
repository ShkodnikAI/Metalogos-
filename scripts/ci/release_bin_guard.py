#!/usr/bin/env python3
"""The release-bin guard (naryad №572, gh#973).

№567 (gh#931) moved the `mlog` bin into the metalogos-server crate;
the GitHub release workflows were NOT updated with it, and the next
release build of main failed ("no bin target named `mlog` in
default-run packages" — Actions run 37158618960, job 111307142094,
exit 101). Test-CI never executes the release build, so the regression
sailed through a 45/45-green push. This guard makes the CLASS fail on
every push/PR, before a release day:

  1. `cargo metadata --no-deps` — the workspace must carry EXACTLY ONE
     bin target named `mlog`, and its owning package is read from the
     manifest graph (no hardcoded name: a legitimate future move of
     the bin does not false-positive here).
  2. The release workflows FOLLOW the bin: every line that builds
     `--bin mlog` in build.yml / release.yml must pin the owning
     package with `-p <owner>` — the "bin moved, workflows forgot"
     split-brain is caught in the same commit that moves it.

Fail-closed: unreadable metadata or a missing workflow file exits 2 —
an unanswerable question is never a silent pass.

Exit codes: 0 = green; 1 = guard tripped (the class); 2 = fail-closed.

Usage:
    python3 scripts/ci/release_bin_guard.py
    python3 scripts/ci/release_bin_guard.py --self-test
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
BUILD_YML = REPO_ROOT / ".github" / "workflows" / "build.yml"
RELEASE_YML = REPO_ROOT / ".github" / "workflows" / "release.yml"
BIN_NAME = "mlog"
# The workflows whose build lines must follow the bin (naryad №572:
# both are release-pipeline workflows; ci.yml's debug-build is NOT in
# the contract — it never packages artifacts).
WORKFLOWS = (BUILD_YML, RELEASE_YML)


class GuardTripped(Exception):
    """The guard found the class (exit 1)."""


def read_bin_owner() -> str:
    """The single owning package of the `mlog` bin, from cargo metadata."""
    try:
        out = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            capture_output=True, text=True, check=True,
        ).stdout
        metadata = json.loads(out)
    except FileNotFoundError:
        raise FailClosed("cargo not found — the toolchain must be installed")
    except (subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        raise FailClosed(f"cargo metadata unreadable: {exc}")
    owners = sorted({
        package["name"]
        for package in metadata.get("packages", [])
        for target in package.get("targets", [])
        if target.get("name") == BIN_NAME and "bin" in target.get("kind", [])
    })
    if not owners:
        raise GuardTripped(
            f"no bin target named `{BIN_NAME}` in the workspace — "
            f"the release pipeline has nothing to build")
    if len(owners) > 1:
        raise GuardTripped(
            f"bin `{BIN_NAME}` is defined by {len(owners)} packages "
            f"({', '.join(owners)}) — a split-brain workspace; the release "
            f"pipeline cannot know which one it ships")
    return owners[0]


def check_workflow(path: Path, owner: str) -> None:
    """Every `--bin mlog` build line pins `-p <owner>`; at least one exists."""
    if not path.is_file():
        raise FailClosed(f"workflow file missing: {path}")
    build_lines = [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if "--bin mlog" in line
    ]
    if not build_lines:
        raise GuardTripped(
            f"{path.name}: no line builds `--bin mlog` — the release "
            f"pipeline lost the bin entirely")
    for line in build_lines:
        if f"-p {owner}" not in line:
            raise GuardTripped(
                f"{path.name}: the build line does not pin the owning "
                f"package (needs `-p {owner}`): {line}")


class FailClosed(Exception):
    """Unreadable input (exit 2)."""


def run_guard() -> str:
    owner = read_bin_owner()
    for workflow in WORKFLOWS:
        check_workflow(workflow, owner)
    return owner


def self_test() -> int:
    """Both paths on synthetic inputs (the №535 tamper-test shape)."""
    good_meta = json.dumps({"packages": [
        {"name": "metalogos", "targets": [{"name": "other", "kind": ["lib"]}]},
        {"name": "metalogos-server", "targets": [
            {"name": BIN_NAME, "kind": ["bin"]}]},
    ]})
    # (1) green: the owner is pinned everywhere
    # (2) red: the workflow forgot the -p pin
    # (3) red: two packages own the bin (split-brain)
    # (4) red: the bin vanished from the workspace
    # (5) fail-closed: cargo metadata is a lie (unparsable)
    checks = 0

    def scenario(meta: str, build_yml: str, release_yml: str,
                 expect: str) -> None:
        nonlocal checks
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            wf = root / ".github" / "workflows"
            wf.mkdir(parents=True)
            (wf / BUILD_YML.name).write_text(build_yml, encoding="utf-8")
            (wf / RELEASE_YML.name).write_text(release_yml, encoding="utf-8")
            modules = {
                "json": json, "Path": Path,
                "FailClosed": FailClosed, "GuardTripped": GuardTripped,
                "BUILD_YML": wf / BUILD_YML.name,
                "RELEASE_YML": wf / RELEASE_YML.name,
            }
            saved = {k: globals()[k] for k in modules}
            globals().update(modules)
            try:
                try:
                    owner = read_bin_owner_with(meta)
                    for wf_path in (BUILD_YML, RELEASE_YML):
                        check_workflow(wf_path, owner)
                    got = "green"
                except GuardTripped:
                    got = "tripped"
                except FailClosed:
                    got = "closed"
                assert got == expect, f"expected {expect}, got {got}"
                checks += 1
            finally:
                globals().update(saved)

    def read_bin_owner_with(meta_text: str) -> str:
        """read_bin_owner with the subprocess call replaced by fixture text."""
        try:
            metadata = json.loads(meta_text)
        except json.JSONDecodeError as exc:
            raise FailClosed(f"cargo metadata unreadable: {exc}")
        owners = sorted({
            package["name"]
            for package in metadata.get("packages", [])
            for target in package.get("targets", [])
            if target.get("name") == BIN_NAME and "bin" in target.get("kind", [])
        })
        if not owners:
            raise GuardTripped("no bin target")
        if len(owners) > 1:
            raise GuardTripped("split-brain")
        return owners[0]

    green_build = "run: cargo build --release -p metalogos-server --bin mlog\n"
    forgot_build = "run: cargo build --release --bin mlog\n"

    scenario(good_meta, green_build,
             "run: cargo build --release --locked -p metalogos-server --bin mlog\n",
             "green")
    scenario(good_meta, forgot_build, forgot_build, "tripped")
    split_meta = json.dumps({"packages": [
        {"name": "a", "targets": [{"name": BIN_NAME, "kind": ["bin"]}]},
        {"name": "b", "targets": [{"name": BIN_NAME, "kind": ["bin"]}]},
    ]})
    scenario(split_meta, green_build, green_build, "tripped")
    empty_meta = json.dumps({"packages": [
        {"name": "metalogos", "targets": [{"name": "x", "kind": ["lib"]}]},
    ]})
    scenario(empty_meta, green_build, green_build, "tripped")
    scenario("{not json", green_build, green_build, "closed")

    print(f"self-test: {checks}/5 scenarios green")
    return 0 if checks == 5 else 1


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    try:
        owner = run_guard()
    except GuardTripped as exc:
        print(f"::error::release-bin guard (№572): {exc}")
        return 1
    except FailClosed as exc:
        print(f"::error::release-bin guard (№572) fail-closed: {exc}")
        return 2
    print(f"release-bin guard (№572): bin `{BIN_NAME}` owned by `{owner}`, "
          f"the release workflows follow it")
    return 0


if __name__ == "__main__":
    sys.exit(main())
