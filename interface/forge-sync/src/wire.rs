//! The protocol's messages and their text form: one JSON object per WebSocket text frame.
//!
//! JSON is read with forge-agent's parser, after a bounded pre-scan (size and nesting depth, so a
//! hostile frame cannot exhaust the stack), and written here with **exact** numbers: forge-agent's
//! writer rounds to a few decimals (fine for replies to people, wrong for a param's value, which
//! must reach every client bit for bit).

use crate::{Entry, Refusal};
pub use forge_agent::json::Json;
use forge_lang::error::Op;

/// The largest message either side accepts, in bytes. An op is ~150–250 B; a Welcome carries a
/// whole document (the largest example is far below this).
pub const MAX_MESSAGE: usize = 8 << 20;
/// The deepest nesting of arrays and objects a message may have (an op is 3 deep).
pub const MAX_DEPTH: usize = 16;

/// Client → server.
#[derive(Clone, Debug, PartialEq)]
pub enum Up {
    /// The first message on a connection: which document, the client's stable id (its pending
    /// ops are deduplicated by it across reconnects), the token, and the last seq it applied
    /// (None: it has nothing; send the head as a snapshot).
    Hello {
        doc: String,
        client: String,
        token: String,
        seq: Option<u64>,
        /// The client's protocol version ([`crate::PROTOCOL`]) and kernel build
        /// ([`crate::KERNEL`]): a room holds one kernel, since bit-identity holds only there.
        proto: u32,
        kernel: String,
    },
    /// An op the client applied locally on top of the server's text at `base`.
    Op { base: u64, id: u64, op: Op },
    /// The exact hash of the body the client built at `seq` (divergence detection).
    BuildHash { seq: u64, hash: u64 },
    /// Write the head as a named version (a forge-history commit and tag).
    Version { name: String },
    /// Liveness (also measures the round trip).
    Ping { t: f64 },
    /// The client's text differs from the server's: send a snapshot.
    Resync,
}

/// Server → client.
#[derive(Clone, Debug, PartialEq)]
pub enum Down {
    /// The head as a snapshot: the text at `seq`. `you` is the author name ops from this
    /// connection are committed under.
    Welcome {
        seq: u64,
        text: String,
        you: String,
        /// True: `text` is the document at `seq`, replace yours. False: a catch-up of entries
        /// follows (and `text` is empty).
        snap: bool,
    },
    /// The client's own op `id` committed as `seq` (rebased: `op` is what was applied), with its
    /// inverse (for undo) and the hash of the text after it.
    Ack {
        id: u64,
        seq: u64,
        op: Op,
        inv: Option<Op>,
        th: u64,
    },
    /// The client's own op `id` refused; nothing was written.
    Refused {
        id: u64,
        refusal: Refusal,
    },
    /// Another author's op committed as `seq`.
    Applied {
        seq: u64,
        author: String,
        op: Op,
        th: u64,
    },
    /// A build hash at `seq` differs from the first one reported for it.
    Diverged {
        seq: u64,
        expected: u64,
        got: u64,
        by: String,
    },
    /// A named version was written.
    Versioned {
        name: String,
        seq: u64,
        commit: String,
    },
    Pong {
        t: f64,
    },
    /// Something the server says about the last message; the connection stays.
    Error {
        text: String,
    },
    /// The server closes the connection after this. `retry`: a passing state (busy, stopping,
    /// too slow), reconnect; else it will not be served as it is (a bad token, another kernel or
    /// protocol, a quarantined or unopenable document, a malformed message): do not reconnect.
    Closing {
        text: String,
        retry: bool,
    },
}

/// One committed op as the log stores it.
pub fn entry_json(e: &Entry) -> Json {
    Json::obj(vec![
        ("seq", num(e.seq)),
        ("author", e.author.as_str().into()),
        ("client", e.client.as_str().into()),
        ("id", num(e.id)),
        ("base", num(e.base)),
        ("op", e.op.json()),
        ("inv", e.inv.as_ref().map_or(Json::Null, Op::json)),
        ("th", hex(e.th)),
    ])
}

pub fn entry_from(j: &Json) -> Option<Entry> {
    Some(Entry {
        seq: u(j, "seq")?,
        author: s(j, "author")?,
        client: s(j, "client")?,
        id: u(j, "id")?,
        base: u(j, "base")?,
        op: Op::from_json(j.get("op")?)?,
        inv: match j.get("inv")? {
            Json::Null => None,
            x => Some(Op::from_json(x)?),
        },
        th: unhex(j.get("th")?)?,
    })
}

impl Up {
    pub fn json(&self) -> Json {
        match self {
            Up::Hello {
                doc,
                client,
                token,
                seq,
                proto,
                kernel,
            } => Json::obj(vec![
                ("t", "hello".into()),
                ("doc", doc.as_str().into()),
                ("client", client.as_str().into()),
                ("token", token.as_str().into()),
                ("seq", seq.map_or(Json::Null, num)),
                ("proto", num(*proto as u64)),
                ("kernel", kernel.as_str().into()),
            ]),
            Up::Op { base, id, op } => Json::obj(vec![
                ("t", "op".into()),
                ("base", num(*base)),
                ("id", num(*id)),
                ("op", op.json()),
            ]),
            Up::BuildHash { seq, hash } => Json::obj(vec![
                ("t", "hash".into()),
                ("seq", num(*seq)),
                ("hash", self::hex(*hash)),
            ]),
            Up::Version { name } => Json::obj(vec![("t", "version".into()), ("name", name.as_str().into())]),
            Up::Ping { t } => Json::obj(vec![("t", "ping".into()), ("at", Json::Num(*t))]),
            Up::Resync => Json::obj(vec![("t", "resync".into())]),
        }
    }
    pub fn from_json(j: &Json) -> Option<Up> {
        Some(match s(j, "t")?.as_str() {
            "hello" => Up::Hello {
                doc: s(j, "doc")?,
                client: s(j, "client")?,
                token: s(j, "token")?,
                seq: match j.get("seq")? {
                    Json::Null => None,
                    _ => Some(u(j, "seq")?),
                },
                proto: u32::try_from(u(j, "proto")?).ok()?,
                kernel: s(j, "kernel")?,
            },
            "op" => Up::Op {
                base: u(j, "base")?,
                id: u(j, "id")?,
                op: Op::from_json(j.get("op")?)?,
            },
            "hash" => Up::BuildHash {
                seq: u(j, "seq")?,
                hash: unhex(j.get("hash")?)?,
            },
            "version" => Up::Version { name: s(j, "name")? },
            "resync" => Up::Resync,
            "ping" => Up::Ping {
                t: j.get("at")?.num()?,
            },
            _ => return None,
        })
    }
    pub fn text(&self) -> String {
        write(&self.json())
    }
    pub fn parse(t: &str) -> Result<Up, String> {
        Up::from_json(&read(t)?).ok_or_else(|| "not a client message".into())
    }
}

impl Down {
    pub fn json(&self) -> Json {
        match self {
            Down::Welcome { seq, text, you, snap } => Json::obj(vec![
                ("t", "welcome".into()),
                ("seq", num(*seq)),
                ("text", text.as_str().into()),
                ("you", you.as_str().into()),
                ("snap", (*snap).into()),
            ]),
            Down::Ack { id, seq, op, inv, th } => Json::obj(vec![
                ("t", "ack".into()),
                ("id", num(*id)),
                ("seq", num(*seq)),
                ("op", op.json()),
                ("inv", inv.as_ref().map_or(Json::Null, Op::json)),
                ("th", hex(*th)),
            ]),
            Down::Refused { id, refusal } => Json::obj(vec![
                ("t", "refused".into()),
                ("id", num(*id)),
                ("kind", refusal.kind().into()),
                ("text", refusal.text().into()),
            ]),
            Down::Applied { seq, author, op, th } => Json::obj(vec![
                ("t", "applied".into()),
                ("seq", num(*seq)),
                ("author", author.as_str().into()),
                ("op", op.json()),
                ("th", hex(*th)),
            ]),
            Down::Diverged {
                seq,
                expected,
                got,
                by,
            } => Json::obj(vec![
                ("t", "diverged".into()),
                ("seq", num(*seq)),
                ("expected", hex(*expected)),
                ("got", hex(*got)),
                ("by", by.as_str().into()),
            ]),
            Down::Versioned { name, seq, commit } => Json::obj(vec![
                ("t", "versioned".into()),
                ("name", name.as_str().into()),
                ("seq", num(*seq)),
                ("commit", commit.as_str().into()),
            ]),
            Down::Pong { t } => Json::obj(vec![("t", "pong".into()), ("at", Json::Num(*t))]),
            Down::Error { text } => Json::obj(vec![("t", "error".into()), ("text", text.as_str().into())]),
            Down::Closing { text, retry } => Json::obj(vec![
                ("t", "closing".into()),
                ("text", text.as_str().into()),
                ("retry", (*retry).into()),
            ]),
        }
    }
    pub fn from_json(j: &Json) -> Option<Down> {
        Some(match s(j, "t")?.as_str() {
            "welcome" => Down::Welcome {
                seq: u(j, "seq")?,
                text: s(j, "text")?,
                you: s(j, "you")?,
                snap: matches!(j.get("snap")?, Json::Bool(true)),
            },
            "ack" => Down::Ack {
                id: u(j, "id")?,
                seq: u(j, "seq")?,
                op: Op::from_json(j.get("op")?)?,
                inv: match j.get("inv")? {
                    Json::Null => None,
                    x => Some(Op::from_json(x)?),
                },
                th: unhex(j.get("th")?)?,
            },
            "refused" => Down::Refused {
                id: u(j, "id")?,
                refusal: Refusal::from_wire(&s(j, "kind")?, &s(j, "text")?),
            },
            "applied" => Down::Applied {
                seq: u(j, "seq")?,
                author: s(j, "author")?,
                op: Op::from_json(j.get("op")?)?,
                th: unhex(j.get("th")?)?,
            },
            "diverged" => Down::Diverged {
                seq: u(j, "seq")?,
                expected: unhex(j.get("expected")?)?,
                got: unhex(j.get("got")?)?,
                by: s(j, "by")?,
            },
            "versioned" => Down::Versioned {
                name: s(j, "name")?,
                seq: u(j, "seq")?,
                commit: s(j, "commit")?,
            },
            "pong" => Down::Pong {
                t: j.get("at")?.num()?,
            },
            "error" => Down::Error { text: s(j, "text")? },
            "closing" => Down::Closing {
                text: s(j, "text")?,
                retry: matches!(j.get("retry"), Some(Json::Bool(true))),
            },
            _ => return None,
        })
    }
    pub fn text(&self) -> String {
        write(&self.json())
    }
    pub fn parse(t: &str) -> Result<Down, String> {
        Down::from_json(&read(t)?).ok_or_else(|| "not a server message".into())
    }
}

fn s(j: &Json, k: &str) -> Option<String> {
    j.get(k).and_then(Json::str).map(str::to_string)
}

/// A non-negative integer below 2^53 (exact in an f64).
fn u(j: &Json, k: &str) -> Option<u64> {
    let x = j.get(k)?.num()?;
    (x >= 0.0 && x.fract() == 0.0 && x < 9_007_199_254_740_992.0).then_some(x as u64)
}

fn num(x: u64) -> Json {
    Json::Num(x as f64)
}

/// 64-bit hashes travel as 16 hex digits (an f64 holds only 53 bits).
fn hex(x: u64) -> Json {
    Json::Str(format!("{x:016x}"))
}

fn unhex(j: &Json) -> Option<u64> {
    let t = j.str()?;
    (t.len() == 16).then(|| u64::from_str_radix(t, 16).ok())?
}

/// Parses one message, refusing it before parsing if it is too large or nested too deep.
pub fn read(t: &str) -> Result<Json, String> {
    if t.len() > MAX_MESSAGE {
        return Err(format!("message of {} bytes (at most {MAX_MESSAGE})", t.len()));
    }
    let (mut depth, mut max, mut in_str, mut esc) = (0usize, 0usize, false, false);
    for &b in t.as_bytes() {
        if in_str {
            match (esc, b) {
                (true, _) => esc = false,
                (false, b'\\') => esc = true,
                (false, b'"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'[' | b'{' => {
                depth += 1;
                max = max.max(depth);
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if max > MAX_DEPTH {
            return Err(format!("nested deeper than {MAX_DEPTH}"));
        }
    }
    Json::parse(t)
}

/// Writes JSON with every number exact (Rust's shortest round-trip form). Non-finite numbers
/// are written as null (an op carrying one is refused before it is ever written).
pub fn write(j: &Json) -> String {
    let mut out = String::new();
    write_into(j, &mut out);
    out
}

fn write_into(j: &Json, out: &mut String) {
    match j {
        Json::Num(n) if n.is_finite() => {
            use std::fmt::Write;
            // -0.0 is written "-0", which parses back to -0.0: bits kept
            let _ = write!(out, "{n}");
        }
        Json::Arr(a) => {
            out.push('[');
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_into(v, out);
            }
            out.push(']');
        }
        Json::Obj(f) => {
            out.push('{');
            for (i, (k, v)) in f.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                Json::Str(k.clone()).write(out);
                out.push(':');
                write_into(v, out);
            }
            out.push('}');
        }
        other => other.write(out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_lang::error::{Literal, ParamKey};
    use forge_lang::values::Dim;

    #[test]
    fn numbers_are_exact() {
        for v in [
            0.1,
            1.0 / 3.0,
            1e-300,
            1.7976931348623157e308,
            -0.0,
            5e-324,
            123456.789e10,
        ] {
            let op = Op::SetParam {
                param: ParamKey::parse("boss.d").unwrap(),
                value: Literal {
                    value: v,
                    dim: Dim::Length,
                },
            };
            let m = Up::Op { base: 7, id: 3, op };
            let back = Up::parse(&m.text()).unwrap();
            match (&m, &back) {
                (
                    Up::Op {
                        op: Op::SetParam { value: a, .. },
                        ..
                    },
                    Up::Op {
                        op: Op::SetParam { value: b, .. },
                        ..
                    },
                ) => {
                    assert_eq!(a.value.to_bits(), b.value.to_bits(), "{v}")
                }
                _ => panic!(),
            }
        }
    }

    #[test]
    fn depth_and_size_are_bounded() {
        let deep = "[".repeat(10_000);
        assert!(read(&deep).unwrap_err().contains("nested"));
        // brackets inside strings do not count
        let ok = format!("{{\"t\":\"version\",\"name\":\"{}\"}}", "[".repeat(100));
        assert!(Up::parse(&ok).is_ok());
        let big = "x".repeat(MAX_MESSAGE + 1);
        assert!(read(&big).is_err());
    }

    #[test]
    fn every_message_round_trips() {
        let op = Op::Rename {
            from: "a".into(),
            to: "b".into(),
        };
        for m in [
            Up::Hello {
                doc: "p.fg".into(),
                client: "c1".into(),
                token: "t".into(),
                seq: None,
                proto: 1,
                kernel: "k".into(),
            },
            Up::Hello {
                doc: "p.fg".into(),
                client: "c1".into(),
                token: "t".into(),
                seq: Some(0),
                proto: 1,
                kernel: "k".into(),
            },
            Up::Op {
                base: 1,
                id: 2,
                op: op.clone(),
            },
            Up::BuildHash {
                seq: 4,
                hash: u64::MAX,
            },
            Up::Version { name: "v1".into() },
            Up::Ping { t: 12.5 },
            Up::Resync,
        ] {
            assert_eq!(Up::parse(&m.text()).unwrap(), m);
        }
        for m in [
            Down::Welcome {
                seq: 3,
                text: "shared a = 1\n".into(),
                you: "dev/c1".into(),
                snap: true,
            },
            Down::Ack {
                id: 2,
                seq: 5,
                op: op.clone(),
                inv: Some(op.clone()),
                th: 1,
            },
            Down::Ack {
                id: 2,
                seq: 5,
                op: op.clone(),
                inv: None,
                th: 1,
            },
            Down::Refused {
                id: 9,
                refusal: Refusal::RenameConflict { name: "a".into() },
            },
            Down::Applied {
                seq: 6,
                author: "x".into(),
                op: op.clone(),
                th: 0xdead_beef,
            },
            Down::Diverged {
                seq: 6,
                expected: 1,
                got: 2,
                by: "c".into(),
            },
            Down::Versioned {
                name: "v".into(),
                seq: 6,
                commit: "abc".into(),
            },
            Down::Pong { t: 1.0 },
            Down::Error { text: "no".into() },
            Down::Closing {
                text: "busy".into(),
                retry: true,
            },
            Down::Closing {
                text: "bad token".into(),
                retry: false,
            },
        ] {
            assert_eq!(Down::parse(&m.text()).unwrap(), m);
        }
    }
}
