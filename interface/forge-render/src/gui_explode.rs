//! An assembly's exploded view in the app: the plan made once per solve, the scene moved towards
//! the panel's slider a little each frame, and the trails from home.

use super::Gui;
use crate::explode::{Item, Plan, at, plan};
use crate::origin::WorldVert;
use crate::viewer::now_ms;

/// How fast the view follows the slider: the whole way in this many seconds.
const WHOLE_WAY_S: f32 = 0.45;

#[derive(Default)]
pub struct Explode {
    pub plan: Option<Plan>,
    /// How far apart the view is now, and when it last moved.
    pub now: f32,
    last: Option<f64>,
    /// What the scene was last moved for: its placements and `now`.
    applied: Option<(usize, f32)>,
    /// Each placement's occurrence, by the plan's order.
    occ_of: Vec<usize>,
}

impl Explode {
    /// A new solve: the plan's made again, and put on the scene again.
    pub fn stale(&mut self) {
        self.plan = None;
        self.applied = None;
    }
}

impl Gui {
    fn explode_target(&self) -> (f32, bool) {
        self.shell
            .fg
            .as_ref()
            .and_then(|p| p.asm.as_ref())
            .map_or((0.0, false), |a| (a.explode, a.trails))
    }

    /// The exploded view's plan, from the placements, their meshes' boxes and the browser's units.
    pub fn explode_plan(&self) -> Option<(Plan, Vec<usize>)> {
        let (occ, placed) = self.fg_asm.as_ref()?;
        let a = self.shell.fg.as_ref()?.asm.as_ref()?;
        if a.tree.group.len() != occ.len() {
            return None;
        }
        let mut items = vec![
            Item {
                rows: [[0.0; 4]; 3],
                local: ([0.0; 3], [0.0; 3]),
                group: 0,
                sub: None,
                ground: false,
                axis: None,
            };
            placed.len()
        ];
        let mut occ_of = vec![0; placed.len()];
        for (k, o) in occ.iter().enumerate() {
            let Some(pl) = o.placement else { continue };
            let (mesh, rows) = placed[pl];
            items[pl] = Item {
                rows,
                local: self
                    .fg_asm_boxes
                    .get(mesh)
                    .copied()
                    .unwrap_or(([0.0; 3], [0.0; 3])),
                group: a.tree.group[k],
                sub: a.tree.sub[k],
                ground: a.grounded(k),
                axis: a.tree.axis[k],
            };
            occ_of[pl] = k;
            // a thermoformed part's mould goes with it
            if let Some(m) = o.tooling {
                items[m] = Item {
                    rows: placed[m].1,
                    local: self
                        .fg_asm_boxes
                        .get(placed[m].0)
                        .copied()
                        .unwrap_or(([0.0; 3], [0.0; 3])),
                    ..items[pl].clone()
                };
                occ_of[m] = k;
            }
        }
        Some((plan(&items), occ_of))
    }

    /// One frame of the exploded view: towards the slider; true while it moves.
    pub(super) fn explode_tick(&mut self) -> bool {
        let (target, _) = self.explode_target();
        if self.fg_asm.is_none() {
            return false;
        }
        if self.explode.plan.is_none() && (target > 0.0 || self.explode.now > 0.0) {
            let t = now_ms();
            if let Some((p, occ_of)) = self.explode_plan() {
                self.explode.plan = Some(p);
                self.explode.occ_of = occ_of;
                eprintln!("forge-app: explode plan in {:.0} ms", now_ms() - t);
            }
        }
        let t = now_ms();
        let dt = self.explode.last.map_or(0.0, |l| ((t - l) / 1e3) as f32).min(0.1);
        let moving = self.explode.now != target;
        if moving {
            let step = dt / WHOLE_WAY_S;
            let d = target - self.explode.now;
            self.explode.now = if d.abs() <= step {
                target
            } else {
                self.explode.now + step * d.signum()
            };
            self.explode.last = Some(t);
        } else {
            self.explode.last = None;
        }
        if moving && self.explode.last.is_none() {
            self.explode.last = Some(t);
        }
        let Some(p) = &self.explode.plan else {
            return moving;
        };
        let Some(v) = &mut self.view else { return moving };
        let want = (v.scene.placements.len(), self.explode.now);
        if self.explode.applied != Some(want) && p.unit.len() == want.0 {
            v.scene.set_offsets(&at(p, self.explode.now));
            v.dirty = true;
            self.explode.applied = Some(want);
        }
        moving || self.explode.applied != Some(want)
    }

    pub(super) fn explode_key(&self) -> u64 {
        let (_, trails) = self.explode_target();
        (self.explode.now * 1e4) as u64 * 2 + u64::from(trails)
    }

    /// Each shown part's trail from home to where it's moved to.
    pub(super) fn explode_trails(&self, out: &mut Vec<WorldVert>) {
        let (_, trails) = self.explode_target();
        if !trails || self.explode.now <= 0.0 {
            return;
        }
        let (Some(v), Some((_, placed))) = (&self.view, &self.fg_asm) else {
            return;
        };
        let Some(a) = self.shell.fg.as_ref().and_then(|p| p.asm.as_ref()) else {
            return;
        };
        let shown = a.shown_mask();
        let rgb = crate::viewcube::rgb(0xE0A21B);
        let (cam, size, px) = (&v.nav.camera, v.size, self.pixels_per_point);
        for (pl, (mesh, rows)) in placed.iter().enumerate() {
            if !self
                .explode
                .occ_of
                .get(pl)
                .is_some_and(|&k| shown.get(k).copied().unwrap_or(false))
            {
                continue;
            }
            let d = v.scene.offset_of(pl);
            if d == [0.0; 3] {
                continue;
            }
            let b = self
                .fg_asm_boxes
                .get(*mesh)
                .copied()
                .unwrap_or(([0.0; 3], [0.0; 3]));
            let c = [0, 1, 2].map(|k| (b.0[k] + b.1[k]) * 0.5);
            let home =
                [0, 1, 2].map(|r| rows[r][0] * c[0] + rows[r][1] * c[1] + rows[r][2] * c[2] + rows[r][3]);
            let to = [home[0] + d[0], home[1] + d[1], home[2] + d[2]];
            crate::overlay::trail(home, to, cam, size, px, rgb, out);
        }
    }
}
