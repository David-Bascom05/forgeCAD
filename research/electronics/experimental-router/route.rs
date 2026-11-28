use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap, HashSet},
    env, fs,
};
const G: f64 = 0.125;
const W: usize = 929;
const H: usize = 609;
const N: usize = W * H;
const VIA: f64 = 0.54;
const DRILL: f64 = 0.28;
const TOP: u8 = 1;
const BOT: u8 = 4;
const ALL: u8 = 7;
type Pt = (f64, f64);
fn pos(k: usize) -> Pt {
    (((k % N) % W) as f64 * G, ((k % N) / W) as f64 * G)
}
fn cell(p: Pt) -> usize {
    (p.1 / G).round() as usize * W + (p.0 / G).round() as usize
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
fn width(net: &str) -> f64 {
    match net {
        "V12" => 2.,
        "VIN" | "FUSED" => 1.8,
        "RECT" | "VPROT" => 0.8,
        "SW" | "ISRET" | "SENSE" => 1.4,
        "HO" | "LO" => 0.35,
        "V5" | "V33" | "SW1" => 0.32,
        "GND" => 0.22,
        _ if net.starts_with("OUT") => 0.6,
        _ => 0.155,
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
fn portal(r: &mut Router, p: &Pad, direction: Pt, stagger: bool) -> Option<Pad> {
    if p.hole > 0. {
        return None;
    }
    let net = &p.net;
    let w = if spacing(net) < 0.19 { 0.15 } else { width(net) };
    let along = if direction.0 != 0. { p.a } else { p.b };
    // Try the package toe and heel. Stagger adjacent signals on two rows.
    for &sign in &[1., -1.] {
        let v = (direction.0 * sign, direction.1 * sign);
        let near = along + if stagger { 1.80 } else { 0.34 };
        let mut distances = vec![
            near,
            along + 0.224,
            along + 0.231,
            along + 0.20,
            along + 0.249,
            along + 0.12,
            along,
            0.,
            along * 0.65,
        ];
        distances.extend((0..21).map(|i| along + 0.32 + i as f64 * 0.1));
        for dist in distances {
            let pt = (p.p.0 + v.0 * dist, p.p.1 + v.1 * dist);
            if !safe(r, net, pt, VIA / 2., ALL)
                || r.drills
                    .iter()
                    .any(|&(a, d)| (a.0 - pt.0).hypot(a.1 - pt.1) < (DRILL + d) / 2. + 0.05)
            {
                continue;
            }
            if dist > along + VIA / 2. - 0.021 {
                let intercept = (w / 2. - 0.045).max(0.);
                let from = (
                    p.p.0 + v.0 * (along + intercept),
                    p.p.1 + v.1 * (along + intercept),
                );
                let to = pt;
                if !(0..=30).all(|i| {
                    let t = i as f64 / 30.;
                    safe(
                        r,
                        net,
                        (from.0 + t * (to.0 - from.0), from.1 + t * (to.1 - from.1)),
                        w / 2.,
                        TOP,
                    )
                }) {
                    continue;
                }
                r.trace(net, 0, w, vec![from, to]);
            }
            precise_via(r, net, pt);
            return Some(Pad {
                id: p.id.clone(),
                p: pt,
                net: net.clone(),
                a: VIA / 2.,
                b: VIA / 2.,
                hole: DRILL,
            });
        }
    }
    None
}
fn prepare(r: &mut Router, by: &mut BTreeMap<String, Vec<Pad>>, phase: &str, failed: &HashSet<String>) {
    let mut group = BTreeMap::<String, (f64, f64, usize)>::new();
    for p in by.values().flatten() {
        let z = group.entry(p.id.split('.').next().unwrap().into()).or_default();
        z.0 += p.p.0;
        z.1 += p.p.1;
        z.2 += 1;
    }
    for (n, ps) in by.iter_mut() {
        if ps.len() < 2 || n.starts_with('@') {
            continue;
        }
        for p in ps {
            let (name, num) = p.id.split_once('.').unwrap();
            let wanted = if phase == "V12" {
                n == "V12" && (name.starts_with('U') || name.starts_with('R'))
            } else {
                (failed.contains(&p.id) && name.starts_with("U"))
                    || ((name == "U2" || name == "U4" || name == "U1" || name.starts_with("U1"))
                        && p.a.min(p.b) < 0.40
                        && !["V12", "OUT1", "OUT2", "OUT3", "OUT4", "OUT5", "OUT6"].contains(&n.as_str()))
            };
            if !wanted {
                continue;
            }
            let (x, y, k) = group[name];
            let (dx, dy) = (p.p.0 - x / k as f64, p.p.1 - y / k as f64);
            let v = if p.a > p.b {
                (if dx >= 0. { 1. } else { -1. }, 0.)
            } else {
                (0., if dy >= 0. { 1. } else { -1. })
            };
            if let Some(a) = portal(r, p, v, num.parse::<usize>().unwrap_or(0) % 2 == 0) {
                *p = a;
            }
        }
    }
}
fn route_one(
    r: &mut Router,
    solver: &mut Solver,
    net: String,
    by: &BTreeMap<String, Vec<Pad>>,
    original: &BTreeMap<String, Pad>,
) -> usize {
    let w = width(&net);
    let mut mask = r.masks(&net, w);
    let mut t = Tree::new();
    let mut pads = by[&net].clone();
    let gnd = net == "GND";
    if net == "V12" {
        // A center-fed 5 mm bottom trunk across the fan bank and a 4 mm feeder from the inductor output.
        // Grid points of the actual copper centreline become attachment targets on L4.
        for (dx, dy) in [
            (-0.62, -0.60),
            (0.62, -0.60),
            (-0.62, 0.60),
            (0.62, 0.60),
            (0., 0.),
        ] {
            let p = (46.2 + dx, 12.5 + dy);
            precise_via(r, &net, p);
        }
        let ps = [(13., 61.), (97., 61.)];
        r.trace(&net, 2, 5., ps.to_vec());
        for i in cell(ps[0])..=cell(ps[1]) {
            t.add(i, BOT)
        }
        let br = [(46.25, 12.5), (50.25, 12.5)];
        r.trace(&net, 2, 4., br.to_vec());
        for i in cell(br[0])..=cell(br[1]) {
            t.add(i, BOT)
        }
        let ps = [(50.25, 12.5), (50.25, 61.)];
        r.trace(&net, 2, 4., ps.to_vec());
        for k in (cell(ps[0])..=cell(ps[1])).step_by(W) {
            t.add(k, BOT)
        }
        if let Some(i) = pads.iter().position(|p| p.id == "L1.2") {
            let p = pads.swap_remove(i);
            t.pad(&p);
        }
    } else if gnd {
        for p in &pads {
            if p.hole > 0. {
                t.pad(p)
            }
        }
        pads.retain(|p| p.hole == 0.);
    } else {
        let root = match net.as_str() {
            "SW" => "Q3.2",
            "SENSE" => "R1.2",
            "ISRET" => "Q3.3",
            _ => "",
        };
        let first = pads
            .iter()
            .position(|p| p.id == root)
            .or_else(|| pads.iter().position(|p| p.hole > 0.))
            .unwrap_or_else(|| {
                pads.iter()
                    .enumerate()
                    .max_by(|a, b| (a.1.a * a.1.b).total_cmp(&(b.1.a * b.1.b)))
                    .unwrap()
                    .0
            });
        let p = pads.remove(first);
        t.pad(&p);
        t.pad(&original[&p.id]);
        // Seed a lower-layer portal for wide nets whose root pad cannot admit the full track width.
        if w >= 0.6 && p.hole == 0. {
            if let Some(k) = land(&p)
                .into_iter()
                .filter(|&k| mask[k] & 8 == 0)
                .min_by_key(|&k| {
                    let q = pos(k);
                    ((q.0 - p.p.0).hypot(q.1 - p.p.1) * 1000.) as u32
                })
            {
                r.via(&net, k);
                t.add(k, ALL);
                Geo::Seg(pos(k), pos(k), 0.).scan(DRILL + 0.055, |j, _| mask[j] |= 8);
            }
        }
    }
    let mut failed = 0;
    while !pads.is_empty() {
        let index = if gnd {
            pads.len() - 1
        } else {
            pads.iter()
                .enumerate()
                .min_by_key(|(_, p)| {
                    let k = cell(p.p);
                    let (x, y) = (k % W, k / W);
                    t.points
                        .iter()
                        .map(|&(a, b)| a.abs_diff(x) + b.abs_diff(y))
                        .min()
                        .unwrap_or(0)
                })
                .unwrap()
                .0
        };
        let p = pads.swap_remove(index);
        let branch = if ["SENSE", "ISRET", "SW"].contains(&net.as_str())
            && (p.id.starts_with("U1.")
                || p.id.starts_with("U4.")
                || p.id.starts_with("R26.")
                || p.id.starts_with("C27."))
        {
            0.20
        } else {
            w
        };
        let mut thin;
        if branch != w {
            thin = r.masks(&net, branch);
        } else {
            thin = Vec::new();
        }
        let usemask = if branch != w { &mut thin } else { &mut mask };
        let local = [
            "HSE8", "HO", "LO", "BSRC", "BST", "FB", "VREF3", "SW", "ISRET", "SENSE", "CANH", "CANL",
        ]
        .contains(&net.as_str());
        let res = if local {
            solver.connect(&p, &original[&p.id], &t, usemask, false, true, 12000)
        } else {
            None
        }
        .or_else(|| solver.connect(&p, &original[&p.id], &t, usemask, gnd, false, 500000));
        if let Some(s) = res {
            accept(r, &mut t, &p, s, usemask, branch);
            t.pad(&original[&p.id]);
            if branch != w {
                mask = r.masks(&net, w)
            }
        } else {
            failed += 1;
            r.log.push(format!("{} {}", net, p.id));
        }
    }
    failed
}
impl Router {
    fn tail(&self, idx: [usize; 5]) -> Router {
        Router {
            obs: self.obs[idx[0]..].to_vec(),
            drills: self.drills[idx[1]..].to_vec(),
            tracks: self.tracks[idx[2]..].to_vec(),
            vias: self.vias[idx[3]..].to_vec(),
            log: self.log[idx[4]..].to_vec(),
        }
    }
    fn mark(&self) -> [usize; 5] {
        [
            self.obs.len(),
            self.drills.len(),
            self.tracks.len(),
            self.vias.len(),
            self.log.len(),
        ]
    }
    fn append(&mut self, q: &Router) {
        self.obs.extend_from_slice(&q.obs);
        self.drills.extend_from_slice(&q.drills);
        self.tracks.extend_from_slice(&q.tracks);
        self.vias.extend_from_slice(&q.vias);
        self.log.extend_from_slice(&q.log);
    }
}
fn main() {
    let args: Vec<_> = env::args().collect();
    let mut by = BTreeMap::<String, Vec<Pad>>::new();
    let mut r = Router {
        obs: vec![],
        drills: vec![],
        tracks: vec![],
        vias: vec![],
        log: vec![],
    };
    for line in fs::read_to_string(&args[1]).unwrap().lines() {
        let v: Vec<_> = line.split_whitespace().collect();
        let f = |i: usize| v[i].parse::<f64>().unwrap();
        let p = Pad {
            id: v[0].into(),
            net: v[1].into(),
            p: (f(2), f(3)),
            a: f(4),
            b: f(5),
            hole: f(6),
        };
        r.obs.push(Ob {
            net: p.net.clone(),
            l: if p.hole > 0. { ALL } else { TOP },
            g: shape(&p),
        });
        if p.hole > 0. {
            r.drills.push((p.p, p.hole));
        }
        by.entry(p.net.clone()).or_default().push(p);
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
    let mut order: Vec<_> = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    order.extend("HSE8 HO LO BSRC BST FB VREF3 CANH CANL VIN FUSED RECT SENSE ISRET SW VPROT V12 OUT1 OUT2 OUT3 OUT4 OUT5 OUT6 SW1 GND V33 V5".split_whitespace().map(String::from));
    let mut others: Vec<_> = by.keys().cloned().collect();
    others.sort_by_key(|n| by[n].len());
    order.extend(others);
    let failed: HashSet<String> = fs::read_to_string(args.get(4).map(String::as_str).unwrap_or("/dev/null"))
        .unwrap()
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(String::from))
        .collect();
    let original = by.values().flatten().map(|p| (p.id.clone(), p.clone())).collect();
    prepare(&mut r, &mut by, "V12", &failed);
    let mut seen = HashSet::new();
    order.retain(|n| {
        seen.insert(n.clone())
            && by.get(n).is_some_and(|ps| ps.len() > 1)
            && !n.starts_with('@')
            && n != "V12"
    });
    let mut solver = Solver::new();
    route_one(&mut r, &mut solver, "V12".into(), &by, &original);

    prepare(&mut r, &mut by, "signals", &failed);
    let base = r.clone();
    let mut bank = BTreeMap::<String, Router>::new();
    let mut record = usize::MAX;
    for pass in 0..args.get(5).and_then(|v| v.parse().ok()).unwrap_or(35usize) {
        let alt: HashSet<String> = order.iter().cloned().collect();
        for n in &alt {
            bank.remove(n);
        }
        r = base.clone();
        for pkg in bank.values() {
            r.append(pkg);
        }
        for n in &order {
            let mark = r.mark();
            route_one(&mut r, &mut solver, n.clone(), &by, &original);
            bank.insert(n.clone(), r.tail(mark));
        }
        let count = r.log.len();
        if count < record {
            record = count;
            write_result(&r, &args[2]);
            eprintln!("pass {pass} new best {count}");
        }
        if count == 0 {
            break;
        }
        let mut bad = Vec::new();
        let mut reroute = Vec::new();
        let mut known = HashSet::new();
        for item in &r.log {
            let (n, id) = item.split_once(' ').unwrap();
            if !bank.contains_key(n) {
                continue;
            }
            if known.insert(n.to_string()) {
                bad.push(n.to_string());
            }
            let p = &original[id];
            for o in r.obs.iter().rev() {
                if o.net != n
                    && bank.contains_key(&o.net)
                    && o.g.distance(p.p) < 1.2 + ((pass % 4) as f64) * 0.7
                    && !r.log.iter().any(|e| e.starts_with(&format!("{} ", o.net)))
                    && known.insert(o.net.clone())
                {
                    reroute.push(o.net.clone())
                }
            }
        }
        let mut seed = (pass as u64 + 1) * 72137;
        for a in [&mut bad, &mut reroute] {
            for i in (1..a.len()).rev() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                a.swap(i, (seed as usize >> 16) % (i + 1));
            }
        }
        bad.extend(reroute);
        order = bad;
        if order.is_empty() {
            break;
        }
    }
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
