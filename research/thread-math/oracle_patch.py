#!/usr/bin/env python3
"""Applies (temporarily, never commit the result) a check in curve_surface_in: every closed-form
answer of screw::along is compared with the search on the same arguments, and differences printed.
  python3 research/thread-math/oracle_patch.py; build; run parts with 2> log; git checkout kernel/forge-geom/src/intersect.rs"""
p='kernel/forge-geom/src/intersect.rs'; s=open(p).read()
s=s.replace("""    if let Some(r) = crate::screw::along::curve_along(c, range, s, || near.take().and_then(|f| f())) {
        return r;
    }
    curve_surface_subdivide(c, range, s, near.and_then(|f| f()))""","""    if let Some(r) = crate::screw::along::curve_along(c, range, s, || None) {
        let b = near.take().and_then(|f| f());
        let old = curve_surface_subdivide(c, range, s, b);
        along_check(c, range, s, &r, &old, b);
        return r;
    }
    curve_surface_subdivide(c, range, s, near.and_then(|f| f()))""")
s+='''
pub static ALONG: [std::sync::atomic::AtomicU64; 3] = [const { std::sync::atomic::AtomicU64::new(0) }; 3];
fn along_check(c: &Curve, range: Interval, s: &Surface, new: &CurveSurface, old: &CurveSurface, near: Option<Box3>) {
    use std::sync::atomic::Ordering::Relaxed;
    ALONG[0].fetch_add(1, Relaxed);
    let inb = |p: Point3| near.is_none_or(|b| b.grow(1e-6).contains(p));
    let bad = match (new, old) {
        (CurveSurface::Hits(a), CurveSurface::Hits(b)) => {
            let a: Vec<_> = a.iter().filter(|h| inb(h.point)).collect();
            let b: Vec<_> = b.iter().filter(|h| inb(h.point)).collect();
            ALONG[1].fetch_add(a.len() as u64, Relaxed);
            a.len() != b.len() || a.iter().zip(&b).any(|(x, y)| x.contact != y.contact || x.point.distance(y.point) > 1e-5)
        }
        (x, y) => std::mem::discriminant(x) != std::mem::discriminant(y),
    };
    if bad {
        ALONG[2].fetch_add(1, Relaxed);
        eprintln!("ALONG DIFF {:?} {:?}\\n new {:?}\\n old {:?}\\n near {:?}", range, format!("{:?}", c).chars().take(300).collect::<String>(), new, old, near);
        eprintln!(" surf {}", format!("{:?}", s).chars().take(400).collect::<String>());
    }
}
'''
open(p,'w').write(s)
