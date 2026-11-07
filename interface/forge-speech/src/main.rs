//! forge-speech: what's said, as text, for ForgeCAD's Agent tab. The app starts it on first use
//! and talks to it over stdin and stdout, a line a message (forge-render's `speech` module has
//! the lines). It loads the model at once and keeps it; it takes the microphone (or a WAV file
//! in its place) when asked to listen, says the level and what it's heard so far as it goes, and
//! the whole text when asked to stop.
//!
//! `forge-speech --check`: whether it can run here (exit 0), else why not, on one line.
//! `forge-speech --bench LIST.tsv [PROMPT]`: each `id\tpath.wav`'s text and time, as the app gets it.
//!
//! Its files: `FORGECAD_SPEECH_DIR`, else ~/.cache/forgecad/speech: `lib/` (whisper.cpp's
//! libraries) and the model, `FORGECAD_SPEECH_MODEL` or the first of `MODELS` there.

mod alsa;
mod dl;
mod wav;
mod whisper;

use std::ffi::CString;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RATE: usize = 16_000;
/// The models looked for, best first.
const MODELS: [&str; 4] = [
    "ggml-large-v3-turbo-q8_0.bin",
    "ggml-large-v3-turbo-q5_0.bin",
    "ggml-small.en.bin",
    "ggml-base.en.bin",
];
/// How often what's heard so far is decoded again while listening.
const PARTIAL_EVERY: Duration = Duration::from_millis(400);

fn dir() -> PathBuf {
    if let Some(d) = std::env::var_os("FORGECAD_SPEECH_DIR") {
        return d.into();
    }
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| ".".into());
    cache.join("forgecad").join("speech")
}

fn model(d: &Path) -> Result<PathBuf, String> {
    if let Some(m) = std::env::var_os("FORGECAD_SPEECH_MODEL") {
        let m = PathBuf::from(m);
        let m = if m.is_absolute() { m } else { d.join(m) };
        return if m.is_file() {
            Ok(m)
        } else {
            Err(format!("no model at {}", m.display()))
        };
    }
    MODELS
        .iter()
        .map(|m| d.join(m))
        .find(|m| m.is_file())
        .ok_or_else(|| format!("no speech model in {}", d.display()))
}

fn check() -> Result<(), String> {
    let d = dir();
    for (name, needed) in whisper::LIBS {
        if needed && !d.join("lib").join(name).is_file() {
            return Err(format!("no speech library in {}", d.join("lib").display()));
        }
    }
    model(&d)?;
    Ok(())
}

/// A line out, whole (the threads share stdout).
#[derive(Clone)]
struct Out(Arc<Mutex<std::io::Stdout>>);

impl Out {
    fn say(&self, kind: &str, fields: &[&str]) {
        let mut line = kind.to_string();
        for f in fields {
            line.push('\t');
            line.push_str(&escape(f));
        }
        let mut o = self.0.lock().expect("no panics while held");
        let _ = writeln!(o, "{line}");
        let _ = o.flush();
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

fn unescape(s: &str) -> String {
    let mut o = String::new();
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

/// The prompt whisper is given: the names, as if just said.
fn prompt_of(names: &[String]) -> Option<CString> {
    if names.is_empty() {
        return None;
    }
    let mut p = String::from("ForgeCAD. Names: ");
    for (k, n) in names.iter().enumerate() {
        if p.len() + n.len() > 600 {
            break;
        }
        if k > 0 {
            p.push_str(", ");
        }
        p.push_str(n);
    }
    p.push_str(". Units: mm, degrees.");
    CString::new(p.replace('\0', "")).ok()
}

/// What one listen has heard so far, and whether it's still going.
#[derive(Default)]
struct Take {
    pcm: Vec<f32>,
    listening: bool,
    /// Which listen it is: a feeder of an older one stops.
    n: u64,
}

enum Job {
    /// Decode what's heard of listen `n` (a partial, unless `last`).
    Decode {
        n: u64,
        last: bool,
        stopped: Instant,
    },
    Names(Vec<String>),
}

/// Where the speech is in the take: a quiet start and end cut off (whisper makes words of
/// silence), with a little margin. None if nothing in it is louder than the noise.
fn speech_span(x: &[f32]) -> Option<(usize, usize)> {
    let win = RATE / 50;
    let rms: Vec<f32> = x
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let mut sorted = rms.clone();
    sorted.sort_by(f32::total_cmp);
    let floor = sorted.get(sorted.len() / 10).copied().unwrap_or(0.0);
    let peak = sorted.last().copied().unwrap_or(0.0);
    let gate = (floor * 3.0).max(peak * 0.08).max(0.004);
    if peak < 0.01 {
        return None;
    }
    let first = rms.iter().position(|&r| r > gate)?;
    let last = rms.iter().rposition(|&r| r > gate)?;
    let pad = 15;
    Some((
        first.saturating_sub(pad) * win,
        ((last + 1 + pad) * win).min(x.len()),
    ))
}

fn engine(
    rx: Receiver<Job>,
    take: Arc<Mutex<Take>>,
    current: Arc<AtomicU64>,
    abort: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    out: Out,
) {
    let d = dir();
    let t0 = Instant::now();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8)) as i32;
    let mut w = match model(&d).and_then(|m| whisper::Whisper::load(&d.join("lib"), &m, threads)) {
        Ok(w) => w,
        Err(e) => {
            out.say("error", &[&e]);
            std::process::exit(2);
        }
    };
    // a first run warms the GPU's kernels: the first real one isn't the slow one
    let warm = AtomicBool::new(false);
    let _ = w.decode(&vec![0.0; RATE], None, &warm);
    out.say(
        "ready",
        &[&w.label, &format!("{:.0}", t0.elapsed().as_secs_f64() * 1e3)],
    );
    let mut prompt: Option<CString> = None;
    while let Ok(job) = rx.recv() {
        match job {
            Job::Names(n) => prompt = prompt_of(&n),
            Job::Decode { n, last, stopped } => {
                if n != current.load(Ordering::SeqCst) {
                    continue;
                }
                abort.store(false, Ordering::SeqCst);
                let pcm = take.lock().expect("no panics while held").pcm.clone();
                let span = speech_span(&pcm);
                let text = match span {
                    None => Some(String::new()),
                    // whisper reads at least a second
                    Some((a, b)) => {
                        let mut x = pcm[a..b].to_vec();
                        if x.len() < RATE + RATE / 10 {
                            x.resize(RATE + RATE / 10, 0.0);
                        }
                        w.decode(&x, prompt.as_deref(), &abort)
                    }
                };
                busy.store(false, Ordering::SeqCst);
                if n != current.load(Ordering::SeqCst) {
                    continue;
                }
                match (text, last) {
                    (Some(t), true) => {
                        let ms = stopped.elapsed().as_secs_f64() * 1e3;
                        out.say("final", &[&format!("{ms:.0}"), &t]);
                    }
                    (Some(t), false) => out.say("partial", &[&t]),
                    (None, true) => out.say("final", &["0", ""]),
                    (None, false) => {}
                }
            }
        }
    }
}

/// Samples in, the take grown and the level said, until the take stops.
fn feed(take: &Mutex<Take>, n: u64, out: &Out, chunk: &[f32], last_level: &mut Instant) -> bool {
    let mut t = take.lock().expect("no panics while held");
    if !t.listening || t.n != n {
        return false;
    }
    t.pcm.extend_from_slice(chunk);
    drop(t);
    if last_level.elapsed() >= Duration::from_millis(50) {
        *last_level = Instant::now();
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len().max(1) as f32).sqrt();
        // -60..0 dBFS as 0..1
        let db = 20.0 * rms.max(1e-6).log10();
        out.say(
            "level",
            &[&format!("{:.2}", ((db + 60.0) / 60.0).clamp(0.0, 1.0))],
        );
    }
    true
}

fn listen_mic(n: u64, take: Arc<Mutex<Take>>, out: Out) {
    let alsa = match alsa::Alsa::load() {
        Ok(a) => a,
        Err(e) => return out.say("error", &[&e]),
    };
    let mut mic = match alsa.mic(RATE as u32) {
        Ok(m) => m,
        Err(e) => return out.say("error", &[&e]),
    };
    let mut buf = vec![0i16; RATE / 50];
    let mut last = Instant::now() - Duration::from_secs(1);
    loop {
        let got = match mic.read(&mut buf) {
            Ok(k) => k,
            Err(e) => return out.say("error", &[&e]),
        };
        let chunk: Vec<f32> = buf[..got].iter().map(|&s| s as f32 / 32768.0).collect();
        if !feed(&take, n, &out, &chunk, &mut last) {
            return;
        }
    }
}

/// A WAV file in the microphone's place, at its own pace; then quiet, as a microphone is.
fn listen_wav(n: u64, path: String, take: Arc<Mutex<Take>>, out: Out) {
    let x = match std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| wav::read(&b))
    {
        Ok(x) => x,
        Err(e) => return out.say("error", &[&format!("{path}: {e}")]),
    };
    let step = RATE / 50;
    let t0 = Instant::now();
    let mut last = Instant::now() - Duration::from_secs(1);
    let silence = vec![0.0f32; step];
    for k in 0.. {
        let at = k * step;
        let chunk = if at < x.len() {
            &x[at..(at + step).min(x.len())]
        } else {
            &silence[..]
        };
        if !feed(&take, n, &out, chunk, &mut last) {
            return;
        }
        let due = t0 + Duration::from_secs_f64((k + 1) as f64 * step as f64 / RATE as f64);
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
    }
}

fn serve() {
    let out = Out(Arc::new(Mutex::new(std::io::stdout())));
    let take = Arc::new(Mutex::new(Take::default()));
    let current = Arc::new(AtomicU64::new(0));
    let abort = Arc::new(AtomicBool::new(false));
    let busy = Arc::new(AtomicBool::new(false));
    let (jobs, rx): (Sender<Job>, Receiver<Job>) = channel();
    {
        let (take, current, abort, busy, out) = (
            take.clone(),
            current.clone(),
            abort.clone(),
            busy.clone(),
            out.clone(),
        );
        std::thread::spawn(move || engine(rx, take, current, abort, busy, out));
    }
    // while listening, what's heard so far, again and again (skipped while the engine's busy)
    {
        let (take, current, busy, jobs) = (take.clone(), current.clone(), busy.clone(), jobs.clone());
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(PARTIAL_EVERY);
                let listening = take
                    .lock()
                    .map(|t| t.listening && t.pcm.len() > RATE / 2)
                    .unwrap_or(false);
                if listening && !busy.swap(true, Ordering::SeqCst) {
                    let n = current.load(Ordering::SeqCst);
                    if jobs
                        .send(Job::Decode {
                            n,
                            last: false,
                            stopped: Instant::now(),
                        })
                        .is_err()
                    {
                        return;
                    }
                }
            }
        });
    }
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let mut f = line.split('\t');
        match f.next().unwrap_or("") {
            "names" => {
                let _ = jobs.send(Job::Names(f.map(unescape).collect()));
            }
            cmd @ ("listen" | "listen-wav") => {
                let n = current.fetch_add(1, Ordering::SeqCst) + 1;
                abort.store(true, Ordering::SeqCst);
                {
                    let mut t = take.lock().expect("no panics while held");
                    t.pcm.clear();
                    t.listening = true;
                    t.n = n;
                }
                let (take, out) = (take.clone(), out.clone());
                if cmd == "listen" {
                    std::thread::spawn(move || listen_mic(n, take, out));
                } else {
                    let path = unescape(f.next().unwrap_or(""));
                    std::thread::spawn(move || listen_wav(n, path, take, out));
                }
            }
            "stop" => {
                let stopped = Instant::now();
                take.lock().expect("no panics while held").listening = false;
                // a partial in the way is given up: the last words come first
                abort.store(true, Ordering::SeqCst);
                busy.store(true, Ordering::SeqCst);
                let n = current.load(Ordering::SeqCst);
                let _ = jobs.send(Job::Decode {
                    n,
                    last: true,
                    stopped,
                });
            }
            "cancel" => {
                take.lock().expect("no panics while held").listening = false;
                current.fetch_add(1, Ordering::SeqCst);
                abort.store(true, Ordering::SeqCst);
            }
            "quit" => break,
            _ => {}
        }
    }
}

/// Each clip of the list as the app would hear it: listened to whole, then stopped.
fn bench(list: &str, prompt: Option<&str>) -> Result<(), String> {
    let d = dir();
    let t0 = Instant::now();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8)) as i32;
    let mut w = whisper::Whisper::load(&d.join("lib"), &model(&d)?, threads)?;
    let load = t0.elapsed().as_secs_f64();
    let never = AtomicBool::new(false);
    let t1 = Instant::now();
    let _ = w.decode(&vec![0.0; RATE], None, &never);
    let first = t1.elapsed().as_secs_f64();
    println!(
        "{{\"engine\":\"{}\",\"load_s\":{load:.3},\"first_s\":{first:.3}}}",
        w.label
    );
    let names: Vec<String> = prompt
        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let prompt = prompt_of(&names);
    for line in std::fs::read_to_string(list).map_err(|e| e.to_string())?.lines() {
        let Some((id, path)) = line.split_once('\t') else {
            continue;
        };
        let x = wav::read(&std::fs::read(path).map_err(|e| e.to_string())?)?;
        let t = Instant::now();
        let text = match speech_span(&x) {
            None => String::new(),
            Some((a, b)) => {
                let mut x = x[a..b].to_vec();
                if x.len() < RATE + RATE / 10 {
                    x.resize(RATE + RATE / 10, 0.0);
                }
                w.decode(&x, prompt.as_deref(), &never).unwrap_or_default()
            }
        };
        let ms = t.elapsed().as_secs_f64() * 1e3;
        println!(
            "{{\"id\":\"{id}\",\"ms\":{ms:.1},\"hyp\":\"{}\"}}",
            text.replace('\\', "\\\\").replace('"', "\\\"")
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--check") => match check() {
            Ok(()) => println!("ok"),
            Err(e) => {
                println!("{e}");
                std::process::exit(1);
            }
        },
        Some("--bench") => {
            if let Err(e) = bench(
                args.get(1).map_or("", String::as_str),
                args.get(2).map(String::as_str),
            ) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        _ => serve(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_speech_in_a_take() {
        let mut x = vec![0.0005f32; RATE * 2];
        for (k, s) in x[RATE / 2..RATE].iter_mut().enumerate() {
            *s = 0.3 * (k as f32 * 0.2).sin();
        }
        let (a, b) = speech_span(&x).unwrap();
        assert!(a < RATE / 2 && a + RATE / 2 > RATE / 2 - 1);
        assert!(b > RATE && b < RATE + RATE / 2);
        assert_eq!(speech_span(&[0.0; RATE]), None);
        assert_eq!(unescape(&escape("a\tb\\n")), "a\tb\\n");
        let p = prompt_of(&["hole[3]".into(), "DL-108".into()]).unwrap();
        assert_eq!(
            p.to_str().unwrap(),
            "ForgeCAD. Names: hole[3], DL-108. Units: mm, degrees."
        );
        assert!(prompt_of(&[]).is_none());
    }
}
