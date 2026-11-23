//! What the kernel made of each fillet of a set of .fg parts, with no oracle: for comparing two
//! kernels part by part (research/fillets/census.md). outcome OUT.jsonl FILE.fg.. | LIST.txt
//!
//! One JSON line a fillet feature: built or refused (the error), and for a built one the body's
//! volume, area and face count and whether forge_topo::check finds a problem other than a
//! duplicate name. A part that stops before a fillet gives that fillet as refused and no more.
use forge_model::Feature;
use std::sync::atomic::{AtomicUsize, Ordering};

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '"' && *c != '\\' && (*c as u32) >= 32)
        .collect()
}

fn one(path: &str) -> Vec<String> {
    let id = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string();
    let Ok(src) = std::fs::read_to_string(path) else {
        return vec![format!("{{\"id\":\"{id}\",\"label\":\"unread\"}}")];
    };
    let m = forge_lang::fuzz::build(&src);
    let mut out = Vec::new();
    for (k, f) in m.features.iter().enumerate() {
        if !matches!(f, Feature::Fillet { .. }) {
            continue;
        }
        let head = format!("{{\"id\":\"{id}\",\"feature\":{k}");
        if k >= m.built() {
            let why = m
                .error()
                .map(|e| esc(&forge_lang::fuzz::message(e)))
                .unwrap_or_default();
            out.push(format!("{head},\"outcome\":\"refused\",\"why\":\"{why}\"}}"));
            break;
        }
        let Some(a) = m.body_after(k + 1) else {
            out.push(format!("{head},\"outcome\":\"no body\"}}"));
            continue;
        };
        let valid = forge_topo::check(&a)
            .iter()
            .all(|p| matches!(p, forge_topo::Problem::DuplicateName { .. }));
        let mp = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| forge_topo::mass::mass(&a)));
        let (vol, area) = match mp {
            Ok(Ok(m)) => (m.volume, m.area),
            _ => (f64::NAN, f64::NAN),
        };
        let (vol, area) = (if vol.is_finite() { vol } else { -1.0 }, if area.is_finite() { area } else { -1.0 });
        out.push(format!(
            "{head},\"outcome\":\"built\",\"valid\":{valid},\"vol\":{vol},\"area\":{area},\"faces\":{}}}",
            a.face_ids().count()
        ));
    }
    out
}

/// Each worker's stack: some parts recurse past the 2 MiB a spawned thread gets by default (CADFS
/// 00154651 needs more than 2 and less than 4).
const STACK: usize = 64 << 20;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let files: Vec<String> = if a.len() == 3 && a[2].ends_with(".txt") {
        std::fs::read_to_string(&a[2])
            .expect("a list")
            .lines()
            .map(String::from)
            .collect()
    } else {
        a[2..].to_vec()
    };
    let jobs: usize = std::env::var("JOBS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
    let next = AtomicUsize::new(0);
    let rows = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|sc| {
        for _ in 0..jobs {
            let _ = std::thread::Builder::new().stack_size(STACK).spawn_scoped(sc, || loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(f) = files.get(i) else { break };
                let r = std::panic::catch_unwind(|| one(f))
                    .unwrap_or_else(|_| vec![format!("{{\"file\":\"{}\",\"outcome\":\"panicked\"}}", esc(f))]);
                for l in &r {
                    println!("{l}");
                }
                rows.lock().unwrap().extend(r);
            });
        }
    });
    let mut rows = rows.into_inner().unwrap();
    rows.sort();
    std::fs::write(&a[1], rows.join("\n") + "\n").expect("write");
}
