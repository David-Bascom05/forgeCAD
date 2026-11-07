//! A WAV file's samples as the recogniser takes them: mono, 16 kHz, -1..1.

/// 16-bit or 32-bit float PCM, any channels (averaged), any rate (resampled linearly).
pub fn read(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let u16_at = |k: usize| u16::from_le_bytes([bytes[k], bytes[k + 1]]);
    let u32_at = |k: usize| u32::from_le_bytes([bytes[k], bytes[k + 1], bytes[k + 2], bytes[k + 3]]);
    let (mut fmt, mut data) = (None, None);
    let mut k = 12;
    while k + 8 <= bytes.len() {
        let len = u32_at(k + 4) as usize;
        let body = k + 8;
        let end = (body + len).min(bytes.len());
        match &bytes[k..k + 4] {
            b"fmt " if len >= 16 => {
                fmt = Some((
                    u16_at(body),
                    u16_at(body + 2),
                    u32_at(body + 4),
                    u16_at(body + 14),
                ))
            }
            b"data" => data = Some(&bytes[body..end]),
            _ => {}
        }
        k = body + len + (len & 1);
    }
    let (format, channels, rate, bits) = fmt.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    let ch = channels.max(1) as usize;
    let frames: Vec<f32> = match (format, bits) {
        (1, 16) => data
            .chunks_exact(2 * ch)
            .map(|f| {
                f.chunks_exact(2)
                    .map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0)
                    .sum::<f32>()
                    / ch as f32
            })
            .collect(),
        (3, 32) => data
            .chunks_exact(4 * ch)
            .map(|f| {
                f.chunks_exact(4)
                    .map(|s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
                    .sum::<f32>()
                    / ch as f32
            })
            .collect(),
        _ => return Err(format!("{bits}-bit WAV format {format} isn't read")),
    };
    Ok(resample(&frames, rate, 16_000))
}

pub fn resample(x: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || x.is_empty() {
        return x.to_vec();
    }
    let n = (x.len() as u64 * to as u64 / from as u64) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 * from as f64 / to as f64;
            let k = t as usize;
            let f = (t - k as f64) as f32;
            let a = x[k.min(x.len() - 1)];
            let b = x[(k + 1).min(x.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut o = b"RIFF".to_vec();
        o.extend((36 + data.len() as u32).to_le_bytes());
        o.extend(b"WAVEfmt ");
        o.extend(16u32.to_le_bytes());
        o.extend(1u16.to_le_bytes());
        o.extend(channels.to_le_bytes());
        o.extend(rate.to_le_bytes());
        o.extend((rate * 2 * channels as u32).to_le_bytes());
        o.extend((2 * channels).to_le_bytes());
        o.extend(16u16.to_le_bytes());
        o.extend(b"data");
        o.extend((data.len() as u32).to_le_bytes());
        o.extend(data);
        o
    }

    #[test]
    fn reads_and_resamples() {
        let x = read(&wav(16_000, 1, &[0, 16384, -32768])).unwrap();
        assert_eq!(x, [0.0, 0.5, -1.0]);
        let st = read(&wav(16_000, 2, &[16384, 0, -16384, -16384])).unwrap();
        assert_eq!(st, [0.25, -0.5]);
        assert_eq!(read(&wav(48_000, 1, &[0; 4800])).unwrap().len(), 1600);
        assert!(read(b"RIFFxxxxWAVE").is_err());
        assert!(read(b"nope").is_err());
    }
}
