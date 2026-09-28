//! The icon (upstream `icon.js`).

use super::{PartOutput, SymbolState};
use crate::bbox::{BBox, PartialBBox};
use crate::domain::{Affiliation, Edition};
use crate::error::RenderError;
use crate::generated::{tables, vars};
use crate::ir::{Node, Paint, PathData, PathNode, Style};
use crate::js;
use crate::metadata::Field;
use crate::registry::{IconKey, IconPartContext};
use crate::template::{self, IconContext, Resolver, UserParts, Var};
use alloc::borrow::Cow;
use alloc::vec::Vec;

const UNDEFINED_ICON: &str = "m 94.8206,78.1372 c -0.4542,6.8983 0.6532,14.323 5.3424,19.6985 4.509,5.6933 11.309,9.3573 14.98,15.7283 3.164,6.353 -0.09,14.245 -5.903,17.822 -7.268,4.817 -18.6219,2.785 -22.7328,-5.249 -1.5511,-2.796 -2.3828,-5.931 -2.8815,-9.071 -3.5048,0.416 -7.0093,0.835 -10.5142,1.252 0.8239,8.555 5.2263,17.287 13.2544,21.111 7.8232,3.736 17.1891,3.783 25.3291,1.052 8.846,-3.103 15.737,-11.958 15.171,-21.537 0.05,-6.951 -4.272,-12.85 -9.134,-17.403 -4.526,-4.6949 -11.048,-8.3862 -12.401,-15.2748 -1.215,-2.3639 -0.889,-8.129 -0.889,-8.129 z m -0.6253,-20.5177 0,11.6509 11.6527,0 0,-11.6509 z";

/// Land equipment entities (symbol set 15) whose icon is not scaled for modifiers.
const UNSCALED_EQUIPMENT: [f64; 25] = [
    130100.0, 170000.0, 170400.0, 170600.0, 170700.0, 170800.0, 170900.0, 171100.0, 200200.0,
    200300.0, 200600.0, 200700.0, 200800.0, 200900.0, 201100.0, 201301.0, 201302.0, 201400.0,
    210100.0, 210200.0, 210300.0, 210400.0, 210500.0, 230200.0, 250000.0,
];

const SEA_MINE_EXERCISE: [&str; 5] = ["WMGX--", "WMMX--", "WMFX--", "WMX---", "WMSX--"];

/// Icon parts as upstream passes them to mapping functions (extension
/// parts first, then the generated tables), and the extensions to ask for
/// icons.
struct Parts<'a> {
    resolver: Resolver<'a>,
}

impl Parts<'_> {
    fn part(&self, name: &str) -> Option<Node> {
        self.resolver.part(name)
    }

    /// The icon for `key` from the latest extension defining it.
    fn user_icon(&self, key: IconKey<'_>) -> Option<Node> {
        let UserParts { extensions, ctx } = self.resolver.user;
        let ctx = ctx?;
        extensions
            .iter()
            .rev()
            .find_map(|e| e.icon(ctx, key, &self.resolver))
    }

    /// Icon bounds for `key` from the latest extension defining them.
    fn user_bbox(&self, key: IconKey<'_>) -> Option<BBox> {
        let UserParts { extensions, ctx } = self.resolver.user;
        let ctx = ctx?;
        extensions
            .iter()
            .rev()
            .find_map(|e| e.icon_bbox(ctx, key))
            .map(PartialBBox::complete)
    }
}

/// Affiliation the icon parts are built for (`metadata.affiliation || "Friend"`).
fn part_affiliation(s: &SymbolState<'_>) -> Field<Affiliation> {
    match s.metadata.affiliation {
        Field::Missing | Field::Empty => Field::Known(Affiliation::Friend),
        a => a,
    }
}

fn context(s: &SymbolState<'_>, part_aff: Field<Affiliation>) -> IconContext {
    let (md, st) = (s.metadata, &s.options.style);
    let mut c = IconContext::new();
    c.set_bool(Var::Std2525, md.std2525);
    c.set_bool(Var::Frame, md.frame);
    c.set_bool(Var::NumberSidc, md.number_sidc);
    c.set_bool(Var::AlternateMedal, st.alternate_medal);
    c.set(Var::Mono, u8::from(!st.mono_color.is_empty()));
    c.set(
        Var::Edition,
        match md.edition() {
            Some(Edition::D) => 1,
            Some(Edition::E) => 2,
            _ => 0,
        },
    );
    let np = &md.notpresent;
    let dashes = &s.config.dash_arrays;
    c.set(
        Var::NotPresent,
        if np.is_empty() {
            0
        } else if *np == dashes.pending {
            1
        } else if *np == dashes.anticipated {
            2
        } else {
            1
        },
    );
    let aff_index = vars::AFFILIATIONS
        .iter()
        .position(|&a| Some(a) == part_aff.as_str())
        .unwrap_or(0);
    c.set(Var::Affiliation, u8::try_from(aff_index).unwrap_or(0));
    let geom = md.base_geometry.unwrap_or("none");
    let geom_index = vars::GEOMETRIES
        .iter()
        .position(|&g| g == geom)
        .unwrap_or(vars::GEOMETRIES.len() - 1);
    c.set(Var::Geometry, u8::try_from(geom_index).unwrap_or(0));
    let key = part_aff.known().unwrap_or(Affiliation::Friend);
    let slot = |m: &crate::color::ColorMode| matches!(m.for_affiliation(key), Some(Paint::Color(c)) if !c.is_empty());
    let cs = s.colors;
    for (var, mode) in [
        (Var::SlotFill, &cs.fill_color),
        (Var::SlotFrame, &cs.frame_color),
        (Var::SlotIcon, &cs.icon_color),
        (Var::SlotIconFill, &cs.icon_fill_color),
        (Var::SlotBlack, &cs.black),
        (Var::SlotWhite, &cs.white),
    ] {
        c.set_bool(var, slot(mode));
    }
    c
}

fn undefined_icon(s: &SymbolState<'_>) -> Node {
    let style = Style {
        stroke: Some(Paint::None),
        fill: s.color_of(&s.colors.icon_color),
        ..Style::default()
    };
    Node::Group(alloc::vec![Node::Path(PathNode {
        d: PathData::new(UNDEFINED_ICON),
        style
    })])
}

fn bbox_from(b: [Option<f64>; 4]) -> BBox {
    let [x1, y1, x2, y2] = b;
    PartialBBox { x1, y1, x2, y2 }.complete()
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let (post, gbbox, invalid) = if s.options.style.icon {
        icon(s)?
    } else {
        (Vec::new(), default_icon_bbox(), false)
    };
    let (md, st) = (s.metadata, &s.options.style);
    let mut pre = Vec::new();
    if (!(st.frame && md.fill) || !st.mono_color.is_empty() || md.control_measure())
        && st.outline_width > 0.0
    {
        pre.push(s.outline(&post)?);
    }
    let mut out = PartOutput::new(pre, post, gbbox);
    out.invalid_icon = invalid;
    Ok(out)
}

/// Whether the SIDC's icon exists, looked up as if icons were drawn.
pub(super) fn known(s: &SymbolState<'_>) -> bool {
    icon(s).is_ok_and(|(nodes, _, invalid)| !invalid && !crate::ir::contains_missing(&nodes))
}

fn default_icon_bbox() -> BBox {
    BBox {
        x1: 50.0,
        y1: 50.0,
        x2: 150.0,
        y2: 150.0,
    }
}

/// The icon's instructions and bounds, and whether it was not found.
fn icon(s: &SymbolState<'_>) -> Result<(Vec<Node>, BBox, bool), RenderError> {
    let mut post = Vec::new();
    let mut gbbox = default_icon_bbox();
    let part_aff = part_affiliation(s);
    let symbol_set = js::substr(s.sidc, 4, 2);
    let mapping = if s.metadata.number_sidc {
        symbol_set
            .parse::<i16>()
            .ok()
            .filter(|_| symbol_set.len() == 2)
            .unwrap_or(i16::MIN)
    } else {
        -1
    };
    let dashes = &s.config.dash_arrays;
    // Extensions get typed metadata; without any, none is computed.
    let typed =
        (!s.registry.icons.is_empty()).then(|| crate::domain::Metadata::from_internal(s.metadata));
    let js_metadata = typed.as_ref().map(|_| s.js_metadata());
    let ctx = typed
        .as_ref()
        .zip(js_metadata.as_ref())
        .map(|(typed, js_metadata)| IconPartContext {
            metadata: typed,
            js_metadata,
            colors: s.colors,
            mono_color: &s.options.style.mono_color,
            alternate_medal: s.options.style.alternate_medal,
        });
    let parts = Parts {
        resolver: Resolver {
            colors: s.colors,
            part_affiliation: part_aff.known(),
            mono_color: &s.options.style.mono_color,
            dash_pending: &dashes.pending,
            dash_anticipated: &dashes.anticipated,
            mapping,
            ctx: context(s, part_aff),
            user: UserParts {
                extensions: &s.registry.icons,
                ctx: ctx.as_ref(),
            },
        },
    };
    let invalid = if s.metadata.number_sidc {
        number_icon(s, &parts, &symbol_set, &mut post, &mut gbbox)?
    } else {
        letter_icon(s, &parts, &mut post, &mut gbbox)
    };
    Ok((post, gbbox, invalid))
}

/// Number-table lookup: extension entries first, then generated tables.
fn number_entry(parts: &Parts<'_>, ss: &str, kind: usize, code: &str) -> Option<Node> {
    let key = match kind {
        0 => IconKey::Entity {
            symbol_set: ss,
            entity: code,
        },
        1 => IconKey::Modifier1 {
            symbol_set: ss,
            code,
        },
        _ => IconKey::Modifier2 {
            symbol_set: ss,
            code,
        },
    };
    if let Some(n) = parts.user_icon(key) {
        return Some(n);
    }
    let table = ss
        .parse::<usize>()
        .ok()
        .filter(|_| ss.len() == 2)
        .and_then(|i| tables::NUMBER.get(i))?;
    let entry = template::find(table.get(kind)?, code)?;
    parts.resolver.value(entry)
}

/// Upstream treats a mapping value of `undefined` like a missing key.
fn defined(n: Option<Node>) -> Option<Node> {
    n.filter(|n| !matches!(n, Node::Missing))
}

fn number_icon(
    s: &SymbolState<'_>,
    parts: &Parts<'_>,
    ss: &str,
    post: &mut Vec<Node>,
    gbbox: &mut BBox,
) -> Result<bool, RenderError> {
    let md = s.metadata;
    let fid = js::JsStr::new(&md.function_id);
    let fid6 = fid.substr(0, 6);
    let mut invalid = false;
    let mut main = defined(number_entry(parts, ss, 0, &fid6));
    if main.is_none() && js::string_to_number(&fid.substr(4, 2)) >= 95.0 {
        main = defined(number_entry(parts, ss, 0, &(fid.substr(0, 4) + "00")));
    }
    let m1_code = md.flags.modifier1.as_deref().unwrap_or("");
    let m2_code = md.flags.modifier2.as_deref().unwrap_or("");
    match main {
        None => {
            if !(fid6 == "000000" || fid6.is_empty()) {
                post.push(undefined_icon(s));
                invalid = true;
            }
        }
        Some(icon) => post.push(scale_for_modifiers(ss, &fid6, m1_code, m2_code, icon)?),
    }
    let special = parts
        .user_bbox(IconKey::Entity {
            symbol_set: ss,
            entity: &fid6,
        })
        .or_else(|| {
            let table = ss
                .parse::<usize>()
                .ok()
                .filter(|_| ss.len() == 2)
                .and_then(|i| tables::NUMBER.get(i))?;
            let entry = template::find(table.get(3)?, &fid6)?;
            parts.resolver.bbox(entry).map(bbox_from)
        });
    if let Some(b) = special {
        *gbbox = b;
    }
    let hq_part = match &*fid.substr(4, 2) {
        "95" => Some("GR.IC.FF.HEADQUARTERS OR HEADQUARTERS ELEMENT"),
        "96" => Some("GR.IC.FF.DIVISION AND BELOW SUPPORT"),
        "97" => Some("GR.IC.FF.CORPS SUPPORT"),
        "98" => Some("GR.IC.FF.THEATRE SUPPORT"),
        _ => None,
    };
    if let Some(name) = hq_part {
        post.push(parts.part(name).unwrap_or(Node::Missing));
    }
    for (kind, code, fid_code) in [
        (1, m1_code, fid.substr(6, 2)),
        (2, m2_code, fid.substr(8, 2)),
    ] {
        let key = if code.starts_with('0') {
            if fid_code == "00" {
                continue;
            }
            fid_code
        } else {
            Cow::Borrowed(code)
        };
        match defined(number_entry(parts, ss, kind, &key)) {
            Some(n) => post.push(n),
            None => invalid = true,
        }
    }
    Ok(invalid)
}

/// Scaling of land equipment and dismounted weapons so sector modifiers fit.
fn scale_for_modifiers(
    ss: &str,
    fid6: &str,
    m1: &str,
    m2: &str,
    icon: Node,
) -> Result<Node, RenderError> {
    let main_sidc = js::string_to_number(fid6);
    let scaled = (ss == "27" && (110301.0..=110403.0).contains(&main_sidc))
        || (ss == "15" && !UNSCALED_EQUIPMENT.contains(&main_sidc));
    if !scaled {
        return Ok(icon);
    }
    let (has1, has2) = (m1 != "000", m2 != "000");
    let node = match (has1, has2) {
        (true, true) => super::scale(0.45, icon, true)?,
        (false, true) => super::translate(0.0, -10.0, super::scale(0.7, icon, true)?),
        (true, false) => super::translate(0.0, 10.0, super::scale(0.7, icon, true)?),
        (false, false) => super::scale(1.0, icon, true)?,
    };
    Ok(Node::Group(alloc::vec![node]))
}

fn letter_icon(
    s: &SymbolState<'_>,
    parts: &Parts<'_>,
    post: &mut Vec<Node>,
    gbbox: &mut BBox,
) -> bool {
    let md = s.metadata;
    let mut invalid = false;
    if SEA_MINE_EXERCISE.contains(&md.function_id.as_str()) {
        gbbox.y1 = 10.0;
        if md.affiliation.known() != Some(Affiliation::Unknown) {
            gbbox.x2 = md.geometry_bbox().x2 + 20.0;
        }
    }
    let sidc = js::JsStr::new(s.sidc);
    let generic = alloc::format!(
        "{}-{}-{}",
        sidc.substr(0, 1),
        sidc.substr(2, 1),
        sidc.substr(4, 6)
    );
    let key = IconKey::Letter { generic: &generic };
    let found = parts.user_icon(key).map(Some).or_else(|| {
        let entry = template::find(tables::LETTER_ICONS, &generic)?;
        Some(parts.resolver.value(entry))
    });
    match found {
        Some(Some(icon)) => post.push(icon),
        _ => {
            let f = sidc.substr(4, 6);
            if !(f == "------" || f.is_empty()) {
                post.push(undefined_icon(s));
                invalid = true;
            }
        }
    }
    let special = parts.user_bbox(key).or_else(|| {
        let entry = template::find(tables::LETTER_BBOX, &generic)?;
        parts.resolver.bbox(entry).map(bbox_from)
    });
    if let Some(b) = special {
        *gbbox = b;
    }
    invalid
}
