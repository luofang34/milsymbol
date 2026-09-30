//! Numeric checks on options.

use super::SymbolOptions;
use crate::error::RenderError;

fn finite(name: &'static str, v: f64) -> Result<(), RenderError> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(RenderError::InvalidOption {
            name,
            reason: "must be a finite number",
        })
    }
}

fn finite_opt(name: &'static str, v: Option<f64>) -> Result<(), RenderError> {
    v.map_or(Ok(()), |v| finite(name, v))
}

impl SymbolOptions {
    /// Rejects numbers that cannot be written into an SVG: NaN and
    /// infinities.
    pub(crate) fn check_numbers(&self) -> Result<(), RenderError> {
        let st = &self.style;
        finite_opt("direction", self.direction)?;
        finite_opt("speedLeader", self.speed_leader)?;
        finite_opt("stack", self.stack)?;
        finite_opt("hqStaffLength", st.hq_staff_length)?;
        finite_opt("infoOutlineWidth", st.info_outline_width)?;
        for (name, v) in [
            ("fillOpacity", st.fill_opacity),
            ("infoSize", st.info_size),
            ("outlineWidth", st.outline_width),
            ("padding", st.padding),
            ("size", st.size),
            ("strokeWidth", st.stroke_width),
        ] {
            finite(name, v)?;
        }
        Ok(())
    }
}
