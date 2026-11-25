use forge_cam_geom::{
    boolean::{Op, boolean},
    region::{Loop, Region, Seg, p2},
};
fn main() {
    let coords: Vec<f64> = include_str!("centres.txt")
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    let a = Region {
        loops: vec![{
            let v = [p2(80., 10.), p2(170., 10.), p2(170., 90.), p2(80., 90.)];
            Loop {
                segs: (0..4)
                    .map(|i| Seg::Line {
                        a: v[i],
                        b: v[(i + 1) % 4],
                    })
                    .collect(),
            }
        }],
    };
    let n: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let b = Region {
        loops: coords
            .chunks_exact(2)
            .take(n)
            .map(|xy| {
                let c = p2(xy[0], xy[1]);
                let a = p2(xy[0] + 0.2, xy[1]);
                Loop {
                    segs: vec![Seg::Arc {
                        a,
                        b: a,
                        c,
                        ccw: true,
                    }],
                }
            })
            .collect(),
    };
    let t = std::time::Instant::now();
    let out = boolean(&a, &b, Op::Diff);
    println!("{n} {:?} {:?}", out.map(|x| x.len()), t.elapsed());
}
