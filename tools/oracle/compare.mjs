// Compares two record files (oracle vs Rust) produced from the same cases.
// Usage: node compare.mjs cases.jsonl oracle.jsonl rust.jsonl [maxReports]
import fs from "node:fs";

const [casesFile, aFile, bFile, maxArg] = process.argv.slice(2);
// Streams non-empty lines of a file (record files can exceed V8's string limit).
function* lines(f) {
  const fd = fs.openSync(f, "r");
  const buf = Buffer.alloc(1 << 24);
  let rest = Buffer.alloc(0);
  for (;;) {
    const n = fs.readSync(fd, buf, 0, buf.length, null);
    if (n === 0) break;
    let chunk = Buffer.concat([rest, buf.subarray(0, n)]);
    let start = 0;
    for (let i = chunk.indexOf(10); i !== -1; i = chunk.indexOf(10, start)) {
      const line = chunk.toString("utf8", start, i);
      if (line.trim()) yield line;
      start = i + 1;
    }
    rest = Buffer.from(chunk.subarray(start));
  }
  if (rest.length && rest.toString("utf8").trim()) yield rest.toString("utf8");
  fs.closeSync(fd);
}
const max = Number(maxArg || 10);
const ia = lines(aFile);
const ib = lines(bFile);

function firstDiff(x, y, path = "") {
  if (typeof x !== typeof y || Array.isArray(x) !== Array.isArray(y) || x === null || y === null) {
    return x === y ? null : `${path}: ${JSON.stringify(x)?.slice(0, 300)} != ${JSON.stringify(y)?.slice(0, 300)}`;
  }
  if (typeof x !== "object") return x === y ? null : `${path}: ${JSON.stringify(x).slice(0, 300)} != ${JSON.stringify(y).slice(0, 300)}`;
  const keys = new Set([...Object.keys(x), ...Object.keys(y)]);
  for (const k of keys) {
    const d = firstDiff(x[k], y[k], `${path}.${k}`);
    if (d) return d;
  }
  return null;
}

let svgBad = 0, semBad = 0, errBad = 0, reported = 0;
const byPath = new Map();
let i = -1;
for (const caseLine of lines(casesFile)) {
  i++;
  const x = JSON.parse(ia.next().value || "{}");
  const y = JSON.parse(ib.next().value || "{}");
  let problem = null;
  if ((x.error !== undefined) !== (y.error !== undefined)) {
    errBad++;
    problem = `error mismatch: oracle=${x.error} rust=${y.error}`;
  } else if (x.error === undefined) {
    const semDiff = x.sem === y.sem ? null : firstDiff(JSON.parse(x.sem), JSON.parse(y.sem), "sem");
    if (semDiff) semBad++;
    if (x.svg !== y.svg) svgBad++;
    if (semDiff || x.svg !== y.svg) {
      problem = semDiff || svgDiff(x.svg, y.svg);
      const key = (semDiff || "svg").replace(/\[\d+\]|\.\d+/g, "[]").split(":")[0];
      byPath.set(key, (byPath.get(key) || 0) + 1);
    }
  }
  if (problem && reported++ < max) console.log(`#${i} ${caseLine}\n   ${problem}`);
}
function svgDiff(s, t) {
  let i = 0;
  while (i < s.length && s[i] === t[i]) i++;
  return `svg differs at ${i}:\n     oracle …${s.slice(Math.max(0, i - 80), i + 120)}\n     rust   …${t.slice(Math.max(0, i - 80), i + 120)}`;
}
console.log(`cases ${i + 1}: svg mismatches ${svgBad}, semantic mismatches ${semBad}, error mismatches ${errBad}`);
for (const [k, v] of [...byPath].sort((p, q) => q[1] - p[1]).slice(0, 15)) console.log(`  ${v}\t${k}`);
process.exit(svgBad + semBad + errBad ? 1 : 0);
