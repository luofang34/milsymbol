#!/usr/bin/env bash
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

if failures:
    print("\n".join(failures), file=sys.stderr)
    sys.exit(1)
print("lint configuration is identical in every crate")
PY
