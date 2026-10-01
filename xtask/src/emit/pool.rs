//! Deduplicating pools of template nodes, styles, bounding boxes and entries.
//!
//! Pool indices follow first-use order, so the emitted tables only stay
//! stable if entries are visited in the same order on every run.

use super::literals::{rbool, rdash, rf64, rnum, ropt, rpaint, rs, rstr};
use crate::Error;
use serde_json::Value;
use std::collections::HashMap;

/// A node as stored in the packed `NODES` table.
pub(super) enum Packed {
    Path {
        path: usize,
        style: usize,
    },
    Text {
        text: usize,
        style: usize,
    },
    Group {
        start: usize,
        len: usize,
    },
    Ref(usize),
    /// A full `TNode` literal kept in the wide side table.
    Wide(String),
}

/// One table entry: dependency mask and its mixed-radix row values
/// (`u32::MAX` where upstream defines no value).
pub(super) struct EntryRec {
    pub mask: u32,
    pub vals: Vec<u32>,
}

/// Accumulated literals of the generated tables.
#[derive(Default)]
pub(super) struct Pools {
    pub styles: Vec<String>,
    style_index: HashMap<String, usize>,
    pub texts: Vec<String>,
    text_index: HashMap<String, usize>,
    pub paths: Vec<String>,
    path_index: HashMap<String, usize>,
    pub nodes: Vec<Packed>,
    node_index: HashMap<String, usize>,
    pub kids: Vec<usize>,
    pub bboxes: Vec<String>,
    bbox_index: HashMap<String, usize>,
    pub entries: Vec<EntryRec>,
    entry_index: HashMap<String, usize>,
    /// Part name → index in the byte-sorted part list.
    pub part_ids: HashMap<String, usize>,
    /// Referenced names upstream never defines, in first-use order; their
    /// reference index follows the defined parts.
    pub extra_names: Vec<String>,
    /// Domain size of each context variable.
    pub domain_sizes: Vec<usize>,
}

fn intern(list: &mut Vec<String>, index: &mut HashMap<String, usize>, lit: String) -> usize {
    if let Some(&i) = index.get(&lit) {
        return i;
    }
    list.push(lit.clone());
    index.insert(lit, list.len() - 1);
    list.len() - 1
}

impl Pools {
    fn style_of(&mut self, t: &Value) -> Result<usize, Error> {
        let fields = [
            format!("fill: {}", ropt(t.get("fill"), rpaint)?),
            format!("fill_opacity: {}", ropt(t.get("fillopacity"), rnum)?),
            format!("stroke: {}", ropt(t.get("stroke"), rpaint)?),
            format!("stroke_width: {}", ropt(t.get("strokewidth"), rnum)?),
            format!("dash: {}", ropt(t.get("strokedasharray"), rdash)?),
            format!("line_cap: {}", ropt(t.get("linecap"), rs)?),
            format!(
                "non_scaling_stroke: {}",
                ropt(t.get("non_scaling_stroke"), rnum)?
            ),
            format!("style_fill: {}", ropt(t.get("styleFill"), rbool)?),
            format!("icon: {}", ropt(t.get("icon"), rbool)?),
            format!("clip_path: {}", ropt(t.get("clipPath"), rs)?),
        ];
        let lit = format!("TStyle {{ {} }}", fields.join(", "));
        Ok(intern(&mut self.styles, &mut self.style_index, lit))
    }

    fn kids_of(&mut self, list: &[Value]) -> Result<(usize, usize), Error> {
        let idx = list
            .iter()
            .map(|t| self.node_of(t))
            .collect::<Result<Vec<_>, _>>()?;
        let start = self.kids.len();
        self.kids.extend(&idx);
        Ok((start, idx.len()))
    }

    fn kids_lit(&mut self, list: &[Value]) -> Result<String, Error> {
        let (start, len) = self.kids_of(list)?;
        Ok(format!("Kids {{ start: {start}, len: {len} }}"))
    }

    /// Children of a group-like template (`draw` must be a group).
    fn draw_of(&mut self, t: &Value) -> Result<String, Error> {
        match t.get("draw") {
            None => self.kids_lit(&[]),
            Some(d) => match d.get("group").and_then(Value::as_array) {
                Some(g) => self.kids_lit(g),
                None => Err(format!("non-array draw in {t}").into()),
            },
        }
    }

    fn text_of(&mut self, t: &Value) -> Result<usize, Error> {
        let get = |k: &str| t.get(k);
        let text = get("text")
            .map(super::literals::field_s)
            .transpose()?
            .unwrap_or("");
        let lit = format!(
            "TText {{ x: {}, y: {}, text: {}, size: {}, family: {}, weight: {}, anchor: {}, baseline: {} }}",
            rnum(get("x").unwrap_or(&Value::Null))?,
            rnum(get("y").unwrap_or(&Value::Null))?,
            rstr(text),
            ropt(get("fontsize"), rnum)?,
            ropt(get("fontfamily"), rs)?,
            ropt(get("fontweight"), rs)?,
            ropt(get("textanchor"), rs)?,
            ropt(get("alignmentBaseline"), rs)?,
        );
        Ok(intern(&mut self.texts, &mut self.text_index, lit))
    }

    fn shape_of(&mut self, t: &Value) -> Result<Packed, Error> {
        let style = self.style_of(t)?;
        let num = |k: &str| rnum(t.get(k).unwrap_or(&Value::Null));
        let wide = |lit: String| Packed::Wide(lit);
        Ok(match t.get("type").and_then(Value::as_str) {
            Some("path") => {
                let d = t
                    .get("d")
                    .map(super::literals::field_s)
                    .transpose()?
                    .unwrap_or("");
                let path = intern(&mut self.paths, &mut self.path_index, rstr(d));
                Packed::Path { path, style }
            }
            Some("circle") => wide(format!(
                "TNode::Circle {{ cx: {}, cy: {}, r: {}, style: {style} }}",
                num("cx")?,
                num("cy")?,
                num("r")?
            )),
            Some("text") => Packed::Text {
                text: self.text_of(t)?,
                style,
            },
            Some("translate") => {
                let kids = self.draw_of(t)?;
                wide(format!(
                    "TNode::Translate {{ x: {}, y: {}, kids: {kids}, style: {style} }}",
                    num("x")?,
                    num("y")?
                ))
            }
            Some("rotate") => {
                let kids = self.draw_of(t)?;
                let (d, x, y) = (num("degree")?, num("x")?, num("y")?);
                wide(format!(
                    "TNode::Rotate {{ degree: {d}, x: {x}, y: {y}, kids: {kids}, style: {style} }}"
                ))
            }
            Some("scale") => {
                let kids = self.draw_of(t)?;
                wide(format!(
                    "TNode::Scale {{ factor: {}, kids: {kids}, style: {style} }}",
                    num("factor")?
                ))
            }
            _ => return Err(format!("bad node {t}").into()),
        })
    }

    /// Pool index of a node template.
    pub(super) fn node_of(&mut self, t: &Value) -> Result<usize, Error> {
        let key = t.to_string();
        if let Some(&i) = self.node_index.get(&key) {
            return Ok(i);
        }
        let packed = if t.get("missing").is_some_and(|m| m.as_bool() == Some(true)) {
            Packed::Wide(String::from("TNode::Missing"))
        } else if let Some(s) = t.get("scalar") {
            Packed::Wide(format!("TNode::Scalar({})", rnum(s)?))
        } else if let Some(r) = t.get("ref").and_then(Value::as_str) {
            let id = match self.part_ids.get(r) {
                Some(&id) => id,
                None => {
                    let pos = match self.extra_names.iter().position(|n| n == r) {
                        Some(p) => p,
                        None => {
                            self.extra_names.push(r.to_string());
                            self.extra_names.len() - 1
                        }
                    };
                    self.part_ids.len() + pos
                }
            };
            Packed::Ref(id)
        } else if let Some(g) = t.get("group").and_then(Value::as_array) {
            let (start, len) = self.kids_of(g)?;
            Packed::Group { start, len }
        } else {
            self.shape_of(t)?
        };
        self.nodes.push(packed);
        self.node_index.insert(key, self.nodes.len() - 1);
        Ok(self.nodes.len() - 1)
    }

    fn bbox_of(&mut self, b: &Value) -> Result<usize, Error> {
        let coords = ["x1", "y1", "x2", "y2"].map(|k| ropt(b.get(k), rf64));
        let coords = coords.into_iter().collect::<Result<Vec<_>, _>>()?;
        let lit = format!("[{}]", coords.join(", "));
        Ok(intern(&mut self.bboxes, &mut self.bbox_index, lit))
    }

    /// Row value literal of one serialized template (or `"absent"`).
    fn row_value(&mut self, v: &str) -> Result<u32, Error> {
        if v == "absent" {
            return Ok(u32::MAX);
        }
        let t: Value = serde_json::from_str(v)?;
        let index = match t.get("bbox") {
            Some(Value::Null) => return Ok(u32::MAX),
            Some(b) => self.bbox_of(b)?,
            None => self.node_of(&t)?,
        };
        u32::try_from(index).map_err(|_| "row index overflow".into())
    }

    /// Entry index of a table entry `{key, deps, rows}`.
    pub(super) fn entry_of(&mut self, e: &Value) -> Result<usize, Error> {
        let key = e.get("key").and_then(Value::as_str).unwrap_or("?");
        let deps: Vec<usize> = e
            .get("deps")
            .and_then(Value::as_array)
            .map(|d| {
                d.iter()
                    .filter_map(Value::as_u64)
                    .filter_map(|v| usize::try_from(v).ok())
                    .collect()
            })
            .unwrap_or_default();
        let rows: HashMap<&str, &str> = e
            .get("rows")
            .and_then(Value::as_array)
            .map(|r| {
                r.iter()
                    .filter_map(|pair| Some((pair.get(0)?.as_str()?, pair.get(1)?.as_str()?)))
                    .collect()
            })
            .unwrap_or_default();
        let mut combos: Vec<Vec<usize>> = vec![Vec::new()];
        for &vi in &deps {
            let size = self
                .domain_sizes
                .get(vi)
                .copied()
                .ok_or_else(|| format!("bad dependency {vi} in {key}"))?;
            combos = combos
                .iter()
                .flat_map(|pre| (0..size).map(move |a| [pre.as_slice(), &[a]].concat()))
                .collect();
        }
        let mut vals = Vec::with_capacity(combos.len());
        for combo in &combos {
            let k = combo
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let v = rows
                .get(k.as_str())
                .ok_or_else(|| format!("missing row {k} for {key}"))?;
            vals.push(self.row_value(v)?);
        }
        let mask = deps.iter().fold(0u32, |m, &vi| m | (1 << vi));
        let sig = format!(
            "{mask}|{}",
            vals.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        if let Some(&i) = self.entry_index.get(&sig) {
            return Ok(i);
        }
        self.entries.push(EntryRec { mask, vals });
        self.entry_index.insert(sig, self.entries.len() - 1);
        Ok(self.entries.len() - 1)
    }
}
