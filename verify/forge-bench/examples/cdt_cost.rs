use forge_math::Point2;
use forge_mesh::triangulate;
use std::time::Instant;
fn main() {
    for &(nb, ni) in &[(200usize, 0usize), (200, 400), (200, 1600), (800, 6400)] {
        let side = nb / 4;
        let mut pts = Vec::new();
        for k in 0..4 {
            for i in 0..side {
                let t = i as f64 / side as f64;
                pts.push(match k {
                    0 => Point2::new(t, 0.0),
                    1 => Point2::new(1.0, t),
                    2 => Point2::new(1.0 - t, 1.0),
                    _ => Point2::new(0.0, 1.0 - t),
                });
            }
        }
        let g = (ni as f64).sqrt() as usize;
        let interior: Vec<Point2> = (0..g * g)
            .map(|i| {
                Point2::new(
                    (i % g) as f64 / g as f64 + 0.5 / g as f64,
                    (i / g) as f64 / g as f64 + 0.5 / g as f64,
                )
            })
            .collect();
        let lp = vec![(0..pts.len() as u32).collect::<Vec<_>>()];
        let t0 = Instant::now();
        let reps = 20;
        let mut tris = 0;
        for _ in 0..reps {
            tris = triangulate(&pts, &lp, &interior).unwrap().triangles.len();
        }
        let us = t0.elapsed().as_secs_f64() * 1e6 / reps as f64;
        println!(
            "boundary {nb:5} interior {ni:5}: {us:9.1} us, {tris} tris, {:.3} us/tri",
            us / tris as f64
        );
    }
}
