//! Ways a file goes wrong. Generic byte edits for any file, and edits that know the format:
//! a STEP reference pointed elsewhere, an IGES pointer, a `.fg` count made huge, an XML element
//! repeated, a zip member made wrong and packed again. A mutant is a function of its seed file and
//! a number, so any case is made again from the two.

use forge_math::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Step,
    Iges,
    Fg,
    Zip,
    Text,
    Binary,
}

/// Numbers that break things: zero, signs, the ends of every width, the ends of floats.
const NUMS: &[&str] = &[
    "0",
    "-0.",
    "-1",
    "1.E308",
    "-1.E308",
    "1.E-308",
    "1E400",
    "NaN",
    "inf",
    "2147483647",
    "2147483648",
    "4294967295",
    "4294967296",
    "-2147483649",
    "9223372036854775807",
    "18446744073709551616",
    "99999999999999999999999999",
    "1000000",
    "1E20",
    "0.0000001",
    "65535",
    "65536",
];

const STEP_TOKENS: &[&[u8]] = &[
    b"#",
    b"(",
    b")",
    b",",
    b";",
    b"'",
    b"$",
    b"*",
    b"=",
    b".T.",
    b".F.",
    b".U.",
    b"#0",
    b"#1",
    b"#4294967295",
    b"#4294967296",
    b"#99999999",
    b"()",
    b"((((",
    b"\x00",
    b"\xff",
    b"/*",
    b"*/",
    b"''",
    b"\\X2\\",
    b"\\S\\",
    b"ENDSEC;",
    b"DATA;",
    b"END-ISO-10303-21;",
    b"#1=#1;",
    b"(#1,#1,#1)",
];

const FG_TOKENS: &[&str] = &[
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    ",",
    ":",
    "=",
    "\"",
    "#",
    "deg",
    "mm",
    "in",
    "thru",
    "count(",
    "1e308",
    "-1",
    "0",
    "NaN",
    "\u{feff}",
    "\u{0}",
    "é",
    "sketch",
    "extrude(",
    "pattern(",
    "lin(",
    "fillet(",
    "edges(of: ",
    "param ",
    "shared ",
    "fn ",
    "component ",
    "assembly ",
    "import(\"",
    "..",
    ".",
    "@",
    "\\",
    "\n",
    "\t",
];

/// A mutant of `seed`, its edits drawn from `r`; `other` is a second seed to splice from.
pub fn mutate(f: Format, seed: &[u8], other: &[u8], r: &mut Rng) -> Vec<u8> {
    let mut b = seed.to_vec();
    let edits = [1, 1, 1, 2, 3, 4, 8, 16, 64][r.below(9)];
    for _ in 0..edits {
        if f == Format::Zip {
            b = zip_edit(&b, other, r);
            continue;
        }
        // half the edits know the format, half are bytes
        if r.bool() {
            format_edit(f, &mut b, r);
        } else {
            byte_edit(&mut b, other, r);
        }
        // edits on edits can grow a mutant without end: at most 64 MB
        b.truncate(64 << 20);
    }
    b
}

fn at(b: &[u8], r: &mut Rng) -> usize {
    if b.is_empty() { 0 } else { r.below(b.len()) }
}

/// An edit that knows nothing of the format.
pub fn byte_edit(b: &mut Vec<u8>, other: &[u8], r: &mut Rng) {
    let n = b.len();
    let i = at(b, r);
    match r.below(11) {
        0 if n > 0 => b[i] ^= 1 << r.below(8),
        1 if n > 0 => b[i] = [0, 0xff, 0x7f, 0x80, b'0', b'9', b' '][r.below(7)],
        2 => {
            let k = r.below(64) + 1;
            b.drain(i..(i + k).min(n));
        }
        3 => b.truncate(i),
        4 => {
            // a run repeated: a list made long, a section doubled
            let k = (r.below(256) + 1).min(n - i.min(n));
            let run = b[i..i + k].to_vec();
            let times = [2, 3, 10, 100, 1000][r.below(5)];
            // one splice of the whole repeat (one each was quadratic), at most 64 MB
            let times = times.min((64 << 20) / run.len().max(1));
            b.splice(i..i, run.repeat(times));
        }
        5 if !other.is_empty() => {
            // a piece of another file
            let j = r.below(other.len());
            let k = (r.below(4096) + 1).min(other.len() - j);
            b.splice(i..i, other[j..j + k].iter().copied());
        }
        6 if n >= 4 => {
            // a little-endian count or offset made extreme
            let i = i.min(n - 4) & !3;
            let v: u32 = [0, 1, 0x7fff_ffff, 0xffff_ffff, 0x8000_0000, 0x0010_0000, n as u32][r.below(7)];
            b[i..i + 4].copy_from_slice(&v.to_le_bytes());
        }
        7 if n >= 2 => {
            let i = i.min(n - 2);
            let v: u16 = [0, 1, 0x7fff, 0xffff, 0x8000][r.below(5)];
            b[i..i + 2].copy_from_slice(&v.to_le_bytes());
        }
        8 if n > 1 => {
            // two pieces swapped
            let j = r.below(n);
            let (a, c) = (i.min(j), i.max(j));
            let k = (r.below(64) + 1).min(c - a).min(n - c);
            for d in 0..k {
                b.swap(a + d, c + d);
            }
        }
        9 => b.insert(i.min(n), r.below(256) as u8),
        _ => {
            let k = r.below(n.max(1)).min(n);
            b.truncate(k);
        }
    }
}

fn numbers(b: &[u8]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let d = |c: u8| c.is_ascii_digit();
        if d(b[i]) || (b[i] == b'-' && b.get(i + 1).copied().is_some_and(d)) {
            let s = i;
            i += 1;
            while i < b.len() && (d(b[i]) || matches!(b[i], b'.' | b'e' | b'E' | b'+' | b'-')) {
                i += 1;
            }
            // not a reference's digits or a name's
            let before = if s > 0 { b[s - 1] } else { b' ' };
            if before != b'#' && !before.is_ascii_alphabetic() && before != b'_' {
                out.push((s, i));
            }
        } else {
            i += 1;
        }
    }
    out
}

fn refs(b: &[u8]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'#' && b.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            let s = i;
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            out.push((s, i));
        } else {
            i += 1;
        }
    }
    out
}

/// Lines (or STEP instances, to their `;`).
fn spans(b: &[u8], end: u8) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut s = 0;
    for (i, &c) in b.iter().enumerate() {
        if c == end {
            out.push((s, i + 1));
            s = i + 1;
        }
    }
    if s < b.len() {
        out.push((s, b.len()));
    }
    out
}

fn pick<T: Copy>(v: &[T], r: &mut Rng) -> Option<T> {
    if v.is_empty() {
        None
    } else {
        Some(v[r.below(v.len())])
    }
}

fn replace(b: &mut Vec<u8>, (s, e): (usize, usize), with: &[u8]) {
    b.splice(s..e, with.iter().copied());
}

/// A number made extreme.
fn number_edit(b: &mut Vec<u8>, r: &mut Rng) {
    let ns = numbers(b);
    if let Some(sp) = pick(&ns, r) {
        let mut v = NUMS[r.below(NUMS.len())].to_string();
        // a STEP real keeps its dot
        if r.bool() && !v.contains(['.', 'E', 'N', 'i']) {
            v.push('.');
        }
        replace(b, sp, v.as_bytes());
    }
}

fn format_edit(f: Format, b: &mut Vec<u8>, r: &mut Rng) {
    match f {
        Format::Step => step_edit(b, r),
        Format::Iges => iges_edit(b, r),
        Format::Fg => fg_edit(b, r),
        Format::Text => text_edit(b, r),
        Format::Binary | Format::Zip => byte_edit(b, &[], r),
    }
}

fn step_edit(b: &mut Vec<u8>, r: &mut Rng) {
    match r.below(10) {
        0 | 1 => number_edit(b, r),
        2 | 3 => {
            // a reference pointed elsewhere: at itself, at a wrong kind, at nothing
            let rs = refs(b);
            if let (Some(a), Some(o)) = (pick(&rs, r), pick(&rs, r)) {
                let with = match r.below(4) {
                    0 => b"#0".to_vec(),
                    1 => b"#99999999".to_vec(),
                    _ => b[o.0..o.1].to_vec(),
                };
                replace(b, a, &with);
            }
        }
        4 => {
            // an instance dropped
            let ls = spans(b, b';');
            if let Some(sp) = pick(&ls, r) {
                replace(b, sp, b"");
            }
        }
        5 => {
            // an instance repeated, its id the same (a duplicate) or its references made long
            let ls = spans(b, b';');
            if let Some(sp) = pick(&ls, r) {
                let line = b[sp.0..sp.1].to_vec();
                b.splice(sp.1..sp.1, line.repeat(r.below(50) + 1));
            }
        }
        6 => {
            // a list nested deep, or made very long
            let i = at(b, r);
            let deep = [8, 33, 200, 10_000][r.below(4)];
            if r.bool() {
                let mut s = vec![b'('; deep];
                s.extend(std::iter::repeat_n(b')', deep));
                b.splice(i..i, s);
            } else {
                let mut s = vec![];
                for k in 0..deep {
                    s.extend_from_slice(format!(",#{}", k % 50 + 1).as_bytes());
                }
                if let Some(p) = b[i..].iter().position(|&c| c == b')') {
                    b.splice(i + p..i + p, s);
                }
            }
        }
        7 => {
            // a keyword swapped for another's
            let words = keywords(b);
            if let (Some(a), Some(o)) = (pick(&words, r), pick(&words, r)) {
                let with = b[o.0..o.1].to_vec();
                replace(b, a, &with);
            }
        }
        8 => {
            let i = at(b, r);
            let t = STEP_TOKENS[r.below(STEP_TOKENS.len())];
            b.splice(i..i, t.iter().copied());
        }
        _ => {
            // the instances of a list cut: one argument fewer or more
            let i = at(b, r);
            if let Some(p) = b[i..].iter().position(|&c| c == b',') {
                if r.bool() {
                    b.remove(i + p);
                } else {
                    b.splice(i + p..i + p, b",$".iter().copied());
                }
            }
        }
    }
}

/// `=KEYWORD(` names, as spans of the name.
fn keywords(b: &[u8]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            let mut s = i + 1;
            while s < b.len() && b[s] == b' ' {
                s += 1;
            }
            let mut e = s;
            while e < b.len() && (b[e].is_ascii_uppercase() || b[e] == b'_' || b[e].is_ascii_digit()) {
                e += 1;
            }
            if e > s + 2 && b.get(e) == Some(&b'(') {
                out.push((s, e));
            }
            i = e.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

/// IGES: 80-column cards. Its pointers (D section) and parameters (P section) made wrong, the
/// columns kept so the reader goes on past the card.
fn iges_edit(b: &mut Vec<u8>, r: &mut Rng) {
    let ls = spans(b, b'\n');
    let Some(sp) = pick(&ls, r) else { return };
    let line = &b[sp.0..sp.1];
    let section = line.get(72).copied().unwrap_or(b' ');
    match r.below(6) {
        0..=2 if section == b'D' || section == b'P' => {
            // one 8-column field (D) or one parameter (P) made extreme, the card's width kept
            let mut l = line.to_vec();
            if section == b'D' {
                let f = r.below(9) * 8;
                let v = [
                    "       0", "      -1", "99999999", "       1", " 1000000", "-9999999",
                ][r.below(6)];
                if l.len() >= f + 8 {
                    l[f..f + 8].copy_from_slice(v.as_bytes());
                }
            } else {
                let ns: Vec<_> = numbers(&l[..64.min(l.len())]).into_iter().collect();
                if let Some((s, e)) = pick(&ns, r) {
                    let v = ["0", "-1", "1.E308", "99999999", "1.E-308", "NaN", "2147483648"][r.below(7)];
                    let mut w = v.as_bytes().to_vec();
                    w.resize(e - s, b' ');
                    w.truncate(e - s);
                    l[s..e].copy_from_slice(&w);
                }
            }
            replace(b, sp, &l);
        }
        3 => replace(b, sp, b""),
        4 => {
            let l = line.to_vec();
            b.splice(sp.1..sp.1, l.repeat(r.below(20) + 1));
        }
        _ => number_edit(b, r),
    }
}

fn fg_edit(b: &mut Vec<u8>, r: &mut Rng) {
    match r.below(9) {
        0 | 1 => number_edit(b, r),
        2 => {
            let i = at(b, r);
            let t = FG_TOKENS[r.below(FG_TOKENS.len())];
            b.splice(i..i, t.bytes());
        }
        3 => {
            // nesting deep
            let i = at(b, r);
            let deep = [20, 200, 5000, 100_000][r.below(4)];
            let (o, c) = [("(", ")"), ("[", "]"), ("{", "}"), ("-", ""), ("(1+", ")")][r.below(5)];
            let mut s = o.repeat(deep);
            s.push('1');
            s.push_str(&c.repeat(deep));
            b.splice(i..i, s.bytes());
        }
        4 => {
            // a count made huge
            let i = at(b, r);
            let s = [
                "count(1000000000)",
                "count(99999)",
                "n: count(4294967296)",
                "step: 1e-300",
                "r: 1e300",
                "d: 0",
                "t: -0",
            ][r.below(7)];
            b.splice(i..i, s.bytes());
        }
        5 => {
            // a line dropped, repeated or moved
            let ls = spans(b, b'\n');
            if let Some(sp) = pick(&ls, r) {
                let l = b[sp.0..sp.1].to_vec();
                match r.below(3) {
                    0 => replace(b, sp, b""),
                    1 => {
                        b.splice(sp.1..sp.1, l.repeat(r.below(20) + 1));
                    }
                    _ => {
                        replace(b, sp, b"");
                        let j = at(b, r);
                        b.splice(j..j, l);
                    }
                }
            }
        }
        6 => {
            // a name used before it is made, or one of a builtin's
            let words = idents(b);
            if let (Some(a), Some(o)) = (pick(&words, r), pick(&words, r)) {
                let with = b[o.0..o.1].to_vec();
                replace(b, a, &with);
            }
        }
        7 => {
            // a string left open, or a comment
            let i = at(b, r);
            b.insert(i, b'"');
        }
        _ => text_edit(b, r),
    }
}

fn idents(b: &[u8]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_alphabetic() || b[i] == b'_' {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push((s, i));
        } else {
            i += 1;
        }
    }
    out
}

/// Any text: numbers, lines, and for XML an element repeated, a tag cut, an entity.
fn text_edit(b: &mut Vec<u8>, r: &mut Rng) {
    match r.below(7) {
        0 | 1 => number_edit(b, r),
        2 => {
            // an XML element (or any <..>) repeated or nested in itself
            let i = at(b, r);
            if let Some(s) = b[i..].iter().position(|&c| c == b'<').map(|p| p + i)
                && let Some(e) = b[s..].iter().position(|&c| c == b'>').map(|p| p + s + 1)
            {
                let tag = b[s..e].to_vec();
                b.splice(e..e, tag.repeat([1, 10, 1000][r.below(3)]));
            }
        }
        3 => {
            let i = at(b, r);
            let t = [
                "&#xFFFFFFFF;",
                "&amp",
                "&unknown;",
                "<![CDATA[",
                "]]>",
                "</",
                "<a>",
                "\"",
                "'",
                "=",
                "\u{0}",
                "<!DOCTYPE x [<!ENTITY a \"aaaaaaaaaa\">]>",
            ][r.below(12)];
            b.splice(i..i, t.bytes());
        }
        4 => {
            // an attribute's value swapped for another's
            let qs: Vec<(usize, usize)> = quoted(b);
            if let (Some(a), Some(o)) = (pick(&qs, r), pick(&qs, r)) {
                let with = b[o.0..o.1].to_vec();
                replace(b, a, &with);
            }
        }
        5 => {
            let ls = spans(b, b'\n');
            if let Some(sp) = pick(&ls, r) {
                replace(b, sp, b"");
            }
        }
        _ => byte_edit(b, &[], r),
    }
}

fn quoted(b: &[u8]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'"' {
            let s = i + 1;
            match b[s..].iter().position(|&c| c == b'"') {
                Some(p) => {
                    out.push((s, s + p));
                    i = s + p + 1;
                }
                None => break,
            }
        } else {
            i += 1;
        }
    }
    out
}

/// A zip's member made wrong and packed again, a member dropped or added, or the archive's own
/// bytes broken.
fn zip_edit(b: &[u8], other: &[u8], r: &mut Rng) -> Vec<u8> {
    let Some(mut ms) = crate::zip::unpack(b) else {
        let mut v = b.to_vec();
        byte_edit(&mut v, other, r);
        return v;
    };
    match r.below(8) {
        0 => {
            let mut v = b.to_vec();
            byte_edit(&mut v, other, r);
            v
        }
        1 if !ms.is_empty() => {
            ms.remove(r.below(ms.len()));
            crate::zip::pack(&ms)
        }
        2 if !ms.is_empty() => {
            // a member with another's name, or a name with a path in it
            let k = r.below(ms.len());
            ms[k].0 = ["../../x.brp", "Document.xml", "", "a/b/c.brp", "PadShape.brp"][r.below(5)].into();
            crate::zip::pack(&ms)
        }
        3 if !ms.is_empty() => {
            // a member deflated from far more than it seems: a bomb of zeros
            let k = r.below(ms.len());
            ms[k].1 = true;
            ms[k].2 = vec![b'0'; [1 << 20, 16 << 20][r.below(2)]];
            crate::zip::pack(&ms)
        }
        _ if !ms.is_empty() => {
            let k = r.below(ms.len());
            // XML and OpenCascade's .brp are both text
            let f = Format::Text;
            let mut m = ms[k].2.clone();
            for _ in 0..[1, 2, 4, 16][r.below(4)] {
                if r.bool() {
                    format_edit(f, &mut m, r);
                } else {
                    byte_edit(&mut m, other, r);
                }
            }
            ms[k].2 = m;
            if r.below(4) == 0 {
                ms[k].1 = !ms[k].1;
            }
            crate::zip::pack(&ms)
        }
        _ => b.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mutant_is_made_again_from_its_seed_and_number() {
        let s = b"#1=CARTESIAN_POINT('',(0.,1.,2.));\n#2=DIRECTION('',(0.,0.,1.));\n".to_vec();
        for k in 0..50 {
            let a = mutate(Format::Step, &s, &s, &mut Rng::new(k));
            let b = mutate(Format::Step, &s, &s, &mut Rng::new(k));
            assert_eq!(a, b);
        }
    }

    #[test]
    fn a_zip_packs_back_to_its_members() {
        for (_, z) in crate::zip::fcstd_seeds() {
            let ms = crate::zip::unpack(&z).expect("reads");
            assert_eq!(crate::zip::unpack(&crate::zip::pack(&ms)), Some(ms));
        }
    }
}
