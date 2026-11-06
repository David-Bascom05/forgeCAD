//! A file's model space (or a layout) as SVG, for looking at the scene.
//! `dwgsvg FILE OUT.svg [--layout NAME] [--chord X] [--window X0 Y0 X1 Y1] [--stats] [--dump TYPE] [--boxfont] [--repeat N]`
use forge_dwg::fonts::FontSet;
use forge_dwg::scene::{BoxFont, Fonts, Options, Prim, Space, build};
use forge_dwg::types::Data;
use std::collections::BTreeMap;
use std::fmt::Write;

type Img = Vec<[u8; 3]>;
type Put<'a> = &'a mut dyn FnMut(&mut Img, f64, f64, [u8; 3]);

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let d = forge_dwg::file::read(&std::fs::read(&a[1]).unwrap()).unwrap();
    let mut space = Space::Model;
    let mut opts = Options::default();
    let mut stats = false;
    let mut window = None;
    let mut boxes = false;
    let mut repeat = 1;
    let mut k = 3;
    while k < a.len() {
        match a[k].as_str() {
            "--layout" => {
                k += 1;
                let want = a[k].to_lowercase();
                let mut found = None;
                for (h, o) in &d.objects {
                    if let Data::Layout(l) = &o.data {
                        eprintln!("layout {:?} {h:#x}", l.name.text());
                        if l.name.text().to_lowercase() == want {
                            found = Some(*h);
                        }
                    }
                }
                space = Space::Layout(found.expect("no such layout"));
            }
            "--chord" => {
                k += 1;
                opts.chord = a[k].parse().unwrap();
            }
            "--stats" => stats = true,
            "--boxfont" => boxes = true,
            "--repeat" => {
                k += 1;
                repeat = a[k].parse().unwrap();
            }
            "--window" => {
                let v: Vec<f64> = (0..4).map(|i| a[k + 1 + i].parse().unwrap()).collect();
                window = Some(([v[0], v[1]], [v[2], v[3]]));
                k += 4;
            }
            "--dump" => {
                // every object whose type name contains the argument, as decoded
                k += 1;
                for (h, o) in &d.objects {
                    let name = o.data.name();
                    if name.to_lowercase().contains(&a[k].to_lowercase()) {
                        let mut s = format!("{:#?}", o.data);
                        s.truncate(6000);
                        eprintln!(
                            "{h:#x} owner {:#x} ent {:?}\n{s}",
                            o.owner.handle,
                            o.ent.as_ref().map(|e| (
                                e.entmode,
                                e.layer.handle,
                                &e.color,
                                e.ltype_flags,
                                e.ltype.handle,
                                e.lineweight,
                                e.ltype_scale
                            ))
                        );
                    }
                }
            }
            x => panic!("unknown option {x}"),
        }
        k += 1;
    }
    if stats {
        let mut n: BTreeMap<String, usize> = BTreeMap::new();
        for o in d.objects.values() {
            if o.ent.is_some() {
                *n.entry(o.data.name()).or_default() += 1;
            }
        }
        for (t, c) in n {
            eprintln!("{c:6} {t}");
        }
        // the splines' shapes: (degree, control points, fit points, rational) counts
        let mut sp: BTreeMap<(i32, usize, usize, bool), usize> = BTreeMap::new();
        for o in d.objects.values() {
            if let Data::Spline(x) = &o.data {
                let c = x.points.len().min(99);
                *sp.entry((x.degree, c, x.fit.len().min(99), x.weighted || x.rational))
                    .or_default() += 1;
            }
        }
        let mut spv: Vec<_> = sp.into_iter().collect();
        spv.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (k, c) in spv.iter().take(8) {
            eprintln!("splines {c:6} (degree, ctrl, fit, rational) {k:?}");
        }
    }
    let t0 = std::time::Instant::now();
    let fs = FontSet::new();
    let fonts: &dyn Fonts = if boxes { &BoxFont } else { &fs };
    let mut s = build(&d, space, fonts, &opts);
    let mut best = t0.elapsed();
    for _ in 1..repeat {
        let t = std::time::Instant::now();
        let s2 = build(&d, space, fonts, &opts);
        best = best.min(t.elapsed());
        s = s2;
    }
    eprintln!("built in {best:?} (the fastest of {repeat})");
    if stats {
        // the entities that reach furthest, and the item count per entity type
        let mut far: BTreeMap<u32, f64> = BTreeMap::new();
        let mut per: BTreeMap<String, usize> = BTreeMap::new();
        for it in &s.items {
            let pts: Vec<&[f64; 2]> = match &it.prim {
                Prim::Path { pts, .. } | Prim::Segments { pts } => pts.iter().collect(),
                Prim::Fill { rings } | Prim::Mask { rings } => rings.iter().flatten().collect(),
            };
            let m = pts.iter().map(|p| p[0].abs().max(p[1].abs())).fold(0.0, f64::max);
            let e = far.entry(it.entity).or_default();
            *e = e.max(m);
            *per.entry(d.objects[&it.entity].data.name()).or_default() += 1;
        }
        let mut v: Vec<(u32, f64)> = far.into_iter().collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (h, m) in v.iter().take(5) {
            eprintln!("far {h:#x} {} {m:.0}", d.objects[h].data.name());
        }
        for (t, c) in per {
            eprintln!("items {c:7} {t}");
        }
        let (mut np, mut ns) = (0usize, 0usize);
        for it in &s.items {
            match &it.prim {
                Prim::Segments { pts } => ns += pts.len(),
                Prim::Path { pts, .. } => np += pts.len(),
                _ => {}
            }
        }
        eprintln!("path points {np}, segment points {ns}");
    }
    let Some((lo, hi)) = window.or(s.extents) else {
        eprintln!("empty");
        return;
    };
    if a[2].ends_with(".ppm") {
        // a raster, for drawings too big to look at as SVG
        std::fs::write(&a[2], raster(&s, lo, hi, 1600)).unwrap();
        eprintln!("{} items, extents {lo:?} {hi:?}", s.items.len());
        return;
    }
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let scale = 1600.0 / w.max(h).max(1e-9);
    let mut out = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' style='background:#1e1e1e'>\n\
         <rect width='100%' height='100%' fill='#1e1e1e'/>\n",
        (w * scale).ceil() + 20.0,
        (h * scale).ceil() + 20.0
    );
    let p = |q: &[f64; 2]| {
        format!(
            "{:.2},{:.2}",
            (q[0] - lo[0]) * scale + 10.0,
            (hi[1] - q[1]) * scale + 10.0
        )
    };
    let ring = |rings: &Vec<Vec<[f64; 2]>>| -> String {
        rings
            .iter()
            .map(|r| format!("M{}Z", r.iter().map(p).collect::<Vec<_>>().join("L")))
            .collect()
    };
    let px_per_mm = 1600.0 / 420.0; // a sheet's worth across
    for it in &s.items {
        let c = format!("rgb({},{},{})", it.rgb[0], it.rgb[1], it.rgb[2]);
        let sw = (it.weight as f64 * px_per_mm).max(0.6);
        match &it.prim {
            Prim::Path { pts, closed } => {
                let tag = if *closed { "polygon" } else { "polyline" };
                let pts: Vec<String> = pts.iter().map(p).collect();
                let _ = writeln!(
                    out,
                    "<{tag} points='{}' fill='none' stroke='{c}' stroke-width='{sw:.2}' stroke-linecap='round' stroke-linejoin='round'/>",
                    pts.join(" ")
                );
            }
            Prim::Segments { pts } => {
                let d: String = pts
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|w| format!("M{}L{}", p(&w[0]), p(&w[1])))
                    .collect();
                let _ = writeln!(
                    out,
                    "<path d='{d}' fill='none' stroke='{c}' stroke-width='{sw:.2}' stroke-linecap='round'/>"
                );
            }
            Prim::Fill { rings } => {
                let _ = writeln!(out, "<path d='{}' fill='{c}' fill-rule='evenodd'/>", ring(rings));
            }
            Prim::Mask { rings } => {
                let _ = writeln!(
                    out,
                    "<path d='{}' fill='#1e1e1e' fill-rule='evenodd'/>",
                    ring(rings)
                );
            }
        }
    }
    out.push_str("</svg>\n");
    std::fs::write(&a[2], out).unwrap();
    eprintln!(
        "{} items, {} layers, extents {lo:?} {hi:?}",
        s.items.len(),
        s.layers.len()
    );
}

/// The scene drawn into a PPM image `size` pixels across (lines one pixel wide, fills even-odd).
fn raster(s: &forge_dwg::scene::Scene, lo: [f64; 2], hi: [f64; 2], size: usize) -> Vec<u8> {
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    let k = (size as f64 - 20.0) / w.max(h).max(1e-12);
    let (pw, ph) = ((w * k) as usize + 20, (h * k) as usize + 20);
    let bg = [30u8, 30, 30];
    let mut img = vec![bg; pw * ph];
    let px = |p: &[f64; 2]| [(p[0] - lo[0]) * k + 10.0, (hi[1] - p[1]) * k + 10.0];
    let mut put = |img: &mut Vec<[u8; 3]>, x: f64, y: f64, c: [u8; 3]| {
        if x >= 0.0 && y >= 0.0 && (x as usize) < pw && (y as usize) < ph {
            img[y as usize * pw + x as usize] = c;
        }
    };
    let line = |img: &mut Vec<[u8; 3]>, a: [f64; 2], b: [f64; 2], c: [u8; 3], put: Put| {
        let n = ((b[0] - a[0]).abs().max((b[1] - a[1]).abs()).ceil() as usize).clamp(1, 100_000);
        for i in 0..=n {
            let t = i as f64 / n as f64;
            put(img, a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, c);
        }
    };
    let fill = |img: &mut Vec<[u8; 3]>, rings: &[Vec<[f64; 2]>], c: [u8; 3]| {
        let rs: Vec<Vec<[f64; 2]>> = rings.iter().map(|r| r.iter().map(&px).collect()).collect();
        let (mut y0, mut y1) = (f64::MAX, f64::MIN);
        for p in rs.iter().flatten() {
            y0 = y0.min(p[1]);
            y1 = y1.max(p[1]);
        }
        let (y0, y1) = (y0.max(0.0) as usize, (y1.min(ph as f64 - 1.0)).max(0.0) as usize);
        let mut xs = vec![];
        for y in y0..=y1 {
            let yc = y as f64 + 0.5;
            xs.clear();
            for r in &rs {
                for i in 0..r.len() {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    if (a[1] <= yc) != (b[1] <= yc) {
                        xs.push(a[0] + (yc - a[1]) / (b[1] - a[1]) * (b[0] - a[0]));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for p in xs.as_chunks::<2>().0 {
                let (x0, x1) = (
                    p[0].max(0.0) as usize,
                    (p[1].min(pw as f64 - 1.0)).max(0.0) as usize,
                );
                for x in x0..=x1.min(pw - 1) {
                    img[y * pw + x] = c;
                }
            }
        }
    };
    for it in &s.items {
        match &it.prim {
            Prim::Path { pts, closed } => {
                for w in pts.windows(2) {
                    line(&mut img, px(&w[0]), px(&w[1]), it.rgb, &mut put);
                }
                if *closed && pts.len() > 2 {
                    line(&mut img, px(&pts[pts.len() - 1]), px(&pts[0]), it.rgb, &mut put);
                }
                if pts.len() == 1 {
                    let q = px(&pts[0]);
                    put(&mut img, q[0], q[1], it.rgb);
                }
            }
            Prim::Segments { pts } => {
                for w in pts.as_chunks::<2>().0 {
                    line(&mut img, px(&w[0]), px(&w[1]), it.rgb, &mut put);
                }
            }
            Prim::Fill { rings } => fill(&mut img, rings, it.rgb),
            Prim::Mask { rings } => fill(&mut img, rings, bg),
        }
    }
    let mut out = format!("P6 {pw} {ph} 255\n").into_bytes();
    for c in img {
        out.extend_from_slice(&c);
    }
    out
}
