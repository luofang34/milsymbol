#!/usr/bin/env python3
"""Convert drawings from the exact PDFs named in pin.json into native paths."""

import argparse
import hashlib
import json
import os
import runpy
from pathlib import Path

import drawing_config as config


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mil2525e1", type=Path)
    parser.add_argument("app6e2", type=Path)
    parser.add_argument("--cached-index", action="store_true",
                        help="reuse the page index in target/standards-drawings")
    args = parser.parse_args()
    pins = json.loads(Path(__file__).with_name("pin.json").read_text())["standards"]
    for pin in pins:
        path = getattr(args, pin["id"]).resolve()
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != pin["sha256"]:
            parser.error(f"{path}: SHA-256 {digest}, expected {pin['sha256']}")
        config.SOURCES[pin["id"]] = str(path)
    config.WORK.mkdir(parents=True, exist_ok=True)
    os.chdir(config.ROOT)
    stages = ["index", "match", "icons", "metoc", "controls", "context", "leadership", "emit"]
    for stage in stages:
        if stage == "index" and args.cached_index:
            continue
        runpy.run_path(str(Path(__file__).with_name(f"drawing_{stage}.py")))
        if stage in ["icons", "metoc"]:
            failures = json.loads((config.WORK / f"{stage}-fail.json").read_text())
            if failures:
                raise RuntimeError(f"{stage}: {len(failures)} unconverted drawings: {failures}")


if __name__ == "__main__":
    main()
