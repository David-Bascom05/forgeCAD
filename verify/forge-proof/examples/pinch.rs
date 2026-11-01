//! Is a ZeroThickness refusal true? Rebuilds the part up to the refused line and the line's tool on
//! its own, then counts the pieces of the result's material on a small sphere about the reported
//! point (Fibonacci samples, joined when neighbours). Two or more pieces: material really meets
//! itself there. One: the refusal is false (research/find-parts). Its two sectors meet at the poles of
//! a line of contact, so the set is eroded by one neighbour first.
//! cargo run --release -p forge-proof --example pinch -- FILE.fc ..
use forge_agent::model::Model;
use forge_bool::{Index, Where};
use forge_math::{Point3, Vec3};

// the three numbers after "at (" in a refusal, or after x: y: z: in a debug form
fn point(s: &str) -> Option<Point3> {
    let num = |t: &str| {
        let t: String = t
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, '-' | '.' | 'e'))
            .collect();
        t.parse::<f64>().ok()
    };
    if let Some(i) = s.find("at (") {
        let v: Vec<f64> = s[i + 4..].split(", ").take(3).filter_map(num).collect();
        return (v.len() == 3).then(|| Point3::new(v[0], v[1], v[2]));
    }
    let at = |k: &str| s.find(k).and_then(|i| num(&s[i + k.len()..]));
    Some(Point3::new(at("x: ")?, at("y: ")?, at("z: ")?))
}

fn pieces(at: Point3, r: f64, inside: &dyn Fn(Point3) -> bool) -> usize {
    let n = 40000;
    let golden = std::f64::consts::PI * (3.0 - 5.0f64.sqrt());
    let dirs: Vec<Vec3> = (0..n)
        .map(|i| {
            let z = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
            let (s, c) = forge_math::dmath::sin_cos(golden * i as f64);
            let q = (1.0 - z * z).sqrt();
            Vec3::new(q * c, q * s, z)
        })
        .collect();
    let all: Vec<bool> = (0..n).map(|i| inside(at + dirs[i] * r)).collect();
    // neighbours lie about sqrt(4 pi / n) apart
    let reach = 1.8 * (4.0 * std::f64::consts::PI / n as f64).sqrt();
    // sectors about a line touch at its two poles, tapering to nothing: eroded, they part
    // (samples run in z order, so neighbours are near in index too)
    let band = |i: usize| {
        let w = (reach * n as f64 / 2.0) as usize + 1;
        i.saturating_sub(w)..(i + w + 1).min(n)
    };
    let on: Vec<usize> = (0..n)
        .filter(|&i| all[i] && band(i).all(|j| all[j] || (dirs[i] - dirs[j]).length() > reach))
        .collect();
    let mut group: Vec<usize> = (0..on.len()).collect();
    fn find(g: &mut [usize], i: usize) -> usize {
        let mut k = i;
        while g[k] != k {
            g[k] = g[g[k]];
            k = g[k];
        }
        k
    }
    for a in 0..on.len() {
        for b in a + 1..on.len() {
            if on[b] >= band(on[a]).end {
                break;
            }
            if (dirs[on[a]] - dirs[on[b]]).length() <= reach {
                let (x, y) = (find(&mut group, a), find(&mut group, b));
                group[x] = y;
            }
        }
    }
    (0..on.len()).filter(|&i| find(&mut group, i) == i).count()
}

fn main() {
    for file in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&file).expect("read");
        let m = Model::new(&src);
        let Some(err) = &m.error else {
            println!("{file} builds");
            continue;
        };
        let Some(at) = point(&err.msg) else {
            println!("{file} no point: {}", err.msg);
            continue;
        };
        let lines: Vec<&str> = src.lines().collect();
        let line = lines[err.at];
        let body = line.split_once(": ").map_or(line, |b| b.1);
        let mut toks: Vec<&str> = body.split_whitespace().collect();
        let combine = toks[0] == "combine";
        let op = if !combine && ["add", "cut", "keep", "new"].contains(&toks[1]) {
            toks.remove(1)
        } else {
            "add"
        };
        if toks[toks.len() - 2] == "in" {
            toks.truncate(toks.len() - 2);
        }
        let tool = if combine {
            None
        } else {
            Some(Model::new(&toks.join(" ")))
        };
        let tc = tool.as_ref().and_then(|t| t.body()).map(|b| b.compact());
        let ti = tc.as_ref().map(Index::new);
        let solids: Vec<_> = m
            .solids_after(m.built())
            .iter()
            .map(|s| s.body.compact())
            .collect();
        let index: Vec<Index> = solids.iter().map(Index::new).collect();
        let mut best = 0;
        for ai in &index {
            // (the reported point is rounded: any solid whose box holds it)
            if !ai.bbox.grow(1e-2).contains(at) {
                continue;
            }
            // (a tangent contact's gap grows with the square of the radius: small spheres miss it)
            for r in [1e-2, 5e-2, 0.2, 1.0] {
                let k = pieces(at, r, &|q| {
                    let ia = ai.classify(q) == Where::Inside;
                    let Some(ti) = &ti else {
                        return index.iter().any(|x| x.classify(q) == Where::Inside);
                    };
                    let ib = ti.classify(q) == Where::Inside;
                    match op {
                        "cut" => ia && !ib,
                        "keep" => ia && ib,
                        _ => ia || ib,
                    }
                });
                best = best.max(k);
            }
        }
        let (x, y, z) = (at.x, at.y, at.z);
        println!("{file} {op} at {x:.4} {y:.4} {z:.4}: {best} pieces");
    }
}
