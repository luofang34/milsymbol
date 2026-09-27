//! Renderer-wide configuration (global state in milsymbol.js).

use crate::color::ColorMode;
use crate::generated::misc;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// The symbology standard used when a symbol does not override it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Standard {
    /// MIL-STD-2525 (upstream default).
    #[default]
    Mil2525,
    /// NATO APP-6.
    App6,
}

/// Dash arrays of not-present frames and feint/dummy indicators
/// (upstream `ms.setDashArrays`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashArrays {
    /// Pending / suspect / assumed friend frames.
    pub pending: String,
    /// Planned / anticipated frames.
    pub anticipated: String,
    /// Feint/dummy indicator.
    pub feint_dummy: String,
}

impl Default for DashArrays {
    fn default() -> Self {
        DashArrays {
            pending: String::from("4,4"),
            anticipated: String::from("8,12"),
            feint_dummy: String::from("8,8"),
        }
    }
}

/// Configuration shared by all symbols a [`Renderer`](crate::Renderer) draws.
#[derive(Debug, Clone, PartialEq)]
pub struct RendererConfig {
    /// Default standard (upstream `ms.setStandard`).
    pub standard: Standard,
    /// Dash arrays (upstream `ms.setDashArrays`).
    pub dash_arrays: DashArrays,
    /// Default headquarters staff length (upstream `ms.setHqStaffLength`).
    pub hq_staff_length: f64,
    /// Registered colour modes by name (upstream `ms.setColorMode`).
    pub color_modes: BTreeMap<String, ColorMode>,
}

impl Default for RendererConfig {
    fn default() -> Self {
        let color_modes = misc::COLOR_MODES
            .iter()
            .map(|(name, values)| (String::from(*name), ColorMode::from_static(values)))
            .collect();
        RendererConfig {
            standard: Standard::Mil2525,
            dash_arrays: DashArrays::default(),
            hq_staff_length: 100.0,
            color_modes,
        }
    }
}

impl RendererConfig {
    /// A registered colour mode.
    pub fn color_mode(&self, name: &str) -> Option<&ColorMode> {
        self.color_modes.get(name)
    }
}
