use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
    env, fs,
};
const G: f64 = 0.08;
const W: usize = 1451;
const H: usize = 951;
const N: usize = W * H;
const VIA: f64 = 0.55;
const DRILL: f64 = 0.28;
const TOP: u8 = 1;
const ALL: u8 = 7;
type Pt = (f64, f64);
fn pos(k: usize) -> Pt {
    (((k % N) % W) as f64 * G, ((k % N) / W) as f64 * G)
}
#[derive(Clone)]
struct Pad {
    id: String,
    net: String,
    p: Pt,
    a: f64,
    b: f64,
    hole: f64,
}
#[derive(Clone)]
enum Geo {
    Box(Pt, f64, f64),
    Seg(Pt, Pt, f64),
}
impl Geo {
    fn bound(&self) -> (Pt, Pt) {
        match *self {
            Self::Box((x, y), a, b) => ((x - a, y - b), (x + a, y + b)),
            Self::Seg((x, y), (u, v), r) => ((x.min(u) - r, y.min(v) - r), (x.max(u) + r, y.max(v) + r)),
        }
    }
    fn distance(&self, (x, y): Pt) -> f64 {
        match *self {
            Self::Box((a, b), w, h) => ((x - a).abs() - w).max(0.).hypot(((y - b).abs() - h).max(0.)),
            Self::Seg((a, b), (c, d), r) => {
                let (dx, dy) = (c - a, d - b);
                let t = (((x - a) * dx + (y - b) * dy) / (dx * dx + dy * dy).max(1e-20)).clamp(0., 1.);
                (x - a - t * dx).hypot(y - b - t * dy) - r
            }
        }
    }
    fn scan(&self, extra: f64, mut visit: impl FnMut(usize, f64)) {
        let ((x, y), (u, v)) = self.bound();
        for j in (((y - extra) / G).ceil() as usize).min(H)..=(((v + extra) / G).floor() as usize).min(H - 1)
        {
            for i in
                (((x - extra) / G).ceil() as usize).min(W)..=(((u + extra) / G).floor() as usize).min(W - 1)
            {
                let d = self.distance((i as f64 * G, j as f64 * G));
                if d <= extra {
                    visit(j * W + i, d)
                }
            }
        }
    }
}
#[derive(Clone)]
struct Ob {
    net: String,
    l: u8,
    g: Geo,
}
#[derive(Clone)]
struct Trace {
    net: String,
    l: usize,
    w: f64,
    pts: Vec<Pt>,
}
#[derive(Clone, Default)]
struct Router {
    obs: Vec<Ob>,
    drills: Vec<(Pt, f64)>,
    tracks: Vec<Trace>,
    vias: Vec<(String, Pt)>,
    log: Vec<String>,
}
fn spacing(net: &str) -> f64 {
    if net == "V12" || net.starts_with("OUT") || ["VIN", "FUSED", "RECT", "VPROT"].contains(&net) {
        0.2
    } else {
        0.145
    }
}
fn shape(p: &Pad) -> Geo {
    if p.hole > 0. {
        Geo::Seg(p.p, p.p, p.a)
    } else {
        Geo::Box(p.p, p.a, p.b)
    }
}
impl Router {
    fn via(&mut self, net: &str, k: usize) {
        let p = pos(k);
        self.obs.push(Ob {
            net: net.into(),
            l: ALL,
            g: Geo::Seg(p, p, VIA / 2.),
        });
        self.drills.push((p, DRILL));
        self.vias.push((net.into(), p));
    }
    fn trace(&mut self, net: &str, l: usize, w: f64, pts: Vec<Pt>) {
        if pts.len() < 2 {
            return;
        }
        for p in pts.windows(2) {
            self.obs.push(Ob {
                net: net.into(),
                l: 1 << l,
                g: Geo::Seg(p[0], p[1], w / 2.),
            })
        }
        self.tracks.push(Trace {
            net: net.into(),
            l,
            w,
            pts,
        })
    }
    fn masks(&self, net: &str, w: f64) -> Vec<u8> {
        let mut mask = vec![0u8; N];
        let worst = w.max(VIA) / 2. + 0.225;
        for o in &self.obs {
            if o.net == net {
                continue;
            }
            let gap = spacing(net).max(spacing(&o.net)) + 0.015;
            o.g.scan(worst, |k, d| {
                if d <= gap + w / 2. {
                    mask[k] |= o.l
                }
                if d <= gap + VIA / 2. {
                    mask[k] |= 8
                }
            });
        }
        for &(p, dr) in &self.drills {
            Geo::Seg(p, p, 0.).scan((dr + DRILL) / 2. + 0.055, |k, _| mask[k] |= 8);
        }
        let edge = 0.47 + w.max(VIA) / 2.;
        for k in 0..N {
            let (x, y) = pos(k);
            let (a, b) = (x.min(116. - x), y.min(76. - y));
            if a < edge || b < edge || a + b < 2. + 1.415 * edge {
                mask[k] = 15;
            }
        }
        mask
    }
}
struct Tree {
    v: Vec<u8>,
    bounds: [usize; 4],
    points: Vec<(usize, usize)>,
}
impl Tree {
    fn new() -> Self {
        Self {
            v: vec![0; N],
            bounds: [W, H, 0, 0],
            points: vec![],
        }
    }
    fn add(&mut self, k: usize, bit: u8) {
        if self.v[k] & bit == bit {
            return;
        }
        self.v[k] |= bit;
        let (x, y) = (k % W, k / W);
        self.points.push((x, y));
        self.bounds[0] = self.bounds[0].min(x);
        self.bounds[1] = self.bounds[1].min(y);
        self.bounds[2] = self.bounds[2].max(x);
        self.bounds[3] = self.bounds[3].max(y);
    }
    fn pad(&mut self, p: &Pad) {
        for k in land(p) {
            self.add(k, if p.hole > 0. { ALL } else { TOP })
        }
    }
    fn dist(&self, k: usize) -> usize {
        let (x, y) = (k % W, k / W);
        let dx = self.bounds[0]
            .saturating_sub(x)
            .max(x.saturating_sub(self.bounds[2]));
        let dy = self.bounds[1]
            .saturating_sub(y)
            .max(y.saturating_sub(self.bounds[3]));
        10 * dx.max(dy) + 4 * dx.min(dy)
    }
}
fn land(p: &Pad) -> Vec<usize> {
    let mut cells = vec![];
    let g = shape(p);
    g.scan(0., |k, _| {
        let (x, y) = pos(k);
        if p.hole == 0. || ((x - p.p.0).hypot(y - p.p.1) > p.hole / 2. + 0.02) {
            cells.push(k)
        }
    });
    cells
}
struct Solver {
    cost: Vec<u32>,
    prev: Vec<usize>,
    tag: Vec<u32>,
    serial: u32,
}
impl Solver {
    fn new() -> Self {
        Self {
            cost: vec![0; 3 * N],
            prev: vec![0; 3 * N],
            tag: vec![0; 3 * N],
            serial: 0,
        }
    }
    fn connect(
        &mut self,
        starts: &Tree,
        p: &Pad,
        original: &Pad,
        t: &Tree,
        m: &[u8],
        gnd: bool,
        restrict_top: bool,
        limit: u32,
    ) -> Option<(Vec<usize>, bool, bool)> {
        self.serial += 1;
        let serial = self.serial;
        let mut heap = BinaryHeap::new();
        let cells = land(p);
        let h = |k: usize| if gnd { 0 } else { t.dist(k) as u32 };
        let mut seed = |k: usize, c: u32, via: bool| {
            if self.tag[k] != serial || c < self.cost[k] {
                self.cost[k] = c;
                self.tag[k] = serial;
                self.prev[k] = if via { usize::MAX - 1 } else { usize::MAX };
                heap.push(Reverse((c + h(k % N), c, k)))
            }
        };
        for &(x, y) in &starts.points {
            let k = y * W + x;
            for l in 0..3 {
                if starts.v[k] & (1 << l) != 0 && m[k] & (1 << l) == 0 {
                    seed(k + l * N, 0, false)
                }
            }
        }
        for &k in &cells {
            for l in 0..3 {
                if restrict_top && l > 0 {
                    continue;
                }
                if m[k] & (1 << l) != 0 {
                    continue;
                }
                if l == 0 || p.hole > 0. {
                    seed(k + l * N, 0, false)
                } else if !gnd && m[k] & 8 == 0 {
                    seed(k + l * N, if p.a.min(p.b) > 0.5 { 165 } else { 235 }, true);
                }
            }
        }
        if p.p != original.p {
            for k in land(original) {
                if m[k] & TOP == 0 {
                    seed(k, 0, false)
                }
            }
        }
        // A legal through via may sit just past the toe of a fine-pitch power pin.
        if !restrict_top && !gnd && p.hole == 0. {
            shape(p).scan(VIA / 2. - 0.12, |k, _| {
                if m[k] & 8 == 0 {
                    for l in 1..3 {
                        if m[k] & (1 << l) == 0 {
                            seed(k + l * N, 245, true)
                        }
                    }
                }
            });
        }
        let mut expanded = 0;
        while let Some(Reverse((_, c, z))) = heap.pop() {
            if self.cost[z] != c {
                continue;
            }
            expanded += 1;
            if expanded > limit {
                return None;
            }
            let k = z % N;
            let l = z / N;
            let arrive = t.v[k] & (1 << l) != 0;
            let endvia = gnd && !arrive && l == 0 && m[k] & 8 == 0;
            if arrive || endvia {
                let mut path = vec![z];
                let mut a = z;
                while self.prev[a] < usize::MAX - 1 {
                    a = self.prev[a];
                    path.push(a)
                }
                path.reverse();
                return Some((path, self.prev[a] == usize::MAX - 1, endvia));
            }
            let mut visit = |b: usize, w: u32| {
                let g = c + w;
                if self.tag[b] != serial || g < self.cost[b] {
                    self.tag[b] = serial;
                    self.cost[b] = g;
                    self.prev[b] = z;
                    heap.push(Reverse((g + h(b % N), g, b)))
                }
            };
            for (dx, dy) in [
                (0, 1),
                (0, -1),
                (1, 0),
                (-1, 0),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let i = (k as isize + dx + dy * W as isize) as usize;
                if m[i] & (1 << l) != 0 {
                    continue;
                }
                if dx != 0
                    && dy != 0
                    && (m[(k as isize + dx) as usize] | m[(k as isize + dy * W as isize) as usize]) & (1 << l)
                        != 0
                {
                    continue;
                }
                let pref = if l == 1 && dx != 0 || l == 2 && dy != 0 {
                    1
                } else {
                    0
                };
                visit(i + l * N, if dx != 0 && dy != 0 { 14 + pref } else { 10 + pref })
            }
            if !restrict_top && !gnd && m[k] & 8 == 0 {
                for ll in 0..3 {
                    if ll != l && m[k] & (1 << ll) == 0 {
                        visit(k + ll * N, 190)
                    }
                }
            }
        }
        None
    }
}
fn accept(r: &mut Router, t: &mut Tree, p: &Pad, res: (Vec<usize>, bool, bool), mask: &mut [u8], w: f64) {
    let (path, startvia, endvia) = res;
    let mut drill = |k: usize, t: &mut Tree, r: &mut Router| {
        r.via(&p.net, k);
        t.add(k, ALL);
        let at = pos(k);
        Geo::Seg(at, at, 0.).scan(DRILL + 0.055, |j, _| mask[j] |= 8);
    };
    if startvia {
        drill(path[0] % N, t, r)
    }
    if endvia {
        drill(path[path.len() - 1] % N, t, r)
    }
    let mut line = vec![pos(path[0])];
    let mut delta = 0isize;
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a / N != b / N {
            if line.last() != Some(&pos(a)) {
                line.push(pos(a))
            }
            r.trace(&p.net, a / N, w, line);
            drill(b % N, t, r);
            line = vec![pos(b)];
            delta = 0;
        } else {
            let d = b as isize - a as isize;
            if delta != 0 && d != delta {
                line.push(pos(a))
            }
            delta = d;
        }
    }
    let end = *path.last().unwrap();
    if line.last() != Some(&pos(end)) {
        line.push(pos(end))
    }
    r.trace(&p.net, end / N, w, line);
    for z in path {
        t.add(z % N, 1 << (z / N))
    }
    t.pad(p);
}
fn safe(r: &Router, net: &str, pt: Pt, rad: f64, l: u8) -> bool {
    r.obs.iter().all(|o| {
        o.net == net || o.l & l == 0 || o.g.distance(pt) > rad + spacing(net).max(spacing(&o.net)) + 0.004
    })
}
fn precise_via(r: &mut Router, net: &str, p: Pt) {
    r.obs.push(Ob {
        net: net.into(),
        l: ALL,
        g: Geo::Seg(p, p, VIA / 2.),
    });
    r.drills.push((p, DRILL));
    r.vias.push((net.into(), p));
}
fn write_result(r: &Router, file: &str) {
    let mut s = String::new();
    for (i, tr) in r.tracks.iter().enumerate() {
        let pts = tr
            .pts
            .iter()
            .map(|p| format!("({:.3},{:.3})", p.0, p.1))
            .collect::<Vec<_>>()
            .join(",");
        s += &format!(
            "wire_{i} = track(net:{},layer:{},width:{}mm,points:[{pts}])\n",
            tr.net,
            ["top", "signal", "bottom"][tr.l],
            tr.w
        );
    }
    for (i, (n, (x, y))) in r.vias.iter().enumerate() {
        s += &format!(
            "stitch_{i} = through_via(net:{n},at:({x:.3},{y:.3}),diameter:{VIA}mm,drill:{DRILL}mm)\n"
        );
    }
    fs::write(file, s).unwrap();
    fs::write(format!("{file}.opens"), r.log.join("\n")).unwrap();
}

fn main() {
    let mut r = Router::default();
    let mut islands = BTreeMap::<String, BTreeMap<i32, Vec<(Ob, Option<Pad>)>>>::new();
    for line in fs::read_to_string("metals.tsv").unwrap().lines() {
        let v: Vec<_> = line.split_whitespace().collect();
        let net = v[1];
        let group: i32 = v[2].parse().unwrap();
        let f = |i: usize| v[i].parse::<f64>().unwrap();
        let mut push = |l: u8, g: Geo, p: Option<Pad>| {
            let ob = Ob {
                net: net.into(),
                l,
                g,
            };
            r.obs.push(ob.clone());
            if group >= 0 {
                islands
                    .entry(net.into())
                    .or_default()
                    .entry(group)
                    .or_default()
                    .push((ob, p));
            }
        };
        match v[3] {
            "p" => {
                let p = Pad {
                    id: v[0].into(),
                    net: net.into(),
                    p: (f(4), f(5)),
                    a: f(6),
                    b: f(7),
                    hole: f(8),
                };
                if p.hole > 0. {
                    r.drills.push((p.p, p.hole));
                }
                push(if p.hole > 0. { ALL } else { TOP }, shape(&p), Some(p));
            }
            "v" => {
                let p = (f(4), f(5));
                r.drills.push((p, DRILL));
                push(ALL, Geo::Seg(p, p, VIA / 2.), None);
            }
            l => {
                let lay: usize = l.parse().unwrap();
                for i in (5..v.len() - 2).step_by(2) {
                    push(
                        1 << lay,
                        Geo::Seg((f(i), f(i + 1)), (f(i + 2), f(i + 3)), f(4) / 2.),
                        None,
                    );
                }
            }
        }
    }
    for x in [3.6, 112.4] {
        for y in [3.6, 72.4] {
            r.obs.push(Ob {
                net: "mount".into(),
                l: ALL,
                g: Geo::Seg((x, y), (x, y), 1.66),
            });
        }
    }
    let operation = env::args().nth(1).unwrap_or_default();
    if operation == "--thermal" {
        for line in fs::read_to_string("pads.tsv").unwrap().lines() {
            let z: Vec<_> = line.split_whitespace().collect();
            let id = z[0];
            if z[1] != "GND"
                || !([
                    "U4.21", "U11.21", "U12.21", "U13.21", "U14.21", "U15.21", "U16.21", "R20.1",
                ]
                .contains(&id)
                    || id.starts_with("C3")
                    || id.starts_with("C4"))
            {
                continue;
            }
            let f = |j: usize| z[j].parse::<f64>().unwrap();
            let (x, y, a, b) = (f(2), f(3), f(4), f(5));
            if a.min(b) < 0.65 {
                continue;
            }
            let mut count = 0;
            for dy in [-0.57, 0.57, 0., -0.90, 0.90] {
                for dx in [-0.57, 0.57, 0.] {
                    let p = (x + dx, y + dy);
                    if dx.abs() + 0.30 > a || dy.abs() + 0.30 > b {
                        continue;
                    }
                    if count < 4
                        && safe(&r, "GND", p, VIA / 2. + 0.02, ALL)
                        && r.drills
                            .iter()
                            .all(|&(q, d)| (q.0 - p.0).hypot(q.1 - p.1) > (d + DRILL) / 2. + 0.38)
                    {
                        precise_via(&mut r, "GND", p);
                        count += 1;
                    }
                }
            }
            eprintln!("{} {}", id, count);
        }
    } else if operation == "--escape" {
        let attempt: usize = env::args().nth(2).unwrap_or("0".into()).parse().unwrap();
        for (net, x, sign) in [("RECT", 23.66, 1.), ("VPROT", 19.34, -1.)] {
            let mut choices = vec![];
            for side in [sign, -sign] {
                let start = (x + 0.50 * side, 25.5f64);
                for i in 0..24 {
                    for j in -9i32..=9i32 {
                        let end = (x + side * (0.90 + i as f64 * 0.0825), 25.5 + j as f64 * 0.083);
                        let toe = (start.0 + 0.21 * side, 25.5);
                        let points = [start, toe, end];
                        if !safe(&r, net, end, VIA / 2. + 0.012, ALL)
                            || !r
                                .drills
                                .iter()
                                .all(|&(q, d)| (q.0 - end.0).hypot(q.1 - end.1) > (d + DRILL) / 2. + 0.09)
                        {
                            continue;
                        }
                        if ![2, 4].iter().any(|&l| safe(&r, net, end, 0.404, l)) {
                            continue;
                        }
                        if !points.windows(2).all(|p| {
                            (0..=32).all(|k| {
                                let t = k as f64 / 32.;
                                safe(
                                    &r,
                                    net,
                                    (p[0].0 + t * (p[1].0 - p[0].0), p[0].1 + t * (p[1].1 - p[0].1)),
                                    0.108,
                                    TOP,
                                )
                            })
                        }) {
                            continue;
                        }
                        choices.push(((end.0 - start.0).hypot(end.1 - start.1), points));
                    }
                }
            }
            choices.sort_by(|a, b| a.0.total_cmp(&b.0));
            if !choices.is_empty() {
                let (d, points) = choices[attempt % choices.len()];
                r.trace(net, 0, 0.20, points.to_vec());
                precise_via(&mut r, net, points[2]);
                eprintln!("{} breakout {:.2}, {} choices", net, d, choices.len());
            } else {
                eprintln!("{} no breakout", net);
            }
        }
    } else {
        let mut solver = Solver::new();
        let mut sequence: Vec<_> = islands.into_iter().collect();
        let first = if operation == "--clock" {
            "HSE8"
        } else {
            operation.as_str()
        };
        sequence.sort_by_key(|e| e.0 != first);
        for (net, groups) in sequence {
            let mut t = Tree::new();
            let w = match net.as_str() {
                "GND" => 0.20,
                "V12" => 2.,
                "VPROT" | "RECT" | "VIN" | "FUSED" => 0.8,
                n if n.starts_with("OUT") => 0.60,
                _ => 0.155,
            };
            let mut m = r.masks(&net, w);
            for (i, group) in groups {
                let mut start = Tree::new();
                let mut pads = vec![];
                for (o, p) in &group {
                    let active = if i == 0 { &mut t } else { &mut start };
                    if let Some(p) = p {
                        pads.push(p.clone());
                        active.pad(p);
                    } else {
                        o.g.scan(0., |k, _| active.add(k, o.l));
                    }
                }
                if i == 0 {
                    continue;
                }
                if t.points.is_empty() {
                    continue;
                }
                let p = &pads[0];
                let res = solver.connect(
                    &start,
                    p,
                    p,
                    &t,
                    &m,
                    false,
                    operation == "--clock" && net == "HSE8",
                    4000000,
                );
                if let Some(s) = res {
                    accept(&mut r, &mut t, p, s, &mut m, w);
                    for (x, y) in start.points {
                        let k = y * W + x;
                        t.add(k, start.v[k]);
                    }
                    eprintln!("joined {} {}", net, p.id);
                }
            }
        }
    }
    write_result(&r, "extra.copper");
}
