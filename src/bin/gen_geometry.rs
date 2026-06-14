use std::fs::File;
use std::io::Write;

use mushroom_bot::types::{Bitboard, COLS, N_CELLS, N_RECTS, ROWS};

#[derive(Clone)]
struct RectMeta {
    r1: u8,
    c1: u8,
    r2: u8,
    c2: u8,
    area: u8,
    cell_mask: Bitboard,
    top_border: Bitboard,
    bottom_border: Bitboard,
    left_border: Bitboard,
    right_border: Bitboard,
}

fn main() {
    std::fs::create_dir_all("data").expect("create data directory");
    let mut rects: Vec<RectMeta> = Vec::with_capacity(N_RECTS);
    let mut cell_to_rects: Vec<Vec<u32>> = vec![Vec::new(); N_CELLS];

    for r1 in 0..ROWS {
        for r2 in r1..ROWS {
            for c1 in 0..COLS {
                for c2 in c1..COLS {
                    let mut cm = Bitboard::empty();
                    let mut tm = Bitboard::empty();
                    let mut bm = Bitboard::empty();
                    let mut lm = Bitboard::empty();
                    let mut rm = Bitboard::empty();

                    for r in r1..=r2 {
                        for c in c1..=c2 {
                            cm.set(r, c);
                            if r == r1 {
                                tm.set(r, c);
                            }
                            if r == r2 {
                                bm.set(r, c);
                            }
                            if c == c1 {
                                lm.set(r, c);
                            }
                            if c == c2 {
                                rm.set(r, c);
                            }
                        }
                    }

                    let idx = rects.len() as u32;
                    for r in r1..=r2 {
                        for c in c1..=c2 {
                            cell_to_rects[r * COLS + c].push(idx);
                        }
                    }

                    rects.push(RectMeta {
                        r1: r1 as u8,
                        c1: c1 as u8,
                        r2: r2 as u8,
                        c2: c2 as u8,
                        area: ((r2 - r1 + 1) * (c2 - c1 + 1)) as u8,
                        cell_mask: cm,
                        top_border: tm,
                        bottom_border: bm,
                        left_border: lm,
                        right_border: rm,
                    });
                }
            }
        }
    }

    assert_eq!(rects.len(), N_RECTS);

    let mut buf = vec![0u8; 72];
    buf[0..4].copy_from_slice(b"GPSB");
    buf[4..8].copy_from_slice(&1u32.to_le_bytes());
    buf[8..12].copy_from_slice(&(N_RECTS as u32).to_le_bytes());
    buf[12..16].copy_from_slice(&72u32.to_le_bytes());

    for r in &rects {
        buf.push(r.r1);
        buf.push(r.c1);
        buf.push(r.r2);
        buf.push(r.c2);
        buf.push(r.area);
        buf.extend_from_slice(&[0u8; 3]);
        buf.extend_from_slice(&r.cell_mask.0[0].to_le_bytes());
        buf.extend_from_slice(&r.cell_mask.0[1].to_le_bytes());
        buf.extend_from_slice(&r.cell_mask.0[2].to_le_bytes());
        for bm in [r.top_border, r.bottom_border, r.left_border, r.right_border] {
            buf.extend_from_slice(&bm.0[0].to_le_bytes());
            buf.extend_from_slice(&bm.0[1].to_le_bytes());
            buf.extend_from_slice(&bm.0[2].to_le_bytes());
        }
    }

    let cell_offset = buf.len() as u32;
    for cell_idx in 0..N_CELLS {
        let rects_for_cell = &cell_to_rects[cell_idx];
        buf.extend_from_slice(&(rects_for_cell.len() as u16).to_le_bytes());
        buf.extend_from_slice(&(rects_for_cell.len() as u16).to_le_bytes());
        for &rid in rects_for_cell {
            buf.extend_from_slice(&(rid as u16).to_le_bytes());
        }
    }

    buf[16..20].copy_from_slice(&0u32.to_le_bytes());
    buf[20..24].copy_from_slice(&0u32.to_le_bytes());
    buf[24..28].copy_from_slice(&cell_offset.to_le_bytes());

    let output_path = "data/geometry.bin";
    let mut file = File::create(output_path).expect("create geometry.bin");
    file.write_all(&buf).expect("write geometry file");

    eprintln!(
        "Written {}: {} rects, {} bytes total",
        output_path,
        N_RECTS,
        72 + buf.len() - 72
    );
}
