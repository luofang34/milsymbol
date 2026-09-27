//! Text amplifiers (upstream `textfields.js`).

use super::engagement::or_color;
use super::{PartOutput, SymbolState, s as lit, style_color_value};
use crate::bbox::{BBox, PartialBBox};
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, Str, TextNode};
use crate::js;
use crate::labels::{self, LabelField};
use crate::options::field;
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

mod fields;
mod layout;

/// Fields whose presence enables the information fields (upstream
/// `textFields`); `country` and the engagement fields do not.
const TRIGGER_FIELDS: [&str; 26] = [
    field::QUANTITY,
    field::REINFORCED_REDUCED,
    field::STAFF_COMMENTS,
    field::ADDITIONAL_INFORMATION,
    field::EVALUATION_RATING,
    field::COMBAT_EFFECTIVENESS,
    field::SIGNATURE_EQUIPMENT,
    field::HIGHER_FORMATION,
    field::HOSTILE,
    field::IFF_SIF,
    field::SIGINT,
    field::UNIQUE_DESIGNATION,
    field::TYPE,
    field::DTG,
    field::ALTITUDE_DEPTH,
    field::LOCATION,
    field::SPEED,
    field::SPECIAL_HEADQUARTERS,
    field::PLATFORM_TYPE,
    field::EQUIPMENT_TEARDOWN_TIME,
    field::COMMON_IDENTIFIER,
    field::AUXILIARY_EQUIPMENT_INDICATOR,
    field::HEADQUARTERS_ELEMENT,
    field::INSTALLATION_COMPOSITION,
    field::GUARDED_UNIT,
    field::SPECIAL_DESIGNATOR,
];

/// Values shared by every text node of the part.
pub(super) struct TextStyle {
    pub color: Option<Paint>,
    pub family: String,
    pub size: f64,
}

impl TextStyle {
    /// A plain information-field text node.
    pub fn text(&self, text: &str, x: f64, y: f64, anchor: &'static str) -> Node {
        let mut t = Node::text(x, y, Str::Owned(String::from(text)));
        t.text_anchor = lit(anchor);
        t.font_size = Some(Num::Number(self.size));
        t.font_family = Some(Cow::Owned(self.family.clone()));
        t.style.fill = self.color.clone();
        t.style.stroke = Some(Paint::None);
        Node::Text(t)
    }
}

fn font_color(s: &SymbolState<'_>) -> Option<Paint> {
    let info = style_color_value(&s.options.style.info_color, s.aff());
    let icon = or_color(
        s.color_of(&s.colors.icon_color),
        s.colors.icon_color.get("Friend"),
    );
    or_color(info, icon)
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let st = &s.options.style;
    let ts = TextStyle {
        color: font_color(s),
        family: st.font_family.clone(),
        size: st.info_size,
    };
    let mut gbbox = BBox::default();
    let mut pre = Vec::new();
    let mut post = Vec::new();
    if let Some(label) = label_override(s) {
        post.push(Node::Group(label_texts(s, &ts, &label, &mut gbbox)));
        if st.outline_width > 0.0 {
            pre.push(s.outline(&post)?);
        }
        return Ok(PartOutput::new(pre, post, gbbox));
    }
    let any_text = TRIGGER_FIELDS.iter().any(|k| !s.options.text(k).is_empty());
    if st.info_fields && any_text {
        layout::draw(s, &ts, &mut post, &mut gbbox);
        let width = match st.info_outline_width {
            Some(w) if w > 0.0 => Some(w),
            None if st.outline_width > 0.0 => Some(st.outline_width),
            _ => None,
        };
        if let Some(w) = width {
            let color = if st.info_outline_color.is_empty() {
                s.outline_color()
            } else {
                Some(Paint::Color(Cow::Owned(st.info_outline_color.clone())))
            };
            pre.push(super::outline_list(&post, w, st.stroke_width, &color)?);
        }
    }
    Ok(PartOutput::new(pre, post, gbbox))
}

/// Label override for this SIDC: extension entries first, then built-ins.
fn label_override(s: &SymbolState<'_>) -> Option<Cow<'static, [LabelField]>> {
    let md = s.metadata;
    let (key, user) = if md.number_sidc {
        if !md.control_measure() {
            return None;
        }
        (js::substr(&md.function_id, 0, 6), &s.registry.number_labels)
    } else {
        let sidc = js::JsStr::new(s.sidc);
        (
            alloc::format!(
                "{}-{}-{}",
                sidc.substr(0, 1),
                sidc.substr(2, 1),
                sidc.substr(4, 6)
            ),
            &s.registry.letter_labels,
        )
    };
    if let Some(v) = user.get(&key) {
        return Some(Cow::Owned(v.clone()));
    }
    labels::builtin(md.number_sidc, &key).map(Cow::Borrowed)
}

fn label_texts(
    s: &SymbolState<'_>,
    ts: &TextStyle,
    fields: &[LabelField],
    gbbox: &mut BBox,
) -> Vec<Node> {
    let mut texts = Vec::new();
    for f in fields {
        let Some(value) = s
            .options
            .text
            .get(f.field.as_ref())
            .filter(|v| !v.is_empty())
        else {
            continue;
        };
        let count = if f.is_array { f.labels.len().max(1) } else { 1 };
        for j in 0..count {
            let Some(lbl) = f.labels.get(j) else { continue };
            let (x, y, size) = (
                lbl.x.unwrap_or(f64::NAN),
                lbl.y.unwrap_or(f64::NAN),
                lbl.font_size.unwrap_or(f64::NAN),
            );
            let mut b = PartialBBox {
                y2: lbl.y,
                y1: Some(y - size),
                ..PartialBBox::default()
            };
            let w = || labels::str_width(value, size, 0.0);
            match lbl.anchor.as_deref() {
                Some("start") => {
                    b.x1 = lbl.x;
                    b.x2 = Some(x + w());
                }
                Some("middle") => {
                    b.x1 = Some(x - w() / 2.0);
                    b.x2 = Some(x + w() / 2.0);
                }
                Some("end") => {
                    b.x1 = Some(x - w());
                    b.x2 = lbl.x;
                }
                _ => {}
            }
            gbbox.merge(b);
            texts.push(Node::Text(label_text(ts, value, lbl)));
        }
    }
    texts
}

fn label_text(ts: &TextStyle, value: &str, lbl: &labels::Label) -> TextNode {
    let num = |v: Option<f64>| Num::Number(v.unwrap_or(f64::NAN));
    let mut t = Node::text(0.0, 0.0, Str::Owned(String::from(value)));
    t.x = num(lbl.x);
    t.y = num(lbl.y);
    t.font_family = Some(Cow::Owned(ts.family.clone()));
    t.style.fill = ts.color.clone();
    t.alignment_baseline = lbl.baseline.clone();
    if let Some(f) = &lbl.fill {
        t.style.fill = Some(Paint::Color(f.clone()));
    }
    if let Some(st) = lbl.stroke {
        t.style.stroke = Some(if st {
            Paint::color("true")
        } else {
            Paint::None
        });
    }
    t.text_anchor = lbl.anchor.clone();
    t.font_size = lbl.font_size.map(Num::Number);
    t.font_weight = lbl.weight.clone();
    t
}
