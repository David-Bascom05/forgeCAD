use crate::sketch::{Constraint, ConstraintId, ConstraintRec, Entity, EntityId, Sketch};
use forge_math::Point2;

/// Residual below which a constraint counts as met (mm).
pub const SOLVED: f64 = 1e-10;
/// Residual above which an unsolved constraint is reported as conflicting (mm).
const CONFLICT: f64 = 1e-7;
const MAX_ITERATIONS: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Solved,
    /// No configuration meets every constraint (they conflict), or the solver
    /// could not find one from here. The sketch is left at the best attempt.
    NotConverged,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolveReport {
    pub status: Status,
    pub iterations: usize,
    pub max_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Analysis {
    /// Independent ways the sketch can still move.
    pub dof: usize,
    /// Entities whose every parameter is pinned down.
    pub fully_constrained: Vec<EntityId>,
    /// Constraints implied by the others (removing any one of these alone
    /// changes nothing).
    pub redundant: Vec<ConstraintId>,
    /// Constraints left unmet at the current parameters.
    pub conflicting: Vec<ConstraintId>,
}

/// Stacked residuals of all constraints, their sparse Jacobian rows
/// (parameter, derivative) and which constraint each row came from (None for
/// an arc's own row).
#[derive(Default)]
struct System {
    r: Vec<f64>,
    /// Row i's entries are `ent[at[i]..at[i + 1]]`.
    ent: Vec<(usize, f64)>,
    at: Vec<usize>,
    origin: Vec<Option<ConstraintId>>,
    /// What made each row, so a group can rebuild only its own rows.
    source: Vec<Source>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    /// Index into the sketch's entities (an arc's equal-radius row).
    Arc(usize),
    /// Index into the sketch's constraints.
    Constraint(usize),
}

impl System {
    fn row(&self, i: usize) -> &[(usize, f64)] {
        &self.ent[self.at[i]..self.at[i + 1]]
    }
    #[cfg(test)]
    fn dense(&self, n: usize) -> Vec<f64> {
        let mut j = vec![0.0; self.r.len() * n];
        for i in 0..self.r.len() {
            for &(v, g) in self.row(i) {
                j[i * n + v] += g;
            }
        }
        j
    }
}

/// Linear ties between parameters: each is its root's value plus an offset, and a root may
/// be pinned to a value.
struct Ties {
    up: Vec<usize>,
    off: Vec<f64>,
    pin: Vec<Option<f64>>,
    /// Ties already implied by the ones before, so far.
    implied: usize,
}

/// How far apart two values meant to be equal may be and still count as one.
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= SOLVED
}

impl Ties {
    /// The root of `i` and `i`'s offset from it.
    fn find(&mut self, i: usize) -> (usize, f64) {
        let (mut r, mut off) = (i, 0.0);
        while self.up[r] != r {
            off += self.off[r];
            r = self.up[r];
        }
        // point every parameter on the way straight at the root
        let mut k = i;
        let mut rest = off;
        while self.up[k] != r && k != r {
            let next = self.up[k];
            let o = self.off[k];
            self.up[k] = r;
            self.off[k] = rest;
            rest -= o;
            k = next;
        }
        (r, off)
    }

    /// Ties `x_j = x_i + d`; false if the ties so far say otherwise.
    fn tie(&mut self, i: usize, j: usize, d: f64) -> bool {
        let ((ri, oi), (rj, oj)) = (self.find(i), self.find(j));
        if ri == rj {
            self.implied += same(oj - oi, d) as usize;
            return same(oj - oi, d);
        }
        // x_rj = x_ri + oi + d - oj
        let o = oi + d - oj;
        match (self.pin[ri], self.pin[rj]) {
            (Some(a), Some(b)) if !same(a + o, b) => return false,
            (Some(_), Some(_)) => self.implied += 1,
            (None, Some(b)) => self.pin[ri] = Some(b - o),
            _ => {}
        }
        self.up[rj] = ri;
        self.off[rj] = o;
        true
    }

    /// Pins `x_i = v`; false if the ties so far say otherwise.
    fn fix(&mut self, i: usize, v: f64) -> bool {
        let (r, o) = self.find(i);
        match self.pin[r] {
            Some(p) => {
                self.implied += same(p + o, v) as usize;
                same(p + o, v)
            }
            None => {
                self.pin[r] = Some(v - o);
                true
            }
        }
    }
}

/// The free roots a solve moves: each parameter's root's column (MAX if pinned), each
/// column's parameters, and each column's damping weight.
struct Tied<'a> {
    col: &'a [usize],
    members: &'a [Vec<usize>],
    w: &'a [f64],
}

/// Groups of parameters linked through some constraint. Each group can be
/// solved on its own, which keeps the dense solves small: a sketch of many
/// separate features costs the sum of its parts, not the cube of its size.
fn components(sys: &System, n: usize) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut used = vec![false; n];
    for row in (0..sys.r.len()).map(|i| sys.row(i)) {
        let Some(&(first, _)) = row.first() else { continue };
        used[first] = true;
        for &(v, _) in &row[1..] {
            used[v] = true;
            let (a, b) = (find(&mut parent, first), find(&mut parent, v));
            if a != b {
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut slot = vec![usize::MAX; n];
    for (v, _) in used.iter().enumerate().filter(|(_, u)| **u) {
        let root = find(&mut parent, v);
        if slot[root] == usize::MAX {
            slot[root] = groups.len();
            groups.push(Vec::new());
        }
        groups[slot[root]].push(v);
    }
    groups
}

impl Sketch {
    fn system(&self, x: &[f64]) -> System {
        let all: Vec<Source> = (0..self.entities.len())
            .map(Source::Arc)
            .chain((0..self.constraints.len()).map(Source::Constraint))
            .collect();
        self.system_of(x, &all)
    }

    fn system_of(&self, x: &[f64], sources: &[Source]) -> System {
        let mut s = System::default();
        self.system_into(x, sources, &mut s);
        s
    }

    fn system_into(&self, x: &[f64], sources: &[Source], s: &mut System) {
        s.r.clear();
        s.ent.clear();
        s.at.clear();
        s.at.push(0);
        s.origin.clear();
        s.source.clear();
        let mut push = |vars: &[usize], d: crate::dual::D, origin: Option<ConstraintId>, src: Source| {
            s.ent.extend(vars.iter().enumerate().map(|(k, &v)| (v, d.g[k])));
            s.at.push(s.ent.len());
            s.r.push(d.v);
            s.origin.push(origin);
            s.source.push(src);
        };
        for &src in sources {
            match src {
                Source::Arc(i) => {
                    if let Some((loc, d)) = self.arc_residual(self.entities[i].id, x) {
                        push(&loc.vars[..loc.n], d, None, src);
                    }
                }
                Source::Constraint(i) => {
                    let rec = &self.constraints[i];
                    let (loc, rows) = self.residuals(rec, x);
                    for &d in &rows.d[..rows.n] {
                        push(&loc.vars[..loc.n], d, Some(rec.id), src);
                    }
                }
            }
        }
    }

    /// Moves the parameters as little as possible until every constraint holds.
    pub fn solve(&mut self) -> SolveReport {
        self.solve_after(None)
    }

    /// `solve`, taking from `prev` (the same sketch solved before, its values perhaps
    /// changed) every linked group whose rows and starting values are bit for bit what they
    /// were: the answer is exactly what solving it again would give.
    pub fn solve_after(&mut self, prev: Option<&Sketch>) -> SolveReport {
        let w = vec![1.0; self.params.len()];
        self.solve_weighted(&w, prev)
    }

    /// Puts a point at `target` and re-solves, moving everything else as little
    /// as possible. If the point cannot reach the target (it is constrained),
    /// it ends wherever the constraints put it nearest its old place.
    pub fn drag(&mut self, point: EntityId, target: Point2) -> Result<SolveReport, crate::SketchError> {
        let (px, py) = self.point_params(point)?;
        let before = self.params.clone();
        let mut w = vec![1.0; self.params.len()];
        w[px] = 1e3;
        w[py] = 1e3;
        // first try holding the point exactly on the target
        self.params[px] = target.x;
        self.params[py] = target.y;
        self.constraints.push(ConstraintRec {
            id: ConstraintId(0),
            c: Constraint::Fixed(point),
            data: [target.x, target.y],
        });
        let held = self.solve_weighted(&w, None);
        self.constraints.pop();
        if held.status == Status::Solved {
            return Ok(held);
        }
        self.params = before;
        self.params[px] = target.x;
        self.params[py] = target.y;
        Ok(self.solve_weighted(&w, None))
    }

    /// Fixed, coincident, concentric, horizontal, vertical and axis-distance constraints tie
    /// parameters linearly; they are met exactly by tying, and only the free roots move.
    /// Returns the ties and which constraints are left as residual rows.
    fn ties(&self) -> (Ties, Vec<Source>, Vec<usize>) {
        let n = self.params.len();
        let mut t = Ties {
            up: (0..n).collect(),
            off: vec![0.0; n],
            pin: vec![None; n],
            implied: 0,
        };
        let mut implied = Vec::new();
        let mut rows: Vec<Source> = (0..self.entities.len())
            .filter(|&i| matches!(self.entities[i].kind, Entity::Arc { .. }))
            .map(Source::Arc)
            .collect();
        let xy = |p: EntityId| self.point_params(p).expect("validated point");
        let center = |e: EntityId| match self.rec(e).map(|r| r.kind) {
            Ok(Entity::Circle { center, .. } | Entity::Arc { center, .. }) => xy(center),
            _ => unreachable!("validated round"),
        };
        let ends = |l: EntityId| match self.rec(l).map(|r| r.kind) {
            Ok(Entity::Line { a, b }) => (xy(a), xy(b)),
            _ => unreachable!("validated line"),
        };
        for (k, rec) in self.constraints.iter().enumerate() {
            let before = t.implied;
            let ok = match rec.c {
                Constraint::Fixed(p) => {
                    let (x, y) = xy(p);
                    t.fix(x, rec.data[0]) & t.fix(y, rec.data[1])
                }
                Constraint::Coincident(p, q) => {
                    let ((px, py), (qx, qy)) = (xy(p), xy(q));
                    t.tie(px, qx, 0.0) & t.tie(py, qy, 0.0)
                }
                Constraint::Concentric(a, b) => {
                    let ((px, py), (qx, qy)) = (center(a), center(b));
                    t.tie(px, qx, 0.0) & t.tie(py, qy, 0.0)
                }
                Constraint::Horizontal(l) => {
                    let ((_, ay), (_, by)) = ends(l);
                    t.tie(ay, by, 0.0)
                }
                Constraint::Vertical(l) => {
                    let ((ax, _), (bx, _)) = ends(l);
                    t.tie(ax, bx, 0.0)
                }
                Constraint::HorizontalDistance(p, q, d) => t.tie(xy(p).0, xy(q).0, d),
                Constraint::VerticalDistance(p, q, d) => t.tie(xy(p).1, xy(q).1, d),
                _ => false,
            };
            if !ok {
                rows.push(Source::Constraint(k));
            } else if t.implied > before {
                implied.push(k);
            }
        }
        (t, rows, implied)
    }

    /// Constraints the ones before them already imply, at the current parameters: each
    /// adds nothing, so the sketch is over-constrained (in order, so the later of two is named).
    pub fn implied(&self) -> Vec<ConstraintId> {
        let (mut ties, sources, mut out) = self.ties();
        let n = self.params.len();
        let mut col = vec![usize::MAX; n];
        let mut free = 0;
        for i in 0..n {
            let (r, _) = ties.find(i);
            if ties.pin[r].is_none() {
                if col[r] == usize::MAX {
                    col[r] = free;
                    free += 1;
                }
                col[i] = col[r];
            }
        }
        let sys = self.system_of(&self.params, &sources);
        // rows reduced against the independent ones before them, by their pivot columns
        let mut basis: Vec<(usize, Vec<f64>)> = Vec::new();
        for i in 0..sys.r.len() {
            let mut v = vec![0.0; free];
            for &(p, g) in sys.row(i) {
                if col[p] != usize::MAX {
                    v[col[p]] += g;
                }
            }
            let scale = v.iter().fold(0.0f64, |m, x| m.max(x.abs()));
            for (pc, b) in &basis {
                let f = v[*pc];
                if f != 0.0 {
                    v.iter_mut().zip(b).for_each(|(x, y)| *x -= f * y);
                }
            }
            let (pc, big) =
                v.iter().enumerate().fold(
                    (0, 0.0f64),
                    |m, (k, x)| if x.abs() > m.1 { (k, x.abs()) } else { m },
                );
            if big <= 1e-9 * scale.max(1e-300) {
                if let Source::Constraint(k) = sys.source[i] {
                    out.push(k);
                }
                continue;
            }
            let p = v[pc];
            v.iter_mut().for_each(|x| *x /= p);
            basis.push((pc, v));
        }
        out.sort_unstable();
        out.dedup();
        out.into_iter().map(|k| self.constraints[k].id).collect()
    }

    /// Levenberg-Marquardt with per-parameter damping weights, over the parameters left free
    /// by the ties, one linked group at a time: each step solves [J; sqrt(lambda) W] dx =
    /// [-r; 0] by QR, so as lambda shrinks the step tends to the W-weighted smallest change
    /// that meets the constraints, and free geometry stays put.
    fn solve_weighted(&mut self, w: &[f64], prev: Option<&Sketch>) -> SolveReport {
        let n = self.params.len();
        let (mut ties, sources, _) = self.ties();
        // each parameter's root, and the roots that still move
        let mut col = vec![usize::MAX; n];
        let mut members: Vec<Vec<usize>> = Vec::new();
        let mut cw: Vec<f64> = Vec::new();
        for i in 0..n {
            let (r, off) = ties.find(i);
            self.params[i] = match ties.pin[r] {
                Some(v) => v + off,
                None => self.params[r] + off,
            };
            if ties.pin[r].is_none() {
                if col[r] == usize::MAX {
                    col[r] = members.len();
                    members.push(Vec::new());
                    cw.push(0.0);
                }
                let c = col[r];
                col[i] = c;
                members[c].push(i);
                cw[c] = cw[c].max(w[i]);
            }
        }
        let tied = Tied {
            col: &col,
            members: &members,
            w: &cw,
        };
        let full = self.system_of(&self.params, &sources);
        // the rows over the moving roots
        let mut rows = System::default();
        rows.at.push(0);
        for i in 0..full.r.len() {
            let row = full.row(i).iter().filter(|e| col[e.0] != usize::MAX);
            rows.ent.extend(row.map(|&(v, g)| (col[v], g)));
            rows.at.push(rows.ent.len());
            rows.r.push(full.r[i]);
        }
        let roots: Vec<usize> = (0..n)
            .map(|i| {
                if col[i] == usize::MAX {
                    usize::MAX
                } else {
                    members[col[i]][0]
                }
            })
            .collect();
        let prev = prev.filter(|p| {
            p.last.is_some() && p.entities == self.entities && p.constraints.len() == self.constraints.len()
        });
        let same_group = |rows_of: Vec<usize>, s: &Sketch| -> bool {
            let Some((snap, root)) = prev.and_then(|p| p.last.as_ref()) else {
                return false;
            };
            let p = prev.expect("checked");
            rows_of.into_iter().all(|i| {
                let ok_src = match full.source[i] {
                    Source::Constraint(k) => {
                        let (a, b) = (&s.constraints[k], &p.constraints[k]);
                        a.c == b.c && a.data.map(f64::to_bits) == b.data.map(f64::to_bits)
                    }
                    Source::Arc(_) => true,
                };
                ok_src
                    && full
                        .row(i)
                        .iter()
                        .all(|&(v, _)| s.params[v].to_bits() == snap[v].to_bits() && roots[v] == root[v])
            })
        };
        let groups = components(&rows, members.len());
        let mut group = vec![usize::MAX; members.len()];
        for (k, g) in groups.iter().enumerate() {
            g.iter().for_each(|&c| group[c] = k);
        }
        // rows nothing can move stay as they are; a group already met needs no solve
        let mut max_residual = 0.0f64;
        let mut worst = vec![0.0f64; groups.len()];
        for i in 0..rows.r.len() {
            match rows.row(i).first() {
                None => max_residual = max_residual.max(full.r[i].abs()),
                Some(&(c, _)) => worst[group[c]] = worst[group[c]].max(full.r[i].abs()),
            }
        }
        let (rows, group) = (&rows, &group);
        let in_group = |k: usize| -> Vec<usize> {
            (0..rows.r.len())
                .filter(|&i| rows.row(i).first().is_some_and(|&(c, _)| group[c] == k))
                .collect()
        };
        let reuse: Vec<bool> = (0..groups.len())
            .map(|k| worst[k] > SOLVED && same_group(in_group(k), self))
            .collect();
        let snapped = self.params.clone();
        let mut iterations = 0;
        for (k, g) in groups.iter().enumerate() {
            if worst[k] <= SOLVED {
                continue;
            }
            let mut src: Vec<Source> = (0..rows.r.len())
                .filter(|&i| rows.row(i).first().is_some_and(|&(c, _)| group[c] == k))
                .map(|i| full.source[i])
                .collect();
            src.dedup();
            if reuse[k] {
                let p = prev.expect("checked");
                for &c in g {
                    for &v in &members[c] {
                        self.params[v] = p.params[v];
                    }
                }
                continue;
            }
            let (it, r) = self.solve_group(g, &src, &tied);
            iterations = iterations.max(it);
            max_residual = max_residual.max(r);
        }
        self.last = Some((snapped, roots));
        let status = if max_residual <= SOLVED {
            Status::Solved
        } else {
            Status::NotConverged
        };
        SolveReport {
            status,
            iterations,
            max_residual,
        }
    }

    /// Levenberg-Marquardt on one group of free roots, everything else held still. Returns
    /// the iterations used and the largest residual left.
    fn solve_group(&mut self, vars: &[usize], sources: &[Source], t: &Tied) -> (usize, f64) {
        let n = vars.len();
        let mut local = vec![usize::MAX; t.members.len()];
        for (k, &v) in vars.iter().enumerate() {
            local[v] = k;
        }
        let at = |v: usize| {
            if t.col[v] == usize::MAX {
                usize::MAX
            } else {
                local[t.col[v]]
            }
        };
        let max_abs = |r: &[f64]| r.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let mut x = self.params.clone();
        let mut trial = x.clone();
        let (mut sys, mut next) = (self.system_of(&x, sources), System::default());
        let mut cost: f64 = sys.r.iter().map(|v| v * v).sum();
        let mut lambda: f64 = 1e-6;
        let mut it = 0;
        let (mut a, mut dx) = (Vec::new(), Vec::new());
        while it < MAX_ITERATIONS && max_abs(&sys.r) > SOLVED {
            it += 1;
            // the damped normal equations (J'J + lambda W^2) dx = -J'r
            a.clear();
            a.resize(n * n, 0.0);
            dx.clear();
            dx.resize(n, 0.0);
            for (i, &r) in sys.r.iter().enumerate() {
                let row = sys.row(i);
                for &(v, gv) in row {
                    let c = at(v);
                    if c == usize::MAX {
                        continue;
                    }
                    dx[c] -= gv * r;
                    for &(u, gu) in row {
                        let d = at(u);
                        if d != usize::MAX {
                            a[c * n + d] += gv * gu;
                        }
                    }
                }
            }
            for (k, &v) in vars.iter().enumerate() {
                a[k * n + k] += lambda * t.w[v] * t.w[v];
            }
            if !cholesky_solve(&mut a, &mut dx, n) {
                lambda *= 10.0;
                if lambda > 1e12 {
                    break;
                }
                continue;
            }
            trial.copy_from_slice(&x);
            for (k, &v) in vars.iter().enumerate() {
                for &p in &t.members[v] {
                    trial[p] += dx[k];
                }
            }
            self.system_into(&trial, sources, &mut next);
            let tcost: f64 = next.r.iter().map(|v| v * v).sum();
            if tcost < cost {
                let tiny = dx.iter().fold(0.0f64, |m, v| m.max(v.abs()))
                    <= 1e-15 * (1.0 + vars.iter().fold(0.0f64, |m, &v| m.max(x[t.members[v][0]].abs())));
                std::mem::swap(&mut sys, &mut next);
                std::mem::swap(&mut x, &mut trial);
                cost = tcost;
                lambda = (lambda * 0.1).max(1e-9);
                if tiny {
                    break;
                }
            } else {
                lambda *= 10.0;
                if lambda > 1e12 {
                    break;
                }
            }
        }
        self.params = x;
        (it, max_abs(&sys.r))
    }

    /// Degrees of freedom, fully constrained entities, redundant and
    /// conflicting constraints, all at the current parameters.
    ///
    /// The Jacobian is block diagonal over the groups of parameters that rows tie together, so
    /// every rank is taken per block: independent entities cost a small block each, not a pass
    /// over the whole sketch.
    pub fn analyze(&self) -> Analysis {
        let n = self.params.len();
        let sys = self.system(&self.params);
        let m = sys.r.len();
        let blocks = Blocks::new(&sys, n);
        // the tolerance is the one a dense rank of the whole matrix would use
        let row_max: Vec<f64> = (0..m)
            .map(|i| sys.row(i).iter().fold(0.0f64, |s, e| s.max(e.1.abs())))
            .collect();
        let scale = row_max.iter().fold(0.0f64, |s, v| s.max(*v));
        let base_rank: Vec<usize> = (0..blocks.cols.len())
            .map(|b| blocks.rank(&sys, b, &[], &[], scale))
            .collect();
        let base: usize = base_rank.iter().sum();

        let mut fully_constrained = Vec::new();
        for e in &self.entities {
            let ps = self.entity_params(e.id);
            if ps.is_empty() {
                continue;
            }
            let mut touched: Vec<usize> = ps.iter().map(|&p| blocks.of_col[p]).collect();
            touched.sort_unstable();
            touched.dedup();
            let scale = scale.max(1.0);
            if touched
                .iter()
                .all(|&b| blocks.rank(&sys, b, &[], &ps, scale) == base_rank[b])
            {
                fully_constrained.push(e.id);
            }
        }

        let mut rows_of: std::collections::HashMap<ConstraintId, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, o) in sys.origin.iter().enumerate() {
            if let Some(c) = o {
                rows_of.entry(*c).or_default().push(i);
            }
        }
        let mut by_max: Vec<usize> = (0..m).collect();
        by_max.sort_by(|a, b| row_max[*b].total_cmp(&row_max[*a]));
        let mut redundant = Vec::new();
        for rec in &self.constraints {
            let Some(drop) = rows_of.get(&rec.id) else {
                continue;
            };
            let scale = by_max
                .iter()
                .find(|i| !drop.contains(i))
                .map_or(0.0, |&i| row_max[i]);
            let mut touched: Vec<usize> = drop.iter().filter_map(|&i| blocks.of_row[i]).collect();
            touched.sort_unstable();
            touched.dedup();
            let same = touched
                .iter()
                .all(|&b| blocks.rank(&sys, b, drop, &[], scale) == base_rank[b]);
            // with every row gone the dense rank was 0 whatever the tolerance
            if same && (scale > 0.0 || base == 0) {
                redundant.push(rec.id);
            }
        }

        let mut conflicting: Vec<ConstraintId> = (0..m)
            .filter(|&i| sys.r[i].abs() > CONFLICT)
            .filter_map(|i| sys.origin[i])
            .collect();
        conflicting.dedup();
        Analysis {
            dof: n - base,
            fully_constrained,
            redundant,
            conflicting,
        }
    }

    /// True when every entity is fully constrained and nothing conflicts.
    pub fn is_fully_constrained(&self) -> bool {
        let a = self.analyze();
        a.dof == 0 && a.conflicting.is_empty()
    }
}

/// Solves `a x = b` for symmetric positive definite `a` (n x n, overwritten), `b` becoming
/// `x`; false if `a` is not positive definite.
fn cholesky_solve(a: &mut [f64], b: &mut [f64], n: usize) -> bool {
    for j in 0..n {
        let mut d = a[j * n + j];
        for k in 0..j {
            d -= a[j * n + k] * a[j * n + k];
        }
        if d.is_nan() || d <= 0.0 {
            return false;
        }
        let d = d.sqrt();
        a[j * n + j] = d;
        for i in j + 1..n {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= a[i * n + k] * a[j * n + k];
            }
            a[i * n + j] = s / d;
        }
    }
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= a[i * n + k] * b[k];
        }
        b[i] = s / a[i * n + i];
    }
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= a[k * n + i] * b[k];
        }
        b[i] = s / a[i * n + i];
    }
    b.iter().all(|v| v.is_finite())
}

/// The dense analysis `analyze` replaced, kept as the reference it is tested against.
#[cfg(test)]
impl Sketch {
    pub(crate) fn analyze_dense(&self) -> Analysis {
        let dense_rank =
            |a: &[f64], m: usize, n: usize| rank(a, m, n, a.iter().fold(0.0f64, |s, v| s.max(v.abs())));
        let n = self.params.len();
        let sys = self.system(&self.params);
        let m = sys.r.len();
        let dense = sys.dense(n);
        let base = dense_rank(&dense, m, n);
        let mut fully_constrained = Vec::new();
        for e in &self.entities {
            let ps = self.entity_params(e.id);
            if ps.is_empty() {
                continue;
            }
            let mut a = dense.clone();
            for &p in &ps {
                let mut row = vec![0.0; n];
                row[p] = 1.0;
                a.extend(row);
            }
            if dense_rank(&a, m + ps.len(), n) == base {
                fully_constrained.push(e.id);
            }
        }
        let mut redundant = Vec::new();
        for rec in &self.constraints {
            let keep: Vec<usize> = (0..m).filter(|&i| sys.origin[i] != Some(rec.id)).collect();
            if keep.len() == m {
                continue;
            }
            let a: Vec<f64> = keep
                .iter()
                .flat_map(|&i| dense[i * n..(i + 1) * n].iter().copied())
                .collect();
            if dense_rank(&a, keep.len(), n) == base {
                redundant.push(rec.id);
            }
        }
        let mut conflicting: Vec<ConstraintId> = (0..m)
            .filter(|&i| sys.r[i].abs() > CONFLICT)
            .filter_map(|i| sys.origin[i])
            .collect();
        conflicting.dedup();
        Analysis {
            dof: n - base,
            fully_constrained,
            redundant,
            conflicting,
        }
    }
}

/// The connected blocks of a Jacobian: parameters tied by a row are in one block, with the
/// rows over them.
struct Blocks {
    of_col: Vec<usize>,
    /// None for a row with no entries.
    of_row: Vec<Option<usize>>,
    cols: Vec<Vec<usize>>,
    rows: Vec<Vec<usize>>,
}

impl Blocks {
    fn new(sys: &System, n: usize) -> Blocks {
        let mut up: Vec<usize> = (0..n).collect();
        fn find(up: &mut [usize], mut x: usize) -> usize {
            while up[x] != x {
                up[x] = up[up[x]];
                x = up[x];
            }
            x
        }
        for i in 0..sys.r.len() {
            let row = sys.row(i);
            if let Some(&(first, _)) = row.first() {
                for &(v, _) in &row[1..] {
                    let (a, b) = (find(&mut up, first), find(&mut up, v));
                    up[a] = b;
                }
            }
        }
        let mut id = vec![usize::MAX; n];
        let mut cols: Vec<Vec<usize>> = Vec::new();
        let of_col: Vec<usize> = (0..n)
            .map(|c| {
                let root = find(&mut up, c);
                if id[root] == usize::MAX {
                    id[root] = cols.len();
                    cols.push(Vec::new());
                }
                cols[id[root]].push(c);
                id[root]
            })
            .collect();
        let mut rows = vec![Vec::new(); cols.len()];
        let of_row: Vec<Option<usize>> = (0..sys.r.len())
            .map(|i| sys.row(i).first().map(|&(v, _)| of_col[v]))
            .collect();
        for (i, b) in of_row.iter().enumerate() {
            if let Some(b) = b {
                rows[*b].push(i);
            }
        }
        Blocks {
            of_col,
            of_row,
            cols,
            rows,
        }
    }

    /// Rank of block `b`'s rows, less the rows in `drop`, plus a unit row for each parameter
    /// of `pin` in the block; `scale` sets the tolerance as for the whole matrix.
    fn rank(&self, sys: &System, b: usize, drop: &[usize], pin: &[usize], scale: f64) -> usize {
        let cols = &self.cols[b];
        let n = cols.len();
        let local = |c: usize| cols.binary_search(&c).ok();
        let mut a = Vec::new();
        let mut m = 0;
        for &i in self.rows[b].iter().filter(|i| !drop.contains(i)) {
            let at = a.len();
            a.resize(at + n, 0.0);
            for &(v, g) in sys.row(i) {
                a[at + local(v).expect("a row stays in its block")] += g;
            }
            m += 1;
        }
        for c in pin.iter().filter_map(|&p| local(p)) {
            let at = a.len();
            a.resize(at + n, 0.0);
            a[at + c] = 1.0;
            m += 1;
        }
        rank(&a, m, n, scale)
    }
}

/// Numerical rank by Gaussian elimination with complete pivoting.
/// `scale` is the largest entry of the matrix the tolerance is taken from.
fn rank(a: &[f64], m: usize, n: usize, scale: f64) -> usize {
    let mut a = a.to_vec();
    if scale == 0.0 {
        return 0;
    }
    let tol = scale * 1e-9;
    let mut rows: Vec<usize> = (0..m).collect();
    let mut cols: Vec<usize> = (0..n).collect();
    let mut r = 0;
    while r < m.min(n) {
        let (mut bi, mut bj, mut best) = (r, r, 0.0);
        for i in r..m {
            for j in r..n {
                let v = a[rows[i] * n + cols[j]].abs();
                if v > best {
                    best = v;
                    bi = i;
                    bj = j;
                }
            }
        }
        if best <= tol {
            break;
        }
        rows.swap(r, bi);
        cols.swap(r, bj);
        let pr = rows[r];
        let pivot = a[pr * n + cols[r]];
        for &ri in &rows[r + 1..] {
            let f = a[ri * n + cols[r]] / pivot;
            if f != 0.0 {
                for &cj in &cols[r..] {
                    a[ri * n + cj] -= f * a[pr * n + cj];
                }
            }
        }
        r += 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::rank;

    #[test]
    fn rank_basics() {
        assert_eq!(rank(&[1.0, 2.0, 2.0, 4.0], 2, 2, 4.0), 1);
        assert_eq!(rank(&[1.0, 0.0, 0.0, 1.0], 2, 2, 1.0), 2);
        assert_eq!(rank(&[0.0; 6], 2, 3, 0.0), 0);
        assert_eq!(rank(&[1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0], 3, 3, 2.0), 2);
    }
}
