//! One real file through one of our readers (and, for the writers, back out again): what it
//! made of it, as one JSON line on stdout, for research/formats/judge.py to hold against an
//! independent reader of the same file. A panic is caught and printed as `{"panic": ..}`; a
//! crash or hang is the judge's to see (it runs one process per file).
//!
//! ```text
//! formats-probe mesh FILE.stl|obj|3mf          triangles, vertices, closed, open edges, volume, area, box
//! formats-probe mesh-rewrite FILE OUT.stl      read, then written as binary STL (to_binary_stl)
//! formats-probe font FILE.ttf [MAXCP]          units/em, and per code point: glyph, advance, outline
//! formats-probe shx FILE.shx                   shapes, each one's strokes' box and pen end
//! formats-probe dwg FILE.dwg                   version, entities by type, layers, warnings
//! formats-probe dwg-rewrite FILE.dwg OUT.dwg   read, then written back in its own version
//! formats-probe fcstd FILE.FCStd OUTDIR        bodies: status, why, part file
//! formats-probe step-rewrite FILE OUT.step     STEP/IGES/.brep read, each solid exported again
//! ```
use std::fmt::Write;

fn q(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            c if (c as u32) < 32 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o + "\""
}

fn num(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        "null".into()
    }
}

fn err(e: impl std::fmt::Display) -> String {
    format!(
        "{{\"err\":{}}}",
        q(&e.to_string().chars().take(300).collect::<String>())
    )
}

fn read_mesh(path: &str, d: &[u8]) -> Result<forge_mesh::TriMesh, String> {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "stl" => forge_mesh::read::stl(d).map_err(|e| e.to_string()),
        "obj" => forge_mesh::read::obj(d).map_err(|e| e.to_string()),
        "3mf" => forge_import::threemf::read(d).map_err(|e| e.to_string()),
        _ => Err(format!("{ext:?}: not a mesh")),
    }
}

fn mesh(path: &str, d: &[u8]) -> String {
    let m = match read_mesh(path, d) {
        Ok(m) => m,
        Err(e) => return err(e),
    };
    let s = forge_mesh::read::summary(&m);
    // triangles with two corners the same vertex (the reader keeps them; the judge counts them)
    let degen = m
        .triangles
        .iter()
        .filter(|t| t[0] == t[1] || t[1] == t[2] || t[0] == t[2])
        .count();
    // the box of the vertices the triangles use (an OBJ may list others)
    let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for t in &m.triangles {
        for &i in t {
            let p = m.positions[i as usize];
            for (k, c) in [p.x, p.y, p.z].into_iter().enumerate() {
                lo[k] = lo[k].min(c);
                hi[k] = hi[k].max(c);
            }
        }
    }
    let rbox: Vec<String> = lo.iter().chain(hi.iter()).map(|&x| num(x)).collect();
    format!(
        "{{\"rbox\":[{}],\"triangles\":{},\"vertices\":{},\"degenerate\":{degen},\"closed\":{},\"open_edges\":{},\"volume\":{},\"area\":{},\"box\":[{},{},{},{},{},{}]}}",
        rbox.join(","),
        s.triangles,
        s.vertices,
        s.closed,
        s.open_edges,
        s.volume.map_or("null".into(), num),
        num(s.area),
        num(s.bbox.0[0]),
        num(s.bbox.0[1]),
        num(s.bbox.0[2]),
        num(s.bbox.1[0]),
        num(s.bbox.1[1]),
        num(s.bbox.1[2])
    )
}

fn mesh_rewrite(path: &str, d: &[u8], out: &str) -> String {
    match read_mesh(path, d) {
        Ok(m) => {
            let b = forge_mesh::to_binary_stl(&m, "formats-probe");
            match std::fs::write(out, &b) {
                Ok(()) => format!("{{\"written\":{},\"triangles\":{}}}", b.len(), m.triangles.len()),
                Err(e) => err(e),
            }
        }
        Err(e) => err(e),
    }
}

fn seg_json(o: &mut String, s: &forge_font::Seg) {
    match *s {
        forge_font::Seg::Line(a, b) => {
            let _ = write!(o, "[{},{},{},{}]", a.x, a.y, b.x, b.y);
        }
        forge_font::Seg::Quad(a, c, b) => {
            let _ = write!(o, "[{},{},{},{},{},{}]", a.x, a.y, c.x, c.y, b.x, b.y);
        }
    }
}

/// Both of our TrueType readers: forge-font (outlines, advances) and forge-dwg's styles over
/// it (it must take what forge-font takes).
fn font(d: &[u8], max_cp: u32) -> String {
    let dwg = match forge_dwg::fonts::ttf::TtfFont::parse(d.to_vec()) {
        Ok(_) => "true".to_string(),
        Err(e) => q(&e),
    };
    let f = match forge_font::Font::parse(d.to_vec()) {
        Ok(f) => f,
        Err(e) => return format!("{{\"err\":{},\"dwg\":{dwg}}}", q(&format!("{e:?}"))),
    };
    let mut o = format!(
        "{{\"upem\":{},\"dwg\":{dwg},\"warnings\":{},\"cp\":{{",
        f.units_per_em,
        f.warnings.len()
    );
    let mut first = true;
    for cp in 0..=max_cp {
        let Some(c) = char::from_u32(cp) else { continue };
        let g = f.glyph(c);
        if g == 0 {
            continue;
        }
        if !first {
            o.push(',');
        }
        first = false;
        let _ = write!(o, "\"{cp}\":{{\"g\":{g},\"adv\":{},", f.advance(g));
        match f.outline(g) {
            Ok(contours) => {
                o.push_str("\"c\":[");
                for (k, c) in contours.iter().enumerate() {
                    if k > 0 {
                        o.push(',');
                    }
                    o.push('[');
                    for (j, s) in c.iter().enumerate() {
                        if j > 0 {
                            o.push(',');
                        }
                        seg_json(&mut o, s);
                    }
                    o.push(']');
                }
                o.push_str("]}");
            }
            Err(e) => {
                let _ = write!(o, "\"err\":{}}}", q(&format!("{e:?}")));
            }
        }
    }
    o + "}}"
}

fn shx(d: &[u8]) -> String {
    let f = match forge_dwg::fonts::shx::ShxFont::parse(d.to_vec()) {
        Ok(f) => f,
        Err(e) => return err(format!("{e:?}")),
    };
    let mut o = format!(
        "{{\"shapes\":{},\"height\":{},\"run\":{{",
        f.len(),
        num(f.height())
    );
    let mut first = true;
    for n in 0..=u16::MAX {
        if !f.has(n) {
            continue;
        }
        if !first {
            o.push(',');
        }
        first = false;
        match f.run(n, false) {
            Some((strokes, end)) => {
                let pts: Vec<[f64; 2]> = strokes.iter().flat_map(|s| s.iter().copied()).collect();
                let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
                for p in &pts {
                    for k in 0..2 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
                let _ = write!(
                    o,
                    "\"{n}\":{{\"strokes\":{},\"points\":{},\"end\":[{},{}],\"box\":[{},{},{},{}]}}",
                    strokes.len(),
                    pts.len(),
                    num(end[0]),
                    num(end[1]),
                    num(lo[0]),
                    num(lo[1]),
                    num(hi[0]),
                    num(hi[1])
                );
            }
            None => {
                let _ = write!(o, "\"{n}\":null");
            }
        }
    }
    o + "}}"
}

/// Objects keyed as the file types them: a fixed type's number, a class's DXF name; entities
/// apart, and the layers' names.
fn dwg(d: &[u8]) -> String {
    use forge_dwg::types::Data;
    let dr = match forge_dwg::file::read(d) {
        Ok(x) => x,
        Err(e) => return err(e.0),
    };
    let list = dr.class_list();
    let key = |t: u16| -> String {
        if t < 500 {
            t.to_string()
        } else {
            list.get(t as usize - 500)
                .map_or(format!("class {t}"), |c| c.0.clone())
        }
    };
    let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
    let mut ents: std::collections::BTreeMap<String, usize> = Default::default();
    let mut layers = Vec::new();
    let mut raw = 0;
    for o in dr.objects.values() {
        *kinds.entry(key(o.type_code)).or_default() += 1;
        if o.is_entity() {
            *ents.entry(key(o.type_code)).or_default() += 1;
        }
        if matches!(o.data, Data::Raw(_)) {
            raw += 1;
        }
        if let Data::Layer(l) = &o.data {
            layers.push(q(&l.entry.name.text()));
        }
    }
    let j = |m: &std::collections::BTreeMap<String, usize>| -> String {
        m.iter()
            .map(|(k, n)| format!("{}:{n}", q(k)))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "{{\"version\":{},\"objects\":{},\"raw\":{raw},\"kinds\":{{{}}},\"entities\":{{{}}},\"layers\":[{}],\"warnings\":{},\"warning\":{}}}",
        q(&format!("{:?}", dr.ver())),
        dr.objects.len(),
        j(&kinds),
        j(&ents),
        layers.join(","),
        dr.warnings.len(),
        q(dr.warnings.first().map_or("", |s| s.as_str()))
    )
}

fn dwg_rewrite(d: &[u8], out: &str) -> String {
    let dr = match forge_dwg::file::read(d) {
        Ok(x) => x,
        Err(e) => return err(e.0),
    };
    match forge_dwg::file::write(&dr) {
        Ok(b) => match std::fs::write(out, &b) {
            Ok(()) => format!("{{\"written\":{}}}", b.len()),
            Err(e) => err(e),
        },
        Err(e) => format!("{{\"err\":{},\"stage\":\"write\"}}", q(&e.0)),
    }
}

fn body_json(b: &forge_topo::Body) -> String {
    let problems = forge_topo::check(b);
    let (vol, area, merr) = match forge_topo::mass::mass_rel(b, 1e-6) {
        Ok(m) => (m.volume, m.area, String::new()),
        Err(e) => (f64::NAN, f64::NAN, e.to_string()),
    };
    format!(
        "{{\"faces\":{},\"problems\":{},\"first\":{},\"vol\":{},\"area\":{},\"merr\":{}}}",
        b.faces.len(),
        problems.len(),
        q(&problems
            .first()
            .map(|p| format!("{p:?}"))
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect::<String>()),
        num(vol),
        num(area),
        q(&merr)
    )
}

/// Each body's translation: its status and why not, its part file written to OUT/<k>.fg for
/// the judge to build.
fn fcstd(d: &[u8], out: &str) -> String {
    match forge_import::fcstd::translate(d) {
        Ok(bodies) => {
            let b: Vec<String> = bodies
                .iter()
                .enumerate()
                .map(|(k, b)| {
                    let mut file = String::new();
                    if let (Some(fg), false) = (&b.fg, out.is_empty()) {
                        file = format!("{out}/{k}.fg");
                        let _ = std::fs::write(&file, fg);
                    }
                    format!(
                        "{{\"name\":{},\"status\":{},\"why\":{},\"fg\":{}}}",
                        q(&b.name),
                        q(b.status()),
                        q(&b.fail
                            .as_ref()
                            .map(|f| format!("{f:?}"))
                            .unwrap_or_default()
                            .chars()
                            .take(160)
                            .collect::<String>()),
                        q(&file)
                    )
                })
                .collect();
            format!("{{\"bodies\":[{}]}}", b.join(","))
        }
        Err(e) => err(e),
    }
}

fn step_rewrite(d: &[u8], out: &str) -> String {
    let imp = match forge_step::import(d, &forge_step::Limits::default()) {
        Ok(i) => i,
        Err(e) => return err(e),
    };
    let mut text = Vec::new();
    let mut solids = Vec::new();
    for (k, s) in imp.solids.iter().enumerate() {
        let opt = forge_step::Options {
            name: format!("solid{k}"),
            ..Default::default()
        };
        match forge_step::export(&s.body, &opt) {
            Ok(e) => {
                let path = format!("{out}.{k}.step");
                let _ = std::fs::write(&path, e.text.as_bytes());
                text.push(q(&path));
                solids.push(body_json(&s.body));
            }
            Err(e) => solids.push(err(e)),
        }
    }
    format!(
        "{{\"files\":[{}],\"solids\":[{}]}}",
        text.join(","),
        solids.join(",")
    )
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!(
            "formats-probe mesh|mesh-rewrite|font|shx|dwg|dwg-rewrite|fcstd|step-rewrite FILE [OUT|MAXCP]"
        );
        std::process::exit(2);
    }
    let d = match std::fs::read(&a[2]) {
        Ok(d) => d,
        Err(e) => {
            println!("{}", err(e));
            return;
        }
    };
    std::panic::set_hook(Box::new(|info| {
        let at = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let msg = info
            .payload()
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| info.payload().downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        println!(
            "{{\"panic\":{}}}",
            q(&format!("{at}: {}", msg.chars().take(200).collect::<String>()))
        );
    }));
    let out = a.get(3).cloned().unwrap_or_default();
    let r = std::panic::catch_unwind(|| match a[1].as_str() {
        "mesh" => mesh(&a[2], &d),
        "mesh-rewrite" => mesh_rewrite(&a[2], &d, &out),
        "font" => font(&d, out.parse().unwrap_or(0xFFFF)),
        "shx" => shx(&d),
        "dwg" => dwg(&d),
        "dwg-rewrite" => dwg_rewrite(&d, &out),
        "fcstd" => fcstd(&d, &out),
        "step-rewrite" => step_rewrite(&d, &out),
        m => err(format!("{m}: no such mode")),
    });
    if let Ok(s) = r {
        println!("{s}");
    }
}
