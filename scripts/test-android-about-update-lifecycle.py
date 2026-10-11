#!/usr/bin/env python3
"""Ensure AboutPage does not preserve an abandoned update operation across view rebuilds."""
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/home/AboutPage.java"


class AndroidAboutUpdateLifecycleContract(unittest.TestCase):
    def test_destroy_view_resets_transient_update_state(self):
        source = SOURCE.read_text(encoding="utf-8")
        start = source.index("@Override public void onDestroyView()")
        end = source.index("\n    /**", start)
        body = source[start:end]
        self.assertIn("state = State.IDLE;", body)
        self.assertIn("downloaded = null;", body)


if __name__ == "__main__":
    unittest.main()
