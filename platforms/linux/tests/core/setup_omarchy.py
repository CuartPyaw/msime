#!/usr/bin/env python3
"""msime-linux-setup 在 Omarchy 上把主题钩子链进 ~/.config/omarchy/hooks/theme-set.d，并立即按当前主题生成一次 Omarchy 皮肤；--unregister 只移除自己的链接。

HOME 指向临时目录，msime-linux-settings 用一个记录参数和环境的桩代替。不需要 Omarchy 本体，也不碰真实的家目录。
"""
import importlib.machinery
import importlib.util
import io
import os
import sys
import tempfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/msime-linux-setup"

SETTINGS_STUB = """#!/bin/sh
printf '%s %s\\n' "$*" "$MSIME_CLIENT_HOST_OPTIONS" >> "$(dirname "$0")/settings.log"
exit "${SETTINGS_EXIT:-0}"
"""


def load_setup():
    spec = importlib.util.spec_from_loader(
        "msime_client_setup", importlib.machinery.SourceFileLoader("msime_client_setup", str(SCRIPT))
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run(function, *arguments):
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        result = function(*arguments)
    return result, out.getvalue(), err.getvalue()


def main() -> int:
    setup = load_setup()
    with tempfile.TemporaryDirectory() as name:
        scratch = Path(name)
        home = scratch / "home"
        home.mkdir()
        os.environ["HOME"] = str(home)
        prefix = scratch / "prefix"
        hook = prefix / setup.OMARCHY_HOOK
        hook.parent.mkdir(parents=True)
        hook.write_text("#!/bin/bash\n")
        settings = prefix / "bin/msime-linux-settings"
        settings.parent.mkdir(parents=True)
        settings.write_text(SETTINGS_STUB)
        settings.chmod(0o755)
        log = prefix / "bin/settings.log"
        options = scratch / "state/runtime-options.json"
        link = home / ".config/omarchy/hooks/theme-set.d/msime"

        # 不是 Omarchy：什么都不做。
        _, out, err = run(setup.link_omarchy_theme_hook, prefix, options)
        assert out == err == "" and not link.is_symlink() and not log.exists(), (out, err)

        # Omarchy：链上钩子，按当前主题生成一次皮肤，告诉用户去「主题」菜单里选。
        (home / ".local/state/omarchy/current/theme").mkdir(parents=True)
        (home / ".config/omarchy").mkdir(parents=True, exist_ok=True)
        _, out, err = run(setup.link_omarchy_theme_hook, prefix, options)
        assert err == "", err
        assert "「Omarchy」" in out, out
        assert os.readlink(link) == str(hook)
        assert log.read_text() == f"--sync-omarchy-theme {options}\n", log.read_text()

        # 再跑一次（更新路径）：替换自己的链接，照常同步。
        run(setup.link_omarchy_theme_hook, prefix, options)
        assert os.readlink(link) == str(hook) and len(log.read_text().splitlines()) == 2

        # 同步失败：钩子仍然留着，下次切主题再生成；不让安装失败。
        os.environ["SETTINGS_EXIT"] = "1"
        _, out, err = run(setup.link_omarchy_theme_hook, prefix, options)
        del os.environ["SETTINGS_EXIT"]
        assert "下次切换 Omarchy 主题时会再生成" in err, err
        assert link.is_symlink()

        # 卸载：移除自己的链接。
        removed, _, _ = run(setup.unlink_omarchy_theme_hook)
        assert removed and not link.is_symlink(), removed
        assert run(setup.unlink_omarchy_theme_hook)[0] is None

        # 同名的文件是用户自己的：不覆盖，卸载也不删。
        link.write_text("#!/bin/bash\necho mine\n")
        _, out, err = run(setup.link_omarchy_theme_hook, prefix, options)
        assert "不是水杉的钩子" in err, err
        assert link.read_text() == "#!/bin/bash\necho mine\n"
        assert run(setup.unlink_omarchy_theme_hook)[0] is None and link.exists()
    return 0


if __name__ == "__main__":
    sys.exit(main())
