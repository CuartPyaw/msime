"""Second batch of scripted golden scenarios. Reuses the fixtures and helpers of tools/engine-golden/gen_scenarios.py (executed into this namespace, its own output goes to a scratch dir), then defines only new scenarios. Fixture SQL is copied verbatim from the reference ctests (paths in each scenario's `source`)."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
NEW_OUT = os.path.abspath(sys.argv[1])
EXISTING = "/Users/e-hu/worktrees/engine-rust/crates/engine/tests/golden/scenarios"
BASE = "/Users/e-hu/worktrees/engine-rust/tools/engine-golden/gen_scenarios.py"

_argv = sys.argv
sys.argv = [BASE, os.path.join(HERE, "orig-gen")]
_stdout = sys.stdout
sys.stdout = open(os.devnull, "w")
exec(compile(open(BASE, encoding="utf-8").read(), BASE, "exec"))
sys.stdout = _stdout
sys.argv = _argv
scenarios.clear()  # noqa: F821 (defined by the exec above)

BATCHES = {}
_current = [None]


def batch(name):
    _current[0] = name


_add = add  # noqa: F821


def add(name, source, fixture, options, steps, note=None, covers=None):  # noqa: F811
    _add(name, source, fixture, options, steps, note=note, covers=covers)
    BATCHES.setdefault(_current[0], []).append(name)


for module in sorted(os.listdir(os.path.join(HERE, "batches"))):
    if module.endswith(".py"):
        batch(module[:-3])
        path = os.path.join(HERE, "batches", module)
        exec(compile(open(path, encoding="utf-8").read(), path, "exec"))

existing = {f[:-5] for f in os.listdir(EXISTING) if f.endswith(".json")}
os.makedirs(NEW_OUT, exist_ok=True)
names = set()
for s in scenarios:  # noqa: F821
    assert s["name"] not in names, s["name"]
    assert s["name"] not in existing, "collides with an existing scenario: " + s["name"]
    names.add(s["name"])
    with open(os.path.join(NEW_OUT, s["name"] + ".json"), "w", encoding="utf-8") as f:
        json.dump(s, f, ensure_ascii=False, indent=1)
        f.write("\n")
with open(os.path.join(NEW_OUT, "_batches.json"), "w", encoding="utf-8") as f:
    json.dump(BATCHES, f, ensure_ascii=False, indent=1)
print(len(scenarios))  # noqa: F821
