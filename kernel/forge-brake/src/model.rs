//! The part as the press sees it: rigid walls and bend zones in their developed coordinates, and
//! the exact partial refolds of audit2 §2.0.
//!
//! A fold state gives every bend j a turn φ_j in [0, π). Walls are rigid. Zone j keeps its bend
//! allowance BA_j (the neutral layer's length), so its neutral radius is rn' = BA_j / φ_j and every
//! layer moves with it: a point (u, y, w) of the zone (u across from wall a's tangent line, y along,
//! w through the material, 0 on the reference skin) goes to angle ψ = u φ / BA at radius
//! ρ0' - σ w, with ρ0' = rn' + (ρ0 - rn). The neutral layer stays isometric at every φ (item 2);
//! at φ_j = θ_j the state is the folded part, at φ_j = 0 the flat. The formulas use sinc and
//! (1 - cos x) / x, so φ → 0 is exact.
use crate::geo::{Iso3, Region, Rigid2, Seg, Slot, V3};
use crate::num::Sc;
use forge_math::{Point2, Vec2, dmath};
use forge_ops::Segment;
use forge_sheet::{JointId, JointKind, Map, Sheet, SheetError, WallId};

#[derive(Clone, Debug, PartialEq)]
pub enum ModelError {
    /// The sheet does not build folded.
    Build(SheetError),
    /// A hem: it needs a pre-bend and a flattening die, which the simulator does not model.
    Hem { joint: JointId },
    /// A rip or a tangent joint: nothing for the press.
    NotBend { joint: JointId },
    /// A wall outline with a spline: its distances are not in closed form here.
    Spline { wall: WallId },
    /// The flat pattern has a spline (a cut wrapped across a bend, say).
    PatternSpline,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WallM {
    pub id: WallId,
    /// Wall coordinates to developed.
    pub dev: Rigid2,
    /// The outline (and its own holes), in wall coordinates, to the mold lines.
    pub outline: Vec<Vec<Seg>>,
    /// The slots of the zones it borders, in wall coordinates.
    pub slots: Vec<Slot>,
    /// Straight outline segments no joint uses, by segment id: where a backgauge may stop.
    pub free: Vec<(u64, Seg)>,
    /// Outline segment ids that carry a joint.
    pub joined: Vec<u64>,
}

impl WallM {
    /// The wall's material, in its coordinates: its outline within the flat pattern (`pattern`,
    /// developed), less the zones' slots.
    pub fn region(&self, pattern: &[Vec<Seg>]) -> Region {
        let to_wall = self.dev.inverse();
        let clip = pattern
            .iter()
            .map(|l| l.iter().map(|s| s.moved(&to_wall)).collect())
            .collect();
        Region::with(self.outline.clone(), clip, self.slots.clone())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BendM {
    pub joint: JointId,
    /// Wall indices (into `Model::walls`): b is turned off a.
    pub a: usize,
    pub b: usize,
    /// The turn, > 0.
    pub theta: f64,
    /// +1: towards wall a's +z (the reference skin is the inside); -1 away.
    pub sigma: f64,
    /// Inside radius at θ.
    pub r: f64,
    pub ba: f64,
    pub len: f64,
    /// Neutral radius at θ: BA / θ.
    pub rn: f64,
    /// ρ0 - rn: every layer's offset from the neutral one at the reference skin.
    pub c0: f64,
    /// Zone coordinates to developed.
    pub zdev: Rigid2,
}

impl BendM {
    /// A zone point (u, y, w) at turn φ, in zone coordinates.
    pub fn fold<T: Sc>(&self, u: T, y: T, w: T, phi: T) -> V3<T> {
        let psi = (u * phi).div_c(self.ba);
        let x = u * psi.sinc() + (w * (-self.sigma) + self.c0) * psi.sin();
        let z = (u * psi.vsc() + psi.vers() * self.c0) * self.sigma + w * psi.cos();
        V3::new(x, y, z)
    }

    /// Where the zone's far tangent line goes at turn φ: the point (BA, 0, 0) and the axes there.
    pub fn end<T: Sc>(&self, phi: T) -> (V3<T>, V3<T>, V3<T>) {
        let ba = T::c(self.ba);
        let e = self.fold(ba, T::c(0.0), T::c(0.0), phi);
        let (s, c) = (phi.sin(), phi.cos());
        let z0 = T::c(0.0);
        let tx = V3::new(c, z0, s * self.sigma);
        let nz = V3::new(s * (-self.sigma), z0, c);
        (e, tx, nz)
    }

    /// Zone-end coordinates (x' = u - BA) at turn φ to zone coordinates.
    pub fn end_apply<T: Sc>(&self, phi: T, q: V3<T>) -> V3<T> {
        let (e, tx, nz) = self.end(phi);
        let yv = V3::new(T::c(0.0), q.y, T::c(0.0));
        e.plus(tx.scale(q.x)).plus(yv).plus(nz.scale(q.z))
    }

    pub fn end_iso(&self, phi: f64) -> Iso3 {
        let (e, tx, nz) = self.end(phi);
        Iso3 {
            o: e.arr(),
            c: [tx.arr(), [0.0, 1.0, 0.0], nz.arr()],
        }
    }

    /// The inside radius at turn φ (> 0).
    pub fn inside_radius(&self, phi: f64) -> f64 {
        self.ba / phi + (self.r - self.rn)
    }

    /// w of the inside skin.
    pub fn w_inside(&self, t: f64) -> f64 {
        if self.sigma > 0.0 { 0.0 } else { -t }
    }

    /// The zone's material in its coordinates: its slot within the flat pattern (developed).
    pub fn region(&self, pattern: &[Vec<Seg>]) -> Region {
        let to_zone = self.zdev.inverse();
        let clip = pattern
            .iter()
            .map(|l| l.iter().map(|s| s.moved(&to_zone)).collect())
            .collect();
        Region::with(vec![self.slot().segs().to_vec()], clip, vec![])
    }

    /// The zone's slot: [0, BA] x [0, len] in zone coordinates.
    pub fn slot(&self) -> Slot {
        Slot {
            at: Rigid2::ID,
            w: self.ba,
            h: self.len,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub t: f64,
    pub walls: Vec<WallM>,
    pub bends: Vec<BendM>,
    /// Each wall's bends (indices into `bends`) turned off it.
    pub kids: Vec<Vec<usize>>,
    /// The bend each wall is turned by (the first wall: none).
    pub parent: Vec<Option<usize>>,
    /// The flat pattern's loops, developed: what is really there (holes, reliefs, corners).
    pub pattern: Vec<Vec<Seg>>,
}

fn seg_of(s: &Segment, wall: WallId) -> Result<Vec<Seg>, ModelError> {
    Ok(match s {
        Segment::Line { a, b } => vec![Seg::Line { a: *a, b: *b }],
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => vec![Seg::Arc {
            c: *center,
            r: *radius,
            start: *start,
            sweep: *sweep,
        }],
        Segment::Circle { center, radius, ccw } => {
            let s = if *ccw { dmath::PI } else { -dmath::PI };
            vec![
                Seg::Arc {
                    c: *center,
                    r: *radius,
                    start: 0.0,
                    sweep: s,
                },
                Seg::Arc {
                    c: *center,
                    r: *radius,
                    start: dmath::PI,
                    sweep: s,
                },
            ]
        }
        Segment::Spline(_) => return Err(ModelError::Spline { wall }),
    })
}

impl Model {
    /// The model of a sheet: its layout from the folded build, its walls' outlines and its bends.
    pub fn new(sheet: &Sheet) -> Result<Model, ModelError> {
        for j in &sheet.joints {
            match j.kind {
                JointKind::Bend { .. } => {}
                JointKind::Hem(_) => return Err(ModelError::Hem { joint: j.id }),
                _ => return Err(ModelError::NotBend { joint: j.id }),
            }
        }
        let built = forge_sheet::build_with(sheet, Map::Folded).map_err(ModelError::Build)?;
        let flat = forge_sheet::build_with(sheet, Map::Flat).map_err(ModelError::Build)?;
        let mut pattern = Vec::new();
        for c in flat
            .pattern
            .as_ref()
            .map(|p| p.contours.as_slice())
            .unwrap_or(&[])
        {
            let mut l = Vec::new();
            for g in c {
                match g {
                    forge_sheet::Curve2::Line { a, b } => l.push(Seg::Line { a: *a, b: *b }),
                    forge_sheet::Curve2::Arc {
                        c, r, start, sweep, ..
                    } => l.push(Seg::Arc {
                        c: *c,
                        r: *r,
                        start: *start,
                        sweep: *sweep,
                    }),
                    forge_sheet::Curve2::Spline { .. } => return Err(ModelError::PatternSpline),
                }
            }
            pattern.push(l);
        }
        let lay = &built.layout;
        let t = sheet.gauge.thickness;
        let windex = |id: WallId| {
            sheet
                .walls
                .iter()
                .position(|w| w.id == id)
                .expect("a wall of the sheet")
        };
        let mut walls = Vec::with_capacity(sheet.walls.len());
        for w in &sheet.walls {
            let place = lay
                .walls
                .iter()
                .find(|p| p.wall == w.id)
                .expect("every wall laid out");
            let mut outline = Vec::new();
            let mut free = Vec::new();
            let joined: Vec<u64> = sheet
                .joints
                .iter()
                .filter_map(|j| {
                    if j.a == w.id {
                        Some(j.on_a)
                    } else if j.b == w.id {
                        Some(j.on_b)
                    } else {
                        None
                    }
                })
                .collect();
            for lp in &w.outline {
                let mut l = Vec::new();
                for (s, id) in &lp.segments {
                    let g = seg_of(s, w.id)?;
                    if let [Seg::Line { .. }] = g.as_slice()
                        && !joined.contains(id)
                    {
                        free.push((*id, g[0]));
                    }
                    l.extend(g);
                }
                outline.push(l);
            }
            walls.push(WallM {
                id: w.id,
                dev: Rigid2 {
                    o: place.developed.origin,
                    x: place.developed.x,
                },
                outline,
                slots: Vec::new(),
                free,
                joined,
            });
        }
        let mut bends = Vec::new();
        for j in &sheet.joints {
            let bp = lay
                .bends
                .iter()
                .find(|b| b.joint == j.id)
                .expect("every bend laid out");
            let theta = bp.angle.abs();
            let sigma = bp.angle.signum();
            let rn = bp.allowance / theta;
            let rho0 = if sigma > 0.0 { bp.radius } else { bp.radius + t };
            bends.push(BendM {
                joint: j.id,
                a: windex(bp.a),
                b: windex(bp.b),
                theta,
                sigma,
                r: bp.radius,
                ba: bp.allowance,
                len: bp.length,
                rn,
                c0: rho0 - rn,
                zdev: Rigid2 {
                    o: bp.developed.origin,
                    x: bp.developed.x,
                },
            });
        }
        let mut kids = vec![Vec::new(); walls.len()];
        let mut parent = vec![None; walls.len()];
        for (k, b) in bends.iter().enumerate() {
            kids[b.a].push(k);
            parent[b.b] = Some(k);
            // each wall loses the slot of every zone it borders
            for wi in [b.a, b.b] {
                let to_wall = walls[wi].dev.inverse().then(&b.zdev);
                let slot = b.slot().moved(&to_wall);
                walls[wi].slots.push(slot);
            }
        }
        Ok(Model {
            t,
            walls,
            bends,
            kids,
            parent,
            pattern,
        })
    }

    /// Each wall's frame (wall coordinates to the part's, the first wall's) in the fold state
    /// `phi` (a turn per bend).
    pub fn frames(&self, phi: &[f64]) -> Vec<Iso3> {
        let mut f = vec![Iso3::ID; self.walls.len()];
        let mut done = vec![false; self.walls.len()];
        done[0] = true;
        let mut queue = vec![0usize];
        while let Some(w) = queue.pop() {
            for &k in &self.kids[w] {
                let b = &self.bends[k];
                let fz = self.zone_frame(&f, k);
                let to_b = Rigid2::shift(-b.ba, 0.0)
                    .then(&b.zdev.inverse())
                    .then(&self.walls[b.b].dev);
                f[b.b] = fz.then(&b.end_iso(phi[k])).then(&to_b.lift());
                done[b.b] = true;
                queue.push(b.b);
            }
        }
        f
    }

    /// Zone k's coordinates to the part's, given the walls' frames.
    pub fn zone_frame(&self, frames: &[Iso3], k: usize) -> Iso3 {
        let b = &self.bends[k];
        frames[b.a].then(&self.walls[b.a].dev.inverse().then(&b.zdev).lift())
    }

    /// Whether wall `w` lies past bend `k` (in the subtree of its wall b).
    pub fn beyond(&self, k: usize, w: usize) -> bool {
        let mut cur = w;
        loop {
            if cur == self.bends[k].b {
                return true;
            }
            match self.parent[cur] {
                Some(p) => cur = self.bends[p].a,
                None => return false,
            }
        }
    }

    /// The bends between wall `w` and bend `k`'s zone (exclusive), on the path through the tree.
    pub fn path_bends(&self, k: usize, w: usize) -> Vec<usize> {
        let b = &self.bends[k];
        let root_of = |mut cur: usize| {
            let mut v = Vec::new();
            while let Some(p) = self.parent[cur] {
                v.push(p);
                cur = self.bends[p].a;
            }
            v
        };
        let mine = root_of(w);
        if self.beyond(k, w) {
            // from w up to b
            let mut out = Vec::new();
            let mut cur = w;
            while cur != b.b {
                let p = self.parent[cur].expect("below b");
                out.push(p);
                cur = self.bends[p].a;
            }
            out
        } else {
            // from w to a: up from w to the common ancestor, down to a
            let theirs = root_of(b.a);
            let mut out: Vec<usize> = mine.iter().copied().filter(|p| !theirs.contains(p)).collect();
            out.extend(theirs.iter().copied().filter(|p| !mine.contains(p)));
            out
        }
    }
}

/// A unit 2D vector at angle `a`.
pub fn dir(a: f64) -> Vec2 {
    let (s, c) = dmath::sin_cos(a);
    Vec2::new(c, s)
}

pub fn p2(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}
