
// independent sanity: material a fillet added must lie in the morphological closing of the part before,
// material it removed outside the opening, on a grid, from meshes only. Near an open end a bead exceeds the
// closing by a lens under r deep; a wrong blend exceeds it further.
fn voxel(m: &forge_mesh::TriMesh, o: [f64; 3], h: f64, d: [usize; 3]) -> Vec<bool> {
    let mut cols: Vec<Vec<(f32, i8)>> = vec![Vec::new(); d[0] * d[1]];
    let (ox, oy) = (o[0] + 0.37123 * h, o[1] + 0.61871 * h);
    for t in &m.triangles {
        let p: Vec<[f64; 3]> = t.iter().map(|&i| { let q = m.positions[i as usize]; [q.x, q.y, q.z] }).collect();
        let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if area == 0.0 { continue; }
        let f = |k: usize, f0: f64| p.iter().map(|q| (q[k] - f0) / h).fold((f64::MAX, f64::MIN), |a, v| (a.0.min(v), a.1.max(v)));
        let (lx, hx) = f(0, ox);
        let (ly, hy) = f(1, oy);
        let (i0, i1) = (lx.ceil().max(0.0) as usize, hx.floor().min(d[0] as f64 - 1.0));
        let (j0, j1) = (ly.ceil().max(0.0) as usize, hy.floor().min(d[1] as f64 - 1.0));
        if i1 < 0.0 || j1 < 0.0 { continue; }
        for i in i0..=(i1 as usize) {
            let x = ox + i as f64 * h;
            for j in j0..=(j1 as usize) {
                let y = oy + j as f64 * h;
                let w0 = ((p[1][0] - x) * (p[2][1] - y) - (p[2][0] - x) * (p[1][1] - y)) / area;
                let w1 = ((p[2][0] - x) * (p[0][1] - y) - (p[0][0] - x) * (p[2][1] - y)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let z = w0 * p[0][2] + w1 * p[1][2] + w2 * p[2][2];
                    cols[i * d[1] + j].push((((z - o[2]) / h) as f32, if area > 0.0 { 1 } else { -1 }));
                }
            }
        }
    }
    let mut v = vec![false; d[0] * d[1] * d[2]];
    for i in 0..d[0] {
        for j in 0..d[1] {
            let c = &mut cols[i * d[1] + j];
            c.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut w = 0i32;
            let mut prev = f32::MIN;
            for &(z, s) in c.iter() {
                if w != 0 && prev < d[2] as f32 {
                    let (a, b) = (prev.ceil().max(0.0) as usize, (z.floor() as f64).min(d[2] as f64 - 1.0));
                    if b >= 0.0 { for k in a..=(b as usize) { v[((i * d[1] + j) * d[2]) + k] = true; } }
                }
                w -= i32::from(s);
                prev = z;
            }
        }
    }
    v
}
fn edt1(f: &mut [f32], v: &mut [usize], z: &mut [f32], o: &mut [f32]) {
    let n = f.len();
    let mut k = 0;
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    for q in 1..n {
        if f[q] >= 1e18 { continue; }
        if f[v[0]] >= 1e18 { v[0] = q; continue; }
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
            if s <= z[k] && k > 0 { k -= 1; continue; }
            if s <= z[k] { v[0] = q; z[1] = f32::INFINITY; break; }
            k += 1; v[k] = q; z[k] = s; z[k + 1] = f32::INFINITY; break;
        }
    }
    if f[v[0]] >= 1e18 { return; }
    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f32 { k += 1; }
        let p = v[k];
        let d = q as f32 - p as f32;
        o[q] = d * d + f[p];
    }
    f.copy_from_slice(&o[..n]);
}
// squared distance in cells to the nearest true cell
fn edt(m: &[bool], d: [usize; 3]) -> Vec<f32> {
    let [a, b, c] = d;
    let mut g: Vec<f32> = m.iter().map(|&x| if x { 0.0 } else { 1e20 }).collect();
    let mx = a.max(b).max(c);
    let (mut v, mut z, mut o, mut l) = (vec![0; mx + 1], vec![0f32; mx + 2], vec![0f32; mx + 1], vec![0f32; mx + 1]);
    for i in 0..a { for j in 0..b { let s = (i * b + j) * c; edt1(&mut g[s..s + c], &mut v, &mut z, &mut o); } }
    for i in 0..a { for k in 0..c { for j in 0..b { l[j] = g[(i * b + j) * c + k]; } edt1(&mut l[..b], &mut v, &mut z, &mut o); for j in 0..b { g[(i * b + j) * c + k] = l[j]; } } }
    for j in 0..b { for k in 0..c { for i in 0..a { l[i] = g[(i * b + j) * c + k]; } edt1(&mut l[..a], &mut v, &mut z, &mut o); for i in 0..a { g[(i * b + j) * c + k] = l[i]; } } }
    g
}
fn closure(b: &Body, a: &Body, r: f64) -> Option<String> {
    let q = forge_mesh::Quality::default();
    let (mb, ma) = (forge_mesh::tessellate(b, &q).ok()?, forge_mesh::tessellate(a, &q).ok()?);
    let old: std::collections::BTreeSet<_> = b.face_ids().map(|f| b.face(f).name).collect();
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for f in a.face_ids().filter(|&f| !old.contains(&a.face(f).name)) {
        let x = a.face_box(f);
        for (k, (l, h)) in [(x.x.lo, x.x.hi), (x.y.lo, x.y.hi), (x.z.lo, x.z.hi)].into_iter().enumerate() {
            lo[k] = lo[k].min(l);
            hi[k] = hi[k].max(h);
        }
    }
    let pad = 4.0 * r + 1.0;
    let ext: Vec<f64> = (0..3).map(|k| hi[k] - lo[k] + 2.0 * pad).collect();
    let h = (r / 8.0).max((ext[0] * ext[1] * ext[2] / 6e6).powf(1.0 / 3.0));
    let d = [(ext[0] / h) as usize + 1, (ext[1] / h) as usize + 1, (ext[2] / h) as usize + 1];
    let o = [lo[0] - pad, lo[1] - pad, lo[2] - pad];
    let (vb, va) = (voxel(&mb, o, h, d), voxel(&ma, o, h, d));
    let rc = ((r / h) * (r / h)) as f32;
    let band = (2.0 * r / h) as usize + 2;
    let inner = |n: usize| { let k = n % d[2]; let j = n / d[2] % d[1]; let i = n / d[2] / d[1]; i >= band && j >= band && k >= band && i + band < d[0] && j + band < d[1] && k + band < d[2] };
    // closing: cells not within r of a cell at least r from the part
    let ds = edt(&vb, d);
    let far: Vec<bool> = ds.iter().map(|&x| x >= rc).collect();
    drop(ds);
    let closed: Vec<bool> = edt(&far, d).iter().map(|&x| x > rc).collect();
    let to_closed = edt(&closed, d);
    let (mut add, mut add_out, mut add_deep) = (0u64, 0u64, 0f32);
    for n in 0..vb.len() {
        if va[n] && !vb[n] && inner(n) {
            add += 1;
            if to_closed[n] > 4.0 { add_out += 1; add_deep = add_deep.max(to_closed[n]); }
        }
    }
    drop(to_closed); drop(closed);
    // opening: cells within r of a cell at least r inside
    let inv: Vec<bool> = vb.iter().map(|&x| !x).collect();
    let core: Vec<bool> = edt(&inv, d).iter().map(|&x| x >= rc).collect();
    let opened: Vec<bool> = edt(&core, d).iter().map(|&x| x < rc).collect();
    let to_out = edt(&opened.iter().map(|&x| !x).collect::<Vec<_>>(), d);
    let (mut rem, mut rem_out, mut rem_deep) = (0u64, 0u64, 0f32);
    for n in 0..vb.len() {
        if vb[n] && !va[n] && inner(n) {
            rem += 1;
            if to_out[n] > 4.0 { rem_out += 1; rem_deep = rem_deep.max(to_out[n]); }
        }
    }
    let c = h * h * h;
    Some(format!("h={h:.3} add={:.2} addout={:.3} adddeep={:.3} rem={:.2} remout={:.3} remdeep={:.3}", add as f64 * c, add_out as f64 * c, add_deep.sqrt() as f64 * h / r, rem as f64 * c, rem_out as f64 * c, rem_deep.sqrt() as f64 * h / r))
}
