//! real.rs's exact path for square-root-free values (`rational_sign`, inside `Ctx::sign`)
//! against refinement to the separation bound (`sign_by_refinement`), on random expressions of
//! +, -, *, / over f64 leaves, with exact zeros built in (a*b - b*a, (a/b)*b - a, cross products
//! of collinear points): the two must agree on every sign.
use forge_cam_geom::real::{Ctx, R};

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn r(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
    fn n(&mut self, k: usize) -> usize {
        (self.f() * k as f64) as usize % k
    }
}

fn expr(cx: &mut Ctx, g: &mut Rng, leaves: &[R], depth: u32) -> R {
    if depth == 0 || g.n(4) == 0 {
        return leaves[g.n(leaves.len())];
    }
    let a = expr(cx, g, leaves, depth - 1);
    let b = expr(cx, g, leaves, depth - 1);
    match g.n(5) {
        0 => cx.add(a, b),
        1 => cx.sub(a, b),
        2 => cx.mul(a, b),
        3 if cx.iv(b).lo > 0.0 || cx.iv(b).hi < 0.0 => cx.div(a, b),
        _ => cx.neg(a),
    }
}

#[test]
fn the_rational_path_agrees_with_refinement() {
    let mut g = Rng(11);
    let (mut zeros, mut nonzero) = (0, 0);
    for case in 0..400 {
        let mut cx = Ctx::new();
        let leaves: Vec<R> = (0..5)
            .map(|_| {
                let x = g.r(-50.0, 50.0);
                cx.lit(if case % 3 == 0 { (x * 8.0).round() / 8.0 } else { x })
            })
            .collect();
        let a = expr(&mut cx, &mut g, &leaves, 4);
        let b = expr(&mut cx, &mut g, &leaves, 3);
        let v = match case % 4 {
            // a*b - b*a, built so the filter cannot see it is zero
            0 => {
                let ab = cx.mul(a, b);
                let ba = cx.mul(b, a);
                let s = cx.add(ab, leaves[0]);
                let t = cx.add(ba, leaves[0]);
                cx.sub(s, t)
            }
            // (a / b) * b - a
            1 if cx.iv(b).lo > 0.0 || cx.iv(b).hi < 0.0 => {
                let q = cx.div(a, b);
                let m = cx.mul(q, b);
                cx.sub(m, a)
            }
            // the cross product of p→q with p→(p + t(q − p)): collinear, zero
            2 => {
                let (px, py, qx, qy) = (leaves[0], leaves[1], leaves[2], leaves[3]);
                let t = cx.lit(g.r(-2.0, 2.0));
                let (dx, dy) = (cx.sub(qx, px), cx.sub(qy, py));
                let (tx, ty) = (cx.mul(t, dx), cx.mul(t, dy));
                let (rx, ry) = (cx.add(px, tx), cx.add(py, ty));
                let (ux, uy) = (cx.sub(rx, px), cx.sub(ry, py));
                let c1 = cx.mul(dx, uy);
                let c2 = cx.mul(dy, ux);
                cx.sub(c1, c2)
            }
            _ => cx.sub(a, b),
        };
        let s = cx.sign(v);
        let r = cx.sign_by_refinement(v);
        assert_eq!(s, r, "case {case}");
        if s == std::cmp::Ordering::Equal {
            zeros += 1;
        } else {
            nonzero += 1;
        }
    }
    assert!(zeros >= 150 && nonzero >= 100, "{zeros} zeros, {nonzero} nonzero");
}
