//! Renderer-wide configuration (global state in milsymbol.js).

use crate::color::ColorMode;
use crate::generated::misc;
use crate::ir::Str;
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

/// Which JavaScript engine build the output should match exactly.
///
/// milsymbol.js computes direction-arrow and speed-leader coordinates with
/// `Math.sin`/`Math.cos`. V8's x64 builds evaluate them in plain IEEE
/// arithmetic, while its arm64 builds (Apple Silicon, ARM Linux) use fused
/// multiply-adds, so the last digit of those coordinates differs between
/// platforms. This crate reproduces either one exactly, independently of
/// the platform it runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ReferencePlatform {
    /// V8 on x86-64 (Node/Chrome on Intel and AMD); also what WebAssembly
    /// engines compute.
    #[default]
    X64,
    /// V8 on arm64 (Node/Chrome on Apple Silicon and ARM Linux).
    Arm64,
}

/// Dash arrays of not-present frames and feint/dummy indicators
/// (upstream `ms.setDashArrays`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DashArrays {
    /// Pending / suspect / assumed friend frames.
    pub pending: Str,
    /// Planned / anticipated frames.
    pub anticipated: Str,
    /// Feint/dummy indicator.
    pub feint_dummy: Str,
}

impl DashArrays {
    /// Dash arrays for pending, anticipated and feint/dummy lines.
    pub fn new(
        pending: impl Into<Str>,
        anticipated: impl Into<Str>,
        feint_dummy: impl Into<Str>,
    ) -> Self {
        DashArrays {
            pending: pending.into(),
            anticipated: anticipated.into(),
            feint_dummy: feint_dummy.into(),
        }
    }
}

impl Default for DashArrays {
    fn default() -> Self {
        DashArrays {
            pending: Str::Borrowed("4,4"),
            anticipated: Str::Borrowed("8,12"),
            feint_dummy: Str::Borrowed("8,8"),
        }
    }
}

/// Configuration shared by all symbols a [`Renderer`](crate::Renderer) draws.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RendererConfig {
    /// Default standard (upstream `ms.setStandard`).
    pub standard: Standard,
    /// Dash arrays (upstream `ms.setDashArrays`).
    pub dash_arrays: DashArrays,
    /// Default headquarters staff length (upstream `ms.setHqStaffLength`).
    pub hq_staff_length: f64,
    /// Registered colour modes by name (upstream `ms.setColorMode`).
    pub color_modes: BTreeMap<String, ColorMode>,
    /// Engine build whose floating-point results are reproduced.
    pub reference_platform: ReferencePlatform,
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
            reference_platform: ReferencePlatform::X64,
        }
    }
}

impl RendererConfig {
    /// A registered colour mode.
    pub fn color_mode(&self, name: &str) -> Option<&ColorMode> {
        self.color_modes.get(name)
    }
}
