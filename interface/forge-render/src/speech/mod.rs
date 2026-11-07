//! Speech into the Agent tab's message box. The recogniser is `forge-speech`, a process of its
//! own (interface/forge-speech): it holds the model and the microphone, and this side talks to
//! it over its stdin and stdout, a line a message. Without it the app runs the same; the mic
//! button says speech isn't available.
//!
//! To it: `names\tA\tB..` (the document's names, for biasing), `listen` (the microphone),
//! `listen-wav\tPATH` (a WAV file at its own pace, in the microphone's place), `stop`, `cancel`.
//! From it: `ready\tENGINE\tLOAD_MS`, `level\tL` (0..1), `partial\tTEXT`,
//! `final\tMS\tTEXT` (MS: stop to text), `error\tTEXT`.

pub mod words;

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

pub type Waker = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    Ready {
        engine: String,
        load_ms: f64,
    },
    Level(f32),
    Partial(String),
    Final {
        text: String,
        ms: f64,
    },
    Error(String),
    /// The process ended: why, if it said.
    Gone(String),
}

/// One line from the process.
pub fn parse(line: &str) -> Option<Heard> {
    let mut f = line.trim_end_matches(['\r', '\n']).splitn(3, '\t');
    let kind = f.next()?;
    let a = f.next().unwrap_or("");
    let b = f.next().unwrap_or("");
    Some(match kind {
        "ready" => Heard::Ready {
            engine: a.into(),
            load_ms: b.parse().unwrap_or(0.0),
        },
        "level" => Heard::Level(a.parse::<f32>().ok()?.clamp(0.0, 1.0)),
        "partial" => Heard::Partial(unescape(a)),
        "final" => Heard::Final {
            ms: a.parse().unwrap_or(0.0),
            text: unescape(b),
        },
        "error" => Heard::Error(unescape(a)),
        _ => return None,
    })
}

/// A field with no tab or newline in it.
pub fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

pub fn unescape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            o.push(c);
            continue;
        }
        match it.next() {
            Some('t') => o.push('\t'),
            Some('n') => o.push('\n'),
            Some(c) => o.push(c),
            None => {}
        }
    }
    o
}

/// `forge-speech`: `FORGECAD_SPEECH`, else beside this program.
pub fn find() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FORGECAD_SPEECH") {
        let p = PathBuf::from(p);
        return p.is_file().then_some(p);
    }
    let exe = std::env::current_exe().ok()?;
    let p = exe.parent()?.join("forge-speech");
    p.is_file().then_some(p)
}

pub struct Speech {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Heard>,
}

impl Speech {
    /// The process started; it loads its model at once, and takes the microphone before that's
    /// done, so nothing said is lost.
    pub fn start(exe: &Path, wake: Waker) -> std::io::Result<Speech> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped");
        let out = child.stdout.take().expect("piped");
        let (tx, rx) = channel();
        std::thread::Builder::new().name("speech".into()).spawn(move || {
            let mut last_error = String::new();
            for line in BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                if let Some(h) = parse(&line) {
                    if let Heard::Error(e) = &h {
                        last_error = e.clone();
                    }
                    if tx.send(h).is_err() {
                        return;
                    }
                    wake();
                }
            }
            let _ = tx.send(Heard::Gone(last_error));
            wake();
        })?;
        Ok(Speech { child, stdin, rx })
    }

    fn say(&mut self, line: &str) {
        let _ = writeln!(self.stdin, "{line}");
        let _ = self.stdin.flush();
    }

    pub fn names(&mut self, names: &[String]) {
        let mut line = String::from("names");
        for n in names {
            line.push('\t');
            line.push_str(&escape(n));
        }
        self.say(&line);
    }

    /// From the microphone, or from a WAV file in its place.
    pub fn listen(&mut self, wav: Option<&Path>) {
        match wav {
            Some(w) => self.say(&format!("listen-wav\t{}", escape(&w.display().to_string()))),
            None => self.say("listen"),
        }
    }

    pub fn stop(&mut self) {
        self.say("stop");
    }

    pub fn cancel(&mut self) {
        self.say("cancel");
    }

    pub fn heard(&self) -> Vec<Heard> {
        self.rx.try_iter().collect()
    }
}

impl Drop for Speech {
    fn drop(&mut self) {
        self.say("quit");
        // its own child: ended if it doesn't go by itself
        for _ in 0..20 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_both_ways() {
        assert_eq!(
            parse("ready\twhisper large-v3-turbo cuda\t812.5"),
            Some(Heard::Ready {
                engine: "whisper large-v3-turbo cuda".into(),
                load_ms: 812.5
            })
        );
        assert_eq!(parse("level\t1.7"), Some(Heard::Level(1.0)));
        assert_eq!(
            parse("partial\tmake the\\tbracket"),
            Some(Heard::Partial("make the\tbracket".into()))
        );
        assert_eq!(
            parse("final\t143\tmove hole 3\\\\n"),
            Some(Heard::Final {
                text: "move hole 3\\n".into(),
                ms: 143.0
            })
        );
        assert_eq!(parse("noise"), None);
        let s = "a\tb\nc\\d";
        assert_eq!(unescape(&escape(s)), s);
        assert!(!escape(s).contains(['\t', '\n']));
    }
}
