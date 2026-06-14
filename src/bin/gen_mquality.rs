use std::fs;
use std::path::Path;

use mushroom_bot::mquality::{write_mquality_bytes, MoveQualityTable};

fn main() {
    std::fs::create_dir_all("data").expect("create data directory");
    let args: Vec<String> = std::env::args().collect();
    let mut txt_dir = String::new();
    let mut output = "data/mquality.bin".to_string();
    let mut rect_count = mushroom_bot::types::N_RECTS;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--txt-dir" => {
                i += 1;
                txt_dir = args.get(i).cloned().unwrap_or_default();
            }
            "--output" => {
                i += 1;
                output = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "data/mquality.bin".to_string());
            }
            "--rect-count" => {
                i += 1;
                rect_count = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(rect_count);
            }
            _ => {}
        }
        i += 1;
    }

    let table = if !txt_dir.is_empty() {
        match load_records_from_dir(&txt_dir, rect_count) {
            Some(records) if !records.is_empty() => MoveQualityTable::from_records(rect_count, &records),
            _ => MoveQualityTable::new(rect_count),
        }
    } else {
        MoveQualityTable::new(rect_count)
    };

    let bytes = write_mquality_bytes(&table);
    fs::write(&output, &bytes).expect("write mquality output");
    eprintln!("Written {}: {} bytes", output, bytes.len());
}

fn load_records_from_dir(path: &str, rect_count: usize) -> Option<Vec<(usize, usize, usize, f32, f32)>> {
    let mut records = Vec::new();
    for entry in fs::read_dir(Path::new(path)).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "txt" && ext != "csv" {
            continue;
        }
        let text = fs::read_to_string(&path).ok()?;
        for line in text.lines() {
            if let Some(record) = parse_record(line, rect_count) {
                records.push(record);
            }
        }
    }
    Some(records)
}

fn parse_record(line: &str, rect_count: usize) -> Option<(usize, usize, usize, f32, f32)> {
    if line.starts_with('#') || line.trim().is_empty() {
        return None;
    }

    let vals: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|s| !s.trim().is_empty())
        .collect();

    if vals.len() >= 9 {
        let rect_id = vals[3].parse::<usize>().ok()?;
        let phase = vals[4].parse::<usize>().ok()?.min(2);
        let bucket = vals[5].parse::<usize>().ok()?.min(7);
        let move_value = vals[6].parse::<f32>().ok()?;
        let outcome = vals[7].parse::<f32>().ok()?;
        if rect_id >= rect_count {
            return None;
        }
        return Some((rect_id, phase, bucket, outcome, move_value));
    }

    if vals.len() >= 5 {
        let rect_id = vals[0].parse::<usize>().ok()?;
        let phase = vals[1].parse::<usize>().ok()?.min(2);
        let bucket = vals[2].parse::<usize>().ok()?.min(7);
        let outcome = vals[3].parse::<f32>().ok()?;
        let move_value = vals[4].parse::<f32>().ok()?;
        if rect_id >= rect_count {
            return None;
        }
        return Some((rect_id, phase, bucket, outcome, move_value));
    }

    None
}
