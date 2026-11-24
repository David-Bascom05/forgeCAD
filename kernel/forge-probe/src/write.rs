//! The canonical writer: `parse(write(ds)) == ds` and `write(parse(write(ds))) == write(ds)`.
use crate::{Device, Prim};
use forge_tooling::cite::CiteStatus;
use std::fmt::Write;

fn q(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn n(x: f64) -> String {
    format!("{x}")
}

fn arr(v: &[f64]) -> String {
    format!("[{}]", v.iter().map(|x| n(*x)).collect::<Vec<_>>().join(", "))
}

fn status(s: CiteStatus) -> &'static str {
    match s {
        CiteStatus::Published => "published",
        CiteStatus::Estimate => "estimate",
        CiteStatus::Unknown => "unknown",
        CiteStatus::Conflict => "conflict",
        CiteStatus::User => "user",
    }
}

/// One file of records, each starting `[[probe]]`; `header` as `#` comment lines first.
pub fn file(header: &str, ds: &[Device]) -> String {
    let mut o = String::new();
    for l in header.lines() {
        let _ = writeln!(o, "# {l}");
    }
    for d in ds {
        if !o.is_empty() {
            o.push('\n');
        }
        record(&mut o, d);
    }
    o
}

fn record(o: &mut String, d: &Device) {
    let mut kv = |k: &str, v: String| {
        let _ = writeln!(o, "{k} = {v}");
    };
    kv("[[probe]]\nid", q(&d.id));
    kv("kind", q(d.kind.word()));
    if !d.vendor.is_empty() {
        kv("vendor", q(&d.vendor));
    }
    if !d.product.is_empty() {
        kv("product", q(&d.product));
    }
    if !d.note.is_empty() {
        kv("note", q(&d.note));
    }
    if d.real {
        kv("real", "true".into());
    }
    kv("sensing", q(d.sensing.word()));
    if let Some(t) = d.transmission {
        kv("transmission", q(t.word()));
    }
    if let Some(x) = d.repeat {
        kv("repeat", n(x));
    }
    if let Some(x) = d.max_feed {
        kv("max_feed", n(x));
    }
    if let Some(x) = d.overtravel {
        kv("overtravel", arr(&x));
    }
    if let Some(x) = d.stylus {
        kv("stylus", arr(&x));
    }
    if let Some(x) = d.trigger {
        kv("trigger", arr(&x));
    }
    if let Some(x) = d.disc_d {
        kv("disc_d", n(x));
    }
    if let Some(x) = d.min_tool_d {
        kv("min_tool_d", n(x));
    }
    if let Some(x) = d.mass {
        kv("mass", n(x));
    }
    let rings: Vec<String> = d
        .bodies
        .iter()
        .filter_map(Prim::ring)
        .map(|(r, a, b)| arr(&[r, a, b]))
        .collect();
    if !rings.is_empty() {
        let _ = writeln!(o, "body = [{}]", rings.join(", "));
    }
    for b in &d.bodies {
        if b.ring().is_some() {
            continue;
        }
        if let Prim::Cyl { base, axis, r, h } = b {
            let _ = writeln!(
                o,
                "[[probe.cyl]]\nbase = {}\naxis = {}\nr = {}\nh = {}",
                arr(base),
                arr(axis),
                n(*r),
                n(*h)
            );
        }
    }
    for b in &d.bodies {
        if let Prim::Box { min, max } = b {
            let _ = writeln!(o, "[[probe.box]]\nmin = {}\nmax = {}", arr(min), arr(max));
        }
    }
    for f in &d.offers {
        let _ = writeln!(
            o,
            "[[probe.offer]]\nmachines = [{}]\nstatus = {}\nsource = {}\ntaken = {}",
            f.machines.iter().map(|m| q(m)).collect::<Vec<_>>().join(", "),
            q(&f.status),
            q(&f.source),
            q(&f.taken)
        );
        if !f.note.is_empty() {
            let _ = writeln!(o, "note = {}", q(&f.note));
        }
    }
    for c in &d.cites {
        let _ = writeln!(
            o,
            "[[probe.cite]]\npath = {}\nstatus = {}",
            q(&c.path),
            q(status(c.status))
        );
        for (k, v) in [
            ("source", &c.source),
            ("taken", &c.taken),
            ("value", &c.value),
            ("basis", &c.basis),
            ("note", &c.note),
        ] {
            if !v.is_empty() {
                let _ = writeln!(o, "{k} = {}", q(v));
            }
        }
    }
}
