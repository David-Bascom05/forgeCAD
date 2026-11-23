//! The kernel side of the fillet spec's differential test.
//!
//! `fillet-spec parts F.fg ..`: every edge each fillet of each part picks, rolled alone at its radius
//! on the body before that fillet (`forge_ops::fillet::edge_blend`, the kernel's own `roll`).
//! `fillet-spec random N SEED`: every edge of N random parts (prisms with bosses, holes and cones), at
//! random radii. One JSON line per edge on stdout, every f64 exact as `[m, e]` (m · 2^e), the format
//! of verify/forge-exact/ForgeExact/Ref/Input.lean. The Lean driver `fillet-check` judges each line.

use forge_geom::blend::{Blend, spine_closest};
use forge_geom::{Curve, CurveGeom, Surface};
use forge_math::{Frame, Point3, Vec3};
use forge_topo::{Body, EdgeId};
use std::fmt::Write;

fn d(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return "[0,0]".into();
    }
    let bits = x.to_bits();
    let neg = bits >> 63 == 1;
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let frac = bits & ((1u64 << 52) - 1);
    let (mut m, mut e) = if exp == 0 { (frac, -1074) } else { (frac | (1u64 << 52), exp - 1075) };
    while m & 1 == 0 {
        m >>= 1;
        e += 1;
    }
    format!("[{}{},{}]", if neg { "-" } else { "" }, m, e)
}

fn p(q: Point3) -> String {
    format!("[{},{},{}]", d(q.x), d(q.y), d(q.z))
}

fn v(q: Vec3) -> String {
    format!("[{},{},{}]", d(q.x), d(q.y), d(q.z))
}

fn frame(f: &Frame) -> String {
    format!("\"o\":{},\"z\":{},\"x\":{}", p(f.origin), v(f.z().get()), v(f.x().get()))
}

fn surf(s: &Surface) -> String {
    match s {
        Surface::Plane(q) => format!("{{\"t\":\"plane\",{}}}", frame(&q.frame)),
        Surface::Cylinder(c) => format!("{{\"t\":\"cylinder\",{},\"r\":{}}}", frame(&c.frame), d(c.radius)),
        Surface::Cone(c) => format!(
            "{{\"t\":\"cone\",{},\"r\":{},\"sin\":{},\"cos\":{}}}",
            frame(&c.frame),
            d(c.radius),
            d(c.sin_a()),
            d(c.cos_a())
        ),
        Surface::Sphere(c) => format!("{{\"t\":\"sphere\",{},\"r\":{}}}", frame(&c.frame), d(c.radius)),
        Surface::Torus(t) => format!(
            "{{\"t\":\"torus\",{},\"major\":{},\"minor\":{}}}",
            frame(&t.frame),
            d(t.major),
            d(t.minor)
        ),
        Surface::Procedural(_) => "{\"t\":\"pipe\"}".into(),
    }
}

fn spine(c: &Curve) -> String {
    match c {
        Curve::Line(l) => format!("{{\"t\":\"line\",\"o\":{},\"d\":{}}}", p(l.origin), v(l.dir.get())),
        Curve::Circle(k) => format!("{{\"t\":\"circle\",{},\"r\":{}}}", frame(&k.frame), d(k.radius)),
        _ => "{\"t\":\"other\"}".into(),
    }
}

/// One edge rolled alone at radius r, as a JSON line.
fn line(src: &str, body: &Body, e: EdgeId, r: f64) -> String {
    let mut out = format!("{{\"src\":\"{}\",\"edge\":{},\"r\":{}", src.replace('"', "'"), e.index(), d(r));
    match forge_ops::fillet::edge_blend(body, e, r) {
        Err(err) => {
            let _ = write!(out, ",\"refused\":\"{err:?}\"}}");
        }
        Ok((b, springs, convex)) => {
            let _ = write!(out, ",\"convex\":{convex},{}", blend(body, e, &b, &springs));
            out.push('}');
        }
    }
    out
}

fn blend(body: &Body, e: EdgeId, b: &Blend, springs: &[Curve; 2]) -> String {
    let x = body.edge(e);
    let ts: Vec<f64> = [0.1, 0.5, 0.9]
        .iter()
        .map(|&s| spine_closest(&b.spine, x.curve.eval(x.range.lo + x.range.width() * s)))
        .collect();
    let pts = |c: &Curve| ts.iter().map(|&t| p(c.eval(t))).collect::<Vec<_>>().join(",");
    format!(
        "\"a\":{},\"b\":{},\"da\":{},\"db\":{},\"radius\":{},\"surf\":{},\"spine\":{},\"centres\":[{}],\"springs\":[[{}],[{}]]",
        surf(&b.a),
        surf(&b.b),
        d(b.da),
        d(b.db),
        d(b.radius),
        surf(&b.surface),
        spine(&b.spine),
        pts(&b.spine),
        pts(&springs[0]),
        pts(&springs[1])
    )
}

fn parts(files: &[String]) {
    for f in files {
        let Ok(src) = std::fs::read_to_string(f) else {
            eprintln!("{f}: unreadable");
            continue;
        };
        let m = forge_lang::fuzz::build(&src);
        for k in 0..m.features.len().min(m.built()) {
            let forge_model::Feature::Fillet { edges, .. } = &m.features[k] else {
                continue;
            };
            let Some(body) = m.body_after(k) else { continue };
            let body = if body.dead > 0 { body.compact() } else { body };
            for (name, radius) in edges {
                let forge_ops::Radius::Constant(r) = radius else { continue };
                let Some(e) = body.edge_by_name(*name) else { continue };
                println!("{}", line(&format!("{f}#{k}"), &body, e, *r));
            }
        }
    }
}

/// A small deterministic generator (SplitMix64).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// A value in [lo, hi), on a grid of 1/64 so the parts read back exactly.
    fn grid(&mut self, lo: f64, hi: f64) -> f64 {
        let steps = ((hi - lo) * 64.0) as u64;
        lo + (self.next() % steps.max(1)) as f64 / 64.0
    }
}

/// A random part: a convex prism, with a boss, a blind hole and a cone revolved on its top.
fn random_part(g: &mut Rng) -> String {
    let n = 3 + (g.next() % 4) as usize;
    let rad = g.grid(30.0, 50.0);
    // a convex polygon: angles in order round the circle, radii varying a little
    let mut angles: Vec<f64> = (0..n).map(|i| (i as f64 + g.grid(0.0, 0.6)) * 360.0 / n as f64).collect();
    angles.sort_by(f64::total_cmp);
    let mut s = String::new();
    let _ = writeln!(s, "shared t = {}", g.grid(8.0, 20.0));
    let _ = writeln!(s, "param boss.h = {}", g.grid(4.0, 12.0));
    let _ = writeln!(s, "param hd.d = {}", 2.0 * g.grid(2.0, 6.0));
    let _ = writeln!(s, "param hd.depth = {}", g.grid(2.0, 6.0));
    let _ = writeln!(s, "shared bc = {}", g.grid(4.0, 8.0));
    for (i, a) in angles.iter().enumerate() {
        let _ = writeln!(s, "shared pa{i} = {a}deg");
    }
    let _ = writeln!(s, "shared pr = {rad}");
    let _ = writeln!(s, "sketch plan(on: xy) {{");
    let pts: Vec<String> = (0..n).map(|i| format!("(pr * cos(pa{i}), pr * sin(pa{i}))")).collect();
    let _ = writeln!(s, "  outline = poly([{}], closed: true)", pts.join(", "));
    let _ = writeln!(s, "}}");
    let _ = writeln!(s, "base = extrude(plan, t)");
    let _ = writeln!(s, "sketch top(on: base.end) {{");
    let _ = writeln!(s, "  c = circle((pr / 4, 0), bc)");
    let _ = writeln!(s, "  h = pt(-pr / 4, 0)");
    let _ = writeln!(s, "}}");
    let _ = writeln!(s, "boss = extrude(top.c, h)");
    let _ = writeln!(s, "hd = hole(at: top.h, d: d, depth: depth)");
    s
}

fn random(n: usize, seed: u64) {
    let mut g = Rng(seed);
    for i in 0..n {
        let src = random_part(&mut g);
        let m = forge_lang::fuzz::build(&src);
        let Some(body) = m.body() else {
            eprintln!("random {seed}/{i}: did not build: {:?}", m.error());
            continue;
        };
        let body = if body.dead > 0 { body.compact() } else { body };
        let edges: Vec<EdgeId> = body.edge_ids().collect();
        for e in edges {
            let r = g.grid(0.25, 6.0);
            println!("{}", line(&format!("random {seed}/{i}"), &body, e, r));
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("parts") => parts(&args[1..]),
        Some("random") => {
            let n = args.get(1).and_then(|x| x.parse().ok()).unwrap_or(20);
            let seed = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(1);
            random(n, seed);
        }
        _ => eprintln!("usage: fillet-spec parts F.fg .. | fillet-spec random N SEED"),
    }
}
