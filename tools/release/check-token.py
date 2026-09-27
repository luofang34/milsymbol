#!/usr/bin/env python3
"""Checks that CARGO_REGISTRY_TOKEN can publish the (not yet existing)
`milsymbol` crate, without publishing anything.

crates.io's publish endpoint parses the metadata, then authenticates the
token for that crate (`publish-new` scope), checks the owner's verified email
and rate limit, and only then validates the uploaded tarball. This script sends
correct metadata with a deliberately malformed tarball, so:

  * a usable token gets past authentication and is rejected at tarball
    validation ("malformed") -> exit 0; nothing can be published;
  * a missing/invalid/wrongly scoped token or unverified email is rejected
    before that -> exit 1 with crates.io's message.

The token is read from the environment and never printed. The attempt counts
against the publish-new rate limit once.
"""
import json
import os
import struct
import sys
import tomllib
import urllib.error
import urllib.request

token = os.environ.get("CARGO_REGISTRY_TOKEN", "")
if not token:
    sys.exit("CARGO_REGISTRY_TOKEN is not set")

with open("Cargo.toml", "rb") as f:
    pkg = tomllib.load(f)["package"]

metadata = {
    "name": pkg["name"],
    "vers": pkg["version"],
    "deps": [],
    "features": {},
    "authors": pkg.get("authors", []),
    "description": pkg.get("description"),
    "documentation": pkg.get("documentation"),
    "homepage": pkg.get("homepage"),
    "readme": None,
    "readme_file": pkg.get("readme"),
    "keywords": pkg.get("keywords", []),
    "categories": pkg.get("categories", []),
    "license": pkg.get("license"),
    "license_file": None,
    "repository": pkg.get("repository"),
    "badges": {},
    "links": None,
    "rust_version": pkg.get("rust-version"),
}
meta = json.dumps(metadata).encode()
tarball = b"\0" * 16  # not a gzip stream: rejected after authentication
body = struct.pack("<I", len(meta)) + meta + struct.pack("<I", len(tarball)) + tarball

req = urllib.request.Request(
    "https://crates.io/api/v1/crates/new",
    data=body,
    method="PUT",
    headers={"Authorization": token, "User-Agent": "milsymbol-release-token-check"},
)
try:
    with urllib.request.urlopen(req) as resp:
        # Unreachable with a malformed tarball; fail loudly if it ever happens.
        sys.exit(f"unexpected success ({resp.status}): {resp.read()[:300]!r}")
except urllib.error.HTTPError as e:
    detail = e.read().decode(errors="replace")
    print(f"crates.io answered {e.code}: {detail}")
    if e.code == 400 and "tarball" in detail:
        print("OK: token is valid for publishing new crate `%s`, email verified, not rate limited" % pkg["name"])
        sys.exit(0)
    sys.exit("token check failed")
