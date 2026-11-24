//! MP4 (ISO/IEC 14496-12, the ISO base media file format, with 14496-15's `avc1` sample entry):
//! one H.264 video track of frames the browser's WebCodecs encoder made (length-prefixed NAL units,
//! and its `avcC` decoder configuration). The index (`moov`) comes before the data, so the file
//! starts playing before it has all arrived; all frames sit in one chunk. Frames are in decode
//! order with no reordering (no B-frames: no `ctts`), as WebCodecs and openh264 encode by default.

/// An encoded frame: its NAL units each behind a 4-byte big-endian length (AVCC), and whether it
/// is a key (IDR) frame.
#[derive(Clone, Debug)]
pub struct Sample {
    pub data: Vec<u8>,
    pub key: bool,
}

/// The time scale: 90 kHz, as video usually counts.
const SCALE: u32 = 90_000;

fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend(kind);
    out.extend(body);
    out
}

/// A full box: version and flags before its body.
fn full(kind: &[u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut b = vec![version];
    b.extend(&flags.to_be_bytes()[1..]);
    b.extend(body);
    bx(kind, &b)
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// The identity matrix, as `mvhd` and `tkhd` write it (16.16 and 2.30 fixed point).
const MATRIX: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];

/// The frames, `fps` a second, `size` pixels, as one MP4 file; `avcc` is the decoder
/// configuration record (WebCodecs' `decoderConfig.description`).
pub fn mux(size: (u32, u32), fps: f64, samples: &[Sample], avcc: &[u8]) -> Result<Vec<u8>, String> {
    if samples.is_empty() {
        return Err("no frames".into());
    }
    if !samples[0].key {
        return Err("the first frame must be a key frame".into());
    }
    if !(fps > 0.0 && fps <= 1000.0) {
        return Err(format!("{fps} frames a second"));
    }
    if avcc.len() < 7 || avcc[0] != 1 {
        return Err("not an AVC decoder configuration record".into());
    }
    let (w, h) = size;
    if w == 0 || h == 0 || w > 0xffff || h > 0xffff {
        return Err(format!("{w} x {h}"));
    }
    let delta = (SCALE as f64 / fps).round() as u32;
    let duration = delta as u64 * samples.len() as u64;
    let ftyp = bx(
        b"ftyp",
        &[b"isom".as_slice(), &be32(0x200), b"isomiso2avc1mp41"].concat(),
    );

    let mut mvhd = vec![0u8; 0];
    mvhd.extend(be32(0));
    mvhd.extend(be32(0));
    mvhd.extend(be32(SCALE));
    mvhd.extend(be32(duration as u32));
    mvhd.extend(be32(0x0001_0000)); // rate 1.0
    mvhd.extend([0x01, 0x00]); // volume 1.0
    mvhd.extend([0u8; 10]);
    for m in MATRIX {
        mvhd.extend(be32(m));
    }
    mvhd.extend([0u8; 24]);
    mvhd.extend(be32(2)); // next track id
    let mvhd = full(b"mvhd", 0, 0, &mvhd);

    let mut tkhd = vec![];
    tkhd.extend(be32(0));
    tkhd.extend(be32(0));
    tkhd.extend(be32(1)); // track id
    tkhd.extend(be32(0));
    tkhd.extend(be32(duration as u32));
    tkhd.extend([0u8; 8]);
    tkhd.extend([0u8; 4]); // layer, alternate group
    tkhd.extend([0u8; 4]); // volume 0 for video, reserved
    for m in MATRIX {
        tkhd.extend(be32(m));
    }
    tkhd.extend(be32(w << 16));
    tkhd.extend(be32(h << 16));
    let tkhd = full(b"tkhd", 0, 3, &tkhd); // enabled, in movie

    let mut mdhd = vec![];
    mdhd.extend(be32(0));
    mdhd.extend(be32(0));
    mdhd.extend(be32(SCALE));
    mdhd.extend(be32(duration as u32));
    mdhd.extend([0x55, 0xC4, 0, 0]); // language "und", pre-defined
    let mdhd = full(b"mdhd", 0, 0, &mdhd);
    let hdlr = full(
        b"hdlr",
        0,
        0,
        &[&[0u8; 4][..], b"vide", &[0u8; 12], b"ForgeCAD video\0"].concat(),
    );
    let vmhd = full(b"vmhd", 0, 1, &[0u8; 8]);
    let dinf = bx(
        b"dinf",
        &full(b"dref", 0, 0, &[&be32(1)[..], &full(b"url ", 0, 1, &[])].concat()),
    );

    let mut avc1 = vec![0u8; 6];
    avc1.extend([0, 1]); // data reference index
    avc1.extend([0u8; 16]);
    avc1.extend((w as u16).to_be_bytes());
    avc1.extend((h as u16).to_be_bytes());
    avc1.extend(be32(0x0048_0000)); // 72 dpi
    avc1.extend(be32(0x0048_0000));
    avc1.extend(be32(0));
    avc1.extend([0, 1]); // one frame per sample
    avc1.extend([0u8; 32]); // compressor name
    avc1.extend([0, 0x18, 0xff, 0xff]); // depth 24, pre-defined -1
    avc1.extend(bx(b"avcC", avcc));
    let stsd = full(b"stsd", 0, 0, &[&be32(1)[..], &bx(b"avc1", &avc1)].concat());
    let stts = full(
        b"stts",
        0,
        0,
        &[be32(1), be32(samples.len() as u32), be32(delta)].concat(),
    );
    let keys: Vec<u8> = samples
        .iter()
        .enumerate()
        .filter(|(_, s)| s.key)
        .flat_map(|(k, _)| be32(k as u32 + 1))
        .collect();
    let stss = full(
        b"stss",
        0,
        0,
        &[&be32((keys.len() / 4) as u32)[..], &keys].concat(),
    );
    let stsc = full(
        b"stsc",
        0,
        0,
        &[be32(1), be32(1), be32(samples.len() as u32), be32(1)].concat(),
    );
    let mut stsz = vec![];
    stsz.extend(be32(0));
    stsz.extend(be32(samples.len() as u32));
    for s in samples {
        stsz.extend(be32(
            u32::try_from(s.data.len()).map_err(|_| "a frame past 4 GB")?,
        ));
    }
    let stsz = full(b"stsz", 0, 0, &stsz);
    // the chunk's offset: the whole moov (its size doesn't depend on the value) and mdat's header
    let moov_with = |offset: u64| {
        let co64 = full(b"co64", 0, 0, &[&be32(1)[..], &offset.to_be_bytes()].concat());
        let stbl = bx(b"stbl", &[&stsd[..], &stts, &stss, &stsc, &stsz, &co64].concat());
        let minf = bx(b"minf", &[&vmhd[..], &dinf, &stbl].concat());
        let mdia = bx(b"mdia", &[&mdhd[..], &hdlr, &minf].concat());
        let trak = bx(b"trak", &[&tkhd[..], &mdia].concat());
        bx(b"moov", &[&mvhd[..], &trak].concat())
    };
    let data_len: u64 = samples.iter().map(|s| s.data.len() as u64).sum();
    let large = data_len + 8 > u32::MAX as u64;
    let mdat_head = if large { 16 } else { 8 };
    let moov_len = moov_with(0).len() as u64;
    let offset = ftyp.len() as u64 + moov_len + mdat_head;
    let mut out = ftyp;
    out.extend(moov_with(offset));
    if large {
        out.extend(be32(1));
        out.extend(b"mdat");
        out.extend((data_len + 16).to_be_bytes());
    } else {
        out.extend(be32((data_len + 8) as u32));
        out.extend(b"mdat");
    }
    for s in samples {
        out.extend(&s.data);
    }
    Ok(out)
}

/// An Annex B stream (start codes, as ffmpeg's raw `.h264` writes it) as AVCC samples, one per
/// access unit, and the `avcC` record from its first SPS and PPS. For tests and for streams from
/// encoders that write start codes.
pub fn from_annex_b(stream: &[u8]) -> Result<(Vec<Sample>, Vec<u8>), String> {
    // the NAL units between start codes
    let mut nals: Vec<&[u8]> = Vec::new();
    let mut k = 0;
    let mut start = None;
    while k + 3 <= stream.len() {
        if stream[k] == 0 && stream[k + 1] == 0 && stream[k + 2] == 1 {
            if let Some(s) = start {
                let mut end = k;
                while end > s && stream[end - 1] == 0 {
                    end -= 1;
                }
                nals.push(&stream[s..end]);
            }
            k += 3;
            start = Some(k);
        } else {
            k += 1;
        }
    }
    if let Some(s) = start {
        nals.push(&stream[s..]);
    }
    let (mut sps, mut pps) = (None, None);
    let mut samples: Vec<Sample> = Vec::new();
    let mut cur = Sample {
        data: Vec::new(),
        key: false,
    };
    for n in nals {
        if n.is_empty() {
            continue;
        }
        let kind = n[0] & 0x1f;
        match kind {
            7 => {
                sps.get_or_insert(n.to_vec());
            }
            8 => {
                pps.get_or_insert(n.to_vec());
            }
            9 => {
                // an access unit delimiter starts a new picture
                if !cur.data.is_empty() {
                    samples.push(std::mem::replace(
                        &mut cur,
                        Sample {
                            data: Vec::new(),
                            key: false,
                        },
                    ));
                }
            }
            1 | 5 => {
                // a slice whose first_mb_in_slice is 0 (its first bit set) starts a picture
                let first_slice = n.get(1).is_some_and(|b| b & 0x80 != 0);
                if first_slice && !cur.data.is_empty() {
                    samples.push(std::mem::replace(
                        &mut cur,
                        Sample {
                            data: Vec::new(),
                            key: false,
                        },
                    ));
                }
                cur.key |= kind == 5;
                cur.data.extend((n.len() as u32).to_be_bytes());
                cur.data.extend(n);
            }
            _ => {}
        }
    }
    if !cur.data.is_empty() {
        samples.push(cur);
    }
    let (Some(sps), Some(pps)) = (sps, pps) else {
        return Err("no SPS and PPS in the stream".into());
    };
    if sps.len() < 4 {
        return Err("a short SPS".into());
    }
    let mut avcc = vec![1, sps[1], sps[2], sps[3], 0xff, 0xe1];
    avcc.extend((sps.len() as u16).to_be_bytes());
    avcc.extend(&sps);
    avcc.push(1);
    avcc.extend((pps.len() as u16).to_be_bytes());
    avcc.extend(&pps);
    Ok((samples, avcc))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box's kind and its body's range, from `at`.
    fn read(b: &[u8], at: usize) -> ([u8; 4], usize, usize) {
        let size = u32::from_be_bytes(b[at..at + 4].try_into().unwrap()) as usize;
        let kind: [u8; 4] = b[at + 4..at + 8].try_into().unwrap();
        (kind, at + 8, at + size)
    }

    fn find(b: &[u8], from: usize, end: usize, path: &[&[u8; 4]]) -> Option<(usize, usize)> {
        let mut at = from;
        while at < end {
            let (k, s, e) = read(b, at);
            if &k == path[0] {
                if path.len() == 1 {
                    return Some((s, e));
                }
                // the sample description and a few full boxes hold their children after a header
                let skip = match &k {
                    b"stsd" => 8,
                    b"avc1" => 78,
                    _ => 0,
                };
                return find(b, s + skip, e, &path[1..]);
            }
            at = e;
        }
        None
    }

    fn fake(n: usize) -> (Vec<Sample>, Vec<u8>) {
        let samples = (0..n)
            .map(|k| Sample {
                data: [&(3u32).to_be_bytes()[..], &[0x65, k as u8, 7]].concat(),
                key: k % 10 == 0,
            })
            .collect();
        (samples, vec![1, 0x42, 0xc0, 0x1e, 0xff, 0xe1, 0, 0])
    }

    #[test]
    fn the_index_points_at_every_frame() {
        let (samples, avcc) = fake(25);
        let m = mux((320, 240), 25.0, &samples, &avcc).unwrap();
        assert_eq!(&m[4..8], b"ftyp");
        let (s, e) = find(
            &m,
            0,
            m.len(),
            &[b"moov", b"trak", b"mdia", b"minf", b"stbl", b"stsz"],
        )
        .unwrap();
        let n = u32::from_be_bytes(m[s + 8..s + 12].try_into().unwrap()) as usize;
        assert_eq!(n, 25);
        assert_eq!(e - s, 12 + 4 * 25);
        let (cs, _) = find(
            &m,
            0,
            m.len(),
            &[b"moov", b"trak", b"mdia", b"minf", b"stbl", b"co64"],
        )
        .unwrap();
        let offset = u64::from_be_bytes(m[cs + 8..cs + 16].try_into().unwrap()) as usize;
        // the frames lie end to end from that offset
        let mut at = offset;
        for smp in &samples {
            assert_eq!(&m[at..at + smp.data.len()], smp.data.as_slice());
            at += smp.data.len();
        }
        assert_eq!(at, m.len());
        let (ks, _) = find(
            &m,
            0,
            m.len(),
            &[b"moov", b"trak", b"mdia", b"minf", b"stbl", b"stss"],
        )
        .unwrap();
        assert_eq!(
            u32::from_be_bytes(m[ks + 4..ks + 8].try_into().unwrap()),
            3,
            "keys at 1, 11, 21"
        );
        assert!(
            find(
                &m,
                0,
                m.len(),
                &[
                    b"moov", b"trak", b"mdia", b"minf", b"stbl", b"stsd", b"avc1", b"avcC"
                ]
            )
            .is_some()
        );
    }

    #[test]
    fn bad_input_is_refused() {
        let (mut s, a) = fake(3);
        assert!(mux((0, 10), 30.0, &s, &a).is_err());
        assert!(mux((10, 10), 30.0, &s, &[0, 1]).is_err());
        s[0].key = false;
        assert!(mux((10, 10), 30.0, &s, &a).is_err());
    }

    /// A real H.264 stream (Annex B, `H264_FILE`, e.g. ffmpeg's `-f h264`) muxed, for a real
    /// player to check: writes `$MP4_CHECK_DIR/t.mp4`.
    #[test]
    fn a_real_stream_muxes() {
        let (Ok(f), Ok(dir)) = (std::env::var("H264_FILE"), std::env::var("MP4_CHECK_DIR")) else {
            return;
        };
        let (samples, avcc) = from_annex_b(&std::fs::read(f).unwrap()).unwrap();
        let w: u32 = std::env::var("H264_W").unwrap().parse().unwrap();
        let h: u32 = std::env::var("H264_H").unwrap().parse().unwrap();
        let m = mux((w, h), 30.0, &samples, &avcc).unwrap();
        std::fs::write(format!("{dir}/t.mp4"), m).unwrap();
    }
}
