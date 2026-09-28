import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

test("JSON lines preserve Unicode line separators inside text", () => {
  const cases = [
    { sidc: "10031000001211000000", options: { uniqueDesignation: "A\u2028B\u2029C😀" } },
    { sidc: "SFGPUCI-----", options: { uniqueDesignation: "last record" } },
  ];
  const result = spawnSync(process.execPath, [fileURLToPath(new URL("./render.mjs", import.meta.url))], {
    input: cases.map(JSON.stringify).join("\r\n\r\n"),
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  const records = result.stdout.trimEnd().split("\n").map(JSON.parse);
  assert.equal(records.length, cases.length);
  for (const [i, record] of records.entries()) {
    assert.equal(JSON.parse(record.sem).options.uniqueDesignation, cases[i].options.uniqueDesignation);
    assert.ok(record.svg.endsWith("</svg>"));
  }
});
