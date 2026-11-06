//! Text as a drawing stores it, read into what to draw: AutoCAD's %% control codes (%%d degree,
//! %%p plus-minus, %%c diameter, %%% a percent sign, %%nnn character nnn, %%u and %%o underline
//! and overline on or off) and the \U+XXXX (a Unicode character) and \M+nXXXX (a double-byte
//! character for a big font) escapes.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Ch(char),
    /// %%nnn: the font's character (an SHX font's shape) nnn.
    Code(u16),
    /// A big-font character: lead byte high.
    Dbcs(u16),
    Under,
    Over,
}

fn hex(cs: &[char]) -> Option<u32> {
    if cs.len() != 4 || !cs.iter().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(&cs.iter().collect::<String>(), 16).ok()
}

pub fn tokens(s: &str) -> Vec<Tok> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = Vec::with_capacity(cs.len());
    let mut k = 0;
    while k < cs.len() {
        let c = cs[k];
        if c == '%' && cs.get(k + 1) == Some(&'%') {
            let t = match cs.get(k + 2).map(|c| c.to_ascii_lowercase()) {
                Some('d') => Some((Tok::Ch('°'), 3)),
                Some('p') => Some((Tok::Ch('±'), 3)),
                Some('c') => Some((Tok::Ch('\u{2300}'), 3)),
                Some('%') => Some((Tok::Ch('%'), 3)),
                Some('u') => Some((Tok::Under, 3)),
                Some('o') => Some((Tok::Over, 3)),
                Some(d) if d.is_ascii_digit() => {
                    let n = cs[k + 2..]
                        .iter()
                        .take(3)
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    let v: u16 = cs[k + 2..k + 2 + n]
                        .iter()
                        .collect::<String>()
                        .parse()
                        .unwrap_or(0);
                    Some((Tok::Code(v), 2 + n))
                }
                _ => None,
            };
            if let Some((t, n)) = t {
                out.push(t);
                k += n;
                continue;
            }
        }
        if c == '\\' && cs.get(k + 2) == Some(&'+') {
            match cs.get(k + 1) {
                Some('U' | 'u') => {
                    if let Some(u) = cs.get(k + 3..k + 7).and_then(hex) {
                        // A surrogate pair is two escapes.
                        if (0xd800..0xdc00).contains(&u)
                            && cs.get(k + 7) == Some(&'\\')
                            && let Some(lo) = cs.get(k + 10..k + 14).and_then(hex)
                            && (0xdc00..0xe000).contains(&lo)
                        {
                            let c = 0x10000 + ((u - 0xd800) << 10) + (lo - 0xdc00);
                            out.push(Tok::Ch(char::from_u32(c).unwrap_or('?')));
                            k += 14;
                            continue;
                        }
                        out.push(Tok::Ch(char::from_u32(u).unwrap_or('?')));
                        k += 7;
                        continue;
                    }
                }
                Some('M' | 'm') => {
                    if cs.get(k + 3).is_some_and(|c| c.is_ascii_digit())
                        && let Some(u) = cs.get(k + 4..k + 8).and_then(hex)
                    {
                        out.push(Tok::Dbcs(u as u16));
                        k += 8;
                        continue;
                    }
                }
                _ => {}
            }
        }
        out.push(Tok::Ch(c));
        k += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::Tok::*;
    use super::*;

    #[test]
    fn control_codes() {
        assert_eq!(tokens("%%c25"), vec![Ch('\u{2300}'), Ch('2'), Ch('5')]);
        assert_eq!(
            tokens("45%%D%%p1"),
            vec![Ch('4'), Ch('5'), Ch('°'), Ch('±'), Ch('1')]
        );
        assert_eq!(tokens("%%%"), vec![Ch('%')]);
        assert_eq!(tokens("%%uA%%U"), vec![Under, Ch('A'), Under]);
        assert_eq!(tokens("%%o"), vec![Over]);
        assert_eq!(tokens("%%1291"), vec![Code(129), Ch('1')]);
        assert_eq!(tokens("%%65x"), vec![Code(65), Ch('x')]);
        assert_eq!(tokens("50%"), vec![Ch('5'), Ch('0'), Ch('%')]);
        assert_eq!(tokens("%%z"), vec![Ch('%'), Ch('%'), Ch('z')]);
    }

    #[test]
    fn escapes() {
        assert_eq!(tokens("\\U+00B0C"), vec![Ch('°'), Ch('C')]);
        assert_eq!(tokens("\\U+D83D\\U+DE00"), vec![Ch('\u{1F600}')]);
        assert_eq!(tokens("\\M+18140"), vec![Dbcs(0x8140)]);
        assert_eq!(tokens("\\U+zz"), "\\U+zz".chars().map(Ch).collect::<Vec<_>>());
    }
}
