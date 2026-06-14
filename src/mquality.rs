use std::fs;

use crate::types::*;

pub const MQUALITY_PHASES: usize = 3;
pub const MQUALITY_BUCKETS: usize = 8;

#[derive(Clone, Debug)]
pub struct MoveQualityTable {
    pub rect_count: usize,
    pub phases: usize,
    pub buckets: usize,
    pub win_rates: Vec<f32>,
    pub avg_values: Vec<f32>,
    pub counts: Vec<u32>,
}

impl MoveQualityTable {
    pub fn new(rect_count: usize) -> Self {
        let len = rect_count * MQUALITY_PHASES * MQUALITY_BUCKETS;
        Self {
            rect_count,
            phases: MQUALITY_PHASES,
            buckets: MQUALITY_BUCKETS,
            win_rates: vec![0.5; len],
            avg_values: vec![0.0; len],
            counts: vec![0; len],
        }
    }

    #[inline]
    fn index(&self, rect_id: usize, phase: usize, bucket: usize) -> usize {
        rect_id * self.phases * self.buckets + phase * self.buckets + bucket
    }

    #[inline]
    pub fn get(&self, rect_id: usize, phase: usize, bucket: usize) -> f32 {
        if rect_id >= self.rect_count || phase >= self.phases || bucket >= self.buckets {
            return 0.5;
        }
        self.win_rates[self.index(rect_id, phase, bucket)]
    }

    #[inline]
    pub fn bonus(&self, rect_id: usize, phase: usize, bucket: usize) -> f32 {
        if rect_id >= self.rect_count || phase >= self.phases || bucket >= self.buckets {
            return 0.0;
        }
        let idx = self.index(rect_id, phase, bucket);
        let n = self.counts[idx] as f32;
        let conf = n / (n + 30.0);

        let win_bonus = (self.win_rates[idx] - 0.5) * 1800.0;
        let value_bonus = self.avg_values[idx].clamp(-200.0, 200.0) * 0.5;

        conf * (win_bonus + value_bonus)
    }

    #[inline]
    pub fn avg_value(&self, rect_id: usize, phase: usize, bucket: usize) -> f32 {
        if rect_id >= self.rect_count || phase >= self.phases || bucket >= self.buckets {
            return 0.0;
        }
        self.avg_values[self.index(rect_id, phase, bucket)]
    }

    #[inline]
    pub fn phase_for_position(live_count: u32, legal_moves: usize) -> usize {
        if live_count <= 12 || legal_moves <= 10 {
            2
        } else if live_count <= 25 || legal_moves <= 24 {
            1
        } else {
            0
        }
    }

    #[inline]
    pub fn phase_for_live_count(live_count: u32) -> usize {
        if live_count <= 12 {
            2
        } else if live_count >= 25 {
            0
        } else {
            1
        }
    }

    #[inline]
    pub fn score_bucket(score: i32) -> usize {
        let bucket = (score / 150) + 4;
        bucket.clamp(0, (MQUALITY_BUCKETS - 1) as i32) as usize
    }

    pub fn from_records(rect_count: usize, records: &[(usize, usize, usize, f32, f32)]) -> Self {
        let mut table = Self::new(rect_count);
        let mut outcomes = vec![0.0f32; table.win_rates.len()];
        let mut values = vec![0.0f32; table.win_rates.len()];
        let mut games = vec![0u32; table.win_rates.len()];

        for &(rect_id, phase, bucket, outcome, move_value) in records {
            if rect_id >= rect_count || phase >= table.phases || bucket >= table.buckets {
                continue;
            }
            let idx = table.index(rect_id, phase, bucket);
            outcomes[idx] += outcome;
            values[idx] += move_value;
            games[idx] = games[idx].saturating_add(1);
        }

        let prior = 20.0;
        for i in 0..table.win_rates.len() {
            let n = games[i] as f32;
            table.win_rates[i] = (outcomes[i] + 0.5 * prior) / (n + prior);
            table.avg_values[i] = values[i] / (n + prior);
            table.counts[i] = games[i];
        }

        table
    }
}

impl Default for MoveQualityTable {
    fn default() -> Self {
        Self::new(N_RECTS)
    }
}

pub fn load_mquality_bin(path: &str) -> Option<MoveQualityTable> {
    let data = fs::read(path).ok()?;
    load_mquality_from_bytes(&data)
}

pub fn load_mquality_from_bytes(data: &[u8]) -> Option<MoveQualityTable> {
    if data.len() < 24 || &data[0..4] != b"MQTY" {
        return None;
    }

    let version = u32::from_le_bytes(data[4..8].try_into().ok()?);
    if version < 1 {
        return None;
    }

    let rect_count = u32::from_le_bytes(data[8..12].try_into().ok()?) as usize;
    let phases = u32::from_le_bytes(data[12..16].try_into().ok()?) as usize;
    let buckets = u32::from_le_bytes(data[16..20].try_into().ok()?) as usize;
    let value_count = rect_count.checked_mul(phases)?.checked_mul(buckets)?;
    let mut off = 20usize;
    let mut win_rates = Vec::with_capacity(value_count);
    let mut avg_values = Vec::with_capacity(value_count);
    let mut counts = vec![0; value_count];

    if version == 1 {
        let expected = 20usize + value_count * 4;
        if data.len() < expected {
            return None;
        }
        for _ in 0..value_count {
            let bytes = data.get(off..off + 4)?;
            win_rates.push(f32::from_le_bytes(bytes.try_into().ok()?));
            avg_values.push(0.0);
            off += 4;
        }
    } else if version == 2 {
        let expected = 20usize + value_count * 8;
        if data.len() < expected {
            return None;
        }
        for _ in 0..value_count {
            let bytes = data.get(off..off + 4)?;
            win_rates.push(f32::from_le_bytes(bytes.try_into().ok()?));
            off += 4;
        }
        for _ in 0..value_count {
            let bytes = data.get(off..off + 4)?;
            avg_values.push(f32::from_le_bytes(bytes.try_into().ok()?));
            off += 4;
        }
    } else {
        // version >= 3
        let expected = 20usize + value_count * 12;
        if data.len() < expected {
            return None;
        }
        for _ in 0..value_count {
            let bytes = data.get(off..off + 4)?;
            win_rates.push(f32::from_le_bytes(bytes.try_into().ok()?));
            off += 4;
        }
        for _ in 0..value_count {
            let bytes = data.get(off..off + 4)?;
            avg_values.push(f32::from_le_bytes(bytes.try_into().ok()?));
            off += 4;
        }
        counts.clear();
        for _ in 0..value_count {
            let bytes = data.get(off..off + 4)?;
            counts.push(u32::from_le_bytes(bytes.try_into().ok()?));
            off += 4;
        }
    }

    Some(MoveQualityTable {
        rect_count,
        phases,
        buckets,
        win_rates,
        avg_values,
        counts,
    })
}

pub fn write_mquality_bytes(table: &MoveQualityTable) -> Vec<u8> {
    let mut buf = Vec::with_capacity(20 + table.win_rates.len() * 12);
    buf.extend_from_slice(b"MQTY");
    buf.extend_from_slice(&3u32.to_le_bytes());
    buf.extend_from_slice(&(table.rect_count as u32).to_le_bytes());
    buf.extend_from_slice(&(table.phases as u32).to_le_bytes());
    buf.extend_from_slice(&(table.buckets as u32).to_le_bytes());
    for &rate in &table.win_rates {
        buf.extend_from_slice(&rate.to_le_bytes());
    }
    for &value in &table.avg_values {
        buf.extend_from_slice(&value.to_le_bytes());
    }
    for &count in &table.counts {
        buf.extend_from_slice(&count.to_le_bytes());
    }
    buf
}
