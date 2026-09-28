// JSON string values may contain U+2028/U+2029; only LF separates records.
export async function* jsonLines(input) {
  input.setEncoding("utf8");
  let pending = "";
  for await (const chunk of input) {
    let start = 0;
    for (let end = chunk.indexOf("\n"); end !== -1; end = chunk.indexOf("\n", start)) {
      pending += chunk.slice(start, end);
      if (pending.trim()) yield JSON.parse(pending);
      pending = "";
      start = end + 1;
    }
    pending += chunk.slice(start);
  }
  if (pending.trim()) yield JSON.parse(pending);
}
