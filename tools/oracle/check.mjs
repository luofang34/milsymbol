// One differential pass over a suite, or one shard of it: writes the cases
// and milsymbol.js's comparison records (each case rendered once), and
// checks that the committed fixture holds exactly the lines those records
// give. The Rust side is compared against the same records by
// check-suite.sh.
//
// Usage: node check.mjs <suite> <shard> <shards> <outDir>
// Shard k of n (1-based) takes the cases whose 0-based index i has
// i % n == k - 1, in the cases and in the fixture alike.
//
// Fixture hashes are canonical for V8 on x64 (Math.sin/cos differ on
// arm64), so on other architectures the fixture check is skipped unless
// REQUIRE_FIXTURE_CHECK is set, which makes it an error.
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { comparisonRecord } from "./compare-record.mjs";
import { fixtureLineFromRecord } from "./oracle.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const [suite, shardArg, shardsArg, outDir] = process.argv.slice(2);
const shard = Number(shardArg);
const shards = Number(shardsArg);
if (!suite || !outDir || !(shards >= 1) || !(shard >= 1 && shard <= shards)) {
  console.error("usage: node check.mjs <suite> <shard> <shards> <outDir>");
  process.exit(2);
}
const inShard = (_, i) => i % shards === shard - 1;
const nonEmpty = (l) => l.trim() !== "";

const cases = execFileSync(process.execPath, [path.join(here, "cases.mjs"), suite], {
  maxBuffer: 1 << 30,
  encoding: "utf8",
})
  .split("\n")
  .filter(nonEmpty)
  .filter(inShard);

const name = `${suite}.${shard}-of-${shards}`;
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, `${name}.cases.jsonl`), cases.join("\n") + "\n");

const fixturePath = path.join(here, "..", "..", "tests", "corpus", `${suite}.jsonl.gz`);
const checkFixture = fs.existsSync(fixturePath) && process.arch === "x64";
const records = fs.openSync(path.join(outDir, `${name}.oracle.jsonl`), "w");
const expected = [];
let buffer = [];
for (const line of cases) {
  const c = JSON.parse(line);
  const record = comparisonRecord(c);
  buffer.push(JSON.stringify(record));
  if (buffer.length >= 1000) {
    fs.writeSync(records, buffer.join("\n") + "\n");
    buffer = [];
  }
  if (checkFixture) expected.push(JSON.stringify(fixtureLineFromRecord(c, record)));
}
if (buffer.length) fs.writeSync(records, buffer.join("\n") + "\n");
fs.closeSync(records);
console.error(`${name}: ${cases.length} cases rendered by milsymbol.js`);

if (!fs.existsSync(fixturePath)) {
  console.error(`${name}: no committed fixture`);
} else if (!checkFixture) {
  console.error(`${name}: fixture check needs x64 Node (this is ${process.arch})`);
  if (process.env.REQUIRE_FIXTURE_CHECK) process.exit(1);
} else {
  const committed = zlib
    .gunzipSync(fs.readFileSync(fixturePath))
    .toString("utf8")
    .split("\n")
    .filter(nonEmpty)
    .filter(inShard);
  const differing = expected.flatMap((line, i) => (line === committed[i] ? [] : [i]));
  if (committed.length !== expected.length || differing.length) {
    console.error(`${name}: fixture has ${committed.length} lines, oracle gives ${expected.length}; ${differing.length} differ`);
    for (const i of differing.slice(0, 5)) console.error(`  committed ${committed[i]}\n  oracle    ${expected[i]}`);
    console.error(`regenerate with: node tools/oracle/fixtures.mjs ${suite} (x64 Node)`);
    process.exit(1);
  }
  console.error(`${name}: fixture matches the oracle`);
}
