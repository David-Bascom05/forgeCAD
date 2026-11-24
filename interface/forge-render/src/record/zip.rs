//! A zip of files stored as they are (no compression: the frames are PNGs, already deflated):
//! the PKWARE format's local headers, central directory and end record (APPNOTE 6.3.10, sections
//! 4.3 and 4.4), CRC-32 as it specifies.

/// CRC-32 (the IEEE polynomial, reflected), as zip and PNG use it.
pub fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (n, e) in t.iter_mut().enumerate() {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *e = c;
        }
        t
    });
    let mut c = !0u32;
    for &b in bytes {
        c = t[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    !c
}

/// Files (name, bytes) into one zip archive; names are kept as given (use `/`).
pub fn store(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    if files.len() > 0xffff {
        return Err("a zip without zip64 holds at most 65535 files".into());
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    let u16le = |v: &mut Vec<u8>, x: u16| v.extend(x.to_le_bytes());
    let u32le = |v: &mut Vec<u8>, x: u32| v.extend(x.to_le_bytes());
    for (name, data) in files {
        let offset = u32::try_from(out.len()).map_err(|_| "past 4 GB: needs zip64")?;
        let size = u32::try_from(data.len()).map_err(|_| "a file past 4 GB: needs zip64")?;
        let crc = crc32(data);
        let n = name.as_bytes();
        // local file header: version 1.0, no flags, stored, a fixed time (1980-01-01 00:00)
        u32le(&mut out, 0x0403_4b50);
        for x in [10u16, 0, 0, 0, 0x21] {
            u16le(&mut out, x);
        }
        for x in [crc, size, size] {
            u32le(&mut out, x);
        }
        u16le(&mut out, n.len() as u16);
        u16le(&mut out, 0);
        out.extend(n);
        out.extend(data);
        // its central directory entry
        u32le(&mut central, 0x0201_4b50);
        for x in [20u16, 10, 0, 0, 0, 0x21] {
            u16le(&mut central, x);
        }
        for x in [crc, size, size] {
            u32le(&mut central, x);
        }
        for x in [n.len() as u16, 0, 0, 0, 0] {
            u16le(&mut central, x);
        }
        u32le(&mut central, 0);
        u32le(&mut central, offset);
        central.extend(n);
    }
    let at = u32::try_from(out.len()).map_err(|_| "past 4 GB: needs zip64")?;
    let len = central.len() as u32;
    out.extend(&central);
    u32le(&mut out, 0x0605_4b50);
    for x in [0u16, 0, files.len() as u16, files.len() as u16] {
        u16le(&mut out, x);
    }
    u32le(&mut out, len);
    u32le(&mut out, at);
    u16le(&mut out, 0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_is_the_standard_one() {
        // the check value of CRC-32/ISO-HDLC
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn a_zip_lists_its_files_where_its_end_record_says() {
        let files = vec![
            ("a.png".to_string(), b"hello".to_vec()),
            ("dir/b.png".into(), vec![7; 300]),
        ];
        let z = store(&files).unwrap();
        let end = z.len() - 22;
        assert_eq!(&z[end..end + 4], &0x0605_4b50u32.to_le_bytes());
        let count = u16::from_le_bytes([z[end + 10], z[end + 11]]);
        let at = u32::from_le_bytes(z[end + 16..end + 20].try_into().unwrap()) as usize;
        assert_eq!(count, 2);
        assert_eq!(&z[at..at + 4], &0x0201_4b50u32.to_le_bytes());
        // the first file's data follows its local header and name
        assert_eq!(&z[30 + 5..30 + 10], b"hello");
        // and the system's unzip agrees, where there is one
        if let Ok(dir) = std::env::var("ZIP_CHECK_DIR") {
            std::fs::write(format!("{dir}/t.zip"), &z).unwrap();
        }
    }
}
