//! Text as the file has it: the drawing's code page before R2007, UTF-16 since. Kept as stored, so
//! a string read and written back is the same bytes.

/// A string as stored.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Str {
    /// Bytes in the drawing's code page (R2000, R2004).
    Cp(Vec<u8>),
    /// UTF-16 units (R2007 on).
    Wide(Vec<u16>),
}

impl Default for Str {
    fn default() -> Str {
        Str::Cp(vec![])
    }
}

impl Str {
    /// For a version: code-page bytes (Windows-1252, other characters as \U+XXXX) or UTF-16.
    pub fn new(s: &str, wide: bool) -> Str {
        if wide {
            Str::Wide(s.encode_utf16().collect())
        } else {
            Str::Cp(encode_1252(s))
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Str::Cp(b) => b.is_empty(),
            Str::Wide(w) => w.is_empty(),
        }
    }
    /// The code-page bytes (UTF-16 narrowed through Windows-1252).
    pub fn cp(&self) -> Vec<u8> {
        match self {
            Str::Cp(b) => b.clone(),
            Str::Wide(w) => encode_1252(&String::from_utf16_lossy(w)),
        }
    }
    /// UTF-16 units (code-page bytes read as Windows-1252).
    pub fn wide(&self) -> Vec<u16> {
        match self {
            Str::Cp(b) => decode_1252(b).encode_utf16().collect(),
            Str::Wide(w) => w.clone(),
        }
    }
    /// As text, reading code-page bytes as Windows-1252 and AutoCAD's \U+XXXX escapes.
    /// Trailing NULs (AutoCAD terminates some strings) are left off.
    pub fn text(&self) -> String {
        let s = match self {
            Str::Cp(b) => unescape(&decode_1252(b)),
            Str::Wide(w) => String::from_utf16_lossy(w),
        };
        s.trim_end_matches('\0').to_string()
    }
}

impl std::fmt::Display for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text())
    }
}

/// Windows-1252's 0x80..0x9F.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{2C6}',
    '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}', '\u{90}', '\u{2018}',
    '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{2DC}', '\u{2122}', '\u{161}',
    '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}', '\u{178}',
];

pub fn decode_1252(b: &[u8]) -> String {
    b.iter()
        .map(|&x| {
            if (0x80..0xA0).contains(&x) {
                HIGH[x as usize - 0x80]
            } else {
                x as char
            }
        })
        .collect()
}

pub fn encode_1252(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        let u = c as u32;
        if u < 0x80 || (0xA0..0x100).contains(&u) {
            out.push(u as u8);
        } else if let Some(k) = HIGH.iter().position(|&h| h == c) {
            out.push(0x80 + k as u8);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.extend_from_slice(format!("\\U+{unit:04X}").as_bytes());
            }
        }
    }
    out
}

/// AutoCAD's \U+XXXX escapes as the characters.
fn unescape(s: &str) -> String {
    if !s.contains("\\U+") {
        return s.to_string();
    }
    let mut units: Vec<u16> = vec![];
    let cs: Vec<char> = s.chars().collect();
    let mut k = 0;
    while k < cs.len() {
        if cs[k] == '\\' && k + 6 < cs.len() + 1 && cs.get(k + 1) == Some(&'U') && cs.get(k + 2) == Some(&'+')
        {
            let hex: String = cs[k + 3..(k + 7).min(cs.len())].iter().collect();
            if hex.len() == 4
                && let Ok(u) = u16::from_str_radix(&hex, 16)
            {
                units.push(u);
                k += 7;
                continue;
            }
        }
        let mut buf = [0u16; 2];
        units.extend_from_slice(cs[k].encode_utf16(&mut buf));
        k += 1;
    }
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_page_round_trip() {
        let s = "\u{2300}6 \u{b1}0.1 30\u{b0} \u{20AC}5 \u{4E2D}";
        let c = Str::new(s, false);
        assert_eq!(c.text(), s);
        assert_eq!(Str::new(s, true).text(), s);
        assert_eq!(decode_1252(&encode_1252("caf\u{e9}")), "caf\u{e9}");
    }
}
