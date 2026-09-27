// Compares two record files (oracle vs Rust) produced from the same cases.
// Usage: node compare.mjs cases.jsonl oracle.jsonl rust.jsonl [maxReports]
import fs from "node:fs";

const [casesFile, aFile, bFile, maxArg] = process.argv.slice(2);
const lines = (f) => fs.readFileSync(f, "utf8").split("\n").filter((l) => l.trim());
const cases = lines(casesFile);
const a = lines(aFile);
const b = lines(bFile);
const max = Number(maxArg || 10);

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
for (let i = 0; i < cases.length; i++) {
  const x = JSON.parse(a[i] || "{}");
  const y = JSON.parse(b[i] || "{}");
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
  if (problem && reported++ < max) console.log(`#${i} ${cases[i]}\n   ${problem}`);
}
function svgDiff(s, t) {
  let i = 0;
  while (i < s.length && s[i] === t[i]) i++;
  return `svg differs at ${i}:\n     oracle …${s.slice(Math.max(0, i - 80), i + 120)}\n     rust   …${t.slice(Math.max(0, i - 80), i + 120)}`;
}
console.log(`cases ${cases.length}: svg mismatches ${svgBad}, semantic mismatches ${semBad}, error mismatches ${errBad}`);
for (const [k, v] of [...byPath].sort((p, q) => q[1] - p[1]).slice(0, 15)) console.log(`  ${v}\t${k}`);
process.exit(svgBad + semBad + errBad ? 1 : 0);
