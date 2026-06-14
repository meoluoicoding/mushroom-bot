use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use crate::arena::MoveLog;

pub fn write_logs_csv(path: &str, logs: &[MoveLog]) -> std::io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(path)?;
    let mut w = BufWriter::new(file);
    writeln!(w, "game_id,ply,bot_a,bot_b,mover_a,score_delta,recaptured,fresh,live,net_area,area,live_count,num_moves,outcome,margin,elapsed_ms")?;
    for log in logs {
        writeln!(w, "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.4},{},{:.2}",
            log.game_id, log.ply, log.bot_a, log.bot_b, log.mover_is_a as u8,
            log.score_delta, log.recaptured, log.fresh, log.live,
            log.net_area, log.area, log.live_count, log.num_moves,
            log.outcome, log.margin, log.elapsed_ms,
        )?;
    }
    w.flush()
}

pub fn write_logs_bin(path: &str, logs: &[MoveLog]) -> std::io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(path)?;
    let mut w = BufWriter::new(file);

    let count = logs.len() as u32;
    w.write_all(&count.to_le_bytes())?;

    for log in logs {
        w.write_all(&log.game_id.to_le_bytes())?;
        w.write_all(&log.ply.to_le_bytes())?;
        w.write_all(&log.bot_a.to_le_bytes())?;
        w.write_all(&log.bot_b.to_le_bytes())?;
        w.write_all(&(log.mover_is_a as u8).to_le_bytes())?;
        w.write_all(&log.score_delta.to_le_bytes())?;
        w.write_all(&log.recaptured.to_le_bytes())?;
        w.write_all(&log.fresh.to_le_bytes())?;
        w.write_all(&log.live.to_le_bytes())?;
        w.write_all(&log.net_area.to_le_bytes())?;
        w.write_all(&log.area.to_le_bytes())?;
        w.write_all(&log.live_count.to_le_bytes())?;
        w.write_all(&(log.num_moves as u32).to_le_bytes())?;
        w.write_all(&log.outcome.to_le_bytes())?;
        w.write_all(&log.margin.to_le_bytes())?;
        w.write_all(&log.elapsed_ms.to_le_bytes())?;
    }
    w.flush()
}
