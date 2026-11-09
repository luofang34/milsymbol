//! Requests shared by the native tests and browser bridge.

use milsymbol::options::SymbolOptions;
use milsymbol::sidc::{NumericSidc, Sidc};
use milsymbol::{RenderMode, Standard};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Request {
    pub sidc: Option<String>,
    pub fields: Option<Fields>,
    pub standard: Standard,
    pub mode: RenderMode,
    pub options: SymbolOptions,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            sidc: Some("15031000141211000000".into()),
            fields: None,
            standard: Standard::Mil2525,
            mode: RenderMode::Standards,
            options: SymbolOptions::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Fields {
    pub version: String,
    pub context: String,
    pub identity: String,
    pub symbol_set: String,
    pub status: String,
    pub hq: String,
    pub amplifier: String,
    pub entity: String,
    pub modifier1: String,
    pub modifier2: String,
    pub frame_shape: String,
    pub trailing: String,
}

impl Fields {
    pub(crate) fn from_sidc(sidc: &NumericSidc) -> Self {
        let s = sidc.as_str();
        let field = |a, b| s.get(a..b).unwrap_or("0").to_owned();
        let (m1, m2) = sidc.modifier_codes();
        Self {
            version: field(0, 2),
            context: field(2, 3),
            identity: field(3, 4),
            symbol_set: sidc.symbol_set().into(),
            status: field(6, 7),
            hq: field(7, 8),
            amplifier: sidc.amplifier().into(),
            entity: sidc.entity().into(),
            modifier1: m1.to_string(),
            modifier2: m2.to_string(),
            frame_shape: field(22, 23),
            trailing: s.get(23..30).unwrap_or("0000000").into(),
        }
    }

    pub(crate) fn encode(&self) -> Result<String, Error> {
        let expected = [
            (&self.version, 2),
            (&self.context, 1),
            (&self.identity, 1),
            (&self.symbol_set, 2),
            (&self.status, 1),
            (&self.hq, 1),
            (&self.amplifier, 2),
            (&self.entity, 6),
            (&self.modifier1, 3),
            (&self.modifier2, 3),
            (&self.frame_shape, 1),
            (&self.trailing, 7),
        ];
        if expected.iter().any(|(v, n)| {
            v.len() != *n
                || !v
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase())
        }) {
            return Err(Error::Fields);
        }
        Ok(format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
            self.version,
            self.context,
            self.identity,
            self.symbol_set,
            self.status,
            self.hq,
            self.amplifier,
            self.entity,
            self.modifier1.get(1..).unwrap_or(""),
            self.modifier2.get(1..).unwrap_or(""),
            self.modifier1.get(..1).unwrap_or(""),
            self.modifier2.get(..1).unwrap_or(""),
            self.frame_shape,
            self.trailing
        ))
    }
}

impl Request {
    pub(crate) fn parse(&self) -> Result<Sidc, Error> {
        let code = match &self.fields {
            Some(fields) => fields.encode()?,
            None => self.sidc.clone().unwrap_or_default(),
        };
        Ok(Sidc::parse_with_mode(&code, self.standard, self.mode)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Invalid request JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid SIDC: {0}")]
    Sidc(#[from] milsymbol::sidc::SidcError),
    #[error("Rendering failed: {0}")]
    Render(#[from] milsymbol::RenderError),
    #[error(
        "Each SIDC field must have its specified width and use uppercase hexadecimal characters"
    )]
    Fields,
    #[error("No valid symbol of SIDC version {version} was found for these rendering rules")]
    NoRandom { version: u8 },
}
