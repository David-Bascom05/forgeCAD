//! A private local connection between processes of the same user, at a path in a private folder.
//!
//! Unix (Linux, macOS): a Unix socket at the path, the folder 0700 and the socket 0600.
//! Windows: std has no Unix sockets there, so a TCP listener on 127.0.0.1 with an unguessable
//! token. The path is then a small file holding `port token`, in the user's own app-data folder
//! (no other user can read it); a client reads it, connects, and sends the token as its first
//! line, and the listener drops any connection that does not.

use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Duration;

/// Makes `dir` (and its parents) and closes it to other users where the platform allows.
pub fn private_dir(dir: &Path) -> io::Result<()> {
    forge_dirs::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        forge_dirs::fs::set_permissions(dir, forge_dirs::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Closes a file the user made to other users where the platform allows (Unix: 0600).
pub fn private_file(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        forge_dirs::fs::set_permissions(path, forge_dirs::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub struct Listener {
    #[cfg(unix)]
    inner: std::os::unix::net::UnixListener,
    #[cfg(not(unix))]
    inner: std::net::TcpListener,
    #[cfg(not(unix))]
    token: String,
}

pub struct Stream {
    #[cfg(unix)]
    inner: std::os::unix::net::UnixStream,
    #[cfg(not(unix))]
    inner: std::net::TcpStream,
}

impl Listener {
    /// Listens at `path`, its folder already private. A stale path left by a dead listener is
    /// replaced; one a live listener answers at is an error (`AddrInUse`).
    pub fn bind(path: &Path) -> io::Result<Listener> {
        // WASI (the browser's build, the wasm test suite) listens on nothing: refused, typed
        if cfg!(target_os = "wasi") {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "no local connections under WASI",
            ));
        }
        if path.exists() {
            if connect(path).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "a listener is already active",
                ));
            }
            forge_dirs::fs::remove_file(path)?;
        }
        #[cfg(unix)]
        {
            let inner = std::os::unix::net::UnixListener::bind(path)?;
            private_file(path)?;
            Ok(Listener { inner })
        }
        #[cfg(not(unix))]
        {
            let inner = std::net::TcpListener::bind(("127.0.0.1", 0))?;
            let token = token();
            let port = inner.local_addr()?.port();
            // written whole then renamed in, so a client never reads half a file
            let tmp = path.with_extension("tmp");
            forge_dirs::fs::write(&tmp, format!("{port} {token}\n"))?;
            forge_dirs::fs::rename(&tmp, path)?;
            Ok(Listener { inner, token })
        }
    }

    pub fn set_nonblocking(&self, on: bool) -> io::Result<()> {
        self.inner.set_nonblocking(on)
    }

    /// The next connection. On Windows its first line must be the token (read with a short
    /// timeout); a connection that fails it is `PermissionDenied`, and the caller goes on.
    pub fn accept(&self) -> io::Result<Stream> {
        let (s, _) = self.inner.accept()?;
        #[cfg(not(unix))]
        {
            s.set_nonblocking(false)?;
            s.set_read_timeout(Some(Duration::from_secs(5)))?;
            let mut got = Vec::with_capacity(self.token.len() + 1);
            let mut b = [0u8; 1];
            while got.len() <= self.token.len() {
                match (&s).read(&mut b)? {
                    0 => break,
                    _ if b[0] == b'\n' => break,
                    _ => got.push(b[0]),
                }
            }
            let same = got.len() == self.token.len()
                && got
                    .iter()
                    .zip(self.token.bytes())
                    .fold(0u8, |a, (x, y)| a | (x ^ y))
                    == 0;
            s.set_read_timeout(None)?;
            if !same {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "a connection without the token",
                ));
            }
        }
        Ok(Stream { inner: s })
    }
}

/// Connects to the listener at `path`.
pub fn connect(path: &Path) -> io::Result<Stream> {
    if cfg!(target_os = "wasi") {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no local connections under WASI",
        ));
    }
    #[cfg(unix)]
    {
        Ok(Stream {
            inner: std::os::unix::net::UnixStream::connect(path)?,
        })
    }
    #[cfg(not(unix))]
    {
        let text = forge_dirs::fs::read_to_string(path)?;
        let mut it = text.split_whitespace();
        let (Some(port), Some(token)) = (it.next(), it.next()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a listener's file",
            ));
        };
        let port: u16 = port
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not a listener's file"))?;
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port))?;
        writeln!(s, "{token}")?;
        Ok(Stream { inner: s })
    }
}

/// 128 unguessable bits as hex, from the std hasher's per-process random keys and the clock.
#[cfg(not(unix))]
fn token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for k in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(k);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        h.write_u32(forge_dirs::process_id());
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

impl Stream {
    pub fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(d)
    }
    pub fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.inner.set_write_timeout(d)
    }
    pub fn try_clone(&self) -> io::Result<Stream> {
        Ok(Stream {
            inner: self.inner.try_clone()?,
        })
    }
    pub fn shutdown(&self) -> io::Result<()> {
        self.inner.shutdown(std::net::Shutdown::Both)
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Read for &Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.inner).read(buf)
    }
}
impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl Write for &Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.inner).write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        (&self.inner).flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_os = "wasi"))]
    use std::io::{BufRead, BufReader};

    /// Under WASI there are no local connections: both ends refuse, with no thread or pid.
    #[cfg(target_os = "wasi")]
    #[test]
    fn wasi_refuses_both_ends() {
        let p = std::path::Path::new("/tmp/forge-ipc-wasi.sock");
        assert_eq!(
            Listener::bind(p).err().map(|e| e.kind()),
            Some(io::ErrorKind::Unsupported)
        );
        assert_eq!(
            connect(p).err().map(|e| e.kind()),
            Some(io::ErrorKind::Unsupported)
        );
    }

    // a thread and a socket: native only
    #[cfg(not(target_os = "wasi"))]
    #[test]
    fn a_line_goes_both_ways_and_a_live_path_is_not_taken_over() {
        let dir = forge_dirs::temp_dir().join(format!("forge-ipc-{}", forge_dirs::process_id()));
        private_dir(&dir).unwrap();
        let path = dir.join("t.sock");
        let l = Listener::bind(&path).unwrap();
        let t = std::thread::spawn(move || {
            let s = l.accept().unwrap();
            let mut line = String::new();
            BufReader::new(&s).read_line(&mut line).unwrap();
            writeln!(&s, "got {}", line.trim()).unwrap();
            l
        });
        let s = connect(&path).unwrap();
        writeln!(&s, "hello").unwrap();
        let mut back = String::new();
        BufReader::new(&s).read_line(&mut back).unwrap();
        assert_eq!(back, "got hello\n");
        let _live = t.join().unwrap();
        assert_eq!(
            Listener::bind(&path).err().map(|e| e.kind()),
            Some(io::ErrorKind::AddrInUse)
        );
    }
}
