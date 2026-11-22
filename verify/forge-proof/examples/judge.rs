//! One .fg part file's fate as one JSON line, for tools/find-parts (which runs each part in its
//! own process, so a hang, an abort or a stack overflow costs one case).
//! cargo run --release -p forge-proof --example judge -- FILE.fg | gen SEED LEN [KINDS]
//! gen: a random program (forge_lang::fuzz); each failure's kind, part, then its CPU time.
use forge_lang::fuzz::{Verdict, judge_built, run_kinds};
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

/// The part's box: its meshed triangles (a dome's top) with its lines' and circles' tight boxes.
fn tight_box(b: &forge_topo::Body) -> Option<forge_math::Box3> {
    use forge_math::Box3;
    let b = b.compact();
    let t = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        forge_mesh::tessellate(&b, &forge_mesh::Quality::default())
    }))
    .ok()?
    .ok()?;
    let mut bb = t
        .positions
        .iter()
        .fold(Box3::EMPTY, |a, p| a.hull(&Box3::from_point(*p)));
    for e in b.edge_ids() {
        bb = match &b.edge(e).curve {
            forge_geom::Curve::Line(_) => bb.hull(&b.edge_box(e)),
            forge_geom::Curve::Circle(c) => bb.hull(&c.tight_box(b.edge(e).range)),
            _ => bb,
        };
    }
    Some(bb)
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
    let (v, m) = judge_built(&src);
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
        let mut area = 0.0;
        let mut faces = 0;
        for s in solids {
            faces += s.body.face_ids().count();
            let one =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| forge_topo::mass::mass(&s.body)));
            vol = match (vol, one) {
                (Some(t), Ok(Ok(p))) => {
                    area += p.area;
                    Some(t + p.volume)
                }
                _ => None,
            };
        }
        o += &format!(",\"bodies\":{},\"faces\":{faces}", solids.len());
        if let Some(x) = vol {
            o += &format!(",\"vol\":{x},\"area\":{area}");
        }
        let multi = matches!(&v, Verdict::Fail(s) if s.starts_with("nary: the part is"));
        if (multi || matches!(v, Verdict::Ok))
            && let Some(b) = m.body().and_then(|b| tight_box(&b))
        {
            let (x, y, z) = (b.x, b.y, b.z);
            o += &format!(
                ",\"bbox\":[{},{},{},{},{},{}]",
                x.lo, y.lo, z.lo, x.hi, y.hi, z.hi
            );
            // triangles fall inside a round part's box and face boxes outside it: the two bracket it
            if let Some(b) = m.body() {
                let u = b
                    .face_ids()
                    .map(|f| b.face_box(f))
                    .fold(forge_math::Box3::EMPTY, |a, c| a.hull(&c));
                o += &format!(
                    ",\"bboxu\":[{},{},{},{},{},{}]",
                    u.x.lo, u.y.lo, u.z.lo, u.x.hi, u.y.hi, u.z.hi
                );
            }
        }
        if let Some(f) = m.error() {
            o += &format!(
                ",\"line\":{},\"msg\":\"{}\"",
                m.line_of(&src, f),
                esc(&forge_lang::fuzz::message(f))
            );
        }
    }
    println!("{o},\"cpu\":{}}}", cpu());
}
