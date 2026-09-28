// Renders oracle cases (JSON lines on stdin) through milsymbol.js and prints
// one full record per line: {"svg": …, "sem": …} or {"error": …}.
// Mirrors `cargo run --example dump`.
// Known option-key cases also carry an `expected` upstream control record.
import { jsonLines } from "./json-lines.mjs";
import { comparisonRecord } from "./compare-record.mjs";

const out = [];
for await (const record of jsonLines(process.stdin)) {
  out.push(JSON.stringify(comparisonRecord(record)));
  if (out.length >= 1000) {
    process.stdout.write(out.join("\n") + "\n");
    out.length = 0;
  }
}
if (out.length) process.stdout.write(out.join("\n") + "\n");
