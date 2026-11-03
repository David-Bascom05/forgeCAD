//! Question 2 and 3: an L and a U built directly, cuts across a bend, every number against a
//! closed form. `bracket NAME [out-dir]`; `bracket list`. `cost NAME` for perf stat.
use forge_bool::{Op, boolean};
use forge_geom::{Curve, CurveGeom};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec3, dmath};
use forge_ops::extrude::extrude;
use forge_ops::{FilletEdge, Profile, ProfileLoop, Radius, Segment, fillet};
use forge_topo::mass::mass;
use forge_topo::{Body, EdgeId, Name, check};
use sheet_proto::measure::{bits, report, write_step};
use sheet_proto::reference;
use sheet_proto::region::{Piece, pieces, subtract};
use sheet_proto::sheet::{Chain, Map, build, from_mold, hole, layout, rectangle, total_length};

const T: f64 = 2.0;
const R: f64 = 2.0;
const K: f64 = 0.4;
const WIDTH: f64 = 30.0;

struct Part {
    name: &'static str,
    chain: Chain,
    loops: Vec<Vec<Piece>>,
    /// Flat area in the walls, and in each bend zone (closed form of the part's own trim).
    wall_area: f64,
    zone_area: Vec<f64>,
}

fn ang(deg: f64) -> f64 {
    deg * dmath::PI / 180.0
}

/// The area of the band x0..x1 of the circle (c, a), by the antiderivative of the chord.
fn circle_band(cx: f64, a: f64, x0: f64, x1: f64) -> f64 {
    let f = |x: f64| {
        let u = ((x - cx) / a).clamp(-1.0, 1.0);
        a * a * (u * (1.0 - u * u).sqrt() + dmath::asin(u))
    };
    f(x1) - f(x0)
}

fn parts() -> Vec<Part> {
    let l = |angle: f64| from_mold(T, &[40.0, 30.0], R, angle, K);
    let rect_part = |name: &'static str, chain: Chain| {
        let total = total_length(&chain);
        let zone: Vec<f64> = chain.bends.iter().map(|b| b.ba * WIDTH).collect();
        let za: f64 = zone.iter().sum();
        Part {
            name,
            loops: vec![rectangle(0.0, 0.0, total, WIDTH)],
            wall_area: total * WIDTH - za,
            zone_area: zone,
            chain,
        }
    };
    let mut out = vec![
        rect_part("L90", l(ang(90.0))),
        rect_part("L45", l(ang(45.0))),
        rect_part("L135", l(ang(135.0))),
        rect_part("U90", from_mold(T, &[40.0, 50.0, 30.0], R, ang(90.0), K)),
        rect_part("U60", from_mold(T, &[40.0, 50.0, 30.0], R, ang(60.0), K)),
    ];
    let c90 = l(ang(90.0));
    let (x0, x1) = (c90.walls[0], c90.walls[0] + c90.bends[0].ba);
    let total = total_length(&c90);
    let p = |x: f64, y: f64| Point2::new(x, y);
    // the top edge oblique from end to end: a straight flat line, a helix on the bend
    {
        let (y0, y1) = (WIDTH - 2.0, WIDTH - 12.0);
        let yat = |x: f64| y0 + (y1 - y0) * x / total;
        let c = [p(0.0, 0.0), p(total, 0.0), p(total, y1), p(0.0, y0)];
        let lp = (0..4)
            .map(|i| Piece::Line {
                a: c[i],
                b: c[(i + 1) % 4],
            })
            .collect();
        let zone = 0.5 * (yat(x0) + yat(x1)) * (x1 - x0);
        out.push(Part {
            name: "oblique-edge",
            chain: c90.clone(),
            loops: vec![lp],
            wall_area: 0.5 * (y0 + y1) * total - zone,
            zone_area: vec![zone],
        });
    }
    // a parallelogram window through the bend: two helical walls
    {
        let (xa, xb) = (x0 - 3.0, x1 + 4.0);
        let dy = 5.0;
        let (ya, h) = (9.0, 7.0);
        let c = [p(xa, ya), p(xa, ya + h), p(xb, ya + dy + h), p(xb, ya + dy)];
        let lp = (0..4)
            .map(|i| Piece::Line {
                a: c[i],
                b: c[(i + 1) % 4],
            })
            .collect();
        let zone = (WIDTH - h) * (x1 - x0);
        let cut = h * (xb - xa);
        out.push(Part {
            name: "oblique-window",
            chain: c90.clone(),
            loops: vec![rectangle(0.0, 0.0, total, WIDTH), lp],
            wall_area: total * WIDTH - cut - zone,
            zone_area: vec![zone],
        });
    }
    // circles: inside the bend zone alone, across one tangent line, across both
    let xm = 0.5 * (x0 + x1);
    for (name, cx, a) in [
        ("hole-in-bend", xm, 1.5),
        ("hole-one-line", x0 + 0.5, 3.0),
        ("hole-both-lines", xm, 5.0),
    ] {
        let band = circle_band(cx, a, x0, x1);
        let zone = (x1 - x0) * WIDTH - band;
        out.push(Part {
            name,
            chain: c90.clone(),
            loops: vec![rectangle(0.0, 0.0, total, WIDTH), hole(p(cx, 15.0), a)],
            wall_area: total * WIDTH - dmath::PI * a * a - zone,
            zone_area: vec![zone],
        });
    }
    out
}

/// The boundary area of a part, by integrating each side wall on its own and the skins from flat
/// area: uses neither the kernel's surfaces nor `mass`. In a bend dS = sqrt(rho^2 phi'^2 + y'^2) du drho.
fn true_area(p: &Part) -> f64 {
    let c = &p.chain;
    let t = c.t;
    // strips: (x0, x1, Some(bend))
    let mut strips = Vec::new();
    let mut x = 0.0;
    for (i, w) in c.walls.iter().enumerate() {
        strips.push((x, x + w, None));
        x += w;
        if let Some(b) = c.bends.get(i) {
            strips.push((x, x + b.ba, Some(*b)));
            x += b.ba;
        }
    }
    let gauss = [
        (-0.774_596_669_241_483_4, 5.0 / 18.0),
        (0.0, 8.0 / 18.0),
        (0.774_596_669_241_483_4, 5.0 / 18.0),
    ];
    const GL7: [(f64, f64); 7] = [
        (0.0, 0.417_959_183_673_469_4),
        (0.405_845_151_377_397_2, 0.381_830_050_505_118_9),
        (-0.405_845_151_377_397_2, 0.381_830_050_505_118_9),
        (0.741_531_185_599_394_4, 0.279_705_391_489_276_7),
        (-0.741_531_185_599_394_4, 0.279_705_391_489_276_7),
        (0.949_107_912_342_758_5, 0.129_484_966_168_869_7),
        (-0.949_107_912_342_758_5, 0.129_484_966_168_869_7),
    ];
    let which = |x: f64| {
        strips
            .iter()
            .position(|s: &(f64, f64, Option<sheet_proto::sheet::Bend>)| x < s.1)
            .unwrap_or(strips.len() - 1)
    };
    let mut side = 0.0;
    for l in &p.loops {
        for q in l {
            // spans of u, cut where the piece changes strip (found by bisection)
            let mut cuts = vec![0.0];
            let n = 512;
            for i in 0..n {
                let (mut u0, mut u1) = (i as f64 / n as f64, (i + 1) as f64 / n as f64);
                if which(q.at(u0).x) != which(q.at(u1).x) {
                    let s0 = which(q.at(u0).x);
                    for _ in 0..60 {
                        let m = 0.5 * (u0 + u1);
                        if which(q.at(m).x) == s0 { u0 = m } else { u1 = m }
                    }
                    cuts.push(0.5 * (u0 + u1));
                }
                cuts.push((i + 1) as f64 / n as f64);
            }
            for w in cuts.windows(2) {
                let st = strips[which(q.at(0.5 * (w[0] + w[1])).x)];
                for (gx, gw) in GL7 {
                    let u = 0.5 * (w[0] + w[1]) + 0.5 * (w[1] - w[0]) * gx;
                    let du = 0.5 * (w[1] - w[0]) * gw;
                    let d = q.tangent(u);
                    match st.2 {
                        None => side += t * d.length() * du,
                        Some(b) => {
                            let rn = b.ba / b.angle;
                            for (x, g) in gauss {
                                let rho = b.radius + 0.5 * t * (1.0 + x);
                                side += t * g * ((rho * d.x / rn).powi(2) + d.y * d.y).sqrt() * du;
                            }
                        }
                    }
                }
            }
        }
    }
    let skins: f64 = p
        .zone_area
        .iter()
        .zip(&c.bends)
        .map(|(z, b)| z * (2.0 * b.radius + t) * b.angle / b.ba)
        .sum();
    2.0 * p.wall_area + skins + side
}

/// The same part as today's features: sharp walls, a union, then two fillets. Lengths to the outer
/// mold line, as `from_mold`. An L if `legs` has two, a U (bent the same way) if three, 90 degrees.
fn by_features(legs: &[f64], t: f64, r: f64, width: f64) -> Body {
    let wall = |x0: f64, z0: f64, x1: f64, z1: f64, id: u64| {
        // in the xz plane; the extrude goes along y from y = width down to 0
        let f = Frame::from_axes(
            Point3::new(0.0, width, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, -1.0, 0.0),
        )
        .expect("square axes");
        let lp = Profile::rectangle(Point2::new(x0, z0), x1 - x0, z1 - z0, 0);
        extrude(
            &Profile {
                plane: f,
                loops: vec![lp],
            },
            width,
            id,
        )
        .expect("a box")
    };
    let mut b = wall(0.0, -t, legs[0], 0.0, 1);
    let up = wall(legs[0] - t, -t, legs[0], legs[1] - t, 2);
    b = boolean(&b, &up, Op::Union, 3).expect("the corner");
    let mut edges = vec![((legs[0], -t), r + t), ((legs[0] - t, 0.0), r)];
    if legs.len() == 3 {
        // the third wall goes back over the first, from the top of the second
        let top = legs[1] - t;
        let third = wall(legs[0] - legs[2], top - t, legs[0], top, 4);
        b = boolean(&b, &third, Op::Union, 5).expect("the second corner");
        edges.push(((legs[0], top), r + t));
        edges.push(((legs[0] - t, top - t), r));
    }
    let along_y_at = |b: &Body, x: f64, z: f64| -> EdgeId {
        b.edge_ids()
            .find(|e| {
                let ed = b.edge(*e);
                let (p0, p1) = (ed.curve.eval(ed.range.lo), ed.curve.eval(ed.range.hi));
                matches!(ed.curve, Curve::Line(_))
                    && (p0.x - x).abs() < 1e-9
                    && (p1.x - x).abs() < 1e-9
                    && (p0.z - z).abs() < 1e-9
                    && (p1.z - z).abs() < 1e-9
            })
            .expect("the corner edge")
    };
    // a union leaves coplanar faces split, and the blend would run off the short one
    forge_bool::unify(&mut b);
    let mut b = b.into_compact();
    // one at a time: together they stop at `Overflow`
    for (i, &((x, z), rad)) in edges.iter().enumerate() {
        let pick = [FilletEdge {
            edge: along_y_at(&b, x, z),
            radius: Radius::Constant(rad),
        }];
        b = match fillet(&b, &pick, 6 + i as u64) {
            Ok(n) => n,
            Err(e) => panic!("fillet {i} at {x} {z} r {rad}: {e:?}"),
        };
    }
    b
}

/// How far the body's own edges, unfolded through the inverse map, land from the input outline.
/// Tangent-line edges (inside the material) are left out. Sampled on the kernel's curves.
fn outline_miss(p: &Part, b: &Body, map: Map) -> f64 {
    let strips = layout(&p.chain, map).expect("a chain");
    let mut worst = 0.0f64;
    for e in b.edge_ids() {
        let ed = b.edge(e);
        for i in 0..=16 {
            let t = ed.range.lo + ed.range.width() * i as f64 / 16.0;
            let q = ed.curve.eval(t);
            let mut best = f64::INFINITY;
            for s in &strips {
                let (x, y, w) = s.unfold(q);
                if x < s.x0 - 1e-9 || x > s.x1 + 1e-9 || s.point(x, y, w).distance(q) > 1e-9 {
                    continue;
                }
                if w < -1e-9 || w > p.chain.t + 1e-9 {
                    continue;
                }
                let on_line = (x - s.x0).abs() < 1e-9 || (x - s.x1).abs() < 1e-9;
                let at = Point2::new(x, y);
                let d = p
                    .loops
                    .iter()
                    .flatten()
                    .map(|c| c.at(c.nearest(at)).distance(at))
                    .fold(f64::INFINITY, f64::min);
                best = best.min(if on_line && d > 1e-9 { 0.0 } else { d });
            }
            worst = worst.max(best);
        }
    }
    worst
}

/// Fold then unfold, on every body vertex's source point and on random points in each strip.
fn round_trip(p: &Part) -> f64 {
    let strips = layout(&p.chain, Map::Folded).expect("a chain");
    let mut rng = Rng::new(9);
    let mut worst = 0.0f64;
    for s in &strips {
        for _ in 0..100_000 {
            let (x, y, w) = (
                rng.range(s.x0, s.x1),
                rng.range(0.0, WIDTH),
                rng.range(0.0, p.chain.t),
            );
            let (a, b, c) = s.unfold(s.point(x, y, w));
            worst = worst.max((a - x).abs()).max((b - y).abs()).max((c - w).abs());
        }
    }
    worst
}

fn fuzz(n: u64, seed0: u64) {
    let rows: usize = std::env::var("FUZZ_ROWS").ok().and_then(|v| v.parse().ok()).unwrap_or(400_000);
    let (mut ok, mut refused, mut split) = (0, 0, 0);
    let (mut worst_v, mut worst_ratio) = (0.0f64, 0.0f64);
    let (mut worst_green, mut worst_seed) = ((0.0f64, 0u64), (0.0f64, 0u64));
    let mut over_bound = 0;
    let mut bad = Vec::new();
    let mut hist: std::collections::BTreeMap<String, usize> = Default::default();
    for seed in seed0..seed0 + n {
        let mut g = Rng::new(0xf022_0000 + seed);
        let (t, k) = (g.range(0.5, 4.0), g.range(0.25, 0.5));
        let r = t * g.range(0.5, 4.0);
        let theta = g.range(0.2, 2.9);
        let three = g.bool();
        let ossb = (r + t) * dmath::sin(theta) / (1.0 + dmath::cos(theta));
        let mut leg = || g.range(2.0 * ossb + 8.0, 2.0 * ossb + 60.0);
        let legs: Vec<f64> = if three {
            vec![leg(), leg(), leg()]
        } else {
            vec![leg(), leg()]
        };
        let width = g.range(20.0, 80.0);
        let chain = from_mold(t, &legs, r, theta, k);
        let total = total_length(&chain);
        let wall: Vec<ProfileLoop> = vec![Profile::rectangle(Point2::new(0.0, 0.0), total, width, 0)];
        // cuts mostly near a bend
        let mut cuts = Vec::new();
        for _ in 0..1 + g.below(8) {
            let mut x0 = 0.0;
            let which = g.below(chain.bends.len());
            for i in 0..=which {
                x0 += chain.walls[i];
                if i < which {
                    x0 += chain.bends[i].ba;
                }
            }
            let c = Point2::new(
                x0 + chain.bends[which].ba * g.range(-1.0, 2.0),
                g.range(0.0, width),
            );
            let size = g.range(0.5, 0.2 * width);
            cuts.push(match g.below(3) {
                0 => Profile::circle(c, size, 0),
                1 => {
                    let a = g.range(0.0, dmath::PI);
                    let (s, co) = dmath::sin_cos(a);
                    let at = |x: f64, y: f64| Point2::new(c.x + co * x - s * y, c.y + s * x + co * y);
                    let (w, h) = (size, size * g.range(0.3, 1.0));
                    let q = [at(-w, -h), at(w, -h), at(w, h), at(-w, h)];
                    ProfileLoop::new(
                        (0..4)
                            .map(|i| {
                                (
                                    Segment::Line {
                                        a: q[i],
                                        b: q[(i + 1) % 4],
                                    },
                                    i as u64,
                                )
                            })
                            .collect(),
                    )
                }
                _ => {
                    let a = g.range(0.0, dmath::PI);
                    let (s, co) = dmath::sin_cos(a);
                    let at = |x: f64, y: f64| Point2::new(c.x + co * x - s * y, c.y + s * x + co * y);
                    let (l, q) = (size, size * 0.4);
                    let hp = dmath::PI / 2.0;
                    ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: at(-l, -q),
                                b: at(l, -q),
                            },
                            0,
                        ),
                        (
                            Segment::Arc {
                                center: at(l, 0.0),
                                radius: q,
                                start: a - hp,
                                sweep: 2.0 * hp,
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: at(l, q),
                                b: at(-l, q),
                            },
                            2,
                        ),
                        (
                            Segment::Arc {
                                center: at(-l, 0.0),
                                radius: q,
                                start: a + hp,
                                sweep: 2.0 * hp,
                            },
                            3,
                        ),
                    ])
                }
            });
        }
        let region = match subtract(&wall, &cuts) {
            Ok(r) => r,
            Err(e) => {
                *hist
                    .entry(format!("region {:?}", std::mem::discriminant(&e)))
                    .or_default() += 1;
                bad.push((seed, format!("{e:?}")));
                continue;
            }
        };
        if !region.pinches.is_empty() {
            *hist.entry("pinched".into()).or_default() += 1;
            continue;
        }
        if region.parts.len() != 1 {
            split += 1;
        }
        // truth: scanlines of the inputs, whole and within each bend's zone
        let (a_all, _) = reference::area(&wall, &cuts, rows);
        let mut x = 0.0;
        let mut zones = Vec::new();
        for (i, w) in chain.walls.iter().enumerate() {
            x += w;
            if let Some(b) = chain.bends.get(i) {
                let z = [Profile::rectangle(Point2::new(x, 0.0), b.ba, width, 0)];
                zones.push(reference::area(&z, &cuts, rows).0);
                x += b.ba;
            }
        }
        let za: f64 = zones.iter().sum();
        let rn = chain.bends[0].ba / theta;
        let v_flat = t * a_all;
        let v_fold = t * (a_all - za) + t * za * (r + t / 2.0) / rn;
        let (mut got_fold, mut got_flat) = (0.0, 0.0);
        let mut case_err = 0.0f64;
        let mut fine = true;
        for part in &region.parts {
            let mut loops = vec![part.outer.clone()];
            loops.extend(part.holes.iter().cloned());
            for (map, sum) in [(Map::Folded, &mut got_fold), (Map::Flat, &mut got_flat)] {
                match build(&chain, &loops, map, Name::feature(1)) {
                    Err(e) => {
                        fine = false;
                        let name = format!("{e:?}");
                        let short = name.split(' ').next().unwrap_or("").to_string();
                        *hist.entry(short).or_default() += 1;
                        bad.push((seed, name));
                    }
                    Ok(b) => {
                        let probs = check(&b);
                        if !probs.is_empty() {
                            fine = false;
                            *hist.entry("check".into()).or_default() += 1;
                            bad.push((seed, format!("{map:?} {:?}", probs[0])));
                        }
                        match mass(&b) {
                            Ok(m) => {
                                *sum += m.volume;
                                case_err += m.err[0];
                                worst_ratio = worst_ratio.max(m.err[0]);
                            }
                            Err(e) => {
                                fine = false;
                                *hist.entry("mass".into()).or_default() += 1;
                                bad.push((seed, format!("{map:?} {e}")));
                            }
                        }
                        let mesh = forge_mesh::tessellate(&b, &forge_mesh::Quality::default());
                        match mesh {
                            Ok(m) if m.open_edges().is_empty() => {}
                            Ok(m) => {
                                fine = false;
                                *hist.entry("mesh open".into()).or_default() += 1;
                                bad.push((seed, format!("{map:?} open {}", m.open_edges().len())));
                            }
                            Err(e) => {
                                fine = false;
                                *hist.entry("mesh".into()).or_default() += 1;
                                bad.push((seed, format!("{map:?} {e:?}")));
                            }
                        }
                    }
                }
            }
        }
        if !fine {
            refused += 1;
            continue;
        }
        let miss = (got_fold - v_fold).abs().max((got_flat - v_flat).abs());
        // the same truth by Green's theorem on exact regions, for comparison: Option A's area, not the kernel's mass
        let green_all = region.area();
        let mut green_zone = 0.0;
        let mut x = 0.0;
        for (i, w) in chain.walls.iter().enumerate() {
            x += w;
            if let Some(b) = chain.bends.get(i) {
                let z = [Profile::rectangle(Point2::new(x, 0.0), b.ba, width, 0)];
                green_zone += subtract(&z, &cuts).map_or(f64::NAN, |r| r.area());
                x += b.ba;
            }
        }
        let g_flat = t * green_all;
        let g_fold = t * (green_all - green_zone) + t * green_zone * (r + t / 2.0) / rn;
        let g_miss = (got_fold - g_fold).abs().max((got_flat - g_flat).abs());
        if std::env::var_os("FUZZ_SHOW").is_some() {
            println!("seed {seed} rows {rows} scanline-miss {miss:.3e} green-miss {g_miss:.3e} fold-err-bound t={t:.3}");
        }
        if g_miss > case_err {
            over_bound += 1;
        }
        if g_miss > worst_green.0 {
            worst_green = (g_miss, seed);
        }
        if miss > worst_seed.0 {
            worst_seed = (miss, seed);
        }
        worst_v = worst_v.max(miss);
        if miss > 1e-2 {
            bad.push((
                seed,
                format!("volume fold {got_fold} want {v_fold} flat {got_flat} want {v_flat}"),
            ));
            *hist.entry("volume".into()).or_default() += 1;
        } else {
            ok += 1;
        }
    }
    println!(
        "fuzz {n}: ok {ok} failing {refused} split-parts {split} worst volume miss {worst_v:.2e} worst err {worst_ratio:.2e}"
    );
    println!("  worst scanline miss {:.3e} at seed {}; worst miss against exact-region Green area {:.3e} at seed {}; cases where that miss exceeds the summed err bound: {over_bound}", worst_seed.0, worst_seed.1, worst_green.0, worst_green.1);
    println!("  outcomes {hist:?}");
    for (s, b) in bad.iter().take(12) {
        println!("  seed {s}: {}", b.chars().take(220).collect::<String>());
    }
}

/// The same bracket by features: 40, 30 (and 30) long, 30 wide. `cost` builds it, the rest compares.
fn features_report(legs: &[f64]) {
    let b = by_features(legs, T, R, WIDTH);
    let r = report(&b);
    println!(
        "features {legs:?}: faces {} problems {} V {:.10} err {:.2e} A {:.8} bits {:016x}",
        r.faces, r.problems, r.volume, r.err[0], r.area, r.bits
    );
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let want = a.get(1).map_or("all", String::as_str);
    if want == "fuzz" {
        fuzz(a[2].parse().expect("a count"), a[3].parse().expect("a seed"));
        return;
    }
    if want == "features" {
        features_report(&[40.0, 30.0]);
        features_report(&[40.0, 50.0, 30.0]);
        return;
    }
    if want == "cost-features" {
        let legs: &[f64] = if a[2] == "U" {
            &[40.0, 50.0, 30.0]
        } else {
            &[40.0, 30.0]
        };
        for _ in 0..100 {
            std::hint::black_box(by_features(legs, T, R, WIDTH));
        }
        return;
    }
    if want == "cost-mass" {
        // the build alone with reps = 0: subtract it
        let parts = parts();
        let p = parts.iter().find(|p| p.name == a[2]).expect("a part");
        let reps: usize = a[3].parse().expect("reps");
        let b = build(&p.chain, &p.loops, Map::Folded, Name::feature(1)).expect("builds");
        for _ in 0..reps {
            // a fresh body each time: no caches carried over
            let c = build(&p.chain, &p.loops, Map::Folded, Name::feature(1)).expect("builds");
            std::hint::black_box(mass(&c).ok());
        }
        std::hint::black_box(b);
        return;
    }
    if want == "bits" {
        for p in parts() {
            for map in [Map::Folded, Map::Flat] {
                let b = build(&p.chain, &p.loops, map, Name::feature(1)).expect("builds");
                let m = mass(&b).expect("mass");
                println!(
                    "{} {map:?} body {:016x} volume {:016x} area {:016x}",
                    p.name,
                    bits(&b),
                    m.volume.to_bits(),
                    m.area.to_bits()
                );
            }
        }
        return;
    }
    if want == "refusals" {
        // each named, with the numbers; a real relief ends on a tangent line, so the first needs design
        let c = from_mold(T, &[40.0, 30.0], R, ang(90.0), K);
        let (x0, x1) = (c.walls[0], c.walls[0] + c.bends[0].ba);
        let total = total_length(&c);
        let p = |x, y| Point2::new(x, y);
        let poly = |q: &[Point2]| -> Vec<Piece> {
            (0..q.len())
                .map(|i| Piece::Line {
                    a: q[i],
                    b: q[(i + 1) % q.len()],
                })
                .collect()
        };
        let outer = rectangle(0.0, 0.0, total, WIDTH);
        let cases: Vec<(&str, Vec<Vec<Piece>>)> = vec![
            (
                "relief edge along the tangent line",
                vec![
                    outer.clone(),
                    poly(&[p(x0, 10.0), p(x0, 14.0), p(x0 - 3.0, 14.0), p(x0 - 3.0, 10.0)]),
                ],
            ),
            (
                "hole touching a tangent line",
                vec![outer.clone(), hole(p(x0 - 2.0, 15.0), 2.0)],
            ),
            (
                "corner on a tangent line",
                vec![
                    outer.clone(),
                    poly(&[p(x1, 10.0), p(x1 + 3.0, 12.0), p(x1 + 3.0, 8.0)]),
                ],
            ),
            (
                "a spline loop",
                vec![pieces(&ProfileLoop::new(vec![(
                    Segment::Spline(std::sync::Arc::new(
                        forge_geom::spline::PlaneSpline::through(
                            vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 0.0)],
                            None,
                            None,
                        )
                        .expect("a loop"),
                    )),
                    0,
                )]))],
            ),
        ];
        for (name, l) in cases {
            println!(
                "{name}: {:?}",
                build(&c, &l, Map::Folded, Name::feature(1)).map(|b| b.face_ids().count())
            );
        }
        return;
    }
    if want == "invariants" {
        invariants();
        return;
    }
    if want == "cost" {
        let parts = parts();
        let p = parts.iter().find(|p| p.name == a[2]).expect("a part");
        let map = if a.get(3).map(String::as_str) == Some("flat") {
            Map::Flat
        } else {
            Map::Folded
        };
        for _ in 0..1000 {
            std::hint::black_box(build(&p.chain, &p.loops, map, Name::feature(1)).expect("builds"));
        }
        return;
    }
    let dir = a.get(2).cloned();
    for p in parts() {
        if want != "all" && want != p.name {
            continue;
        }
        for map in [Map::Folded, Map::Flat] {
            let tag = if map == Map::Folded { "fold" } else { "flat" };
            let b = match build(&p.chain, &p.loops, map, Name::feature(1)) {
                Ok(b) => b,
                Err(e) => {
                    println!("{} {tag} REFUSED {e:?}", p.name);
                    continue;
                }
            };
            let r = report(&b);
            let (t, rn) = (p.chain.t, |i: usize| p.chain.bends[i].ba / p.chain.bends[i].angle);
            let v_true = match map {
                Map::Flat => t * (p.wall_area + p.zone_area.iter().sum::<f64>()),
                Map::Folded => {
                    t * p.wall_area
                        + p.zone_area
                            .iter()
                            .enumerate()
                            .map(|(i, z)| t * z * (p.chain.bends[i].radius + t / 2.0) / rn(i))
                            .sum::<f64>()
                }
            };
            println!(
                "{} {tag} outlineMiss {:.2e} certify {}",
                p.name,
                outline_miss(&p, &b, map),
                forge_topo::check::certify(&b).len()
            );
            if map == Map::Folded {
                let at = true_area(&p);
                println!(
                    "{} areaTrue {at:.8} miss {:.2e} errA {:.2e} roundtrip {:.2e}",
                    p.name,
                    (r.area - at).abs(),
                    r.err[1],
                    round_trip(&p)
                );
            }
            println!(
                "{} {tag} faces {} edges {} problems {} open {:?} V {:.10} true {:.10} miss {:.2e} err {:.2e} A {:.8} errA {:.2e} meshV {:.6} meshA {:.6} bits {:016x}{}{}",
                p.name,
                r.faces,
                r.edges,
                r.problems,
                r.open_edges,
                r.volume,
                v_true,
                (r.volume - v_true).abs(),
                r.err[0],
                r.area,
                r.err[1],
                r.mesh_volume,
                r.mesh_area,
                r.bits,
                r.first_problem.map_or(String::new(), |s| format!(" FIRST {s}")),
                r.mass_failed.map_or(String::new(), |s| format!(" MASS {s}")),
            );
            if let Some(d) = &dir {
                let path = format!("{d}/{}-{tag}.step", p.name);
                match write_step(&b, &path) {
                    Ok((f, u)) => println!("  step {path} faces {f} uncertainty {u:.1e}"),
                    Err(e) => println!("  step FAILED {e}"),
                }
            }
        }
    }
}

/// What must not move: K moves nothing folded, R no wall plane; flat length and the volume identity.
fn invariants() {
    let c = |r: f64, k: f64| from_mold(T, &[40.0, 50.0, 30.0], r, ang(90.0), k);
    let loops = |ch: &Chain| vec![rectangle(0.0, 0.0, total_length(ch), WIDTH)];
    let body = |ch: &Chain, m: Map| build(ch, &loops(ch), m, Name::feature(1)).expect("builds");
    let (base, k2) = (c(R, K), c(R, 0.33));
    let (f0, f1) = (body(&base, Map::Folded), body(&k2, Map::Folded));
    let worst = f0
        .vertices
        .iter()
        .zip(&f1.vertices)
        .map(|(a, b)| a.point.distance(b.point))
        .fold(0.0, f64::max);
    println!(
        "K 0.40 to 0.33: folded bodies' bits {} (vertex worst move {worst:.2e})",
        if bits(&f0) == bits(&f1) { "same" } else { "differ" }
    );
    let (s0, s1) = (body(&base, Map::Flat), body(&k2, Map::Flat));
    println!(
        "  flat length {:.9} to {:.9}: expected change {:.9} (2 bends x theta t dK)",
        s0.faces_box().x.width(),
        s1.faces_box().x.width(),
        2.0 * ang(90.0) * T * (K - 0.33)
    );
    let (ta, tb) = (mass(&f0).expect("mass").volume, mass(&s0).expect("mass").volume);
    let want = 2.0 * ang(90.0) * WIDTH * T * T * (0.5 - K);
    println!(
        "  V_fold - V_flat {:.12} against theta L t^2 (1/2 - K) x 2 = {want:.12}, miss {:.2e}",
        ta - tb,
        (ta - tb - want).abs()
    );
    // R: the walls' planes don't move, only where they stop
    let r2 = body(&c(3.5, K), Map::Folded);
    let planes = |b: &Body| {
        let mut p: Vec<[i64; 4]> = b
            .face_ids()
            .filter_map(|f| match &b.face(f).surface {
                forge_geom::Surface::Plane(pl) => {
                    let n = pl.frame.z().get();
                    let d = n.dot(pl.frame.origin.to_vec());
                    let s = if n.x + 1e-3 * n.y + 1e-6 * n.z < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    let q = |v: f64| (v * s * 1e9).round() as i64;
                    Some([q(n.x), q(n.y), q(n.z), q(d)])
                }
                _ => None,
            })
            .filter(|n| n[1] == 0)
            .collect();
        p.sort();
        p.dedup();
        p
    };
    println!(
        "R 2 to 3.5: wall skin planes the same: {}",
        planes(&f0) == planes(&r2)
    );
    // the flat body against the plain extrude of its region
    let total = total_length(&base);
    let e = extrude(
        &Profile {
            plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, -T)),
            loops: vec![Profile::rectangle(Point2::new(0.0, 0.0), total, WIDTH, 0)],
        },
        T,
        1,
    )
    .expect("a box");
    println!(
        "flat: built {:.12} extruded {:.12}",
        mass(&s0).expect("mass").volume,
        mass(&e).expect("mass").volume
    );
    let (bl, bu) = (from_mold(T, &[40.0, 30.0], R, ang(90.0), K), base);
    println!(
        "flat length L {:.9} = (40-4)+BA+(30-4) = {:.9}; U {:.9} = {:.9}",
        total_length(&bl),
        36.0 + 26.0 + ang(90.0) * (R + K * T),
        total_length(&bu),
        36.0 + 42.0 + 26.0 + 2.0 * ang(90.0) * (R + K * T)
    );
    let _ = pieces(&Profile::circle(Point2::new(0.0, 0.0), 1.0, 0));
}
