// Comparison records include an independent upstream control for option
// keys that JavaScript's object semantics drop or treat as methods.
import { renderCase, canonical } from "./oracle.mjs";

function encode(record) {
  return record.error !== undefined
    ? { error: record.error }
    : { svg: record.svg, sem: canonical(record.sem) };
}

export function comparisonRecord(c) {
  const actual = encode(renderCase(c));
  const key = c.known === "proto-key" ? "__proto__"
    : c.known === "throws-upstream" ? "hasOwnProperty" : undefined;
  if (key === undefined) return actual;
  const value = c.options?.[key];
  if (typeof value !== "string") throw new Error(`known case requires a string ${key} option`);
  const options = { ...c.options };
  delete options[key];
  const control = renderCase({ ...c, options });
  if (control.error !== undefined) throw new Error(`known case control failed: ${control.error}`);
  // Defining an own property avoids the legacy __proto__ setter.
  Object.defineProperty(control.sem.options, key, { value, enumerable: true });
  actual.expected = encode(control);
  return actual;
}
