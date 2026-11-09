//! Legacy letter-SIDC icon lookup (upstream `icon.js` letter branch).

use super::{
    IconDest, Parts, SymbolState, bbox_from, finish_slow, undefined_icon, undefined_icon_path,
};
use crate::bbox::BBox;
use crate::domain::Affiliation;
use crate::error::RenderError;
use crate::generated::tables;
use crate::js;
use crate::registry::IconKey;
use crate::small_str::generic_letter_sidc;
use crate::template;
use alloc::vec::Vec;

const SEA_MINE_EXERCISE: [&str; 5] = ["WMGX--", "WMMX--", "WMFX--", "WMX---", "WMSX--"];

pub(super) fn letter_icon_into(
    s: &SymbolState<'_>,
    parts: &Parts<'_>,
    needs_outline: bool,
    mut dest: IconDest<'_>,
    gbbox: &mut BBox,
) -> Result<(), RenderError> {
    let md = s.metadata;
    if SEA_MINE_EXERCISE.contains(&md.function_id.as_str()) {
        gbbox.y1 = 10.0;
        if md.affiliation.known() != Some(Affiliation::Unknown) {
            gbbox.x2 = md.geometry_bbox().x2 + 20.0;
        }
    }
    let sidc = js::JsStr::new(s.sidc);
    let generic = generic_letter_sidc(&sidc);
    let key = IconKey::Letter { generic: &generic };
    if let Some(b) = parts.user_bbox(key).or_else(|| {
        let entry = template::find(tables::LETTER_BBOX, &generic)?;
        parts.resolver.bbox(entry).map(bbox_from)
    }) {
        *gbbox = b;
    }
    if parts.resolver.user.ctx.is_none()
        && !needs_outline
        && let IconDest::Direct {
            acc,
            bbox,
            valid_icon,
        } = &mut dest
    {
        let wrote = match template::find(tables::LETTER_ICONS, &generic) {
            Some(e) => parts.resolver.value_unwrapped(e, false, acc)?,
            None => false,
        };
        if wrote {
            bbox.merge((*gbbox).into());
        } else {
            let f = sidc.substr(4, 6);
            if !(f == "------" || f.is_empty()) {
                acc.push(undefined_icon_path(s));
                **valid_icon = false;
                bbox.merge((*gbbox).into());
            }
        }
        return Ok(());
    }
    let mut post = Vec::new();
    let mut invalid = false;
    let found = parts.user_icon(key).or_else(|| {
        parts
            .resolver
            .value(template::find(tables::LETTER_ICONS, &generic)?)
    });
    if let Some(icon) = found {
        post.push(icon);
    } else {
        let f = sidc.substr(4, 6);
        if !(f == "------" || f.is_empty()) {
            post.push(undefined_icon(s));
            invalid = true;
        }
    }
    finish_slow(s, needs_outline, post, *gbbox, invalid, dest)
}
