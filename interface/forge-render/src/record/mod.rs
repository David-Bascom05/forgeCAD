//! Photos and recordings of any environment (research/render/record.md): the muxers here are
//! plain functions of encoded frames, the same natively and in the browser; the encoding is the
//! bundled ffmpeg's (native, a subprocess) or the browser's own (WebCodecs).

#[cfg(not(target_arch = "wasm32"))]
pub mod ffmpeg;
pub mod mp4;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod zip;

/// The frame at `size` grown to even sides (H.264 wants them), its last column and row repeated;
/// the size it now is.
pub fn even(size: (u32, u32), rgba: &[u8]) -> ((u32, u32), Vec<u8>) {
    let (w, h) = (size.0 as usize, size.1 as usize);
    let (w2, h2) = (w + w % 2, h + h % 2);
    if (w2, h2) == (w, h) {
        return (size, rgba.to_vec());
    }
    let mut out = Vec::with_capacity(w2 * h2 * 4);
    for y in 0..h2 {
        let row = &rgba[y.min(h - 1) * w * 4..][..w * 4];
        out.extend_from_slice(row);
        if w2 > w {
            out.extend_from_slice(&row[(w - 1) * 4..]);
        }
    }
    ((w2 as u32, h2 as u32), out)
}

/// What the browser's encoder handed back (`window.forge_video_take`, web/app.js): little-endian
/// `[u32 n][avcC n bytes][u32 frames]([u8 key][u32 n][n bytes AVCC])..`; the frames and the avcC.
pub fn unpack_frames(b: &[u8]) -> Result<(Vec<mp4::Sample>, Vec<u8>), String> {
    let mut at = 0usize;
    let mut take = |n: usize| -> Result<&[u8], String> {
        let s = b.get(at..at + n).ok_or("the encoder's frames are cut short")?;
        at += n;
        Ok(s)
    };
    let u32le = |s: &[u8]| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize;
    let n = u32le(take(4)?);
    let avcc = take(n)?.to_vec();
    let frames = u32le(take(4)?);
    let mut samples = Vec::with_capacity(frames.min(1 << 16));
    for _ in 0..frames {
        let key = take(1)?[0] == 1;
        let n = u32le(take(4)?);
        samples.push(mp4::Sample {
            data: take(n)?.to_vec(),
            key,
        });
    }
    if avcc.is_empty() {
        return Err("the encoder gave no decoder configuration".into());
    }
    Ok((samples, avcc))
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_odd_frame_grows_to_even_sides_repeating_its_edge() {
        // 3 × 1: pixels 1, 2, 3 → 4 × 2: 1 2 3 3 / 1 2 3 3
        let px = |v: u8| [v, v, v, 255];
        let rgba: Vec<u8> = [1, 2, 3].into_iter().flat_map(px).collect();
        let (size, out) = super::even((3, 1), &rgba);
        assert_eq!(size, (4, 2));
        let want: Vec<u8> = [1, 2, 3, 3, 1, 2, 3, 3].into_iter().flat_map(px).collect();
        assert_eq!(out, want);
        let (size, same) = super::even((4, 2), &want);
        assert_eq!((size, &same), ((4, 2), &want));
    }

    #[test]
    fn the_browsers_frames_unpack() {
        let mut b = vec![];
        b.extend(3u32.to_le_bytes());
        b.extend([1, 2, 3]);
        b.extend(2u32.to_le_bytes());
        b.push(1);
        b.extend(2u32.to_le_bytes());
        b.extend([9, 9]);
        b.push(0);
        b.extend(1u32.to_le_bytes());
        b.push(7);
        let (s, avcc) = super::unpack_frames(&b).unwrap();
        assert_eq!(avcc, [1, 2, 3]);
        assert_eq!(
            (s.len(), s[0].key, &s[0].data[..], s[1].key, &s[1].data[..]),
            (2, true, &[9, 9][..], false, &[7][..])
        );
        assert!(super::unpack_frames(&b[..b.len() - 1]).is_err());
    }
}
