//! One part file's fate as one JSON line, for tools/find-parts (which runs each part in its own
//! process, so a hang, an abort or a stack overflow costs one case).
//! cargo run --release -p forge-proof --example judge -- FILE.fc | gen SEED LEN [KINDS]
//! gen: a random history; each failure's kind, part, then the history's CPU time.
use forge_agent::history::{Verdict, judge_model, run_kinds};
use std::sync::Mutex;

static WHERE: Mutex<String> = Mutex::new(String::new());

fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' | '\\' => {
                o.push('\\');
                o.push(c);
            }
            c if (c as u32) < 32 => o.push(' '),
            c => o.push(c),
        }
    }
    o
}

// utime + stime from /proc/self/stat, after the command's closing bracket
fn cpu() -> f64 {
    let s = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let tail = s.rsplit(')').next().unwrap_or("");
    let f: Vec<&str> = tail.split_whitespace().collect();
    let t = |i: usize| f.get(i).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0);
    (t(11) + t(12)) / 100.0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let file = args.first().expect("a part file");
    if file == "gen" {
        let num = |i: usize| args.get(i).and_then(|a| a.parse().ok()).expect("gen SEED LEN");
        std::panic::set_hook(Box::new(|_| {}));
        let kinds = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(u64::MAX);
        let r = run_kinds(num(1), num(2) as usize, kinds);
        for (what, src) in &r.fails {
            println!(
                "{{\"v\":\"fail\",\"sig\":\"{}\",\"src\":\"{}\"}}",
                esc(what),
                src.replace('\n', "\\n").replace('"', "\\\"")
            );
        }
        println!(
            "{{\"v\":\"ok\",\"sig\":\"\",\"src\":\"{}\",\"cpu\":{}}}",
            r.src.replace('\n', "\\n").replace('"', "\\\""),
            cpu()
        );
        return;
    }
    let src = std::fs::read_to_string(file).expect("read part file");
    std::panic::set_hook(Box::new(|i| {
        if let Some(l) = i.location()
            && let Ok(mut w) = WHERE.lock()
        {
            *w = format!("{}:{}", l.file(), l.line());
        }
    }));
    let (v, m) = judge_model(&src);
    let (word, sig) = match &v {
        Verdict::Ok => ("ok", String::new()),
        Verdict::Refused(s) => ("refused", s.clone()),
        Verdict::Fail(s) => ("fail", s.clone()),
    };
    let mut o = format!(
        "{{\"v\":\"{word}\",\"sig\":\"{}\",\"loc\":\"{}\"",
        esc(&sig),
        esc(&WHERE.lock().map(|w| w.clone()).unwrap_or_default())
    );
    if let Some(m) = &m {
        let solids = m.solids_after(m.built());
        let mut vol = Some(0.0);
        let mut faces = 0;
        for s in solids {
            faces += s.body.face_ids().count();
            let one =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| forge_topo::mass::mass(&s.body)));
            vol = match (vol, one) {
                (Some(t), Ok(Ok(p))) => Some(t + p.volume),
                _ => None,
            };
        }
        o += &format!(",\"bodies\":{},\"faces\":{faces}", solids.len());
        if let Some(x) = vol {
            o += &format!(",\"vol\":{x}");
        }
        if let Some(f) = &m.error {
            o += &format!(",\"line\":{},\"msg\":\"{}\"", f.at, esc(&f.msg));
        }
    }
    println!("{o},\"cpu\":{}}}", cpu());
}
