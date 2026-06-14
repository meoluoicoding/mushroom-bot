fn main() {
    println!("cargo:rerun-if-changed=data/mquality.bin");

    let path = "data/mquality.bin";
    if std::path::Path::new(path).exists() {
        return;
    }

    let _ = std::fs::create_dir_all("data");

    let rect_count: u32 = 10 * 11 / 2 * 17 * 18 / 2;
    let phases: u32 = 3;
    let buckets: u32 = 8;
    let value_count = (rect_count * phases * buckets) as usize;

    let mut buf = Vec::with_capacity(20 + value_count * 12);
    buf.extend_from_slice(b"MQTY");
    buf.extend_from_slice(&3u32.to_le_bytes());
    buf.extend_from_slice(&rect_count.to_le_bytes());
    buf.extend_from_slice(&phases.to_le_bytes());
    buf.extend_from_slice(&buckets.to_le_bytes());

    let default_win: f32 = 0.5;
    for _ in 0..value_count {
        buf.extend_from_slice(&default_win.to_le_bytes());
    }
    let default_val: f32 = 0.0;
    for _ in 0..value_count {
        buf.extend_from_slice(&default_val.to_le_bytes());
    }
    let default_count: u32 = 0;
    for _ in 0..value_count {
        buf.extend_from_slice(&default_count.to_le_bytes());
    }

    let _ = std::fs::write(path, &buf);
}
