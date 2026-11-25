//! `forge-server DIR [--port P] [--bind ADDR] [--token T] [--user NAME:TOKEN]... [--store file|mem]
//!  [--origin URL]... [--checkpoint SECONDS]`
//!
//! Serves the project folder DIR to collaborating clients at `ws://HOST:P/sync`. Phase 1: dev
//! tokens only (each `--user` is an author name and its token; `--token T` is the user `dev`; with
//! none, a token is made and printed). Binds 127.0.0.1 unless `--bind` says otherwise.

use forge_server::{Config, StoreKind, random_token, start};
use std::process::ExitCode;

const USAGE: &str = "forge-server DIR [--port P] [--bind ADDR] [--token T] [--user NAME:TOKEN]... \
                     [--store file|mem] [--origin URL]... [--checkpoint SECONDS]";

fn parse(args: &[String]) -> Result<Config, String> {
    let mut it = args.iter();
    let root = it.next().ok_or(USAGE)?;
    let mut c = Config::new(root.into(), vec![]);
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--port" => c.port = val()?.parse().map_err(|_| "--port: a number")?,
            "--bind" => c.bind = val()?.parse().map_err(|_| "--bind: an IP address")?,
            "--token" => c.users.push(("dev".into(), val()?)),
            "--user" => {
                let v = val()?;
                let (n, t) = v.split_once(':').ok_or("--user NAME:TOKEN")?;
                if n.is_empty() || !n.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)) {
                    return Err("--user: a name is letters, digits, _ and -".into());
                }
                c.users.push((n.into(), t.into()));
            }
            "--store" => {
                c.store = match val()?.as_str() {
                    "file" => StoreKind::File,
                    "mem" => StoreKind::Mem,
                    _ => return Err("--store file|mem".into()),
                }
            }
            "--origin" => c.origins.push(val()?),
            "--checkpoint" => {
                c.checkpoint_idle =
                    std::time::Duration::from_secs(val()?.parse().map_err(|_| "--checkpoint: seconds")?)
            }
            _ => return Err(format!("unknown argument {a}\n{USAGE}")),
        }
    }
    if c.users.iter().any(|(_, t)| t.len() < 16) {
        return Err("a token is at least 16 characters".into());
    }
    Ok(c)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut c = match parse(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if !c.root.is_dir() {
        eprintln!("{}: not a folder", c.root.display());
        return ExitCode::from(2);
    }
    if c.users.is_empty() {
        let t = random_token();
        eprintln!("forge-server: user dev, token {t}");
        c.users.push(("dev".into(), t));
    }
    match start(c.clone()) {
        Ok(h) => {
            eprintln!("forge-server: {} at ws://{}/sync", c.root.display(), h.addr);
            loop {
                std::thread::park();
            }
        }
        Err(e) => {
            eprintln!("forge-server: {e}");
            ExitCode::FAILURE
        }
    }
}
