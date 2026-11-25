//! Which faces' enclosures (forge_cam::enclose) reach furthest past their solid's box.
fn main() {
    let p = std::env::args().nth(1).expect("STEP");
    let im = forge_step::import(&std::fs::read(&p).expect("read"), &forge_step::Limits::default())
        .expect("import");
    let mut rows = vec![];
    for (si, s) in im.solids.iter().enumerate() {
        let b = &s.body;
        let mut sb = forge_math::Box3::EMPTY;
        for f in b.face_ids() {
            sb = sb.hull(&b.face_box(f));
        }
        let lo = [sb.x.lo, sb.y.lo, sb.z.lo];
        let hi = [sb.x.hi, sb.y.hi, sb.z.hi];
        for (k, f) in b.face_ids().enumerate() {
            let mut out = vec![];
            if let Ok(how) = forge_cam::enclose::enclose_face(b, f, k, &mut out) {
                let mut over = 0.0f64;
                for g in &out {
                    for a in 0..3 {
                        over = over.max(lo[a] - g.bbox[a].lo).max(g.bbox[a].hi - hi[a]);
                    }
                }
                let fb = b.face_box(f);
                let s = format!("{:?}", b.face(f).surface);
                rows.push((
                    over,
                    si,
                    k,
                    format!("{how:?} {}", s.chars().take(50).collect::<String>()),
                    [fb.x.hi - fb.x.lo, fb.y.hi - fb.y.lo, fb.z.hi - fb.z.lo],
                ));
            }
        }
    }
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for r in rows.iter().take(15) {
        println!(
            "{:8.2} solid {:2} face {:3} box {:?} {}",
            r.0,
            r.1,
            r.2,
            r.4.map(|x| (x * 10.0).round() / 10.0),
            r.3
        );
    }
}
