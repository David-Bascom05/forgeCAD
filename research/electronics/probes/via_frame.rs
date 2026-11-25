// Verification-only DRC slice. Build with rustc -O via_frame.rs, pass exported fixture paths.
use std::{collections::BTreeSet, env, fs, time::Instant};

#[derive(Copy, Clone)]
struct Via {
    x: i64,
    y: i64,
    diam: i64,
    net: i64,
}

type Findings = BTreeSet<(usize, usize)>;

fn too_close(a: Via, b: Via) -> bool {
    if a.net == b.net {
        return false;
    }
    let dx = i128::from(a.x) - i128::from(b.x);
    let dy = i128::from(a.y) - i128::from(b.y);
    let limit = i128::from(a.diam) + i128::from(b.diam) + 2 * 200_000;
    4 * (dx * dx + dy * dy) < limit * limit
}
fn all(vias: &[Via]) -> Findings {
    let mut found = Findings::new();
    for (j, b) in vias.iter().enumerate() {
        for (i, a) in vias[..j].iter().enumerate() {
            if too_close(*a, *b) {
                found.insert((i, j));
            }
        }
    }
    found
}
fn changed(vias: &[Via], id: usize, base: &Findings) -> Findings {
    let mut found = base.clone();
    found.retain(|&(i, j)| i != id && j != id);
    for (i, &v) in vias.iter().enumerate() {
        if i != id && too_close(v, vias[id]) {
            found.insert((i.min(id), i.max(id)));
        }
    }
    found
}
fn stats(mut ns: Vec<u128>) -> String {
    ns.sort_unstable();
    [50, 95, 99]
        .map(|q| format!("{:.2}", ns[(ns.len() * q).div_ceil(100) - 1] as f64 / 1000.))
        .join(" / ")
}
fn main() {
    for path in env::args().skip(1) {
        let input = fs::read_to_string(&path).unwrap();
        let mut vias: Vec<_> = input
            .lines()
            .map(|row| {
                let v: Vec<i64> = row.split_whitespace().map(|n| n.parse().unwrap()).collect();
                assert!(
                    v.len() == 4
                        && v[2] > 0
                        && v[3] > 0
                        && v[..3].iter().all(|q| q.unsigned_abs() < 1_000_000_000)
                );
                Via {
                    x: v[0],
                    y: v[1],
                    diam: v[2],
                    net: v[3],
                }
            })
            .collect();
        let base = all(&vias);
        let (mut inc, mut full) = (vec![], vec![]);
        let mut different = 0;
        for k in 0..2000 {
            let i = k % vias.len();
            let was = vias[i];
            let near = vias[(i + 1) % vias.len()];
            if k % 3 == 0 {
                vias[i].x = near.x + 200_000;
                vias[i].y = near.y;
            } else {
                vias[i].x += [250_000, -750_000][k % 2];
            }
            let t = Instant::now();
            let now = changed(&vias, i, &base);
            inc.push(t.elapsed().as_nanos());
            let t = Instant::now();
            let reference = all(&vias);
            full.push(t.elapsed().as_nanos());
            assert_eq!(now, reference);
            different += usize::from(now != base);
            vias[i] = was;
        }
        println!(
            "{path}: {} vias, {} initial findings, {different}/2000 changed sets, incremental {} us, full {} us (p50/p95/p99)",
            vias.len(),
            base.len(),
            stats(inc),
            stats(full)
        );
    }
}
