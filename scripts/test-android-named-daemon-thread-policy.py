#!/usr/bin/env python3
"""Android 后台执行器复用统一的命名守护线程工厂。"""

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
JAVA = ROOT / "platforms/android/java/app/msime/android"
SITES = {
    JAVA / "home/AboutPage.java": ("msime-home-network",),
    JAVA / "home/HostTask.java": ("msime-settings-host", "msime-settings-network"),
    JAVA / "home/OnboardingChoices.java": ("msime-onboarding-choices",),
}


def main() -> int:
    errors = []
    policy = (JAVA / "ThreadPolicy.java").read_text(encoding="utf-8")
    if "ThreadFactory namedDaemonFactory(String name)" not in policy:
        errors.append("ThreadPolicy 缺少命名守护线程工厂")

    for path, names in SITES.items():
        source = path.read_text(encoding="utf-8")
        for name in names:
            expected = f'ThreadPolicy.namedDaemonFactory("{name}")'
            if expected not in source:
                errors.append(f"{path}: {name} 未复用 ThreadPolicy")
        if "setDaemon(true)" in source:
            errors.append(f"{path}: 仍在重复配置守护线程")

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("Android background executors use the shared named daemon thread factory")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
