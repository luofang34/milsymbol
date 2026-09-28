// Renders oracle cases (JSON lines on stdin) through milsymbol.js and prints
// one full record per line: {"svg": …, "sem": …} or {"error": …}.
// Mirrors `cargo run --example dump`.
import { jsonLines } from "./json-lines.mjs";
import { renderCase, canonical } from "./oracle.mjs";

const out = [];
for await (const record of jsonLines(process.stdin)) {
  const r = renderCase(record);
  out.push(JSON.stringify(r.error !== undefined ? { error: r.error } : { svg: r.svg, sem: canonical(r.sem) }));
  if (out.length >= 1000) {
    process.stdout.write(out.join("\n") + "\n");
    out.length = 0;
  }
}
if (out.length) process.stdout.write(out.join("\n") + "\n");
