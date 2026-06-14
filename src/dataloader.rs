use crate::types::*;
use std::fs;
use crate::mquality::{load_mquality_from_bytes, MoveQualityTable};

/// Eval weights for the 8-term evaluation function
#[derive(Clone, Copy, Debug)]
pub struct EvalWeights {
    pub territory: f32,
    pub safe_territory: f32,
    pub vulnerability: f32,
    pub steal_potential: f32,
    pub mobility: f32,
    pub connectivity: f32,
    pub corner_bonus: f32,
    pub edge_bonus: f32,
}

impl Default for EvalWeights {
    fn default() -> Self {
        // Balanced style weights
        Self {
            territory: 156.0,
            safe_territory: 211.0,
            vulnerability: -40.0,
            steal_potential: 40.0,
            mobility: 8.0,
            connectivity: 31.0,
            corner_bonus: 26.0,
            edge_bonus: 22.0,
        }
    }
}

impl EvalWeights {
    pub fn from_array(w: &[i32; 8]) -> Self {
        Self {
            territory: w[0] as f32,
            safe_territory: w[1] as f32,
            vulnerability: w[2] as f32,
            steal_potential: w[3] as f32,
            mobility: w[4] as f32,
            connectivity: w[5] as f32,
            corner_bonus: w[6] as f32,
            edge_bonus: w[7] as f32,
        }
    }
}

/// Precomputed rectangle geometry for accelerated evaluation
#[derive(Clone)]
pub struct RectGeometry {
    pub r1: u8,
    pub c1: u8,
    pub r2: u8,
    pub c2: u8,
    pub area: u8,
    pub cell_mask: Bitboard,
    pub top_border: Bitboard,
    pub bottom_border: Bitboard,
    pub left_border: Bitboard,
    pub right_border: Bitboard,
}

/// Complete loaded data from data.bin
#[derive(Clone)]
pub struct GameData {
    pub weights: EvalWeights,
    pub geometry: Vec<RectGeometry>,
    pub cell_to_rects: Vec<Vec<u16>>,
    pub mquality: Option<MoveQualityTable>,
}

impl Default for GameData {
    fn default() -> Self {
        Self {
            weights: EvalWeights::default(),
            geometry: Vec::new(),
            cell_to_rects: vec![Vec::new(); N_CELLS],
            mquality: None,
        }
    }
}

/// Load game data from data.bin (v3 format)
pub fn load_data_bin(path: &str) -> Option<GameData> {
    let data = fs::read(path).ok()?;
    if data.len() < 72 {
        return None;
    }

    // Check magic
    if &data[0..4] != b"CPSB" {
        return None;
    }

    let version = u32::from_le_bytes(data[4..8].try_into().ok()?);
    if version < 3 {
        return None;
    }

    // Read eval offset
    let eval_off = u32::from_le_bytes(data[24..28].try_into().ok()?) as usize;
    if eval_off + 32 > data.len() {
        return None;
    }

    // Read weights
    let mut w = [0i32; 8];
    for i in 0..8 {
        w[i] = i32::from_le_bytes(data[eval_off + i * 4..eval_off + i * 4 + 4].try_into().ok()?);
    }
    let weights = EvalWeights::from_array(&w);

    // Try to read geometry
    let geometry = load_geometry_from_data(&data).unwrap_or_default();
    let cell_to_rects = build_cell_to_rects(&geometry);
    let mquality = load_mquality_from_data(&data);

    Some(GameData {
        weights,
        geometry,
        cell_to_rects,
        mquality,
    })
}

/// Load geometry from a combined data.bin
fn load_geometry_from_data(data: &[u8]) -> Option<Vec<RectGeometry>> {
    if data.len() < 104 {
        return None;
    }

    // geometry starts at offset 104 (72 header + 32 weights)
    let geom_offset = 104usize;
    let rect_size = 128usize; // 8 + 24 + 4*24 = 128 bytes per rect

    let mut geometry = Vec::with_capacity(N_RECTS);

    for i in 0..N_RECTS {
        let off = geom_offset + i * rect_size;
        if off + rect_size > data.len() {
            break;
        }

        let r1 = data[off];
        let c1 = data[off + 1];
        let r2 = data[off + 2];
        let c2 = data[off + 3];
        let area = data[off + 4];

        // Cell mask at offset 8
        let cm = read_bitboard(&data[off + 8..off + 32])?;
        let tm = read_bitboard(&data[off + 32..off + 56])?;
        let bm = read_bitboard(&data[off + 56..off + 80])?;
        let lm = read_bitboard(&data[off + 80..off + 104])?;
        let rm = read_bitboard(&data[off + 104..off + 128])?;

        geometry.push(RectGeometry {
            r1,
            c1,
            r2,
            c2,
            area,
            cell_mask: cm,
            top_border: tm,
            bottom_border: bm,
            left_border: lm,
            right_border: rm,
        });
    }

    Some(geometry)
}

fn read_bitboard(data: &[u8]) -> Option<Bitboard> {
    if data.len() < 24 {
        return None;
    }
    Some(Bitboard([
        u64::from_le_bytes(data[0..8].try_into().ok()?),
        u64::from_le_bytes(data[8..16].try_into().ok()?),
        u64::from_le_bytes(data[16..24].try_into().ok()?),
    ]))
}

fn build_cell_to_rects(geometry: &[RectGeometry]) -> Vec<Vec<u16>> {
    let mut cell_to_rects = vec![Vec::new(); N_CELLS];
    for (idx, rect) in geometry.iter().enumerate() {
        for r in rect.r1..=rect.r2 {
            for c in rect.c1..=rect.c2 {
                let cell_idx = r as usize * COLS + c as usize;
                cell_to_rects[cell_idx].push(idx as u16);
            }
        }
    }
    cell_to_rects
}

fn load_mquality_from_data(data: &[u8]) -> Option<MoveQualityTable> {
    if data.len() < 52 {
        return None;
    }
    let mq_offset = u32::from_le_bytes(data[48..52].try_into().ok()?) as usize;
    if mq_offset == 0 || mq_offset >= data.len() {
        return None;
    }
    load_mquality_from_bytes(&data[mq_offset..])
}

/// Read weights from a data.bin file (lightweight, weights only)
pub fn read_weights_from_file(path: &str) -> Option<EvalWeights> {
    let data = fs::read(path).ok()?;
    if data.len() < 72 {
        return None;
    }
    if &data[0..4] != b"CPSB" {
        return None;
    }
    let version = u32::from_le_bytes(data[4..8].try_into().ok()?);
    if version < 3 {
        return None;
    }
    let eval_off = u32::from_le_bytes(data[24..28].try_into().ok()?) as usize;
    if eval_off + 32 > data.len() {
        return None;
    }
    let mut w = [0i32; 8];
    for i in 0..8 {
        w[i] = i32::from_le_bytes(data[eval_off + i * 4..eval_off + i * 4 + 4].try_into().ok()?);
    }
    Some(EvalWeights::from_array(&w))
}

/// Read weights from a text file (comma or whitespace separated)
pub fn read_weights_from_txt(path: &str) -> Option<EvalWeights> {
    let text = fs::read_to_string(path).ok()?;
    let vals: Vec<i32> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();
    if vals.len() < 8 {
        return None;
    }
    let mut w = [0i32; 8];
    w.copy_from_slice(&vals[0..8]);
    Some(EvalWeights::from_array(&w))
}
