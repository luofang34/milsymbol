#!/usr/bin/env bash
# Regenerates tests/data/standards/*/{codes,headers}.tsv from the pinned
# standards. Dev-only (needs Node and poppler's pdftotext); the PDFs are not in
# the repository and are checked against tools/standards/pin.json first.
#
# usage: tools/standards/extract.sh <MIL-STD-2525E w/Change 1 pdf> <APP-06(E)(2) pdf>
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
data="$here/../../tests/data/standards"
[ $# -eq 2 ] || { echo "usage: $0 <mil2525e1.pdf> <app6e2.pdf>" >&2; exit 2; }

check() {
  local id="$1" pdf="$2" want got
  want="$(node -e 'const p=require(process.argv[1]);const s=p.standards.find(s=>s.id===process.argv[2]);if(!s)process.exit(1);console.log(s.sha256)' "$here/pin.json" "$id")"
  got="$(shasum -a 256 "$pdf" | cut -d' ' -f1)"
  [ "$want" = "$got" ] || { echo "$pdf: sha256 $got, pinned $want for $id" >&2; exit 1; }
}

check mil2525e1 "$1"
check app6e2 "$2"
node "$here/mil2525e1.mjs" "$1" "$data/mil2525e1"
node "$here/app6e2.mjs" "$2" "$data/app6e2"
