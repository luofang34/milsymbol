// Reference vectors for the V8 behaviour the port reproduces bit for bit.
//
//   node vectors.mjs trig    prints tests/data/v8_trig_<arch>.txt for this
//                            Node's architecture: argument, Math.sin and
//                            Math.cos as IEEE-754 bit patterns
//   node vectors.mjs check   checks the committed trig file of this
//                            architecture byte for byte, and that every line
//                            of tests/data/v8_numbers.txt is String(x)
//
// Run with x64 Node and arm64 Node (CI runs both) to cover both trig files.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const data = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "tests", "data");

function mulberry32(seed) {
  return function () {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const bits = new Float64Array(1);
const words = new BigUint64Array(bits.buffer);
const hex = (x) => {
  bits[0] = x;
  return words[0].toString(16).padStart(16, "0");
};
const fromHex = (h) => {
  words[0] = BigInt("0x" + h);
  return bits[0];
};

// The direction suite's arguments (degrees in 0.1° steps, both formulas
// upstream uses), random magnitudes, values near multiples of π/2 and huge
// arguments that need full argument reduction.
function trig() {
  const r = mulberry32(7);
  const xs = [];
  for (let i = 0; i < 3600; i++) {
    const d = i / 10;
    xs.push((d * Math.PI) / 180, (d / 360) * Math.PI * 2);
  }
  for (let i = 0; i < 1500; i++) {
    const e = Math.floor(r() * 60) - 30;
    xs.push((r() * 2 - 1) * Math.pow(2, e));
  }
  for (let i = 0; i < 600; i++) xs.push(((Math.floor(r() * 2e6) - 1e6) * Math.PI) / 2 + (r() - 0.5) * 1e-9);
  // Powers of ten are parsed, not computed: `Math.pow(10, k)` is not
  // correctly rounded on every platform, which would change the arguments.
  for (let i = 0; i < 300; i++) {
    const m = r() * 2 - 1;
    xs.push(m * Number(`1e${Math.floor(r() * 300) + 10}`));
  }
  return [...new Set(xs)].map((x) => [hex(x), hex(Math.sin(x)), hex(Math.cos(x))].join(" ")).join("\n") + "\n";
}

function check() {
  const file = path.join(data, `v8_trig_${process.arch}.txt`);
  const committed = fs.readFileSync(file, "utf8").split("\n");
  const current = trig().split("\n");
  const differing = current.flatMap((l, i) => (l === committed[i] ? [] : [i]));
  const trigOk = committed.length === current.length && differing.length === 0;
  console.error(
    `${path.basename(file)}: ${differing.length} of ${current.length} lines differ from ` +
      `Node ${process.version} (V8 ${process.versions.v8}) on ${process.platform}/${process.arch}`,
  );
  for (const i of differing.slice(0, 5)) console.error(`  committed ${committed[i]}\n  this Node ${current[i]}`);
  const lines = fs.readFileSync(path.join(data, "v8_numbers.txt"), "utf8").split("\n").filter((l) => l);
  const wrong = lines.filter((l) => {
    const [h, s] = l.split(" ");
    return String(fromHex(h)) !== s;
  });
  console.error(`v8_numbers.txt: ${lines.length - wrong.length} of ${lines.length} lines are String(x)`);
  for (const l of wrong.slice(0, 5)) console.error(`  ${l}`);
  if (!trigOk || wrong.length) process.exit(1);
}

const command = process.argv[2];
if (command === "trig") process.stdout.write(trig());
else if (command === "check") check();
else {
  console.error("usage: node vectors.mjs trig|check");
  process.exit(2);
}
