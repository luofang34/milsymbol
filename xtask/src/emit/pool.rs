//! Deduplicating pools of template nodes, styles, bounding boxes and entries.
//!
//! Pool indices follow first-use order, so the emitted tables only stay
//! stable if entries are visited in the same order on every run.

use super::literals::{rbool, rdash, rf64, rnum, ropt, rpaint, rs, rstr};
use crate::Error;
use serde_json::Value;
use std::collections::HashMap;

/// Accumulated literals of the generated tables.
#[derive(Default)]
pub struct Pools {
    pub styles: Vec<String>,
    style_index: HashMap<String, usize>,
    pub texts: Vec<String>,
    pub nodes: Vec<String>,
    node_index: HashMap<String, usize>,
    pub kids: Vec<usize>,
    pub bboxes: Vec<String>,
    bbox_index: HashMap<String, usize>,
    pub entries: Vec<String>,
    entry_index: HashMap<String, usize>,
    pub rows: Vec<String>,
    /// Part name → index in the byte-sorted part list.
    pub part_ids: HashMap<String, usize>,
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

    fn kids_of(&mut self, list: &[Value]) -> Result<String, Error> {
        let idx = list
            .iter()
            .map(|t| self.node_of(t))
            .collect::<Result<Vec<_>, _>>()?;
        let start = self.kids.len();
        self.kids.extend(&idx);
        Ok(format!("Kids {{ start: {start}, len: {} }}", idx.len()))
    }

    /// Children of a group-like template (`draw` must be a group).
    fn draw_of(&mut self, t: &Value) -> Result<String, Error> {
        match t.get("draw") {
            None => self.kids_of(&[]),
            Some(d) => match d.get("group").and_then(Value::as_array) {
                Some(g) => self.kids_of(g),
                None => Err(format!("non-array draw in {t}").into()),
            },
        }
    }

    fn text_of(&mut self, t: &Value) -> Result<String, Error> {
        let get = |k: &str| t.get(k);
        let text = get("text")
            .map(super::literals::field_s)
            .transpose()?
            .unwrap_or("");
        self.texts.push(format!(
            "TText {{ x: {}, y: {}, text: {}, size: {}, family: {}, weight: {}, anchor: {}, baseline: {} }}",
            rnum(get("x").unwrap_or(&Value::Null))?,
            rnum(get("y").unwrap_or(&Value::Null))?,
            rstr(text),
            ropt(get("fontsize"), rnum)?,
            ropt(get("fontfamily"), rs)?,
            ropt(get("fontweight"), rs)?,
            ropt(get("textanchor"), rs)?,
            ropt(get("alignmentBaseline"), rs)?,
        ));
        Ok((self.texts.len() - 1).to_string())
    }

    fn shape_of(&mut self, t: &Value) -> Result<String, Error> {
        let style = self.style_of(t)?;
        let num = |k: &str| rnum(t.get(k).unwrap_or(&Value::Null));
        Ok(match t.get("type").and_then(Value::as_str) {
            Some("path") => {
                let d = t
                    .get("d")
                    .map(super::literals::field_s)
                    .transpose()?
                    .unwrap_or("");
                format!("TNode::Path {{ d: {}, style: {style} }}", rstr(d))
            }
            Some("circle") => {
                format!(
                    "TNode::Circle {{ cx: {}, cy: {}, r: {}, style: {style} }}",
                    num("cx")?,
                    num("cy")?,
                    num("r")?
                )
            }
            Some("text") => format!(
                "TNode::Text {{ text: {}, style: {style} }}",
                self.text_of(t)?
            ),
            Some("translate") => {
                let kids = self.draw_of(t)?;
                format!(
                    "TNode::Translate {{ x: {}, y: {}, kids: {kids}, style: {style} }}",
                    num("x")?,
                    num("y")?
                )
            }
            Some("rotate") => {
                let kids = self.draw_of(t)?;
                let (d, x, y) = (num("degree")?, num("x")?, num("y")?);
                format!(
                    "TNode::Rotate {{ degree: {d}, x: {x}, y: {y}, kids: {kids}, style: {style} }}"
                )
            }
            Some("scale") => {
                let kids = self.draw_of(t)?;
                format!(
                    "TNode::Scale {{ factor: {}, kids: {kids}, style: {style} }}",
                    num("factor")?
                )
            }
            _ => return Err(format!("bad node {t}").into()),
        })
    }

    /// Pool index of a node template.
    pub fn node_of(&mut self, t: &Value) -> Result<usize, Error> {
        let key = t.to_string();
        if let Some(&i) = self.node_index.get(&key) {
            return Ok(i);
        }
        let lit = if t.get("missing").is_some_and(|m| m.as_bool() == Some(true)) {
            String::from("TNode::Missing")
        } else if let Some(s) = t.get("scalar") {
            format!("TNode::Scalar({})", rnum(s)?)
        } else if let Some(r) = t.get("ref").and_then(Value::as_str) {
            match self.part_ids.get(r) {
                Some(id) => format!("TNode::Ref({id})"),
                None => String::from("TNode::Ref(UNKNOWN_PART)"),
            }
        } else if let Some(g) = t.get("group").and_then(Value::as_array) {
            format!("TNode::Group({})", self.kids_of(g)?)
        } else {
            self.shape_of(t)?
        };
        self.nodes.push(lit);
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
    fn row_value(&mut self, v: &str) -> Result<String, Error> {
        if v == "absent" {
            return Ok(String::from("ABSENT"));
        }
        let t: Value = serde_json::from_str(v)?;
        Ok(match t.get("bbox") {
            Some(Value::Null) => String::from("ABSENT"),
            Some(b) => self.bbox_of(b)?.to_string(),
            None => self.node_of(&t)?.to_string(),
        })
    }

    /// Entry index of a table entry `{key, deps, rows}`.
    pub fn entry_of(&mut self, e: &Value) -> Result<usize, Error> {
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
        let sig = format!("{mask}|{}", vals.join(","));
        if let Some(&i) = self.entry_index.get(&sig) {
            return Ok(i);
        }
        let start = self.rows.len();
        self.rows.extend(vals);
        self.entries
            .push(format!("Entry {{ deps: 0b{mask:b}, start: {start} }}"));
        self.entry_index.insert(sig, self.entries.len() - 1);
        Ok(self.entries.len() - 1)
    }
}
