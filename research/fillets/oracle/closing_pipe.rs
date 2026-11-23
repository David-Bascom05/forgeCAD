// closing_pipe FILE.fc H [R]: volume a rolling ball of radius R adds to body+boss (each a capped
// cylinder or a box, read from the part file) by morphological closing, on a grid of step H over
// the box round the boss. Independent of the kernel: exact distances to the two solids, and a point
// is added when it is outside them and no ball of radius R clear of them holds it (the clear
// centre is searched by gradient ascent of the distance inside the ball round the point).
use std::fs;

#[derive(Clone, Copy)]
struct V(f64, f64, f64);
impl V {
    fn add(self, o: V) -> V { V(self.0 + o.0, self.1 + o.1, self.2 + o.2) }
    fn sub(self, o: V) -> V { V(self.0 - o.0, self.1 - o.1, self.2 - o.2) }
    fn mul(self, k: f64) -> V { V(self.0 * k, self.1 * k, self.2 * k) }
    fn dot(self, o: V) -> f64 { self.0 * o.0 + self.1 * o.1 + self.2 * o.2 }
    fn len(self) -> f64 { self.dot(self).sqrt() }
    fn cross(self, o: V) -> V { V(self.1 * o.2 - self.2 * o.1, self.2 * o.0 - self.0 * o.2, self.0 * o.1 - self.1 * o.0) }
}

#[derive(Clone, Copy)]
enum Solid {
    Cyl { o: V, d: V, h: f64, r: f64 },
    Box { o: V, x: V, y: V, z: V, w: f64, l: f64, h: f64 },
}

fn sd(s: &Solid, p: V) -> f64 {
    match *s {
        Solid::Cyl { o, d, h, r } => {
            let v = p.sub(o);
            let a = v.dot(d);
            let rad = v.sub(d.mul(a)).len();
            let e = if a < 0.0 { -a } else if a > h { a - h } else { 0.0 };
            let q = (rad - r).max(0.0);
            if e == 0.0 && q == 0.0 { -(r - rad).min(a).min(h - a) } else { (e * e + q * q).sqrt() }
        }
        Solid::Box { o, x, y, z, w, l, h } => {
            let v = p.sub(o);
            let (a, b, c) = (v.dot(x), v.dot(y), v.dot(z));
            let ex = |t: f64, lo: f64, hi: f64| if t < lo { lo - t } else if t > hi { t - hi } else { 0.0 };
            let (ea, eb, ec) = (ex(a, 0.0, w), ex(b, 0.0, l), ex(c, 0.0, h));
            if ea == 0.0 && eb == 0.0 && ec == 0.0 {
                -(a.min(w - a).min(b).min(l - b).min(c).min(h - c))
            } else {
                (ea * ea + eb * eb + ec * ec).sqrt()
            }
        }
    }
}

fn parse(line: &str) -> Solid {
    let t: Vec<&str> = line.split_whitespace().collect();
    let num = |k: usize| t[k].parse::<f64>().unwrap();
    let i = t.iter().position(|&w| w == "extrude").unwrap();
    let j = if t[i + 1] == "add" { i + 2 } else { i + 1 };
    let h = num(j);
    let zi = t.iter().position(|&w| w == "z").unwrap();
    let z = V(num(zi + 1), num(zi + 2), num(zi + 3));
    let xi = t.iter().position(|&w| w == "x").unwrap();
    let x = V(num(xi + 1), num(xi + 2), num(xi + 3));
    let ai = t.iter().position(|&w| w == "at").unwrap();
    let o = V(num(ai + 1), num(ai + 2), num(ai + 3));
    let z = z.mul(1.0 / z.len());
    let x = x.mul(1.0 / x.len());
    let y = z.cross(x);
    if let Some(ci) = t.iter().position(|&w| w == "circle") {
        let c = o.add(x.mul(num(ci + 1))).add(y.mul(num(ci + 2)));
        Solid::Cyl { o: c, d: z, h, r: num(ci + 3) }
    } else {
        let ri = t.iter().position(|&w| w == "rect").unwrap();
        let c = o.add(x.mul(num(ri + 1))).add(y.mul(num(ri + 2)));
        Solid::Box { o: c, x, y, z, w: num(ri + 3), l: num(ri + 4), h }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = fs::read_to_string(&a[1]).unwrap();
    let lines: Vec<&str> = src.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).collect();
    let (body, boss) = (parse(lines[0]), parse(lines[1]));
    let step: f64 = a[2].parse().unwrap();
    let r: f64 = if a.len() > 3 { a[3].parse().unwrap() } else { lines[2].split_whitespace().nth(1).unwrap().parse().unwrap() };
    let d = |p: V| sd(&body, p).min(sd(&boss, p));
    let grad = |p: V| {
        let e = 1e-6;
        let g = V(d(p.add(V(e, 0., 0.))) - d(p.sub(V(e, 0., 0.))), d(p.add(V(0., e, 0.))) - d(p.sub(V(0., e, 0.))), d(p.add(V(0., 0., e))) - d(p.sub(V(0., 0., e))));
        let l = g.len();
        if l > 0.0 { g.mul(1.0 / l) } else { V(0., 0., 1.) }
    };
    // the box round the boss: the junction lies in it
    let (lo, hi) = match boss {
        Solid::Cyl { o, d: dd, h, r: rr } => {
            let e = o.add(dd.mul(h));
            let g = rr + 2.0 * r;
            (V(o.0.min(e.0) - g, o.1.min(e.1) - g, o.2.min(e.2) - g), V(o.0.max(e.0) + g, o.1.max(e.1) + g, o.2.max(e.2) + g))
        }
        _ => panic!("boss a cylinder"),
    };
    let n = |k: f64| (k / step).ceil() as usize;
    let (nx, ny, nz) = (n(hi.0 - lo.0), n(hi.1 - lo.1), n(hi.2 - lo.2));
    let mut added = 0u64;
    let mut amb = 0u64;
    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let p = V(lo.0 + (i as f64 + 0.5) * step, lo.1 + (j as f64 + 0.5) * step, lo.2 + (k as f64 + 0.5) * step);
                let dp = d(p);
                if dp <= 0.0 || dp >= r {
                    continue;
                }
                // a clear centre in the ball round p: start straight out, then climb
                let mut best = f64::MIN;
                let starts = [grad(p), V(1., 0., 0.), V(-1., 0., 0.), V(0., 1., 0.), V(0., -1., 0.), V(0., 0., 1.), V(0., 0., -1.)];
                let mut covered = false;
                for (si, &s) in starts.iter().enumerate() {
                    let mut c = p.add(s.mul(if si == 0 { r - dp } else { 0.5 * r }));
                    for _ in 0..400 {
                        let dc = d(c);
                        if dc > best { best = dc; }
                        if dc >= r - 1e-9 { covered = true; break; }
                        let mut q = c.add(grad(c).mul((r - dc).max(1e-4)));
                        let off = q.sub(p);
                        let l = off.len();
                        if l > r { q = p.add(off.mul(r / l)); }
                        if q.sub(c).len() < 1e-10 { break; }
                        c = q;
                    }
                    if covered { break; }
                }
                if !covered {
                    added += 1;
                    if best > r - 1e-3 { amb += 1; }
                }
            }
        }
    }
    let cell = step * step * step;
    println!("step {step} r {r} added {:.4} ambiguous {:.4}", added as f64 * cell, amb as f64 * cell);
}
