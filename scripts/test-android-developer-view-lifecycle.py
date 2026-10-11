#!/usr/bin/env python3
"""Ensure DeveloperPage drops transient work and full tokens with its view."""
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/home/DeveloperPage.java"


class AndroidDeveloperViewLifecycleContract(unittest.TestCase):
    def test_destroy_view_clears_busy_state_and_fresh_token(self):
        source = SOURCE.read_text(encoding="utf-8")
        start = source.index("@Override public void onDestroyView()")
        end = source.index("\n    private void reload()", start)
        body = source[start:end]
        self.assertIn("busy = false;", body)
        self.assertIn("freshToken = null;", body)


if __name__ == "__main__":
    unittest.main()
