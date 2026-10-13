//! The icon (upstream `icon.js`).

use super::{PartOutput, SymbolState};
use crate::bbox::{BBox, PartialBBox};
use crate::error::RenderError;
use crate::generated::tables;
use crate::ir::{Node, Paint, PathData, PathNode, Style};
use crate::js;
use crate::node_list::NodeAcc;
use crate::registry::{IconKey, IconPartContext};
use crate::template::{self, Resolver, UserParts};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use app6e2::Kind;

mod app6e2;

mod context;
mod letter;

const UNDEFINED_ICON: &str = "m 94.8206,78.1372 c -0.4542,6.8983 0.6532,14.323 5.3424,19.6985 4.509,5.6933 11.309,9.3573 14.98,15.7283 3.164,6.353 -0.09,14.245 -5.903,17.822 -7.268,4.817 -18.6219,2.785 -22.7328,-5.249 -1.5511,-2.796 -2.3828,-5.931 -2.8815,-9.071 -3.5048,0.416 -7.0093,0.835 -10.5142,1.252 0.8239,8.555 5.2263,17.287 13.2544,21.111 7.8232,3.736 17.1891,3.783 25.3291,1.052 8.846,-3.103 15.737,-11.958 15.171,-21.537 0.05,-6.951 -4.272,-12.85 -9.134,-17.403 -4.526,-4.6949 -11.048,-8.3862 -12.401,-15.2748 -1.215,-2.3639 -0.889,-8.129 -0.889,-8.129 z m -0.6253,-20.5177 0,11.6509 11.6527,0 0,-11.6509 z";

/// Land equipment entities (symbol set 15) whose icon is not scaled for modifiers.
const UNSCALED_EQUIPMENT: [f64; 25] = [
    130100.0, 170000.0, 170400.0, 170600.0, 170700.0, 170800.0, 170900.0, 171100.0, 200200.0,
    200300.0, 200600.0, 200700.0, 200800.0, 200900.0, 201100.0, 201301.0, 201302.0, 201400.0,
    210100.0, 210200.0, 210300.0, 210400.0, 210500.0, 230200.0, 250000.0,
];

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

fn undefined_icon_path(s: &SymbolState<'_>) -> Node {
    let style = Style {
        stroke: Some(Paint::None),
        fill: s.color_of(&s.colors.icon_color),
        ..Style::default()
    };
    Node::Path(PathNode {
        d: PathData::new(UNDEFINED_ICON),
        style,
    })
}

fn undefined_icon(s: &SymbolState<'_>) -> Node {
    Node::Group(alloc::vec![undefined_icon_path(s)])
}

fn bbox_from(b: [Option<f64>; 4]) -> BBox {
    let [x1, y1, x2, y2] = b;
    PartialBBox { x1, y1, x2, y2 }.complete()
}

fn needs_icon_outline(s: &SymbolState<'_>) -> bool {
    let (md, st) = (s.metadata, &s.options.style);
    (!(st.frame && md.fill) || st.mono_color.is_some() || md.control_measure())
        && st.outline_width > 0.0
}

const DEFAULT_ICON_BBOX: BBox = BBox {
    x1: 50.0,
    y1: 50.0,
    x2: 150.0,
    y2: 150.0,
};

enum IconDest<'a> {
    Direct {
        acc: &'a mut NodeAcc,
        bbox: &'a mut BBox,
        valid_icon: &'a mut bool,
    },
    Output(&'a mut PartOutput),
}

pub(super) fn render_direct(
    s: &SymbolState<'_>,
    acc: &mut NodeAcc,
    bbox: &mut BBox,
    valid_icon: &mut bool,
) -> Result<(), RenderError> {
    let needs_outline = needs_icon_outline(s);
    if !s.options.style.icon {
        if needs_outline {
            bbox.merge(DEFAULT_ICON_BBOX.into());
        }
        return Ok(());
    }
    icon_into(
        s,
        needs_outline,
        IconDest::Direct {
            acc,
            bbox,
            valid_icon,
        },
    )
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let needs_outline = needs_icon_outline(s);
    let mut out = PartOutput::new(Vec::new(), Vec::new(), DEFAULT_ICON_BBOX);
    if !s.options.style.icon {
        if needs_outline {
            out.pre.push(s.outline(&[])?);
        }
        return Ok(out);
    }
    icon_into(s, needs_outline, IconDest::Output(&mut out))?;
    Ok(out)
}

/// Whether the SIDC's icon exists, looked up as if icons were drawn.
pub(super) fn known(s: &SymbolState<'_>) -> bool {
    let (mut acc, mut bbox, mut valid) = (NodeAcc::default(), BBox::default(), true);
    icon_into(
        s,
        false,
        IconDest::Direct {
            acc: &mut acc,
            bbox: &mut bbox,
            valid_icon: &mut valid,
        },
    )
    .is_ok_and(|()| valid && !acc.contains_missing())
}

fn finish_slow(
    s: &SymbolState<'_>,
    needs_outline: bool,
    post: Vec<Node>,
    gbbox: BBox,
    invalid: bool,
    dest: IconDest<'_>,
) -> Result<(), RenderError> {
    let mut pre = Vec::new();
    if needs_outline {
        pre.push(s.outline(&post)?);
    }
    match dest {
        IconDest::Direct {
            acc,
            bbox,
            valid_icon,
        } => {
            *valid_icon &= !invalid;
            super::pipeline::merge_vecs(pre, post, gbbox.into(), acc, bbox)?;
        }
        IconDest::Output(out) => {
            out.pre = pre;
            out.post = post;
            out.bbox = gbbox.into();
            out.invalid_icon = invalid;
        }
    }
    Ok(())
}

/// Populates `dest` with the icon's instructions, bounds, and validity.
fn icon_into(
    s: &SymbolState<'_>,
    needs_outline: bool,
    dest: IconDest<'_>,
) -> Result<(), RenderError> {
    let mut gbbox = DEFAULT_ICON_BBOX;
    let part_aff = context::part_affiliation(s);
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
            mono_color: s
                .options
                .style
                .mono_color
                .as_ref()
                .map(crate::options::Color::as_str),
            alternate_medal: s.options.style.alternate_medal,
        });
    let parts = Parts {
        resolver: Resolver {
            colors: s.colors,
            part_affiliation: part_aff.known(),
            mono_color: s.options.style.mono_color.as_ref(),
            icon_font_family: s
                .options
                .style
                .icon_text_uses_font_family
                .then_some(&s.options.style.font_family),
            dash_pending: &dashes.pending,
            dash_anticipated: &dashes.anticipated,
            mapping,
            ctx: context::context(s, part_aff),
            user: UserParts {
                extensions: &s.registry.icons,
                ctx: ctx.as_ref(),
            },
        },
    };
    if s.metadata.number_sidc {
        number_icon_into(s, &parts, &symbol_set, needs_outline, dest, &mut gbbox)
    } else {
        letter::letter_icon_into(s, &parts, needs_outline, dest, &mut gbbox)
    }
}

fn number_table(ss: &str) -> Option<&'static [&'static [(&'static str, u32)]; 4]> {
    ss.parse::<usize>()
        .ok()
        .filter(|_| ss.len() == 2)
        .and_then(|i| tables::NUMBER.get(i))
}

/// Number-table lookup: extension entries first, then, if `builtin`, the
/// generated tables.
fn number_entry(
    parts: &Parts<'_>,
    ss: &str,
    kind: usize,
    code: &str,
    builtin: bool,
) -> Option<Node> {
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
    if !builtin {
        return None;
    }
    let table = number_table(ss)?;
    let entry = template::find(table.get(kind)?, code)?;
    parts.resolver.value(entry)
}

/// Upstream treats a mapping value of `undefined` like a missing key.
fn defined(n: Option<Node>) -> Option<Node> {
    n.filter(|n| !matches!(n, Node::Missing))
}

fn sector_key<'a>(code: &'a str, fid_code: Cow<'a, str>) -> Option<Cow<'a, str>> {
    if code.starts_with('0') {
        (fid_code != "00").then_some(fid_code)
    } else {
        Some(Cow::Borrowed(code))
    }
}

fn number_icon_into(
    s: &SymbolState<'_>,
    parts: &Parts<'_>,
    ss: &str,
    needs_outline: bool,
    mut dest: IconDest<'_>,
    gbbox: &mut BBox,
) -> Result<(), RenderError> {
    let md = s.metadata;
    let fid = js::JsStr::new(&md.function_id);
    let (fid6, fid_hq) = (fid.substr(0, 6), fid.substr(4, 2));
    let hq_part = (match &*fid_hq {
        "95" => Some("GR.IC.FF.HEADQUARTERS OR HEADQUARTERS ELEMENT"),
        "96" => Some("GR.IC.FF.DIVISION AND BELOW SUPPORT"),
        "97" => Some("GR.IC.FF.CORPS SUPPORT"),
        "98" => Some("GR.IC.FF.THEATRE SUPPORT"),
        _ => None,
    })
    .filter(|_| !md.app6e2);
    let (m1_code, m2_code) = (
        md.flags.modifier1.as_deref().unwrap_or(""),
        md.flags.modifier2.as_deref().unwrap_or(""),
    );
    let m1_key = sector_key(m1_code, fid.substr(6, 2));
    let m2_key = sector_key(m2_code, fid.substr(8, 2));
    let builtin = !md.app6e2 || app6e2::draws(ss, Kind::Entity, &fid6);
    let table = number_table(ss).filter(|_| builtin);
    if let Some(b) = parts
        .user_bbox(IconKey::Entity {
            symbol_set: ss,
            entity: &fid6,
        })
        .or_else(|| {
            let entry = template::find(table?.get(3)?, &fid6)?;
            parts.resolver.bbox(entry).map(bbox_from)
        })
    {
        *gbbox = b;
    }
    let main_icon_only = parts.resolver.user.ctx.is_none()
        && !needs_outline
        && !is_scaled_icon(ss, &fid6)
        && hq_part.is_none()
        && m1_key.is_none()
        && m2_key.is_none();
    if main_icon_only {
        if let IconDest::Direct {
            acc,
            bbox,
            valid_icon,
        } = &mut dest
        {
            let find_entry = |code: &str| {
                table
                    .and_then(|t| t.first())
                    .and_then(|t| template::find(t, code))
            };
            let mut wrote = match find_entry(&fid6) {
                Some(e) => parts.resolver.value_unwrapped(e, true, acc)?,
                None => false,
            };
            if !wrote && builtin && js::string_to_number(&fid_hq) >= 95.0 {
                if let Some(e) = find_entry(&(fid.substr(0, 4) + "00")) {
                    wrote = parts.resolver.value_unwrapped(e, true, acc)?;
                }
            }
            if wrote {
                bbox.merge((*gbbox).into());
            } else if !(fid6 == "000000" || fid6.is_empty()) {
                acc.push(undefined_icon_path(s));
                **valid_icon = false;
                bbox.merge((*gbbox).into());
            }
            return Ok(());
        }
    }
    let (post, invalid) = number_icon_slow(
        s,
        parts,
        ss,
        (&fid, &fid6, &fid_hq),
        hq_part,
        (m1_code, m2_code, m1_key, m2_key),
    )?;
    finish_slow(s, needs_outline, post, *gbbox, invalid, dest)
}

type SectorArgs<'a> = (&'a str, &'a str, Option<Cow<'a, str>>, Option<Cow<'a, str>>);

fn number_icon_slow(
    s: &SymbolState<'_>,
    parts: &Parts<'_>,
    ss: &str,
    (fid, fid6, fid_hq): (&js::JsStr<'_>, &str, &str),
    hq_part: Option<&str>,
    (m1_code, m2_code, m1_key, m2_key): SectorArgs<'_>,
) -> Result<(Vec<Node>, bool), RenderError> {
    let mut post = Vec::new();
    let mut invalid = false;
    let builtin = !s.metadata.app6e2 || app6e2::draws(ss, Kind::Entity, fid6);
    let mut main = defined(number_entry(parts, ss, 0, fid6, builtin));
    if main.is_none() && builtin && js::string_to_number(fid_hq) >= 95.0 {
        main = defined(number_entry(parts, ss, 0, &(fid.substr(0, 4) + "00"), true));
    }
    match main {
        None if !(fid6 == "000000" || fid6.is_empty()) => {
            post.push(undefined_icon(s));
            invalid = true;
        }
        None => {}
        Some(icon) => post.push(scale_for_modifiers(ss, fid6, m1_code, m2_code, icon)?),
    }
    if let Some(name) = hq_part {
        post.push(parts.part(name).unwrap_or(Node::Missing));
    }
    for (kind, code, key) in [(1, m1_code, m1_key), (2, m2_code, m2_key)] {
        let Some(key) = key else { continue };
        let sector = if kind == 1 {
            Kind::Modifier1
        } else {
            Kind::Modifier2
        };
        let builtin = !s.metadata.app6e2 || app6e2::draws(ss, sector, code);
        match defined(number_entry(parts, ss, kind, &key, builtin)) {
            Some(n) => post.push(n),
            None => invalid = true,
        }
    }
    Ok((post, invalid))
}

fn is_scaled_icon(ss: &str, fid6: &str) -> bool {
    if ss != "15" && ss != "27" {
        return false;
    }
    let main_sidc = js::string_to_number(fid6);
    (ss == "27" && (110301.0..=110403.0).contains(&main_sidc))
        || (ss == "15" && !UNSCALED_EQUIPMENT.contains(&main_sidc))
}

/// Scaling of land equipment and dismounted weapons so sector modifiers fit.
fn scale_for_modifiers(
    ss: &str,
    fid6: &str,
    m1: &str,
    m2: &str,
    icon: Node,
) -> Result<Node, RenderError> {
    if !is_scaled_icon(ss, fid6) {
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
