//! WebSocket (RFC 6455) over `std::net`, as hosted forgeCAD uses it: text messages only, no
//! extensions, every size bounded.
//!
//! - [`accept`] reads a client's upgrade request (head capped at [`MAX_HEAD`]), lets the caller
//!   check it (Host, Origin, path, token: the caller's policy) and answers 101.
//! - [`connect`] is the native client's side (the desktop app; the browser has its own).
//! - [`Reader::read`] returns whole text messages (fragments joined, at most `max` bytes, UTF-8
//!   checked), answers pings and refuses what the RFC forbids (unmasked client frames, masked
//!   server frames, reserved bits, oversized or fragmented control frames).
//! - [`Writer`] sends text frames; it is a separate handle so one thread can read while another
//!   writes.
//!
//! Deadlines are the socket's: the caller sets read and write timeouts.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;

/// The most an upgrade request's or response's head may hold.
pub const MAX_HEAD: usize = 16 << 10;
const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// This side accepted: it reads masked frames and writes unmasked ones.
    Server,
    /// This side connected: it reads unmasked frames and writes masked ones.
    Client,
}

/// An upgrade request as the server read it.
#[derive(Clone, Debug, Default)]
pub struct Request {
    pub path: String,
    pub query: String,
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn header(&self, k: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
    }
    pub fn query(&self, k: &str) -> Option<&str> {
        self.query.split('&').find_map(|kv| {
            let (a, b) = kv.split_once('=')?;
            (a == k).then_some(b)
        })
    }
}

/// A refusal of an upgrade: the HTTP status and a short reason (sent as the body).
pub struct Refuse(pub u16, pub &'static str);

/// Reads the upgrade request on `s`, asks `check` whether to accept it, and answers. On a
/// refusal the HTTP answer is written and the error says why.
pub fn accept(
    s: TcpStream,
    max: usize,
    check: impl FnOnce(&Request) -> Result<(), Refuse>,
) -> io::Result<(Reader, Writer, Request)> {
    let mut r = BufReader::new(s.try_clone()?);
    let mut w = s;
    let head = read_head(&mut r)?;
    let refuse = |w: &mut TcpStream, code: u16, why: &str| -> io::Error {
        let body = why.as_bytes();
        let _ = write!(
            w,
            "HTTP/1.1 {code} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            reason(code),
            body.len()
        );
        let _ = w.write_all(body);
        io::Error::new(io::ErrorKind::PermissionDenied, format!("{code} {why}"))
    };
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or_default();
    let mut words = first.split(' ');
    let (method, target, version) = (words.next(), words.next().unwrap_or(""), words.next());
    if method != Some("GET") || version != Some("HTTP/1.1") {
        return Err(refuse(&mut w, 400, "a WebSocket upgrade is GET ... HTTP/1.1"));
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let q = Request {
        path: path.into(),
        query: query.into(),
        headers: lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect(),
    };
    let has = |k: &str, token: &str| {
        q.header(k)
            .is_some_and(|v| v.split(',').any(|t| t.trim().eq_ignore_ascii_case(token)))
    };
    if !has("upgrade", "websocket") || !has("connection", "upgrade") {
        return Err(refuse(&mut w, 426, "this endpoint speaks WebSocket only"));
    }
    if q.header("sec-websocket-version") != Some("13") {
        return Err(refuse(&mut w, 426, "WebSocket version 13 only"));
    }
    let key = q.header("sec-websocket-key").unwrap_or("").to_string();
    if unbase64(&key).is_none_or(|k| k.len() != 16) {
        return Err(refuse(&mut w, 400, "a bad Sec-WebSocket-Key"));
    }
    if let Err(Refuse(code, why)) = check(&q) {
        return Err(refuse(&mut w, code, why));
    }
    write!(
        w,
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept_key(&key)
    )?;
    w.flush()?;
    Ok((
        Reader {
            r,
            role: Role::Server,
            max,
            part: vec![],
            started: false,
            fragments: 0,
            frames: 0,
        },
        Writer {
            w,
            role: Role::Server,
            mask: Mask::new(),
        },
        q,
    ))
}

/// Connects to `ws://host:port/path?query` (no TLS: a loopback or LAN server in phase 1).
/// `headers` go into the upgrade request (an Authorization, say).
pub fn connect(
    addr: &str,
    path_and_query: &str,
    headers: &[(&str, &str)],
    max: usize,
) -> io::Result<(Reader, Writer)> {
    let s = TcpStream::connect(addr)?;
    s.set_nodelay(true)?;
    let mut w = s.try_clone()?;
    let mut mask = Mask::new();
    let mut raw = [0u8; 16];
    for c in raw.chunks_mut(4) {
        c.copy_from_slice(&mask.next());
    }
    let key = base64(&raw);
    let mut req = format!(
        "GET {path_and_query} HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n"
    );
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    w.write_all(req.as_bytes())?;
    let mut r = BufReader::new(s);
    let head = read_head(&mut r)?;
    let first = head.split("\r\n").next().unwrap_or_default();
    if !first.starts_with("HTTP/1.1 101") {
        // the refusal's body, for the error
        let len = head
            .split("\r\n")
            .filter_map(|l| l.split_once(':'))
            .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
            .and_then(|(_, v)| v.trim().parse::<usize>().ok())
            .unwrap_or(0)
            .min(4096);
        let mut body = vec![0; len];
        let _ = r.read_exact(&mut body);
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("{first}: {}", String::from_utf8_lossy(&body)),
        ));
    }
    let want = accept_key(&key);
    let got = head
        .split("\r\n")
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("sec-websocket-accept"))
        .map(|(_, v)| v.trim().to_string());
    if got.as_deref() != Some(want.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "a bad Sec-WebSocket-Accept",
        ));
    }
    Ok((
        Reader {
            r,
            role: Role::Client,
            max,
            part: vec![],
            started: false,
            fragments: 0,
            frames: 0,
        },
        Writer {
            w,
            role: Role::Client,
            mask,
        },
    ))
}

fn read_head(r: &mut BufReader<TcpStream>) -> io::Result<String> {
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        // bounded: never more than what is left of MAX_HEAD
        let n = r
            .by_ref()
            .take((MAX_HEAD + 2 - head.len()) as u64)
            .read_until(b'\n', &mut line)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "the head cut short"));
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
        head.extend_from_slice(&line);
        if head.len() > MAX_HEAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the head is too large",
            ));
        }
    }
    String::from_utf8(head).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "the head is not UTF-8"))
}

fn reason(c: u16) -> &'static str {
    match c {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        421 => "Misdirected Request",
        426 => "Upgrade Required",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

/// `Sec-WebSocket-Accept` for a key.
pub fn accept_key(key: &str) -> String {
    let mut h = forge_git::hash::Sha1::new();
    h.update(key.as_bytes());
    h.update(GUID.as_bytes());
    base64(&h.finish())
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(b: &[u8]) -> String {
    let mut out = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                out.push(B64[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn unbase64(s: &str) -> Option<Vec<u8>> {
    let s = s.as_bytes();
    if !s.len().is_multiple_of(4) {
        return None;
    }
    let val = |c: u8| B64.iter().position(|&x| x == c).map(|p| p as u32);
    let mut out = vec![];
    for (i, c) in s.chunks(4).enumerate() {
        let last = i == s.len() / 4 - 1;
        let pad = c.iter().rev().take_while(|&&x| x == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for &x in &c[..4 - pad] {
            n = n << 6 | val(x)?;
        }
        n <<= 6 * pad as u32;
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Some(out)
}

/// Mask keys: not secret (the masking defends proxies against a hostile page, which a native
/// client is not), only varied; std's randomly seeded hasher, stepped.
struct Mask(u64);

impl Mask {
    fn new() -> Mask {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(0x5bd1_e995);
        Mask(h.finish() | 1)
    }
    fn next(&mut self) -> [u8; 4] {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 as u32).to_le_bytes()
    }
}

pub struct Writer {
    w: TcpStream,
    role: Role,
    mask: Mask,
}

impl Writer {
    pub fn send(&mut self, text: &str) -> io::Result<()> {
        let f = frame(0x1, text.as_bytes(), self.role, &mut self.mask);
        self.w.write_all(&f)
    }
    pub fn pong(&mut self, data: &[u8]) -> io::Result<()> {
        let f = frame(0xA, &data[..data.len().min(125)], self.role, &mut self.mask);
        self.w.write_all(&f)
    }
    /// A close frame (code 1000), then the socket shut down for writing.
    pub fn close(&mut self) {
        let f = frame(0x8, &1000u16.to_be_bytes(), self.role, &mut self.mask);
        let _ = self.w.write_all(&f);
        let _ = self.w.shutdown(std::net::Shutdown::Write);
    }
    pub fn stream(&self) -> &TcpStream {
        &self.w
    }
}

fn frame(op: u8, data: &[u8], role: Role, mask: &mut Mask) -> Vec<u8> {
    let masked = role == Role::Client;
    let mut f = Vec::with_capacity(data.len() + 14);
    f.push(0x80 | op);
    let m = if masked { 0x80 } else { 0 };
    match data.len() {
        n if n < 126 => f.push(m | n as u8),
        n if n < 65536 => {
            f.push(m | 126);
            f.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            f.push(m | 127);
            f.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    if masked {
        let k = mask.next();
        f.extend_from_slice(&k);
        f.extend(data.iter().enumerate().map(|(i, b)| b ^ k[i & 3]));
    } else {
        f.extend_from_slice(data);
    }
    f
}

/// What a read gives.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Text(String),
    /// A ping (it may come between a message's fragments: the message so far is kept). Answer
    /// with [`Writer::pong`].
    Ping(Vec<u8>),
    /// The peer closed (or sent a binary message, which this protocol does not use).
    Close,
}

pub struct Reader {
    r: BufReader<TcpStream>,
    role: Role,
    max: usize,
    /// A fragmented message so far (a ping may come between its fragments).
    part: Vec<u8>,
    started: bool,
    /// Fragments of the message being read.
    fragments: usize,
    /// Every frame read so far (control frames too): what a rate limit counts.
    pub frames: u64,
}

fn bad(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_string())
}

impl Reader {
    /// The next whole text message. An error ends the connection (protocol violation, a
    /// message over `max`, a timeout, a reset).
    pub fn read(&mut self) -> io::Result<Msg> {
        loop {
            let mut h = [0u8; 2];
            self.r.read_exact(&mut h)?;
            let (fin, rsv, op) = (h[0] & 0x80 != 0, h[0] & 0x70, h[0] & 0x0f);
            let masked = h[1] & 0x80 != 0;
            if rsv != 0 {
                return Err(bad("reserved bits set"));
            }
            if masked != (self.role == Role::Server) {
                return Err(bad(if masked {
                    "a masked frame from the server"
                } else {
                    "an unmasked frame from a client"
                }));
            }
            let mut len = (h[1] & 0x7f) as u64;
            if len == 126 {
                let mut b = [0u8; 2];
                self.r.read_exact(&mut b)?;
                len = u16::from_be_bytes(b) as u64;
            } else if len == 127 {
                let mut b = [0u8; 8];
                self.r.read_exact(&mut b)?;
                len = u64::from_be_bytes(b);
                // RFC 6455 §5.2: the most significant bit must be 0
                if len >> 63 != 0 {
                    return Err(bad("a 64-bit length with its top bit set"));
                }
            }
            let control = op & 0x8 != 0;
            if control && (len > 125 || !fin) {
                return Err(bad("a control frame over 125 bytes or fragmented"));
            }
            // no addition that can overflow: the room left under the limit, compared
            if !control && len > (self.max.saturating_sub(self.part.len())) as u64 {
                return Err(bad("a message over the limit"));
            }
            self.frames += 1;
            if !control {
                self.fragments += 1;
                if self.fragments > MAX_FRAGMENTS {
                    return Err(bad("a message in too many fragments"));
                }
            }
            let mut key = [0u8; 4];
            if masked {
                self.r.read_exact(&mut key)?;
            }
            let mut data = vec![0u8; len as usize];
            self.r.read_exact(&mut data)?;
            if masked {
                for (i, b) in data.iter_mut().enumerate() {
                    *b ^= key[i & 3];
                }
            }
            match op {
                0x8 => return Ok(Msg::Close),
                // answered by the one writer (frames from two threads could interleave)
                0x9 => return Ok(Msg::Ping(data)),
                0xA => {}
                0x1 | 0x2 if self.started => return Err(bad("a new message inside a fragmented one")),
                0x0 if !self.started => return Err(bad("a continuation with nothing to continue")),
                0x2 => return Ok(Msg::Close),
                0x1 | 0x0 => {
                    self.started = true;
                    self.part.extend_from_slice(&data);
                    if fin {
                        self.started = false;
                        self.fragments = 0;
                        let m = std::mem::take(&mut self.part);
                        return String::from_utf8(m)
                            .map(Msg::Text)
                            .map_err(|_| bad("text that is not UTF-8"));
                    }
                }
                _ => return Err(bad("an unknown opcode")),
            }
        }
    }
    pub fn stream(&self) -> &TcpStream {
        self.r.get_ref()
    }
    /// A new limit on a message's size (small before a peer has authenticated, larger after).
    pub fn set_max(&mut self, max: usize) {
        self.max = max;
    }
}

/// The most fragments one message may come in (empty continuations are frames too).
pub const MAX_FRAGMENTS: usize = 64;

// sockets, threads, links and file locks: not in WASI (the wasm suite runs the workspace there)
#[cfg(all(test, not(target_os = "wasi")))]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn the_rfc_example_key() {
        // RFC 6455 §1.3
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
        for n in 0..40 {
            let b: Vec<u8> = (0..n as u8).map(|x| x.wrapping_mul(37)).collect();
            assert_eq!(unbase64(&base64(&b)).unwrap(), b);
        }
        assert!(unbase64("a=bc").is_none());
    }

    #[test]
    fn messages_both_ways_and_limits() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = l.local_addr().unwrap().to_string();
        let t = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let (mut r, mut w, q) = accept(s, 1000, |q| {
                if q.path == "/sync" {
                    Ok(())
                } else {
                    Err(Refuse(404, "no"))
                }
            })
            .unwrap();
            assert_eq!(q.query("doc"), Some("a.fg"));
            loop {
                match r.read() {
                    Ok(Msg::Text(t)) => w.send(&format!("echo {t}")).unwrap(),
                    Ok(Msg::Ping(p)) => w.pong(&p).unwrap(),
                    Ok(Msg::Close) => break "closed".to_string(),
                    Err(e) => break e.to_string(),
                }
            }
        });
        let (mut r, mut w) = connect(&at, "/sync?doc=a.fg", &[], 1 << 20).unwrap();
        for m in ["hi", &"x".repeat(300), &"y".repeat(900)] {
            w.send(m).unwrap();
            assert_eq!(r.read().unwrap(), Msg::Text(format!("echo {m}")));
        }
        // over the server's limit: it drops the connection
        w.send(&"z".repeat(2000)).unwrap();
        assert!(t.join().unwrap().contains("over the limit"));
    }

    /// A raw client: the upgrade by hand, then whatever bytes a test wants to send.
    fn raw(at: &str) -> TcpStream {
        let mut s = TcpStream::connect(at).unwrap();
        write!(
            s,
            "GET /sync HTTP/1.1\r\nHost: {at}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )
        .unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        assert!(read_head(&mut r).unwrap().starts_with("HTTP/1.1 101"));
        s
    }

    /// A server that reads one message (limit `max`) and reports what the read gave.
    fn server(max: usize) -> (String, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = l.local_addr().unwrap().to_string();
        let t = std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let (mut r, _w, _) = accept(s, max, |_| Ok(())).unwrap();
            let mut out = vec![];
            loop {
                match r.read() {
                    Ok(Msg::Text(t)) => out.push(format!("text {}", t.len())),
                    Ok(Msg::Ping(_)) => out.push("ping".into()),
                    Ok(Msg::Close) => {
                        out.push("close".into());
                        break;
                    }
                    Err(e) => {
                        out.push(format!("err {e}"));
                        break;
                    }
                }
            }
            out.join(", ")
        });
        (at, t)
    }

    fn masked(fin: bool, op: u8, len_bytes: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut f = vec![if fin { 0x80 } else { 0 } | op];
        f.extend_from_slice(len_bytes);
        f[1] |= 0x80;
        let k = [1, 2, 3, 4];
        f.extend_from_slice(&k);
        f.extend(payload.iter().enumerate().map(|(i, b)| b ^ k[i & 3]));
        f
    }

    #[test]
    fn hostile_lengths_are_refused_not_panics() {
        // the security review's PoC: a fragment, then a continuation claiming u64::MAX bytes
        let (at, t) = server(1000);
        let mut s = raw(&at);
        s.write_all(&masked(false, 0x1, &[3], b"abc")).unwrap();
        let mut len = vec![127];
        len.extend_from_slice(&u64::MAX.to_be_bytes());
        s.write_all(&masked(true, 0x0, &len, b"")).unwrap();
        let got = t.join().unwrap();
        assert!(got.contains("top bit"), "{got}");
        // the largest length without the top bit, after a fragment: over the limit, no overflow
        let (at, t) = server(1000);
        let mut s = raw(&at);
        s.write_all(&masked(false, 0x1, &[3], b"abc")).unwrap();
        let mut len = vec![127];
        len.extend_from_slice(&(u64::MAX >> 1).to_be_bytes());
        s.write_all(&masked(true, 0x0, &len, b"")).unwrap();
        assert!(t.join().unwrap().contains("over the limit"));
    }

    #[test]
    fn framing_rules() {
        let cases: Vec<(Vec<u8>, &str)> = vec![
            // fragments joined, a ping between them answered and the message kept
            (
                [
                    masked(false, 0x1, &[2], b"ab"),
                    masked(true, 0x9, &[1], b"p"),
                    masked(true, 0x0, &[2], b"cd"),
                ]
                .concat(),
                "ping, text 4",
            ),
            // an unmasked client frame
            (vec![0x81, 0x01, b'x'], "unmasked"),
            // reserved bits
            (masked(true, 0x1 | 0x40, &[1], b"x"), "reserved"),
            // a control frame over 125 bytes, and a fragmented one
            (masked(true, 0x9, &[126, 0, 126], &[0; 126]), "control"),
            (masked(false, 0x9, &[1], b"x"), "control"),
            // a continuation with nothing to continue; a new message inside a fragmented one
            (masked(true, 0x0, &[1], b"x"), "continuation"),
            (
                [masked(false, 0x1, &[1], b"a"), masked(true, 0x1, &[1], b"b")].concat(),
                "inside",
            ),
            // too many (empty) fragments
            (
                std::iter::once(masked(false, 0x1, &[0], b""))
                    .chain(std::iter::repeat_n(masked(false, 0x0, &[0], b""), 80))
                    .flatten()
                    .collect(),
                "too many fragments",
            ),
            // not UTF-8; an unknown opcode; the close handshake
            (masked(true, 0x1, &[2], &[0xff, 0xfe]), "UTF-8"),
            (masked(true, 0x3, &[1], b"x"), "unknown opcode"),
            (masked(true, 0x8, &[2], &1000u16.to_be_bytes()), "close"),
            // oversize in one frame
            (
                masked(true, 0x1, &[126, 0x04, 0x00], &[b'a'; 1024]),
                "over the limit",
            ),
        ];
        for (bytes, want) in cases {
            let (at, t) = server(1000);
            let mut s = raw(&at);
            s.write_all(&bytes).unwrap();
            // then EOF: a read that wants more ends rather than waits
            s.shutdown(std::net::Shutdown::Write).unwrap();
            let got = t.join().unwrap();
            assert!(got.contains(want), "want {want}: {got}");
        }
    }

    #[test]
    fn a_refused_upgrade_says_why() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let (s, _) = l.accept().unwrap();
            let _ = accept(s, 1000, |_| Err(Refuse(403, "another origin")));
        });
        let e = connect(&at, "/sync", &[], 1000).err().unwrap();
        assert!(
            e.to_string().contains("403") && e.to_string().contains("another origin"),
            "{e}"
        );
    }
}
