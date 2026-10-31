p='kernel/forge-geom/src/intersect.rs'; s=open(p).read()
s=s.replace("""pub(crate) fn curve_surface_subdivide(
    c: &Curve,
    range: Interval,
    s: &Surface,
    near: Option<Box3>,
) -> CurveSurface {
""","""pub static STATS: std::sync::Mutex<std::collections::BTreeMap<String, [u64; 4]>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
pub static EVALS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn dump_stats() { let m = STATS.lock().unwrap(); for (k, v) in m.iter() { eprintln!("{k}\\tcalls {}\\tevals {}\\thits {}\\ton {}", v[0], v[1], v[2], v[3]); } }
pub(crate) fn curve_surface_subdivide(
    c: &Curve,
    range: Interval,
    s: &Surface,
    near: Option<Box3>,
) -> CurveSurface {
    let e0 = EVALS.load(std::sync::atomic::Ordering::Relaxed);
    let r = curve_surface_subdivide2(c, range, s, near);
    let e1 = EVALS.load(std::sync::atomic::Ordering::Relaxed);
    let mut m = STATS.lock().unwrap();
    let v = m.entry(crate::screw::classify_pair(c, s)).or_insert([0; 4]);
    v[0] += 1; v[1] += e1 - e0;
    match &r { CurveSurface::Hits(h) => v[2] += h.len() as u64, CurveSurface::OnSurface | CurveSurface::Overlap => v[3] += 1 }
    r
}
pub(crate) fn curve_surface_subdivide2(
    c: &Curve,
    range: Interval,
    s: &Surface,
    near: Option<Box3>,
) -> CurveSurface {
""")
s=s.replace("""        let m = 0.5 * (a + b);
        let p = c.eval(m);
        let reach = lip * 0.5 * (b - a) + tol::LINEAR;""","""        let m = 0.5 * (a + b);
        let p = c.eval(m);
        EVALS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let reach = lip * 0.5 * (b - a) + tol::LINEAR;""")
open(p,'w').write(s)
p='kernel/forge-geom/src/screw.rs'; s=open(p).read()
s+='''
pub(crate) fn classify_pair(c: &crate::Curve, s: &Surface) -> String {
    use crate::Curve;
    let close = |f: &Frame, g: &Frame| f.origin.distance(g.origin + g.z() * g.z().dot(f.origin - g.origin)) < 1e-6 && f.z().cross(g.z().get()).length() < 1e-9;
    let (cn, cf, cm): (String, Option<Frame>, Option<(f64, f64)>) = match c {
        Curve::Line(l) => ("line".into(), Some(Frame::from_z(l.origin, l.dir)), Some((0.0, 1.0e9))),
        Curve::Circle(k) => ("circle".into(), Some(k.frame), Some((0.0, 0.0))),
        Curve::Procedural(p) => match p.as_any().and_then(|a| a.downcast_ref::<Helix>()) {
            Some(h) => ("helix".into(), Some(h.m.frame), Some((h.m.a, h.m.b))),
            None => ("proc".into(), None, None),
        },
        Curve::Intersection(_) => ("int".into(), None, None),
        _ => ("conic".into(), None, None),
    };
    let (sn, sf, sm): (String, Option<Frame>, Option<(f64, f64)>) = match s {
        Surface::Plane(p) => ("plane".into(), Some(p.frame), None),
        Surface::Cylinder(c) => ("cyl".into(), Some(c.frame), None),
        Surface::Cone(c) => ("cone".into(), Some(c.frame), None),
        Surface::Sphere(_) => ("sphere".into(), None, None),
        Surface::Torus(_) => ("torus".into(), None, None),
        Surface::Procedural(p) => {
            if let Some(f) = p.as_any().and_then(|a| a.downcast_ref::<Screw>()) { ("screw".into(), Some(f.m.frame), Some((f.m.a, f.m.b))) }
            else { let d = format!("{:?}", p); if d.contains("Fence") { ("fence".into(), None, None) } else { ("proc".into(), None, None) } }
        }
    };
    let co = match (cf, sf) { (Some(a), Some(b)) => if close(&a, &b) { "coax" } else { "off" }, _ => "?" };
    let same = match (cm, sm) { (Some(a), Some(b)) => if (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12 { "same" } else if (a.1 - b.1).abs() < 1e-12 { "samepitch" } else { "diff" }, _ => "" };
    format!("{cn} x {sn} {co} {same}")
}
'''
open(p,'w').write(s)
p='interface/forge-agent/examples/featcost.rs'; s=open(p).read()
s=s.replace("""match r {
                Ok(s) => println""","""forge_geom::intersect::dump_stats();
            match r {
                Ok(s) => println""")
open(p,'w').write(s)
