#!/usr/bin/env python3
"""验证交叉构建按 Docker daemon 选择架构并隔离宿主工具缓存。"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent


class CrossContainerPlatformTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        script = self.root / "platforms/windows/build-cross-container.sh"
        script.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "platforms/windows/build-cross-container.sh", script)
        self.script = script
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.log = self.root / "docker.jsonl"
        docker = self.bin / "docker"
        docker.write_text('''#!/usr/bin/env python3
import json, os, sys
with open(os.environ["MSIME_TEST_DOCKER_LOG"], "a") as log:
    log.write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[1] == "info":
    if os.environ.get("MSIME_TEST_DOCKER_DOWN"):
        sys.exit(1)
    if "--format" in sys.argv:
        print(os.environ["MSIME_TEST_DOCKER_PLATFORM"])
elif sys.argv[1] not in ("build", "run"):
    sys.exit(2)
''', encoding="utf-8")
        docker.chmod(0o755)
        # 客户端架构故意固定为 x86_64，选择必须以 daemon 为准。
        uname = self.bin / "uname"
        uname.write_text("#!/bin/sh\necho x86_64\n", encoding="utf-8")
        uname.chmod(0o755)

    def run_script(self, daemon, down=False):
        env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}",
                   MSIME_TEST_DOCKER_LOG=str(self.log),
                   MSIME_TEST_DOCKER_PLATFORM=daemon,
                   DOCKER_DEFAULT_PLATFORM="linux/amd64")
        if down:
            env["MSIME_TEST_DOCKER_DOWN"] = "1"
        result = subprocess.run(["bash", str(self.script), "x86"], env=env,
                                capture_output=True, text=True)
        calls = [json.loads(line) for line in self.log.read_text().splitlines()]
        return result, calls

    def assert_platform(self, daemon, platform, suffix):
        result, calls = self.run_script(daemon)
        self.assertEqual(result.returncode, 0, result.stderr)
        build = next(call for call in calls if call[0] == "build")
        run = next(call for call in calls if call[0] == "run")
        for command in (build, run):
            self.assertEqual(command[command.index("--platform") + 1], platform)
        self.assertEqual(run[-3:], ["bash", "platforms/windows/build-cross.sh", "x86"])
        self.assertIn(f"{self.root}/target/tooling-linux{suffix}:/repo/target/tooling", run)
        self.assertIn(f"MSIME_WINDOWS_DEPS_ROOT=/repo/target/windows-native-deps-linux{suffix}", run)
        self.assertIn("CARGO_HOME=/repo/target/windows-cross/cargo-home", run)
        self.assertTrue((self.root / f"target/tooling-linux{suffix}").is_dir())
        self.assertTrue((self.root / f"target/windows-native-deps-linux{suffix}").is_dir())
        image = build[build.index("-t") + 1]
        self.assertIn(image, run)
        self.assertEqual(image, "msime-cross:local-arm64" if suffix else "msime-cross:local")

    def test_arm_daemon_uses_native_compilers_despite_client_default(self):
        self.assert_platform("linux/aarch64", "linux/arm64", "/arm64")

    def test_arm64_alias_uses_the_same_cache(self):
        self.assert_platform("linux/arm64", "linux/arm64", "/arm64")

    def test_amd_daemon_keeps_existing_cache(self):
        self.assert_platform("linux/x86_64", "linux/amd64", "")

    def test_amd64_alias_keeps_existing_cache(self):
        self.assert_platform("linux/amd64", "linux/amd64", "")

    def test_unsupported_daemon_stops_before_preparing_cache(self):
        for daemon in ("linux/riscv64", "windows/x86_64"):
            with self.subTest(daemon=daemon):
                result, calls = self.run_script(daemon)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(any(call[0] in ("build", "run") for call in calls))
                self.assertFalse((self.root / "target").exists())

    def test_unavailable_daemon_keeps_skip_behavior(self):
        result, calls = self.run_script("linux/aarch64", down=True)
        self.assertEqual(result.returncode, 0)
        self.assertIn("skipped:", result.stdout)
        self.assertFalse(any(call[0] in ("build", "run") for call in calls))


if __name__ == "__main__":
    if os.name != "posix" or not shutil.which("bash"):
        print("skipped: 交叉容器编排回归需要 POSIX 和 bash")
    else:
        unittest.main()
