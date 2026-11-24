//! Animated GIF (GIF89a, the W3C/CompuServe 1990 specification) from RGBA frames: one palette for
//! the whole run, cut from the frames' colours by median cut (Heckbert 1982), each pixel mapped to
//! its nearest entry, LZW by `weezl`, looping forever (the NETSCAPE2.0 application extension).

/// A palette of at most 256 colours for these frames: median cut over a sample of their pixels.
pub fn palette(frames: &[&[u8]], most: usize) -> Vec<[u8; 3]> {
    let mut px: Vec<[u8; 3]> = Vec::new();
    for f in frames {
        let n = f.len() / 4;
        // at most 64k samples a frame, evenly spaced
        let step = (n / 65536).max(1);
        for k in (0..n).step_by(step) {
            px.push([f[k * 4], f[k * 4 + 1], f[k * 4 + 2]]);
        }
    }
    if px.is_empty() {
        return vec![[0; 3]];
    }
    let mut boxes = vec![px];
    while boxes.len() < most {
        // split the box with the widest channel range
        let (k, axis, range) = boxes
            .iter()
            .enumerate()
            .map(|(k, b)| {
                let mut lo = [255u8; 3];
                let mut hi = [0u8; 3];
                for p in b {
                    for c in 0..3 {
                        lo[c] = lo[c].min(p[c]);
                        hi[c] = hi[c].max(p[c]);
                    }
                }
                let (axis, r) = (0..3).map(|c| (c, hi[c] - lo[c])).max_by_key(|x| x.1).unwrap();
                (k, axis, r)
            })
            .max_by_key(|x| x.2)
            .unwrap();
        if range == 0 {
            break;
        }
        let mut b = boxes.swap_remove(k);
        b.sort_unstable_by_key(|p| p[axis]);
        let other = b.split_off(b.len() / 2);
        boxes.push(b);
        boxes.push(other);
    }
    boxes
        .iter()
        .map(|b| {
            let mut s = [0u64; 3];
            for p in b {
                for c in 0..3 {
                    s[c] += p[c] as u64;
                }
            }
            let n = b.len() as u64;
            [(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8]
        })
        .collect()
}

fn nearest(pal: &[[u8; 3]], p: [u8; 3]) -> u8 {
    let d = |q: &[u8; 3]| {
        let e = |c: usize| p[c] as i32 - q[c] as i32;
        e(0) * e(0) + e(1) * e(1) + e(2) * e(2)
    };
    let mut best = (i32::MAX, 0usize);
    for (k, q) in pal.iter().enumerate() {
        let x = d(q);
        if x < best.0 {
            best = (x, k);
        }
    }
    best.1 as u8
}

/// The frames as one looping GIF, each shown `delay_cs` hundredths of a second.
pub fn encode(size: (u32, u32), frames: &[Vec<u8>], delay_cs: u16) -> Result<Vec<u8>, String> {
    let (w, h) = size;
    if w > 0xffff || h > 0xffff || w == 0 || h == 0 {
        return Err(format!("a GIF is 1 to 65535 pixels a side, not {w} x {h}"));
    }
    let need = (w * h * 4) as usize;
    if let Some(f) = frames.iter().find(|f| f.len() != need) {
        return Err(format!("a frame of {} bytes for {w} x {h}", f.len()));
    }
    let refs: Vec<&[u8]> = frames.iter().map(|f| f.as_slice()).collect();
    let mut pal = palette(&refs, 256);
    pal.resize(256, [0; 3]);
    let mut out = b"GIF89a".to_vec();
    out.extend((w as u16).to_le_bytes());
    out.extend((h as u16).to_le_bytes());
    // a global table of 256 colours (size 2^(7+1)), colour resolution 8 bits
    out.extend([0xF7, 0, 0]);
    for c in &pal {
        out.extend(c);
    }
    // loop forever
    out.extend([0x21, 0xFF, 0x0B]);
    out.extend(b"NETSCAPE2.0");
    out.extend([0x03, 0x01, 0x00, 0x00, 0x00]);
    let mut cache = std::collections::HashMap::new();
    for f in frames {
        // graphic control: no transparency, the delay
        out.extend([0x21, 0xF9, 0x04, 0x00]);
        out.extend(delay_cs.to_le_bytes());
        out.extend([0x00, 0x00]);
        // the image, whole frame, no local table
        out.push(0x2C);
        out.extend([0, 0, 0, 0]);
        out.extend((w as u16).to_le_bytes());
        out.extend((h as u16).to_le_bytes());
        out.push(0);
        let idx: Vec<u8> = f
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                let c = [p[0], p[1], p[2]];
                *cache.entry(c).or_insert_with(|| nearest(&pal, c))
            })
            .collect();
        let lzw = weezl::encode::Encoder::new(weezl::BitOrder::Lsb, 8)
            .encode(&idx)
            .map_err(|e| e.to_string())?;
        out.push(8);
        for block in lzw.chunks(255) {
            out.push(block.len() as u8);
            out.extend(block);
        }
        out.push(0);
    }
    out.push(0x3B);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_few_colours_come_back_exactly() {
        let (w, h) = (16u32, 8u32);
        let colours = [[255, 0, 0], [0, 128, 255], [20, 20, 20], [250, 250, 250]];
        let frames: Vec<Vec<u8>> = (0..3)
            .map(|k| {
                (0..w * h)
                    .flat_map(|i| {
                        let c = colours[((i + k) % 4) as usize];
                        [c[0], c[1], c[2], 255]
                    })
                    .collect()
            })
            .collect();
        let g = encode((w, h), &frames, 4).unwrap();
        assert!(g.starts_with(b"GIF89a") && g.ends_with(&[0x3B]));
        // decoded by weezl's own decoder, frame 0's indices map back to its colours
        let pal: Vec<[u8; 3]> = g[13..13 + 768].as_chunks::<3>().0.to_vec();
        let mut at = 13 + 768 + 19; // past the loop extension
        at += 8; // graphic control
        assert_eq!(g[at], 0x2C);
        at += 10;
        assert_eq!(g[at], 8);
        at += 1;
        let mut data = Vec::new();
        while g[at] != 0 {
            let n = g[at] as usize;
            data.extend(&g[at + 1..at + 1 + n]);
            at += 1 + n;
        }
        let idx = weezl::decode::Decoder::new(weezl::BitOrder::Lsb, 8)
            .decode(&data)
            .unwrap();
        assert_eq!(idx.len(), (w * h) as usize);
        for (i, k) in idx.iter().enumerate() {
            let want = colours[i % 4];
            assert_eq!(pal[*k as usize], want, "pixel {i}");
        }
        if let Ok(dir) = std::env::var("GIF_CHECK_DIR") {
            std::fs::write(format!("{dir}/t.gif"), &g).unwrap();
        }
        // and the same frames make the same bytes
        assert_eq!(g, encode((w, h), &frames, 4).unwrap());
    }

    #[test]
    fn a_wrong_frame_is_refused() {
        assert!(encode((4, 4), &[vec![0; 10]], 4).is_err());
        assert!(encode((0, 4), &[], 4).is_err());
    }
}
