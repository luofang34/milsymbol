//! Random SIDCs drawn from the same catalog the lab's selectors show, kept
//! only when the lab's renderer accepts them as fully valid.

use crate::protocol::{Error, Fields};
use milsymbol::{RenderMode, Renderer, Standard};
use serde_json::Value;

/// Candidates tried before giving up; a few percent of combinations are
/// rejected (for example an amplifier that the symbol set does not use).
const ATTEMPTS: usize = 256;

/// SplitMix64: a small deterministic generator, seeded by the caller.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        let n = u64::try_from(n).unwrap_or(u64::MAX).max(1);
        usize::try_from(self.next() % n).unwrap_or(0)
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        items.get(self.below(items.len()))
    }
}

fn text<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
    row.get(key)?.as_str()
}

/// Codes of one header field, such as `status`, from the catalog.
fn header<'a>(catalog: &'a Value, names: &[&str], width: usize) -> Vec<&'a str> {
    catalog
        .get("headers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| text(row, "field").is_some_and(|f| names.contains(&f)))
        .filter_map(|row| text(row, "code"))
        .filter(|code| code.len() == width)
        .collect()
}

/// A header code: the default (`zero`) with probability `keep` percent,
/// otherwise any listed code.
fn header_code(rng: &mut Rng, codes: &[&str], zero: &str, keep: u64) -> String {
    match rng.pick(codes) {
        Some(code) if !rng.chance(keep) => (*code).to_owned(),
        _ => zero.to_owned(),
    }
}

/// A random SIDC of `version` that `renderer` draws completely: a drawable
/// entity, header fields and modifiers listed for it, accepted by
/// [`Renderer::check_sidc`] and valid when rendered.
pub(crate) fn sidc(
    version: u8,
    standard: Standard,
    mode: RenderMode,
    seed: u64,
) -> Result<String, Error> {
    let catalog = crate::catalog::build(version);
    let rows: Vec<&Value> = catalog
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect();
    let entities: Vec<&Value> = rows
        .iter()
        .copied()
        .filter(|r| text(r, "kind") == Some("entity") && text(r, "scope") == Some("Point"))
        .filter(|r| text(r, "set").is_some_and(|s| s.len() == 2))
        .filter(|r| text(r, "code").is_some_and(|c| c.len() == 6))
        .collect();
    let contexts = header(&catalog, &["context"], 1);
    let identities = header(&catalog, &["standard_identity"], 1);
    let statuses = header(&catalog, &["status"], 1);
    let hqs = header(&catalog, &["hq_tf_dummy", "hq_task_force_dummy"], 1);
    let amplifiers = header(&catalog, &["amplifier", "amplifying_descriptor"], 2);
    let renderer = Renderer::builder().standard(standard).mode(mode).build();
    let mut rng = Rng(seed);
    for _ in 0..ATTEMPTS {
        let Some(entity) = rng.pick(&entities) else {
            break;
        };
        let set = text(entity, "set").unwrap_or("10");
        let modifier = |rng: &mut Rng, kind: &str| {
            let listed: Vec<&str> = rows
                .iter()
                .filter(|r| text(r, "kind") == Some(kind))
                .filter(|r| matches!(text(r, "set"), Some(s) if s == set || s == "common"))
                .filter_map(|r| text(r, "code"))
                .filter(|c| c.len() == 3)
                .collect();
            header_code(rng, &listed, "000", 65)
        };
        let fields = Fields {
            version: format!("{version:02}"),
            context: header_code(&mut rng, &contexts, "0", 75),
            identity: header_code(&mut rng, &identities, "3", 0),
            symbol_set: set.to_owned(),
            status: header_code(&mut rng, &statuses, "0", 70),
            hq: header_code(&mut rng, &hqs, "0", 60),
            amplifier: header_code(&mut rng, &amplifiers, "00", 40),
            entity: text(entity, "code").unwrap_or("000000").to_owned(),
            modifier1: modifier(&mut rng, "modifier1"),
            modifier2: modifier(&mut rng, "modifier2"),
            frame_shape: "0".into(),
            trailing: "0000000".into(),
        };
        let mut code = fields.encode()?;
        if code
            .get(20..)
            .is_some_and(|tail| tail.bytes().all(|b| b == b'0'))
        {
            code.truncate(20);
        }
        let Ok(parsed) = renderer.check_sidc(&code) else {
            continue;
        };
        if renderer
            .symbol(&parsed)
            .render()
            .is_ok_and(|s| s.sidc_validity().is_valid())
        {
            return Ok(code);
        }
    }
    Err(Error::NoRandom { version })
}

#[cfg(test)]
mod tests;
