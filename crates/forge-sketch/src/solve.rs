use crate::sketch::{Constraint, ConstraintId, ConstraintRec, EntityId, Sketch};
use forge_math::Point2;
use forge_math::linalg::solve_least_squares;

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

/// Stacked residuals of all constraints, their Jacobian (row-major m x n) and
/// which constraint each row came from (None for an arc's own row).
struct System {
    r: Vec<f64>,
    j: Vec<f64>,
    origin: Vec<Option<ConstraintId>>,
}

impl Sketch {
    fn system(&self, x: &[f64]) -> System {
        let n = x.len();
        let mut s = System {
            r: Vec::new(),
            j: Vec::new(),
            origin: Vec::new(),
        };
        let mut push = |vars: &[usize], d: crate::dual::D, origin: Option<ConstraintId>| {
            let mut row = vec![0.0; n];
            for (k, &v) in vars.iter().enumerate() {
                row[v] += d.g[k];
            }
            s.r.push(d.v);
            s.j.extend(row);
            s.origin.push(origin);
        };
        for e in &self.entities {
            if let Some((loc, d)) = self.arc_residual(e.id, x) {
                push(&loc.vars[..loc.n], d, None);
            }
        }
        for rec in &self.constraints {
            let (loc, rows) = self.residuals(rec, x);
            for d in rows {
                push(&loc.vars[..loc.n], d, Some(rec.id));
            }
        }
        s
    }

    /// Moves the parameters as little as possible until every constraint holds.
    pub fn solve(&mut self) -> SolveReport {
        let w = vec![1.0; self.params.len()];
        self.solve_weighted(&w)
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
        let held = self.solve_weighted(&w);
        self.constraints.pop();
        if held.status == Status::Solved {
            return Ok(held);
        }
        self.params = before;
        self.params[px] = target.x;
        self.params[py] = target.y;
        Ok(self.solve_weighted(&w))
    }

    /// Levenberg-Marquardt with per-parameter damping weights: each step solves
    /// [J; sqrt(lambda) W] dx = [-r; 0] by QR, so as lambda shrinks the step
    /// tends to the W-weighted smallest change that meets the constraints, and
    /// free geometry stays put.
    fn solve_weighted(&mut self, w: &[f64]) -> SolveReport {
        let n = self.params.len();
        let mut x = self.params.clone();
        let mut sys = self.system(&x);
        let mut cost: f64 = sys.r.iter().map(|v| v * v).sum();
        let mut lambda: f64 = 1e-6;
        let mut it = 0;
        let max_abs = |r: &[f64]| r.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        while it < MAX_ITERATIONS && max_abs(&sys.r) > SOLVED && n > 0 {
            it += 1;
            let m = sys.r.len();
            let mut a = sys.j.clone();
            a.resize((m + n) * n, 0.0);
            let sl = lambda.sqrt();
            for i in 0..n {
                a[(m + i) * n + i] = sl * w[i];
            }
            let mut b: Vec<f64> = sys.r.iter().map(|v| -v).collect();
            b.resize(m + n, 0.0);
            let step = solve_least_squares(&a, m + n, n, &b).filter(|d| d.iter().all(|v| v.is_finite()));
            let Some(dx) = step else {
                lambda *= 10.0;
                continue;
            };
            let trial: Vec<f64> = x.iter().zip(&dx).map(|(a, d)| a + d).collect();
            let tsys = self.system(&trial);
            let tcost: f64 = tsys.r.iter().map(|v| v * v).sum();
            if tcost < cost {
                let tiny = dx.iter().fold(0.0f64, |m, v| m.max(v.abs()))
                    <= 1e-15 * (1.0 + x.iter().fold(0.0f64, |m, v| m.max(v.abs())));
                x = trial;
                sys = tsys;
                cost = tcost;
                lambda = (lambda * 0.1).max(1e-12);
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
        let max_residual = max_abs(&sys.r);
        let status = if max_residual <= SOLVED {
            Status::Solved
        } else {
            Status::NotConverged
        };
        SolveReport {
            status,
            iterations: it,
            max_residual,
        }
    }

    /// Degrees of freedom, fully constrained entities, redundant and
    /// conflicting constraints, all at the current parameters.
    pub fn analyze(&self) -> Analysis {
        let n = self.params.len();
        let sys = self.system(&self.params);
        let m = sys.r.len();
        let base = rank(&sys.j, m, n);

        let mut fully_constrained = Vec::new();
        for e in &self.entities {
            let ps = self.entity_params(e.id);
            if ps.is_empty() {
                continue;
            }
            let mut a = sys.j.clone();
            for &p in &ps {
                let mut row = vec![0.0; n];
                row[p] = 1.0;
                a.extend(row);
            }
            if rank(&a, m + ps.len(), n) == base {
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
                .flat_map(|&i| sys.j[i * n..(i + 1) * n].iter().copied())
                .collect();
            if rank(&a, keep.len(), n) == base {
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

/// Numerical rank by Gaussian elimination with complete pivoting.
fn rank(a: &[f64], m: usize, n: usize) -> usize {
    let mut a = a.to_vec();
    let scale = a.iter().fold(0.0f64, |s, v| s.max(v.abs()));
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
        assert_eq!(rank(&[1.0, 2.0, 2.0, 4.0], 2, 2), 1);
        assert_eq!(rank(&[1.0, 0.0, 0.0, 1.0], 2, 2), 2);
        assert_eq!(rank(&[0.0; 6], 2, 3), 0);
        assert_eq!(rank(&[1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0], 3, 3), 2);
    }
}
