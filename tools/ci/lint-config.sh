#!/usr/bin/env bash
# Every workspace member inherits the workspace lint table, every crate outside
# the workspace (such as fuzz/) repeats it, the forbidden lints are forbidden,
# and clippy.toml carries the test exemptions they rely on.
set -euo pipefail
python3 - <<'PY'
import glob
import sys
import tomllib

with open("Cargo.toml", "rb") as f:
    root = tomllib.load(f)
workspace = root["workspace"]
lints = workspace["lints"]
failures = []

for member in workspace["members"]:
    with open(f"{member}/Cargo.toml", "rb") as f:
        manifest = tomllib.load(f)
    if manifest.get("lints") != {"workspace": True}:
        failures.append(f"{member}: must set [lints] workspace = true")

for path in glob.glob("*/Cargo.toml"):
    member = path.split("/")[0]
    if member in workspace["members"]:
        continue
    with open(path, "rb") as f:
        manifest = tomllib.load(f)
    own = manifest.get("lints", {})
    for section, table in lints.items():
        for name, level in table.items():
            if own.get(section, {}).get(name) != level:
                failures.append(f"{member}: lints.{section}.{name} must be {level!r}")

clippy = lints["clippy"]
for name in ["unwrap_used", "expect_used", "panic", "todo", "unimplemented",
             "unreachable", "exit", "mem_forget", "get_unwrap"]:
    if clippy.get(name) != "forbid":
        failures.append(f"workspace.lints.clippy.{name} must be 'forbid'")
if lints["rust"].get("unsafe_code") != "forbid":
    failures.append("workspace.lints.rust.unsafe_code must be 'forbid'")

with open("clippy.toml", "rb") as f:
    conf = tomllib.load(f)
for key in ["allow-dbg-in-tests", "allow-expect-in-tests", "allow-indexing-slicing-in-tests",
            "allow-panic-in-tests", "allow-print-in-tests", "allow-unwrap-in-tests"]:
    if conf.get(key) is not True:
        failures.append(f"clippy.toml: {key} must be true")

if failures:
    print("\n".join(failures), file=sys.stderr)
    sys.exit(1)
print("lint configuration is complete and identical in every crate")
PY
