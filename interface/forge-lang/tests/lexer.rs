//! The lexer assumptions the LL(1) proof rests on (research/forge-lang/ASSUMPTIONS.md, P6): it is
//! deterministic; its tokens and the trivia between them give back the source bytes exactly; and
//! every literal a SetParam writes lexes back as one number token.

use forge_lang::edit::{apply, apply_ast};
use forge_lang::error::{Error, Literal, Op, ParamKey, parse_path};
use forge_lang::syntax::{self, Tok, TokKind, TriviaKind, Unit};
use forge_lang::values::{self, Dim};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Random text from Forge's pieces, with comments, spaces, tabs, carriage returns, line breaks
/// inside and outside brackets, and now and then a byte Forge does not use.
fn text(rng: &mut Rng) -> String {
    const PIECES: [&str; 48] = [
        "x", "plan", "_a1", "shared", "param", "for", "in", "if", "else", "true", "and", "not", "0", "12",
        "2.5", "1e3", "4mm", "90deg", "1.25e-2", "7.", "-3", "\"s\"", "\"\"", "(", ")", "[", "]", "{", "}",
        ",", ":", ".", "..", "..=", "=", "==", "!=", "<=", "<", "+-", "+", "-", "*", "/", "^", "%", ";",
        "#c",
    ];
    const GAPS: [&str; 7] = [" ", "", "\t", "\r\n", "\n", "  # note\n", " \u{e9}"];
    let mut s = String::new();
    for _ in 0..rng.below(60) {
        s += PIECES[rng.below(PIECES.len())];
        let g = rng.below(GAPS.len() * 4);
        s += if g < GAPS.len() { GAPS[g] } else { " " };
    }
    s
}

fn corpus() -> Vec<String> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/parts");
    let mut out = vec![];
    let mut dirs = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "fg") {
                out.push(std::fs::read_to_string(p).expect("part"));
            }
        }
    }
    assert!(out.len() >= 3);
    out
}

fn texts() -> Vec<String> {
    let mut rng = Rng(0x01ec_5eed);
    let mut out = corpus();
    out.extend((0..20000).map(|_| text(&mut rng)));
    out
}

#[test]
fn lexing_is_deterministic() {
    for src in texts() {
        let a = syntax::lex(&src);
        assert_eq!(a, syntax::lex(&src), "{src:?}");
        let b = std::thread::scope(|s| s.spawn(|| syntax::lex(&src)).join().expect("thread"));
        assert_eq!(a, b, "{src:?}");
        // keeping the trivia does not change the tokens
        assert_eq!(a, syntax::lex_lossless(&src).map(|(t, _)| t), "{src:?}");
    }
}

/// Every byte is in exactly one token or one trivia, in order, so concatenating them is the
/// source; each trivia is what its kind says.
#[test]
fn tokens_and_trivia_rebuild_the_source() {
    let mut lexed = 0;
    for src in texts() {
        let Ok((toks, trivia)) = syntax::lex_lossless(&src) else {
            continue;
        };
        lexed += 1;
        let mut pieces: Vec<(u32, u32)> = toks
            .iter()
            .filter(|t| t.kind != TokKind::Eof)
            .map(|t| (t.span.lo, t.span.hi))
            .chain(trivia.iter().map(|t| (t.span.lo, t.span.hi)))
            .collect();
        pieces.sort();
        let mut rebuilt = String::new();
        let mut at = 0;
        for (lo, hi) in pieces {
            assert_eq!(lo, at, "a gap or an overlap at {lo} in {src:?}");
            assert!(hi > lo, "an empty piece at {lo} in {src:?}");
            rebuilt += &src[lo as usize..hi as usize];
            at = hi;
        }
        assert_eq!(rebuilt, src);
        let eof = toks.last().expect("EOF");
        assert_eq!(
            (eof.kind.clone(), eof.span.lo as usize),
            (TokKind::Eof, src.len())
        );
        for t in &trivia {
            let s = &src[t.span.lo as usize..t.span.hi as usize];
            match t.kind {
                TriviaKind::Space => assert!(s.bytes().all(|c| matches!(c, b' ' | b'\t' | b'\r')), "{s:?}"),
                TriviaKind::Comment => assert!(s.starts_with('#') && !s.contains('\n'), "{s:?}"),
                TriviaKind::Newline => assert_eq!(s, "\n"),
            }
        }
    }
    assert!(lexed > 5000, "only {lexed} texts lexed");
}

/// What a SetParam writes for a value, put where it goes (after `=` in a shared or a local
/// param), lexed: the tokens after `=`, the same in both.
fn after_eq(text: &str) -> Vec<Tok> {
    let rest = |line: String| -> Vec<TokKind> {
        let toks = syntax::lex(&line).unwrap_or_else(|e| panic!("{line:?}: {e:?}"));
        let k = toks.iter().position(|t| t.kind == TokKind::Op("=")).expect("=");
        toks[k + 1..]
            .iter()
            .map(|t| t.kind.clone())
            .filter(|t| !matches!(t, TokKind::Nl | TokKind::Eof))
            .collect()
    };
    assert_eq!(
        rest(format!("shared x = {text}")),
        rest(format!("param f[2].x = {text}\n"))
    );
    let toks = syntax::lex(&format!("shared x = {text}")).expect("lexes");
    let k = toks.iter().position(|t| t.kind == TokKind::Op("=")).expect("=");
    toks[k + 1..toks.len() - 1].to_vec()
}

fn values(rng: &mut Rng) -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        -0.1,
        1e-13,
        -1e-13,
        4e-13,
        6e-13,
        1e-12,
        123_456_789.123_456_79,
        1e15,
        -1e15,
        1e21,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        0.5,
        2.5,
        1.0 / 3.0,
        -2.0 / 3.0,
        90.0,
        360.0,
    ];
    for _ in 0..20000 {
        let mant = (rng.next() >> 11) as f64 / (1u64 << 53) as f64;
        let exp = rng.below(40) as i32 - 20;
        let sign = if rng.below(2) == 0 { 1.0 } else { -1.0 };
        v.push(sign * mant * 10f64.powi(exp));
        v.push(sign * rng.below(100000) as f64);
        v.push(sign * rng.below(100000) as f64 / 8.0);
    }
    v
}

#[test]
fn every_setparam_literal_lexes_as_one_number() {
    let mut rng = Rng(0xdeca_f00d);
    for value in values(&mut rng) {
        let written: f64 = values::fmt(value).parse().expect("fmt writes a number");
        for dim in [
            Dim::Length,
            Dim::Angle,
            Dim::Count,
            Dim::Ratio,
            Dim::Area,
            Dim::Volume,
            Dim::Any,
        ] {
            let text = Literal { value, dim }.text();
            let toks = after_eq(&text);
            let num = match dim {
                Dim::Count | Dim::Ratio => {
                    let f = if dim == Dim::Count { "count" } else { "ratio" };
                    assert_eq!(toks.len(), 4, "{text:?}: {toks:?}");
                    assert_eq!(toks[0].kind, TokKind::Id(f.into()));
                    assert_eq!(
                        (&toks[1].kind, &toks[3].kind),
                        (&TokKind::Op("("), &TokKind::Op(")"))
                    );
                    &toks[2]
                }
                _ => {
                    assert_eq!(toks.len(), 1, "{text:?} is not one token: {toks:?}");
                    &toks[0]
                }
            };
            let unit = if dim == Dim::Angle { Unit::Deg } else { Unit::None };
            assert_eq!(num.kind, TokKind::Num(written, unit), "{text:?}");
            // the value written is the value to the 12 decimals fmt keeps (it writes 0 below 1e-12),
            // sign included
            assert!(
                (written - value).abs() <= 1e-12 + value.abs() * 1e-15,
                "{value} wrote {text:?}"
            );
            assert!(
                written == 0.0 || written.signum() == value.signum(),
                "{value} wrote {text:?}"
            );
        }
    }
}

#[test]
fn the_lexer_never_panics() {
    let mut rng = Rng(0xbad_c0de);
    for _ in 0..20000 {
        let n = rng.below(40);
        let bytes: Vec<u8> = (0..n)
            .map(|_| {
                [
                    b'-', b'.', b'1', b'e', b'"', b'#', b'\n', b'(', b'm', 0xc3, 0xa9, b'd', b'g', b'x',
                    b' ', b'=', b'+',
                ][rng.below(17)]
            })
            .collect();
        let s = String::from_utf8_lossy(&bytes).into_owned();
        let _ = syntax::lex(&s);
        let _ = syntax::lex_lossless(&s);
    }
}

/// NaN and the infinities have no literal (`fmt` would write `NaN` or `inf`, which lex as names):
/// SetParam and AddLocalParam refuse them with a typed error, and the text is left as it was.
#[test]
fn a_value_that_is_not_finite_is_refused() {
    let src =
        "shared w = 5\nsketch k(on: xy) {\n  o = rect(corner: (0, 0), size: (w, w))\n}\nf = extrude(k, w)\n";
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let lit = Literal {
            value,
            dim: Dim::Length,
        };
        let ops = [
            Op::SetParam {
                param: ParamKey::parse("w").expect("a param"),
                value: lit,
            },
            Op::AddLocalParam {
                feature: parse_path("f").expect("a path"),
                name: "d".into(),
                value: lit,
            },
        ];
        for op in ops {
            for result in [apply(src, &op).map(|_| ()), apply_ast(src, &op).map(|_| ())] {
                match result {
                    Err(Error::NotFinite { value: v, .. }) => {
                        assert!(v.is_nan() == value.is_nan() && (v.is_nan() || v == value))
                    }
                    other => panic!("{op:?} gave {other:?}"),
                }
            }
        }
    }
    // a finite value still applies
    let op = Op::SetParam {
        param: ParamKey::parse("w").expect("a param"),
        value: Literal {
            value: 7.5,
            dim: Dim::Length,
        },
    };
    assert!(apply(src, &op).expect("applies").contains("shared w = 7.5"));
}
