#!/usr/bin/env python3
"""Exercise the actual publishing change detector without publishing anything."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = yaml.safe_load((ROOT / ".github/workflows/proto-packages.yaml").read_text())
SCRIPT = next(step["run"] for step in WORKFLOW["jobs"]["detect-changes"]["steps"]
              if step.get("id") == "check")


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="epics-nt-workflow-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Contract Test")
        self.git("config", "user.email", "test@example.invalid")
        self.commit("README.md", "baseline")
        self.before = self.git("rev-parse", "HEAD").strip()

    def git(self, *args):
        # Never use a user's commit hooks or signing identity in this temporary repo.
        result = subprocess.run(
            ["git", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", *args],
            cwd=self.root, capture_output=True, text=True, check=True,
        )
        return result.stdout

    def commit(self, path, content):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(content)
        self.git("add", "--", path)
        self.git("commit", "-qm", "test fixture")

    def detect(self, before):
        output = self.root / "job outputs.txt"  # Quoting must handle whitespace.
        output.write_text("")
        subprocess.run(
            ["bash", "-e", "-o", "pipefail", "-c", SCRIPT], cwd=self.root,
            env={**os.environ, "BEFORE_SHA": before, "GITHUB_OUTPUT": str(output)},
            capture_output=True, text=True, check=True,
        )
        return dict(line.split("=", 1) for line in output.read_text().splitlines())

    def test_complete_push_not_only_last_commit(self):
        self.commit("proto/controls/common/v1/epics/types.proto", "schema")
        self.commit("README.md", "docs-only final commit")
        self.assertEqual(self.detect(self.before)["epics_changed"], "true")

    def test_initial_push_and_unavailable_old_endpoint(self):
        self.commit("proto/controls/common/v1/epics/types.proto", "schema")
        for before in ("0" * 40, "f" * 40):
            with self.subTest(before=before):
                self.assertEqual(self.detect(before)["epics_changed"], "true")

    def test_docs_only_push_does_not_publish(self):
        self.commit("README.md", "only docs")
        self.assertEqual(self.detect(self.before),
                         {"daq_changed": "false", "dpm_changed": "false", "epics_changed": "false"})

    def test_build_and_validation_changes_trigger_epics(self):
        for path in ("build/java/controls-epics/pom.xml", "tools/epics_nt/validate.py",
                     ".github/workflows/proto-packages.yaml"):
            with self.subTest(path=path):
                before = self.git("rev-parse", "HEAD").strip()
                self.commit(path, "changed")
                self.assertEqual(self.detect(before)["epics_changed"], "true")

    def test_existing_daq_and_dpm_detection_remains(self):
        for path, expected in (("proto/controls/common/v1/device.proto", "daq_changed"),
                               ("proto/controls/deprecated/dpm.proto", "dpm_changed")):
            before = self.git("rev-parse", "HEAD").strip()
            self.commit(path, "existing schema update")
            output = self.detect(before)
            self.assertEqual(output[expected], "true")
            self.assertEqual(output["epics_changed"], "false")

    def test_epics_publish_has_contract_validation_and_java_tests(self):
        job = WORKFLOW["jobs"]["build-epics"]
        self.assertEqual(job["needs"], "detect-changes")
        self.assertEqual(job["if"], "needs.detect-changes.outputs.epics_changed == 'true'")
        runs = [step["run"] for step in job["steps"] if "run" in step]
        validation = next(index for index, run in enumerate(runs) if "test_contract.py" in run)
        deploy = next(index for index, run in enumerate(runs) if "clean deploy" in run)
        self.assertLess(validation, deploy)
        self.assertNotIn("skipTests", runs[deploy])
        self.assertIn("--settings build/maven/settings.xml", runs[deploy])


if __name__ == "__main__":
    unittest.main(verbosity=2)
