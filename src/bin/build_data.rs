use std::fs;

fn main() {
    std::fs::create_dir_all("data").expect("create data directory");
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: build_data.exe --geometry <geom.bin> --weights <w.txt> [--mquality <mq.bin>] [--output <data.bin>]");
        return;
    }

    let mut geom_path = String::new();
    let mut weights_path = String::new();
    let mut mquality_path = String::new();
    let mut output = "data/data.bin".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--geometry" => { i += 1; geom_path = args[i].clone(); }
            "--weights" => { i += 1; weights_path = args[i].clone(); }
            "--mquality" => { i += 1; mquality_path = args[i].clone(); }
            "--output" => { i += 1; output = args[i].clone(); }
            _ => {}
        }
        i += 1;
    }

    if geom_path.is_empty() {
        geom_path = "data/geometry.bin".to_string();
    }

    // Read geometry
    let geom = fs::read(&geom_path).unwrap_or_else(|e| {
        eprintln!("Warning: cannot read {}: {}. Creating minimal data.bin", geom_path, e);
        Vec::new()
    });

    // Read weights
    let weights: Vec<i32> = if !weights_path.is_empty() {
        let text = fs::read_to_string(&weights_path).unwrap_or_default();
        text.split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect()
    } else {
        // Default weights (balanced style)
        vec![156, 211, -10, 40, 8, 31, 26, 22]
    };

    let mut buf = Vec::new();
    let has_mquality = !mquality_path.is_empty();

    // Header (72 bytes)
    buf.extend_from_slice(b"CPSB");                           // magic
    buf.extend_from_slice(&(if has_mquality { 4u32 } else { 3u32 }).to_le_bytes()); // version
    buf.extend_from_slice(&0u32.to_le_bytes());                // opening_count
    buf.extend_from_slice(&72u32.to_le_bytes());               // opening_offset (none)
    buf.extend_from_slice(&0u32.to_le_bytes());                // endgame_count
    buf.extend_from_slice(&72u32.to_le_bytes());               // endgame_offset (none)
    buf.extend_from_slice(&72u32.to_le_bytes());               // eval_offset
    buf.extend_from_slice(&1u32.to_le_bytes());                // flags (has weights)
    buf.extend_from_slice(&(8415u32).to_le_bytes());           // rect_count
    buf.extend_from_slice(&72u32.to_le_bytes());               // rect_offset (none without geom)
    buf.extend_from_slice(&0u32.to_le_bytes());                // cell_offset
    buf.extend_from_slice(&0u32.to_le_bytes());                // cell_count
    buf.extend_from_slice(&0u32.to_le_bytes());                // mq_offset
    buf.extend_from_slice(&3u32.to_le_bytes());                // mq_phases
    buf.extend_from_slice(&[0u8; 16]);                         // reserved

    // Eval weights (8 × i32 = 32 bytes)
    for &w in &weights {
        buf.extend_from_slice(&w.to_le_bytes());
    }
    while weights.len() < 8 {
        buf.extend_from_slice(&0i32.to_le_bytes());
    }

    // If geometry available, append it
    if !geom.is_empty() {
        // Skip existing header (72 bytes)
        buf.extend_from_slice(&geom[72..]);
    } else {
        // Minimal rect metadata: 8415 entries of 140 bytes each ≈ 1.18 MB
        for _ in 0..8415 {
            buf.extend_from_slice(&[0u8; 140]);
        }
    }

    if has_mquality {
        let mquality = fs::read(&mquality_path).unwrap_or_else(|e| {
            eprintln!("Warning: cannot read {}: {}. Skipping mquality.", mquality_path, e);
            Vec::new()
        });
        if !mquality.is_empty() {
            let mq_offset = buf.len() as u32;
            buf[48..52].copy_from_slice(&mq_offset.to_le_bytes());
            buf.extend_from_slice(&mquality);
        }
    }

    fs::write(&output, &buf).unwrap();
    eprintln!("Written {}: {} bytes", output, buf.len());
}
