// Renders oracle cases (JSON lines on stdin) through milsymbol.js and prints
// one full record per line: {"svg": …, "sem": …} or {"error": …}.
// Mirrors `cargo run --example dump`.
import readline from "node:readline";
import { renderCase, canonical } from "./oracle.mjs";

const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
const out = [];
for await (const line of rl) {
  if (!line.trim()) continue;
  const r = renderCase(JSON.parse(line));
  out.push(JSON.stringify(r.error !== undefined ? { error: r.error } : { svg: r.svg, sem: canonical(r.sem) }));
  if (out.length >= 1000) {
    process.stdout.write(out.join("\n") + "\n");
    out.length = 0;
  }
}
if (out.length) process.stdout.write(out.join("\n") + "\n");
