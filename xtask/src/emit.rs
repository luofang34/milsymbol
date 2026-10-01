//! Emits `src/generated/*.rs` from `tools/codegen/out/{tables,misc}.json`.
//!
//! The generated modules contain data only; `src/template.rs` defines their
//! types and instantiation.

use crate::Error;
use serde_json::Value;
use std::path::Path;

mod files;
mod literals;
mod pack;
mod pathcodec;
mod pool;

use literals::rstr;
use pool::Pools;

/// Context variable order mirrored by `template::Var`.
const EXPECT_VARS: [&str; 15] = [
    "std2525",
    "frame",
    "numberSidc",
    "alternateMedal",
    "mono",
    "edition",
    "notpresent",
    "affiliation",
    "geometry",
    "slot_fillColor",
    "slot_frameColor",
    "slot_iconColor",
    "slot_iconFillColor",
    "slot_black",
    "slot_white",
];

/// A byte-sorted `(code, entry)` list.
type Keyed = Vec<(String, usize)>;

/// Everything the output files need, in emission order.
pub(crate) struct Emitted {
    pools: Pools,
    part_list: Vec<String>,
    overrides: Vec<String>,
    /// `[symbol set][icons, m1, m2, bbox]`.
    number: Vec<[Keyed; 4]>,
    letter_icons: Keyed,
    letter_bbox: Keyed,
}

fn read_json(path: &Path) -> Result<Value, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()).into())
}

fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, Error> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing array `{key}`").into())
}

fn key_of(e: &Value) -> &str {
    e.get("key").and_then(Value::as_str).unwrap_or_default()
}

/// Writes the generated modules under `root/src/generated`.
pub(crate) fn run(root: &Path) -> Result<(), Error> {
    let out = root.join("tools/codegen/out");
    let tables = read_json(&out.join("tables.json"))?;
    let misc = read_json(&out.join("misc.json"))?;
    let vars = array(&tables, "vars")?;
    let names: Vec<&str> = vars
        .iter()
        .filter_map(|v| v.get("name")?.as_str())
        .collect();
    if names != EXPECT_VARS {
        return Err("context variable order changed; update template::Var".into());
    }
    let emitted = build(&tables, vars)?;
    files::write_all(root, &tables, &misc, vars, &emitted)
}

fn build(tables: &Value, vars: &[Value]) -> Result<Emitted, Error> {
    let mut pools = Pools::default();
    pools.domain_sizes = vars
        .iter()
        .map(|v| {
            v.get("domain")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        })
        .collect();
    let parts = array(tables, "parts")?;
    // Parts first, so references resolve to part indices.
    let mut part_entries: Vec<(&str, &Value)> = parts
        .iter()
        .filter_map(|e| key_of(e).strip_prefix("P|").map(|n| (n, e)))
        .collect();
    part_entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    pools.part_ids = part_entries
        .iter()
        .enumerate()
        .map(|(i, (n, _))| ((*n).to_string(), i))
        .collect();
    let mut part_list = Vec::new();
    for (name, e) in &part_entries {
        part_list.push(format!("({}, {})", rstr(name), pools.entry_of(e)?));
    }
    let mut overrides = Vec::new();
    for e in parts.iter().filter(|e| key_of(e).starts_with("O|")) {
        overrides.push(override_literal(&mut pools, e)?);
    }
    let mut number: Vec<[Keyed; 4]> = (0..100).map(|_| Default::default()).collect();
    let (mut letter_icons, mut letter_bbox) = (Vec::new(), Vec::new());
    for e in array(tables, "mappings")? {
        let fields: Vec<&str> = key_of(e).split('|').collect();
        match fields.as_slice() {
            ["N", ss, kind, code @ ..] => {
                let kind = ["icons", "m1", "m2", "bbox"].iter().position(|k| k == kind);
                let slot = ss
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| number.get_mut(i))
                    .zip(kind);
                let Some((kinds, kind)) = slot else {
                    return Err(format!("bad mapping key {}", key_of(e)).into());
                };
                let entry = pools.entry_of(e)?;
                if let Some(list) = kinds.get_mut(kind) {
                    list.push((code.join("|"), entry));
                }
            }
            ["L", "icons", code @ ..] => letter_icons.push((code.join("|"), pools.entry_of(e)?)),
            ["L", "bbox", code @ ..] => letter_bbox.push((code.join("|"), pools.entry_of(e)?)),
            _ => return Err(format!("bad mapping key {}", key_of(e)).into()),
        }
    }
    Ok(Emitted {
        pools,
        part_list,
        overrides,
        number,
        letter_icons,
        letter_bbox,
    })
}

/// Keys look like `O|N|15|name` or `O|L|name`.
fn override_literal(pools: &mut Pools, e: &Value) -> Result<String, Error> {
    let fields: Vec<&str> = key_of(e).split('|').collect();
    let (mapping, name) = match fields.as_slice() {
        ["O", "L", name @ ..] => (-1, name.join("|")),
        ["O", _, ss, name @ ..] => (ss.parse::<i32>()?, name.join("|")),
        _ => return Err(format!("bad override key {}", key_of(e)).into()),
    };
    let part = *pools
        .part_ids
        .get(&name)
        .ok_or_else(|| format!("override of unknown part {name}"))?;
    Ok(format!("({mapping}, {part}, {})", pools.entry_of(e)?))
}
