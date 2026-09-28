import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { comparisonRecord } from "./compare-record.mjs";
import { renderCase, canonical } from "./oracle.mjs";

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

test("known option keys use a safe upstream control and preserve their values", () => {
  const sidc = "10031000001211000000";
  for (const [key, known] of [["__proto__", "proto-key"], ["hasOwnProperty", "throws-upstream"]]) {
    const options = { uniqueDesignation: "A", [key]: "x\"\\😀" };
    const record = comparisonRecord({ sidc, options, known });
    const control = renderCase({ sidc, options: { uniqueDesignation: "A" } });
    assert.equal(record.expected.svg, control.svg);
    const expected = JSON.parse(record.expected.sem);
    assert.equal(expected.options[key], options[key]);
    delete expected.options[key];
    assert.equal(canonical(expected), canonical(control.sem));
    if (known === "throws-upstream") {
      assert.equal(record.error, "options.hasOwnProperty is not a function");
    } else {
      assert.equal(record.svg, control.svg);
      assert.equal(record.sem, canonical(control.sem));
    }
  }
});

test("canonical JSON preserves own prototype-named keys", () => {
  assert.equal(canonical(JSON.parse('{"__proto__":"x","a":1}')), '{"__proto__":"x","a":1}');
});

test("known controls reject unsupported values and upstream control failures", () => {
  const sidc = "10031000001211000000";
  assert.throws(() => comparisonRecord({ sidc, known: "proto-key", options: {} }), /string __proto__/);
  assert.throws(() => comparisonRecord({
    sidc, known: "throws-upstream", options: { hasOwnProperty: "x", colorMode: "invalid" },
  }), /control failed/);
});
