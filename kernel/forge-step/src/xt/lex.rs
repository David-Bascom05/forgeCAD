//! The bytes of a transmit file as the XT Format Reference (chapter 3) lays them out: the text
//! form, and the neutral, typed and bare binary forms. Each read says what it wanted when the
//! bytes run out or do not make it; nothing here panics on any input.

/// Text and binary null values (ch. 3.2, 3.3): read as NaN.
const NULL_INT: i64 = -32764;
const NULL_REAL: f64 = -3.14158e13;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Form {
    Text,
    /// Binary with its byte order: big-endian for neutral binary.
    Binary {
        little: bool,
    },
}

/// Where a read failed, and what it wanted there.
#[derive(Clone, Debug, PartialEq)]
pub struct LexError {
    pub at: usize,
    pub want: &'static str,
}

pub(crate) type L<T> = Result<T, LexError>;

pub(crate) struct Lex<'a> {
    b: &'a [u8],
    pub(crate) at: usize,
    pub(crate) form: Form,
}

/// A text file's data with its record breaks taken out: newlines, carriage returns, and the
/// spaces a writer pads a record with before its break (ch. 3.2). Leading spaces stay.
pub(crate) fn unwrap_text(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    for line in b.split(|&c| c == b'\n') {
        let mut l = line;
        while let [rest @ .., b'\r' | b' '] = l {
            l = rest;
        }
        out.extend(l.iter().filter(|&&c| c != b'\r'));
    }
    out
}

impl<'a> Lex<'a> {
    pub(crate) fn new(b: &'a [u8], form: Form) -> Lex<'a> {
        Lex { b, at: 0, form }
    }

    fn fail<T>(&self, want: &'static str) -> L<T> {
        Err(LexError { at: self.at, want })
    }

    fn take(&mut self, n: usize, want: &'static str) -> L<&'a [u8]> {
        match self.b.get(self.at..self.at.saturating_add(n)) {
            Some(s) => {
                self.at += n;
                Ok(s)
            }
            None => self.fail(want),
        }
    }

    #[cfg(test)]
    pub(crate) fn done(&self) -> bool {
        self.at >= self.b.len()
    }

    /// A text number up to its space, or a lone `?`.
    fn token(&mut self, want: &'static str) -> L<Option<&'a [u8]>> {
        let rest = self.b.get(self.at..).unwrap_or(&[]);
        if rest.first() == Some(&b'?') {
            self.at += 1;
            return Ok(None);
        }
        let Some(n) = rest.iter().position(|&c| c == b' ') else {
            // the last token of a file may end it
            if rest.is_empty() {
                return self.fail(want);
            }
            self.at = self.b.len();
            return Ok(Some(rest));
        };
        if n == 0 {
            return self.fail(want);
        }
        self.at += n + 1;
        Ok(Some(&rest[..n]))
    }

    fn text_int(&mut self, want: &'static str) -> L<Option<i64>> {
        let at = self.at;
        match self.token(want)? {
            None => Ok(None),
            Some(t) => match std::str::from_utf8(t).ok().and_then(|s| s.parse::<i64>().ok()) {
                Some(x) => Ok(Some(x)),
                None => Err(LexError { at, want }),
            },
        }
    }

    fn be<const N: usize>(&mut self, want: &'static str) -> L<[u8; N]> {
        let s = self.take(N, want)?;
        let mut a = [0u8; N];
        a.copy_from_slice(s);
        if let Form::Binary { little: true } = self.form {
            a.reverse();
        }
        Ok(a)
    }

    fn i16b(&mut self, want: &'static str) -> L<i16> {
        Ok(i16::from_be_bytes(self.be::<2>(want)?))
    }

    fn i32b(&mut self, want: &'static str) -> L<i32> {
        Ok(i32::from_be_bytes(self.be::<4>(want)?))
    }

    /// A whole number in text, or a 2-byte one in binary (node types, shorts).
    pub(crate) fn short(&mut self, want: &'static str) -> L<Option<i64>> {
        match self.form {
            Form::Text => self.text_int(want),
            Form::Binary { .. } => {
                let x = i64::from(self.i16b(want)?);
                Ok((x != NULL_INT).then_some(x))
            }
        }
    }

    /// A 4-byte integer in binary (`d` fields, variable lengths).
    pub(crate) fn int(&mut self, want: &'static str) -> L<Option<i64>> {
        match self.form {
            Form::Text => self.text_int(want),
            Form::Binary { .. } => {
                let x = i64::from(self.i32b(want)?);
                Ok((x != NULL_INT).then_some(x))
            }
        }
    }

    /// A byte (`u` fields, counts in the embedded schema).
    pub(crate) fn byte(&mut self, want: &'static str) -> L<Option<i64>> {
        match self.form {
            Form::Text => self.text_int(want),
            Form::Binary { .. } => Ok(Some(i64::from(self.take(1, want)?[0]))),
        }
    }

    /// A pointer index or positive integer: in binary one short, or two for 32767 and over.
    pub(crate) fn index(&mut self, want: &'static str) -> L<Option<i64>> {
        match self.form {
            Form::Text => self.text_int(want),
            Form::Binary { .. } => {
                let r = self.i16b(want)?;
                let (q, r) = if r < 0 {
                    (i64::from(self.i16b(want)?), -i64::from(r))
                } else {
                    (0, i64::from(r))
                };
                Ok(Some(q * 32767 + r - 1))
            }
        }
    }

    /// A character: in text unspaced, with the escapes of ch. 3.2 (`\9` is nine spaces and
    /// comes back as one call's worth each: see `chars`).
    pub(crate) fn char(&mut self, want: &'static str) -> L<u8> {
        let c = self.take(1, want)?[0];
        if self.form == Form::Text && c == b'\\' {
            let e = self.take(1, want)?[0];
            return Ok(match e {
                b'0' => 0,
                b'n' => b'\r',
                b'r' => b'\n',
                other => other,
            });
        }
        Ok(c)
    }

    /// `n` characters of a character array, text escapes expanded.
    pub(crate) fn chars(&mut self, n: usize, want: &'static str) -> L<Vec<u8>> {
        let mut out = Vec::with_capacity(n.min(1 << 16));
        while out.len() < n {
            if self.form == Form::Text
                && self.b.get(self.at) == Some(&b'\\')
                && self.b.get(self.at + 1) == Some(&b'9')
            {
                self.at += 2;
                out.extend(std::iter::repeat_n(b' ', 9));
                continue;
            }
            out.push(self.char(want)?);
        }
        out.truncate(n);
        Ok(out)
    }

    /// A logical: `T` or `F` in text, a byte in binary.
    pub(crate) fn logical(&mut self, want: &'static str) -> L<bool> {
        let c = self.take(1, want)?[0];
        match (self.form, c) {
            (Form::Text, b'T') | (Form::Binary { .. }, 1) => Ok(true),
            (Form::Text, b'F') | (Form::Binary { .. }, 0) => Ok(false),
            _ => {
                self.at -= 1;
                self.fail(want)
            }
        }
    }

    pub(crate) fn real(&mut self, want: &'static str) -> L<f64> {
        match self.form {
            Form::Text => {
                let at = self.at;
                match self.token(want)? {
                    None => Ok(f64::NAN),
                    Some(t) => match std::str::from_utf8(t).ok().and_then(|s| s.parse::<f64>().ok()) {
                        Some(x) if x.is_finite() => Ok(x),
                        _ => Err(LexError { at, want }),
                    },
                }
            }
            Form::Binary { .. } => {
                let x = f64::from_be_bytes(self.be::<8>(want)?);
                Ok(if x == NULL_REAL || !x.is_finite() {
                    f64::NAN
                } else {
                    x
                })
            }
        }
    }

    /// `n` reals that a text file writes as one `?` when null (vectors, intervals, boxes).
    pub(crate) fn reals(&mut self, n: usize, out: &mut Vec<f64>, want: &'static str) -> L<()> {
        if self.form == Form::Text && self.b.get(self.at) == Some(&b'?') {
            self.at += 1;
            out.extend(std::iter::repeat_n(f64::NAN, n));
            return Ok(());
        }
        for _ in 0..n {
            let x = self.real(want)?;
            out.push(x);
        }
        Ok(())
    }

    /// A short string of the embedded schema: its length (a number in text, a byte in binary),
    /// then its characters.
    pub(crate) fn short_string(&mut self, want: &'static str) -> L<Vec<u8>> {
        let n = match self.form {
            Form::Text => self.text_int(want)?,
            Form::Binary { .. } => Some(i64::from(self.take(1, want)?[0])),
        };
        match n {
            Some(n) if (0..=1 << 16).contains(&n) => self.chars(n as usize, want),
            _ => self.fail(want),
        }
    }

    /// A header string: the modeller version (2-byte length in binary) or the schema (4-byte).
    pub(crate) fn header_string(&mut self, long: bool, want: &'static str) -> L<Vec<u8>> {
        let n = match self.form {
            Form::Text => self.text_int(want)?,
            Form::Binary { .. } if long => Some(i64::from(self.i32b(want)?)),
            Form::Binary { .. } => Some(i64::from(self.i16b(want)?)),
        };
        match n {
            Some(n) if (0..=4096).contains(&n) => Ok(self.take(n as usize, want)?.to_vec()),
            _ => self.fail(want),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_records_join_and_lose_their_padding() {
        let t = unwrap_text(b"T51 : a b  \r\n c1\n2 ");
        assert_eq!(t, b"T51 : a b c12 ".to_vec());
    }

    #[test]
    fn text_tokens_nulls_and_unspaced_characters() {
        let b = b"12 ?+.5 ?-1e3 T3 abc";
        let mut l = Lex::new(b, Form::Text);
        assert_eq!(l.int("a").unwrap(), Some(12));
        assert_eq!(l.int("b").unwrap(), None);
        assert_eq!(l.char("c").unwrap(), b'+');
        assert_eq!(l.real("d").unwrap(), 0.5);
        assert!(l.real("e").unwrap().is_nan());
        assert_eq!(l.char("f").unwrap(), b'-');
        assert_eq!(l.real("g").unwrap(), 1e3);
        assert!(l.logical("h").unwrap());
        assert_eq!(l.short_string("i").unwrap(), b"abc".to_vec());
        assert!(l.done());
    }

    #[test]
    fn text_escapes_and_nine_spaces() {
        let mut l = Lex::new(b"a\\\\b\\9c", Form::Text);
        assert_eq!(l.chars(13, "x").unwrap(), b"a\\b         c".to_vec());
    }

    #[test]
    fn binary_indices_small_and_large() {
        // 5 is written 6; 40000 is -(40000 % 32767 + 1) then 40000 / 32767
        let mut b = vec![0, 6];
        b.extend((-((40000 % 32767) + 1) as i16).to_be_bytes());
        b.extend(1i16.to_be_bytes());
        let mut l = Lex::new(&b, Form::Binary { little: false });
        assert_eq!(l.index("a").unwrap(), Some(5));
        assert_eq!(l.index("b").unwrap(), Some(40000));
        let mut l = Lex::new(&[6, 0], Form::Binary { little: true });
        assert_eq!(l.index("c").unwrap(), Some(5));
    }

    #[test]
    fn running_out_says_where() {
        let mut l = Lex::new(b"12", Form::Binary { little: false });
        assert_eq!(
            l.int("an int"),
            Err(LexError {
                at: 0,
                want: "an int"
            })
        );
    }
}
