#!/usr/bin/env python3
"""Ensure LexiconPage does not retain a stale community install lock."""
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/home/LexiconPage.java"


class AndroidLexiconInstallViewLifecycleContract(unittest.TestCase):
    def test_destroy_view_clears_installing_state(self):
        source = SOURCE.read_text(encoding="utf-8")
        start = source.index("@Override public void onDestroyView()")
        end = source.index("\n    // ---- 数据 ----", start)
        body = source[start:end]
        self.assertIn("installing.clear();", body)


if __name__ == "__main__":
    unittest.main()
