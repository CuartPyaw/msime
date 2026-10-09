#!/usr/bin/env python3
"""验证构建方暂存匹配的 MinGW 运行时，Wine 复用它而不再准备编译镜像。"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent


class StagedRuntimeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.windows = self.root / "platforms/windows"
        self.windows.mkdir(parents=True)
        for name in ("stage-runtime.sh", "run-tests-wine.sh"):
            shutil.copyfile(ROOT / "platforms/windows" / name, self.windows / name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.library = self.root / "toolchain"
        self.library.mkdir()
        self.log = self.root / "docker.jsonl"
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}",
                        MSIME_TEST_LIBRARY=str(self.library),
                        MSIME_TEST_DOCKER_LOG=str(self.log))
        self.env.pop("MSIME_WINE_RESOURCES", None)
        self.env.pop("MSIME_MINGW_RUNTIME_DIR", None)
        self.write_command("synthetic-compiler", '''#!/usr/bin/env python3
import os, pathlib, sys
arg = sys.argv[1]
if arg.startswith("-print-file-name="):
    name = arg.split("=", 1)[1]
    path = pathlib.Path(os.environ["MSIME_TEST_LIBRARY"]) / name
    print(path if path.is_file() else name)
else:
    sys.exit(1)
''')
        self.write_command("synthetic-objdump", '''#!/usr/bin/env python3
import os, pathlib, sys
if sys.argv[1] == "-f":
    print("fixture: file format " + pathlib.Path(sys.argv[2]).read_text().strip())
elif sys.argv[1] == "-p":
    for dependency in os.environ.get("MSIME_TEST_IMPORTS", "").split():
        print("DLL Name: " + dependency)
else:
    sys.exit(1)
''')
        for prefix in ("i686", "x86_64"):
            (self.bin / f"{prefix}-w64-mingw32-g++").symlink_to("synthetic-compiler")
            (self.bin / f"{prefix}-w64-mingw32-objdump").symlink_to("synthetic-objdump")
        self.write_command("cargo", "#!/bin/sh\nexit 0\n")
        self.write_command("docker", '''#!/usr/bin/env python3
import json, os, pathlib, sys
call = sys.argv[1:]
entry = {"args": call}
if call[0] == "run":
    for arg in call:
        if arg.endswith(":/rt:ro"):
            folder = pathlib.Path(arg[:-len(":/rt:ro")])
            entry["runtime"] = {p.name: p.read_text() for p in folder.glob("*.dll")}
with open(os.environ["MSIME_TEST_DOCKER_LOG"], "a") as log:
    log.write(json.dumps(entry) + "\\n")
if "runtime" in entry:
    print("PASS windows-synthetic-runtime")
''')

    def write_command(self, name, text):
        path = self.bin / name
        path.write_text(text, encoding="utf-8")
        path.chmod(0o755)

    def names(self, arch):
        return ("libstdc++-6.dll", "libwinpthread-1.dll",
                "libgcc_s_dw2-1.dll" if arch == "x86" else "libgcc_s_seh-1.dll")

    def prepare_library(self, arch):
        for name in self.names(arch):
            (self.library / name).write_text("pei-i386" if arch == "x86" else "pei-x86-64")

    def stage(self, arch):
        output = self.root / "target" / "windows-full" / arch
        return output, subprocess.run(
            ["bash", str(self.windows / "stage-runtime.sh"), arch, "--runtime-only", str(output)],
            env=self.env, capture_output=True, text=True)

    def test_runtime_only_stages_both_architectures_without_product_executables(self):
        for arch in ("x86", "x64"):
            with self.subTest(arch=arch):
                self.prepare_library(arch)
                output, result = self.stage(arch)
                self.assertEqual(result.returncode, 0, result.stderr)
                for name in self.names(arch):
                    self.assertEqual((output / name).read_bytes(), (self.library / name).read_bytes())
                self.assertFalse((output / "run-smoke.ps1").exists())

    def test_runtime_only_rejects_missing_dependency(self):
        self.prepare_library("x86")
        (self.library / "libwinpthread-1.dll").unlink()
        _, result = self.stage("x86")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Missing matching runtime", result.stderr)

    def test_runtime_only_rejects_wrong_pe_architecture(self):
        self.prepare_library("x86")
        (self.library / "libstdc++-6.dll").write_text("pei-x86-64")
        _, result = self.stage("x86")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Wrong runtime architecture", result.stderr)

    def test_default_mode_still_requires_full_build(self):
        self.prepare_library("x86")
        result = subprocess.run(["bash", str(self.windows / "stage-runtime.sh"), "x86"],
                                env=self.env, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Run build-cross.sh first", result.stderr)

    def prepare_full_build(self):
        self.prepare_library("x86")
        build = self.root / "target/windows-full/x86"
        build.mkdir(parents=True)
        for name in (
            "windows-registration-inbox.exe", "windows-focus-router.exe", "windows-main-frame.exe",
            "windows-focus-gate.exe", "windows-input-queue.exe", "windows-session-smoke.exe",
            "windows-reply-codec.exe", "windows-reply-composer.exe", "windows-server-smoke.exe",
            "windows-preview-config.exe", "MetasequoiaImeServer.exe", "msime_host_api.dll",
            "tests/native-pipe/windows-pipe-io.exe",
        ):
            path = build / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("pei-i386")
        runner = self.windows / "tests/tools/run-smoke.ps1"
        runner.parent.mkdir(parents=True)
        runner.write_text("synthetic runner")
        return build

    def test_full_mode_accepts_system_dlls_and_still_rejects_unknown_dependencies(self):
        build = self.prepare_full_build()
        self.env["MSIME_TEST_IMPORTS"] = (
            "CRYPT32.dll WINHTTP.dll DWrite.dll combase.dll d2d1.dll d3d11.dll dcomp.dll "
            "dwmapi.dll mmdevapi.dll oleaut32.dll propsys.dll rpcrt4.dll"
        )
        result = subprocess.run(["bash", str(self.windows / "stage-runtime.sh"), "x86"],
                                env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((build / "run-smoke.ps1").read_text(), "synthetic runner")
        self.env["MSIME_TEST_IMPORTS"] = "synthetic-unknown.dll"
        result = subprocess.run(["bash", str(self.windows / "stage-runtime.sh"), "x86"],
                                env=self.env, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unclassified dependency: synthetic-unknown.dll", result.stderr)

    def test_wine_uses_staged_dlls_without_querying_host_or_cross_compiler(self):
        for arch in ("x86", "x64"):
            with self.subTest(arch=arch):
                self.log.unlink(missing_ok=True)
                build = self.root / "target/windows-full" / arch
                build.mkdir(parents=True)
                for name in self.names(arch):
                    (build / name).write_text(f"synthetic-{arch}-{name}")
                # 故意移走生产工具链：读取已有构建产物不应需要它。
                for name in self.library.glob("*.dll"):
                    name.unlink()
                result = subprocess.run(["bash", str(self.windows / "run-tests-wine.sh"), arch],
                                        env=self.env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                calls = [json.loads(line) for line in self.log.read_text().splitlines()]
                builds = [call["args"] for call in calls if call["args"][0] == "build"]
                self.assertEqual(len(builds), 1, calls)
                self.assertTrue(builds[0][-1].endswith("/platforms/windows/wine"), calls)
                run = next((call for call in calls if "runtime" in call), None)
                self.assertIsNotNone(run, calls)
                self.assertEqual(run["runtime"], {name: f"synthetic-{arch}-{name}" for name in self.names(arch)})
                self.assertEqual(run["args"][run["args"].index("--platform") + 1], "linux/amd64")
                self.assertIn("PASS windows-synthetic-runtime", result.stdout)


if __name__ == "__main__":
    if os.name != "posix" or not shutil.which("bash") or not shutil.which("cmake"):
        print("skipped: 运行时暂存回归需要 POSIX、bash 和 cmake")
    else:
        unittest.main()
