#!/usr/bin/env python3
"""检查 Android 文件清理策略是否集中在共享 FilePolicy。"""
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FILE_POLICY = ROOT / "platforms/android/java/app/msime/android/FilePolicy.java"
UPDATE_API = ROOT / "platforms/android/java/app/msime/android/account/UpdateApi.java"
PROFILE_PAGE = ROOT / "platforms/android/java/app/msime/android/home/ProfilePage.java"


def main() -> None:
    file_policy = FILE_POLICY.read_text(encoding="utf-8")
    update_api = UPDATE_API.read_text(encoding="utf-8")
    profile_page = PROFILE_PAGE.read_text(encoding="utf-8")
    required = (
        "public static void deleteQuietly(File file)",
        "if (file.exists() && !file.delete()) file.deleteOnExit();",
    )
    missing = [snippet for snippet in required if snippet not in file_policy]
    if missing:
        raise AssertionError("FilePolicy 缺少共享文件清理策略：" + ", ".join(missing))
    if "private static void deleteQuietly(File file)" in update_api:
        raise AssertionError("UpdateApi 仍保留重复的文件清理实现")
    if "FilePolicy.deleteQuietly(partial);" not in update_api:
        raise AssertionError("UpdateApi 没有调用共享文件清理策略")
    if "private static void deleteQuietly(File file)" in profile_page:
        raise AssertionError("ProfilePage 仍保留重复的文件清理实现")
    if "FilePolicy.deleteQuietly(file);" not in profile_page:
        raise AssertionError("ProfilePage 没有调用共享文件清理策略")
    print("android file policy: quiet deletion is shared")


if __name__ == "__main__":
    main()
