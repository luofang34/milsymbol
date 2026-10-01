//! Packing of nodes and row tables into the compact generated layout.

use super::pool::{EntryRec, Packed, Pools};
use crate::Error;

/// Node records plus the wide-variant side table.
pub(super) struct PackedNodes {
    pub nodes: Vec<String>,
    pub wide: Vec<String>,
}

pub(super) fn pack_nodes(p: &Pools) -> Result<PackedNodes, Error> {
    let narrow = |v: usize| u16::try_from(v).map_err(|_| Error::from("node field exceeds u16"));
    let mut wide = Vec::new();
    let mut nodes = Vec::with_capacity(p.nodes.len());
    for n in &p.nodes {
        nodes.push(match n {
            Packed::Path { path, style } => format!("PNode(T_PATH, {path}, {})", narrow(*style)?),
            Packed::Text { text, style } => format!("PNode(T_TEXT, {text}, {})", narrow(*style)?),
            Packed::Group { start, len } => format!("PNode(T_GROUP, {start}, {})", narrow(*len)?),
            Packed::Ref(part) => format!("PNode(T_REF, {part}, 0)"),
            Packed::Wide(lit) => {
                wide.push(lit.clone());
                format!("PNode(T_WIDE, {}, 0)", wide.len() - 1)
            }
        });
    }
    Ok(PackedNodes { nodes, wide })
}

/// Entry records, the shared palette and the bit-packed palette indices.
pub(super) struct PackedRows {
    pub entries: Vec<String>,
    pub palette: Vec<u16>,
    pub data: Vec<u8>,
}

const ABSENT_PALETTE: u16 = u16::MAX;

fn set_bits(data: &mut Vec<u8>, base: usize, pos: usize, bits: u32, value: u32) {
    for b in 0..bits as usize {
        if value >> b & 1 == 1 {
            let at = base * 8 + pos + b;
            if data.len() <= at / 8 {
                data.resize(at / 8 + 1, 0);
            }
            if let Some(byte) = data.get_mut(at / 8) {
                *byte |= 1 << (at % 8);
            }
        }
    }
}

pub(super) fn pack_rows(entries: &[EntryRec]) -> Result<PackedRows, Error> {
    let mut out = PackedRows {
        entries: Vec::with_capacity(entries.len()),
        palette: Vec::new(),
        data: Vec::new(),
    };
    for e in entries {
        let mut distinct: Vec<u32> = Vec::new();
        for &v in &e.vals {
            if !distinct.contains(&v) {
                distinct.push(v);
            }
        }
        let mask = e.mask;
        if distinct.len() == 1 {
            let v = distinct.first().copied().unwrap_or(u32::MAX);
            out.entries.push(format!(
                "Entry {{ deps: 0b{mask:b}, bits: 0, a: {v}, b: 0 }}"
            ));
            continue;
        }
        let bits = usize::BITS - (distinct.len() - 1).leading_zeros();
        let (pal_start, base) = (out.palette.len(), out.data.len());
        for &v in &distinct {
            out.palette.push(match v {
                u32::MAX => ABSENT_PALETTE,
                v => u16::try_from(v)
                    .ok()
                    .filter(|&x| x != ABSENT_PALETTE)
                    .ok_or("row value does not fit the u16 palette")?,
            });
        }
        let used = (e.vals.len() * bits as usize).div_ceil(8);
        out.data.resize(base + used, 0);
        for (i, v) in e.vals.iter().enumerate() {
            let idx = distinct.iter().position(|d| d == v).unwrap_or(0);
            let idx = u32::try_from(idx).map_err(|_| "palette index overflow")?;
            set_bits(&mut out.data, base, i * bits as usize, bits, idx);
        }
        out.entries.push(format!(
            "Entry {{ deps: 0b{mask:b}, bits: {bits}, a: {pal_start}, b: {base} }}"
        ));
    }
    Ok(out)
}
