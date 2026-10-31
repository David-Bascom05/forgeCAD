//! Time each feature of the torture part: build the history up to n, take the difference.
use forge_bool::{Op, boolean_into};
use forge_math::{Point2, Vec2, dmath};
use forge_model::{Combine, Extent, Feature, Turn, torture};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use std::time::Instant;
fn main() {
    let part = torture::part(usize::MAX);
    let mut body = None;
    let mut times: Vec<(f64, usize, &'static str)> = Vec::new();
    for (i, f) in part.features.iter().enumerate() {
        let id = i as u64 + 1;
        let (tool, op, kind) = match f {
            Feature::Extrude {
                profile,
                extent: Extent::Distance(distance),
                op,
            } => (extrude(profile, *distance, id).unwrap(), *op, "extrude"),
            Feature::Revolve {
                profile,
                axis,
                extent: Turn::Angle(angle),
                op,
            } => (revolve(profile, *axis, *angle, id).unwrap(), *op, "revolve"),
            Feature::Ball { frame, radius, op } => {
                let q = dmath::FRAC_PI_2;
                let half = ProfileLoop::new(vec![
                    (
                        Segment::Arc {
                            center: Point2::ORIGIN,
                            radius: *radius,
                            start: -q,
                            sweep: 2.0 * q,
                        },
                        0,
                    ),
                    (
                        Segment::Line {
                            a: Point2::new(0.0, *radius),
                            b: Point2::new(0.0, -*radius),
                        },
                        1,
                    ),
                ]);
                let p = Profile {
                    plane: *frame,
                    loops: vec![half],
                };
                (
                    revolve(
                        &p,
                        Axis2 {
                            origin: Point2::ORIGIN,
                            dir: Vec2::Y,
                        },
                        dmath::TAU,
                        id,
                    )
                    .unwrap(),
                    *op,
                    "ball",
                )
            }
            _ => continue,
        };
        let t = Instant::now();
        body = Some(match (body, op) {
            (_, Combine::New) => tool,
            (Some(mut b), op) => {
                let op = match op {
                    Combine::Add => Op::Union,
                    Combine::Cut => Op::Subtract,
                    _ => Op::Intersect,
                };
                boolean_into(&mut b, &tool, op, id + 1_000_000).unwrap();
                if b.dead > b.faces.len() / 2 {
                    b.compact()
                } else {
                    b
                }
            }
            (None, _) => unreachable!(),
        });
        times.push((t.elapsed().as_secs_f64() * 1e6, i, kind));
    }
    let mut by_kind = std::collections::BTreeMap::new();
    for &(us, _, k) in &times {
        let e = by_kind.entry(k).or_insert((0.0, 0));
        e.0 += us;
        e.1 += 1;
    }
    for (k, (us, n)) in by_kind {
        println!(
            "{k:8} {n:4} features {us:10.0} us total {:8.1} us each",
            us / n as f64
        );
    }
    times.sort_by(|a, b| b.0.total_cmp(&a.0));
    for &(us, i, k) in times.iter().take(8) {
        println!("slowest: feature {i} ({k}) {us:.0} us");
    }
}
