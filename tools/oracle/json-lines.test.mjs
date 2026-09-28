import assert from "node:assert/strict";
import { Readable } from "node:stream";
import test from "node:test";
import { jsonLines } from "./json-lines.mjs";

async function read(chunks) {
  const records = [];
  for await (const record of jsonLines(Readable.from(chunks))) records.push(record);
  return records;
}

test("records survive UTF-8 chunk boundaries, CRLF, blank lines and EOF", async () => {
  const expected = [{ text: "A\u2028B\u2029C😀" }, { value: 42 }];
  const input = Buffer.from("\n" + expected.map(JSON.stringify).join("\r\n\r\n"));
  for (const chunks of [[input], [...input].map((byte) => Buffer.from([byte]))]) {
    assert.deepEqual(await read(chunks), expected);
  }
});

test("invalid JSON rejects the input instead of skipping the record", async () => {
  await assert.rejects(read(['{"value":}\n']), SyntaxError);
  await assert.rejects(read(['{"value":']), SyntaxError);
});
