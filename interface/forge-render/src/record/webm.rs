//! WebM (the Matroska subset browsers play, matroska.org's specification and the WebM project's
//! container guidelines): one video track of already-encoded frames (VP9 or AV1 from WebCodecs in
//! the browser, AV1 from an encoder natively) in clusters of at most a second each, with cues so a
//! player can seek. Sizes are written exactly (no "unknown size" elements), so the file is whole.

/// A track's codec, as Matroska names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    Vp9,
    Av1,
}

impl Codec {
    fn id(self) -> &'static str {
        match self {
            Codec::Vp9 => "V_VP9",
            Codec::Av1 => "V_AV1",
        }
    }
}

/// An encoded frame: its bytes and whether it is a key frame.
#[derive(Clone, Debug)]
pub struct Frame {
    pub data: Vec<u8>,
    pub key: bool,
}

/// An element's size as an EBML variable-length integer (8 bytes when it doesn't fit smaller).
fn vint(n: u64) -> Vec<u8> {
    for len in 1..=8u32 {
        let max = (1u64 << (7 * len)) - 1;
        if n < max {
            let mut v = vec![0u8; len as usize];
            for (k, b) in v.iter_mut().enumerate() {
                *b = (n >> (8 * (len as usize - 1 - k))) as u8;
            }
            v[0] |= 0x80 >> (len - 1);
            return v;
        }
    }
    let mut v = vec![0x01];
    v.extend(&n.to_be_bytes()[1..]);
    v
}

fn el(id: u32, body: &[u8]) -> Vec<u8> {
    let idb = id.to_be_bytes();
    let skip = idb.iter().position(|&b| b != 0).unwrap_or(3);
    let mut out = idb[skip..].to_vec();
    out.extend(vint(body.len() as u64));
    out.extend(body);
    out
}

fn uint(id: u32, v: u64) -> Vec<u8> {
    let b = v.to_be_bytes();
    let skip = b.iter().position(|&x| x != 0).unwrap_or(7);
    el(id, &b[skip..])
}

fn float(id: u32, v: f64) -> Vec<u8> {
    el(id, &v.to_be_bytes())
}

fn text(id: u32, s: &str) -> Vec<u8> {
    el(id, s.as_bytes())
}

/// The frames, `fps` a second, `size` pixels, as one WebM file; `codec_private` is the codec's
/// configuration record where it has one (AV1's `av1C`; VP9 needs none).
pub fn mux(
    codec: Codec,
    size: (u32, u32),
    fps: f64,
    frames: &[Frame],
    codec_private: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    if frames.is_empty() {
        return Err("no frames".into());
    }
    if !frames[0].key {
        return Err("the first frame must be a key frame".into());
    }
    if !(fps > 0.0 && fps <= 1000.0) {
        return Err(format!("{fps} frames a second"));
    }
    // timestamps in milliseconds (TimestampScale 1 000 000 ns)
    let ms = |k: usize| (k as f64 * 1000.0 / fps).round() as u64;
    let header = [
        uint(0x4286, 1),
        uint(0x42F7, 1),
        uint(0x42F2, 4),
        uint(0x42F3, 8),
        text(0x4282, "webm"),
        uint(0x4287, 4),
        uint(0x4285, 2),
    ]
    .concat();
    let ebml = el(0x1A45_DFA3, &header);
    let info = el(
        0x1549_A966,
        &[
            uint(0x2AD7B1, 1_000_000),
            float(0x4489, ms(frames.len()) as f64),
            text(0x4D80, "ForgeCAD"),
            text(0x5741, "ForgeCAD"),
        ]
        .concat(),
    );
    let video = el(
        0xE0,
        &[uint(0xB0, size.0 as u64), uint(0xBA, size.1 as u64)].concat(),
    );
    let mut entry = [
        uint(0xD7, 1),
        uint(0x73C5, 1),
        uint(0x83, 1),
        text(0x86, codec.id()),
        uint(0x23E383, (1e9 / fps).round() as u64),
    ]
    .concat();
    if let Some(p) = codec_private {
        entry.extend(el(0x63A2, p));
    }
    entry.extend(video);
    let tracks = el(0x1654_AE6B, &el(0xAE, &entry));
    // clusters: a new one at each key frame and at least every second
    let mut clusters = Vec::new();
    let mut cues: Vec<(u64, usize)> = Vec::new();
    let mut k = 0;
    while k < frames.len() {
        let start = ms(k);
        let mut body = uint(0xE7, start);
        let first = k;
        while k < frames.len() && (k == first || (!frames[k].key && ms(k) - start < 1000)) {
            let rel = (ms(k) - start) as i16;
            let mut block = vec![0x81];
            block.extend(rel.to_be_bytes());
            block.push(if frames[k].key { 0x80 } else { 0x00 });
            block.extend(&frames[k].data);
            body.extend(el(0xA3, &block));
            k += 1;
        }
        if frames[first].key {
            cues.push((start, clusters.len()));
        }
        clusters.push(el(0x1F43_B675, &body));
    }
    // the cues point at clusters by their position from the segment's data start: info, tracks,
    // then the cues themselves come before the clusters, so their own size is solved for
    let cues_for = |cue_len: usize| -> Vec<u8> {
        let mut at = info.len() + tracks.len() + cue_len;
        let mut offsets = Vec::new();
        for c in &clusters {
            offsets.push(at);
            at += c.len();
        }
        let points: Vec<u8> = cues
            .iter()
            .flat_map(|(t, c)| {
                el(
                    0xBB,
                    &[
                        uint(0xB3, *t),
                        el(0xB7, &[uint(0xF7, 1), uint(0xF1, offsets[*c] as u64)].concat()),
                    ]
                    .concat(),
                )
            })
            .collect();
        el(0x1C53_BB6B, &points)
    };
    let mut cue = cues_for(0);
    for _ in 0..8 {
        let next = cues_for(cue.len());
        if next.len() == cue.len() {
            cue = next;
            break;
        }
        cue = next;
    }
    let segment_body = [info, tracks, cue, clusters.concat()].concat();
    let mut out = ebml;
    out.extend(el(0x1853_8067, &segment_body));
    Ok(out)
}

/// An EBML element read back: id, the body's range. For tests and for checking a file.
pub fn read(bytes: &[u8], at: usize) -> Option<(u32, usize, usize)> {
    let first = *bytes.get(at)?;
    let id_len = first.leading_zeros() as usize + 1;
    if id_len > 4 {
        return None;
    }
    let mut id = 0u32;
    for b in bytes.get(at..at + id_len)? {
        id = (id << 8) | *b as u32;
    }
    let s0 = *bytes.get(at + id_len)?;
    let s_len = s0.leading_zeros() as usize + 1;
    if s_len > 8 {
        return None;
    }
    let mut size = (s0 as u64) & (0xFF >> s_len);
    for b in bytes.get(at + id_len + 1..at + id_len + s_len)? {
        size = (size << 8) | *b as u64;
    }
    let start = at + id_len + s_len;
    let end = start.checked_add(size as usize)?;
    (end <= bytes.len()).then_some((id, start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn children(b: &[u8], mut at: usize, end: usize) -> Vec<(u32, usize, usize)> {
        let mut v = Vec::new();
        while at < end {
            let e = read(b, at).expect("an element");
            v.push(e);
            at = e.2;
        }
        v
    }

    #[test]
    fn the_file_is_well_formed_and_holds_every_frame() {
        let frames: Vec<Frame> = (0..75)
            .map(|k| Frame {
                data: vec![k as u8; 10 + k],
                key: k % 30 == 0,
            })
            .collect();
        let w = mux(Codec::Vp9, (64, 48), 30.0, &frames, None).unwrap();
        let top = children(&w, 0, w.len());
        assert_eq!(
            top.iter().map(|e| e.0).collect::<Vec<_>>(),
            [0x1A45_DFA3, 0x1853_8067]
        );
        let seg = top[1];
        let inside = children(&w, seg.1, seg.2);
        let ids: Vec<u32> = inside.iter().map(|e| e.0).collect();
        assert_eq!(&ids[..3], &[0x1549_A966, 0x1654_AE6B, 0x1C53_BB6B]);
        // every frame in a block, in order, its timestamp right
        let mut seen = 0;
        for c in inside.iter().filter(|e| e.0 == 0x1F43_B675) {
            let parts = children(&w, c.1, c.2);
            let (_, ts, te) = parts[0];
            let base = w[ts..te].iter().fold(0u64, |a, b| (a << 8) | *b as u64);
            for b in &parts[1..] {
                assert_eq!(b.0, 0xA3);
                let rel = i16::from_be_bytes([w[b.1 + 1], w[b.1 + 2]]) as u64;
                assert_eq!(base + rel, (seen as f64 * 1000.0 / 30.0).round() as u64);
                assert_eq!(w[b.1 + 3] & 0x80 != 0, frames[seen].key);
                assert_eq!(&w[b.1 + 4..b.2], frames[seen].data.as_slice());
                seen += 1;
            }
        }
        assert_eq!(seen, 75);
        // each cue points at a cluster
        let cues = inside[2];
        for p in children(&w, cues.1, cues.2) {
            let parts = children(&w, p.1, p.2);
            let pos = children(&w, parts[1].1, parts[1].2)[1];
            let off = w[pos.1..pos.2].iter().fold(0usize, |a, b| (a << 8) | *b as usize);
            assert_eq!(read(&w, seg.1 + off).unwrap().0, 0x1F43_B675);
        }
        if let Ok(dir) = std::env::var("WEBM_CHECK_DIR") {
            std::fs::write(format!("{dir}/t.webm"), &w).unwrap();
        }
    }

    /// A real VP9 stream (an IVF file, `IVF_FILE`, e.g. from ffmpeg) muxed, to check with a real
    /// player: writes `$WEBM_CHECK_DIR/vp9.webm`.
    #[test]
    fn a_real_vp9_stream_muxes() {
        let (Ok(ivf), Ok(dir)) = (std::env::var("IVF_FILE"), std::env::var("WEBM_CHECK_DIR")) else {
            return;
        };
        let b = std::fs::read(ivf).unwrap();
        assert_eq!(&b[..4], b"DKIF");
        let w = u16::from_le_bytes([b[12], b[13]]) as u32;
        let h = u16::from_le_bytes([b[14], b[15]]) as u32;
        let mut at = u16::from_le_bytes([b[6], b[7]]) as usize;
        let mut frames = Vec::new();
        while at + 12 <= b.len() {
            let n = u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize;
            let data = b[at + 12..at + 12 + n].to_vec();
            // VP9 profile 0: marker, profile, show_existing_frame, then frame_type (0 is key)
            let key = data[0] & 0x08 == 0 && data[0] & 0x04 == 0;
            frames.push(Frame { data, key });
            at += 12 + n;
        }
        let out = mux(Codec::Vp9, (w, h), 30.0, &frames, None).unwrap();
        std::fs::write(format!("{dir}/vp9.webm"), out).unwrap();
    }

    #[test]
    fn vints_round_trip() {
        for n in [0u64, 1, 126, 127, 128, 16382, 16383, 1 << 20, 1 << 40] {
            let v = vint(n);
            let len = v[0].leading_zeros() as usize + 1;
            assert_eq!(len, v.len());
            let mut x = (v[0] as u64) & (0xFF >> len);
            for b in &v[1..] {
                x = (x << 8) | *b as u64;
            }
            assert_eq!(x, n, "{n}");
        }
    }
}
