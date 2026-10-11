#!/usr/bin/env python3
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[1]
JAVA=ROOT/"platforms/android/java"
UI=JAVA/"app/msime/android/home/Ui.java"
def main():
    errors=[]; ui=UI.read_text(encoding="utf-8")
    for name in ("column","row","weightWrap","wrap"):
        if f"public static" in ui and f" {name}(" in ui:
            errors.append(f"{UI}: 仍保留 {name} 转发")
    for p in JAVA.rglob("*.java"):
        s=p.read_text(encoding="utf-8")
        for name in ("column","row","weightWrap","wrap"):
            if f"Ui.{name}(" in s: errors.append(f"{p}: 仍调用 Ui.{name}")
    if errors: print("\n".join(errors),file=sys.stderr); return 1
    print("Android layout factories use LayoutPolicy directly")
    return 0
if __name__=="__main__": raise SystemExit(main())
