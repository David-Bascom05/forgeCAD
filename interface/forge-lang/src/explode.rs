//! An assembly's exploded view: where each occurrence goes, fully apart. Units (a real
//! subassembly, or a browser group such as Frame or Casters) move out first, each the least way
//! that clears every unit already placed, so no two units' boxes overlap; a fastener rides with the
//! part it sits in, then backs out along its own axis by its whole length, clear of its hole, and
//! fasteners stacked on one axis (screw, washer, nut) are spaced so none overlaps another.

/// A pose as the renderer keeps it: three rows of a 3x4 matrix.
pub type Rows = [[f32; 4]; 3];

/// Whether a hardware part backs out towards its head (a screw or stud, its z < 0) rather than
/// off the other way (a nut or washer), by its file's name.
pub fn screw_like(file: &str) -> bool {
    let stem = file
        .rsplit('/')
        .next()
        .unwrap_or(file)
        .trim_end_matches(".fg")
        .to_ascii_lowercase();
    const SCREWS: [&str; 16] = [
        "iso4014", "iso4017", "iso4762", "iso7380", "iso7045", "iso7046", "iso7379", "iso10642", "iso1207",
        "iso4026", "iso4027", "iso4028", "iso4029", "iso14579", "iso14580", "iso14583",
    ];
    stem.contains("screw")
        || stem.contains("bolt")
        || stem.contains("stud")
        || stem.starts_with("pem_fh")
        || SCREWS.iter().any(|s| stem.starts_with(s))
}

/// A box: its low and high corners.
pub type Aabb = ([f32; 3], [f32; 3]);

#[derive(Clone, Debug)]
pub struct Item {
    /// Its pose.
    pub rows: Rows,
    /// The part's box in its own space.
    pub local: Aabb,
    /// Its browser group, by the browser's order (Frame first).
    pub group: usize,
    /// The real subassembly it's in, if any.
    pub sub: Option<usize>,
    /// It's (in) a grounded instance: its group stays where it is.
    pub ground: bool,
    /// A fastener: which way along its own z it backs out (-1 a screw or stud, head at z < 0;
    /// +1 a nut or washer, off the bolt's end).
    pub axis: Option<f32>,
}

/// Each item's move, fully exploded: its unit's, and its own along its axis.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub unit: Vec<[f32; 3]>,
    pub own: Vec<[f32; 3]>,
    /// The unit each item moved with (a fastener's is its host's).
    pub units: Vec<usize>,
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn len(a: [f32; 3]) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// The box of a part's box placed by `rows`.
pub fn world_box(rows: &Rows, local: &Aabb) -> Aabb {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for k in 0..8 {
        let p = [
            if k & 1 == 0 { local.0[0] } else { local.1[0] },
            if k & 2 == 0 { local.0[1] } else { local.1[1] },
            if k & 4 == 0 { local.0[2] } else { local.1[2] },
        ];
        for r in 0..3 {
            let w = rows[r][0] * p[0] + rows[r][1] * p[1] + rows[r][2] * p[2] + rows[r][3];
            lo[r] = lo[r].min(w);
            hi[r] = hi[r].max(w);
        }
    }
    (lo, hi)
}

pub fn shifted(b: &Aabb, d: [f32; 3]) -> Aabb {
    (add(b.0, d), add(b.1, d))
}

fn union(a: &Aabb, b: &Aabb) -> Aabb {
    (
        [0, 1, 2].map(|k| a.0[k].min(b.0[k])),
        [0, 1, 2].map(|k| a.1[k].max(b.1[k])),
    )
}

/// Whether two boxes come closer than `gap`.
pub fn overlap(a: &Aabb, b: &Aabb, gap: f32) -> bool {
    (0..3).all(|k| a.0[k] < b.1[k] + gap && b.0[k] < a.1[k] + gap)
}

fn volume(b: &Aabb) -> f32 {
    (0..3).map(|k| (b.1[k] - b.0[k]).max(1e-3)).product()
}

fn middle(b: &Aabb) -> [f32; 3] {
    [0, 1, 2].map(|k| (b.0[k] + b.1[k]) * 0.5)
}

fn z_of(rows: &Rows) -> [f32; 3] {
    [rows[0][2], rows[1][2], rows[2][2]]
}

fn origin_of(rows: &Rows) -> [f32; 3] {
    [rows[0][3], rows[1][3], rows[2][3]]
}

fn inside(p: [f32; 3], b: &Aabb, gap: f32) -> bool {
    (0..3).all(|k| p[k] >= b.0[k] - gap && p[k] <= b.1[k] + gap)
}

/// How far off the middle (in shares of the assembly's half size) a group is on one side.
const SIDE: f32 = 0.35;
/// How far apart units stand once clear, in shares of the assembly's half diagonal.
const SPREAD: f32 = 0.15;

pub fn plan(items: &[Item]) -> Plan {
    let n = items.len();
    if n == 0 {
        return Plan::default();
    }
    let boxes: Vec<Aabb> = items.iter().map(|i| world_box(&i.rows, &i.local)).collect();
    let all = boxes.iter().skip(1).fold(boxes[0], |a, b| union(&a, b));
    let half = [0, 1, 2].map(|k| ((all.1[k] - all.0[k]) * 0.5).max(1e-3));
    let reach = len(half);
    let centre = middle(&all);
    let gap = (reach * 0.01).max(0.5);
    // where a box is, in shares of the assembly's half size each way
    let share = |b: &Aabb| {
        let m = middle(b);
        [0, 1, 2].map(|k| (m[k] - centre[k]) / half[k])
    };
    // the axis a side is on: its biggest share, and which way
    let side_of = |s: [f32; 3]| -> Option<[f32; 3]> {
        let k = (0..3).max_by(|&a, &b| s[a].abs().total_cmp(&s[b].abs()))?;
        (s[k].abs() >= SIDE).then(|| {
            let mut d = [0.0; 3];
            d[k] = s[k].signum();
            d
        })
    };
    let solid: Vec<usize> = (0..n).filter(|&i| items[i].axis.is_none()).collect();
    let group_box = |pick: &dyn Fn(usize) -> bool| -> Option<Aabb> {
        solid
            .iter()
            .filter(|&&i| pick(i))
            .map(|&i| boxes[i])
            .reduce(|a, b| union(&a, &b))
    };
    // the group that stays: the biggest, the browser's first of two near it
    let groups: Vec<usize> = {
        let mut g: Vec<usize> = solid.iter().map(|&i| items[i].group).collect();
        g.sort();
        g.dedup();
        g
    };
    let vol: Vec<(usize, f32)> = groups
        .iter()
        .filter_map(|&g| {
            group_box(&|i| items[i].group == g && items[i].sub.is_none()).map(|b| (g, volume(&b)))
        })
        .collect();
    let top = vol.iter().map(|v| v.1).fold(0.0, f32::max);
    // the grounded part's group, else the biggest
    let fixed_group = solid
        .iter()
        .find(|&&i| items[i].ground)
        .map(|&i| items[i].group)
        .or_else(|| vol.iter().find(|v| v.1 >= top * 0.9).map(|v| v.0));
    // a subassembly's group: its biggest part's
    let mut sub_group: std::collections::BTreeMap<usize, (usize, f32)> = Default::default();
    for &i in &solid {
        if let Some(s) = items[i].sub {
            let v = volume(&boxes[i]);
            let e = sub_group.entry(s).or_insert((items[i].group, v));
            if v > e.1 {
                *e = (items[i].group, v);
            }
        }
    }
    // units: 0 the one that stays; a subassembly whole; an off-middle group whole; a group in the
    // middle split by the side each part is on
    let mut key_of: Vec<(u8, usize, [i8; 3])> = vec![(0, 0, [0; 3]); n];
    let group_side: std::collections::BTreeMap<usize, Option<[f32; 3]>> = groups
        .iter()
        .map(|&g| {
            (
                g,
                group_box(&|i| items[i].group == g && items[i].sub.is_none())
                    .and_then(|b| side_of(share(&b))),
            )
        })
        .collect();
    let mut way: std::collections::HashMap<(u8, usize, [i8; 3]), Option<[f32; 3]>> = Default::default();
    for &i in &solid {
        let it = &items[i];
        let key = match it.sub {
            Some(s) if sub_group.get(&s).map(|x| x.0) == fixed_group => (0, 0, [0; 3]),
            Some(s) => {
                let b = group_box(&|j| items[j].sub == Some(s)).unwrap_or(boxes[i]);
                way.insert((1, s, [0; 3]), side_of(share(&b)));
                (1, s, [0; 3])
            }
            None if Some(it.group) == fixed_group => (0, 0, [0; 3]),
            None => match group_side.get(&it.group).copied().flatten() {
                Some(d) => {
                    way.insert((2, it.group, [0; 3]), Some(d));
                    (2, it.group, [0; 3])
                }
                None => {
                    let d = side_of(share(&boxes[i]));
                    let tag = d.map_or([0; 3], |d| d.map(|x| x as i8));
                    way.insert((3, it.group, tag), d);
                    (3, it.group, tag)
                }
            },
        };
        key_of[i] = key;
    }
    let mut keys: Vec<(u8, usize, [i8; 3])> = key_of
        .iter()
        .enumerate()
        .filter(|x| items[x.0].axis.is_none())
        .map(|x| *x.1)
        .collect();
    keys.sort();
    keys.dedup();
    if !keys.contains(&(0, 0, [0; 3])) {
        keys.insert(0, (0, 0, [0; 3]));
    }
    let unit_of = |k: &(u8, usize, [i8; 3])| keys.iter().position(|x| x == k).unwrap_or(0);
    let mut units: Vec<usize> = key_of.iter().map(unit_of).collect();
    // a fastener rides with the smallest part it touches: the one it's in
    for (i, it) in items.iter().enumerate() {
        if it.axis.is_none() {
            continue;
        }
        let at = origin_of(&it.rows);
        let host = solid
            .iter()
            .copied()
            .filter(|&j| overlap(&boxes[i], &boxes[j], gap))
            .min_by(|&a, &b| {
                // the part its origin is in first, then the smallest
                (!inside(at, &boxes[a], gap))
                    .cmp(&!inside(at, &boxes[b], gap))
                    .then(volume(&boxes[a]).total_cmp(&volume(&boxes[b])))
            });
        units[i] = host.map_or(0, |h| units[h]);
    }
    // each fastener's own way out: its whole length along its axis, and a gap
    let dirs: Vec<[f32; 3]> = items
        .iter()
        .map(|i| match i.axis {
            Some(s) => {
                let z = z_of(&i.rows);
                scale(z, s / len(z).max(1e-6))
            }
            None => [0.0; 3],
        })
        .collect();
    let mut own: Vec<[f32; 3]> = items
        .iter()
        .zip(&dirs)
        .map(|(i, d)| match i.axis {
            Some(_) => scale(*d, i.local.1[2] - i.local.0[2] + gap),
            None => [0.0; 3],
        })
        .collect();
    // fasteners that would end up overlapping (a washer under a screw's head, two nuts on one
    // stud): the one further along its way goes on past the other, till none do
    let mut by_unit: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for i in (0..n).filter(|&i| items[i].axis.is_some()) {
        by_unit.entry(units[i]).or_default().push(i);
    }
    for list in by_unit.values() {
        for _ in 0..64 {
            let mut moved = false;
            for (a, &i) in list.iter().enumerate() {
                for &j in &list[a + 1..] {
                    let (bi, bj) = (shifted(&boxes[i], own[i]), shifted(&boxes[j], own[j]));
                    if !overlap(&bi, &bj, gap * 0.5) {
                        continue;
                    }
                    let lead = |x: usize, y: usize| {
                        let d = dirs[x];
                        let (cx, cy) = (middle(&boxes[x]), middle(&boxes[y]));
                        (0..3).map(|k| (cx[k] - cy[k]) * d[k]).sum::<f32>()
                    };
                    let (go, past) = if lead(i, j) >= lead(j, i) { (i, j) } else { (j, i) };
                    let span = items[past].local.1[2] - items[past].local.0[2];
                    own[go] = add(own[go], scale(dirs[go], span.max(gap) + gap));
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
    }
    // each unit's box, its fasteners where they end up
    let count = keys.len();
    let mut ubox: Vec<Option<Aabb>> = vec![None; count];
    let mut uhome: Vec<Option<Aabb>> = vec![None; count];
    for i in 0..n {
        let b = shifted(&boxes[i], own[i]);
        let u = units[i];
        ubox[u] = Some(ubox[u].map_or(b, |x| union(&x, &b)));
        uhome[u] = Some(uhome[u].map_or(boxes[i], |x| union(&x, &boxes[i])));
    }
    let mut off = vec![[0.0f32; 3]; count];
    // unit 0 stays; the rest, nearest the middle first, each the least way out (its side's, or
    // the best axis for one in the middle) that clears every unit placed
    let mut placed: Vec<Aabb> = ubox[0].into_iter().collect();
    let mut order: Vec<usize> = (1..count).filter(|&u| ubox[u].is_some()).collect();
    let dist = |u: usize| len(share(&uhome[u].unwrap()));
    order.sort_by(|&a, &b| dist(a).total_cmp(&dist(b)));
    let step = (reach * 0.005).max(0.25);
    for u in order {
        let b = ubox[u].unwrap();
        // one inside what stays (a unit in a rack) slides out sideways, the least way
        let held = uhome[0].is_some_and(|f| {
            let h = uhome[u].unwrap();
            (0..3).all(|k| h.0[k] >= f.0[k] - gap && h.1[k] <= f.1[k] + gap)
        });
        let axes = |n: usize| -> Vec<[f32; 3]> {
            (0..n)
                .map(|k| {
                    let mut d = [0.0; 3];
                    d[k / 2] = if k % 2 == 0 { -1.0 } else { 1.0 };
                    d
                })
                .collect()
        };
        let ways: Vec<[f32; 3]> = match way.get(&keys[u]).copied().flatten() {
            Some(_) if held => axes(4),
            Some(d) => vec![d],
            None => axes(if held { 4 } else { 6 }),
        };
        let mut best: Option<([f32; 3], f32)> = None;
        for d in ways {
            let mut t = gap;
            while t < reach * 8.0 && placed.iter().any(|p| overlap(&shifted(&b, scale(d, t)), p, gap)) {
                t += step;
            }
            if best.is_none_or(|x| t < x.1) {
                best = Some((scale(d, t), t));
            }
        }
        // clear, and then apart by a fixed share of the whole: clear by a hair reads as together
        let d = best.map_or([0.0; 3], |b| {
            let l = len(b.0).max(1e-6);
            scale(b.0, (l + reach * SPREAD) / l)
        });
        off[u] = d;
        placed.push(shifted(&b, d));
    }
    Plan {
        unit: units.iter().map(|&u| off[u]).collect(),
        own,
        units,
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Each item's move at `t` (0 home, 1 fully apart): the units first, then the fasteners.
pub fn at(plan: &Plan, t: f32) -> Vec<[f32; 3]> {
    let (a, b) = (smooth(t / 0.6), smooth((t - 0.4) / 0.6));
    plan.unit
        .iter()
        .zip(&plan.own)
        .map(|(u, o)| add(scale(*u, a), scale(*o, b)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_pos(p: [f32; 3]) -> Rows {
        [
            [1.0, 0.0, 0.0, p[0]],
            [0.0, 1.0, 0.0, p[1]],
            [0.0, 0.0, 1.0, p[2]],
        ]
    }

    /// Two plates stacked (each its own unit), a bolt down through both (head up, its z down),
    /// a washer under the head and a nut under the lower plate.
    fn joint() -> Vec<Item> {
        let flip = |p: [f32; 3]| -> Rows {
            // z pointing down: the bolt goes in from above
            [
                [1.0, 0.0, 0.0, p[0]],
                [0.0, -1.0, 0.0, p[1]],
                [0.0, 0.0, -1.0, p[2]],
            ]
        };
        vec![
            Item {
                rows: at_pos([0.0; 3]),
                local: ([-30.0, -30.0, 0.0], [30.0, 30.0, 3.0]),
                group: 0,
                sub: None,
                ground: false,
                axis: None,
            },
            Item {
                rows: at_pos([0.0, 0.0, 3.0]),
                local: ([-30.0, -30.0, 0.0], [30.0, 30.0, 3.0]),
                group: 1,
                sub: None,
                ground: false,
                axis: None,
            },
            // bolt: head z in -4..0, shank 0..20, its origin on the top face
            Item {
                rows: flip([0.0, 0.0, 7.6]),
                local: ([-5.0, -5.0, -4.0], [5.0, 5.0, 20.0]),
                group: 2,
                sub: None,
                ground: false,
                axis: Some(-1.0),
            },
            // washer under the head
            Item {
                rows: at_pos([0.0, 0.0, 6.0]),
                local: ([-6.0, -6.0, 0.0], [6.0, 6.0, 1.6]),
                group: 2,
                sub: None,
                ground: false,
                axis: Some(1.0),
            },
            // nut under the lower plate, bearing face up: its z down
            Item {
                rows: flip([0.0, 0.0, 0.0]),
                local: ([-5.0, -5.0, 0.0], [5.0, 5.0, 5.0]),
                group: 2,
                sub: None,
                ground: false,
                axis: Some(1.0),
            },
        ]
    }

    #[test]
    fn fully_exploded_nothing_overlaps() {
        let items = joint();
        let p = plan(&items);
        let moved = at(&p, 1.0);
        let boxes: Vec<Aabb> = items
            .iter()
            .zip(&moved)
            .map(|(i, d)| shifted(&world_box(&i.rows, &i.local), *d))
            .collect();
        for a in 0..boxes.len() {
            for b in a + 1..boxes.len() {
                assert!(
                    !overlap(&boxes[a], &boxes[b], 0.0),
                    "{a} and {b} overlap: {:?} {:?}",
                    boxes[a],
                    boxes[b]
                );
            }
        }
        // home is home
        assert!(at(&p, 0.0).iter().all(|d| *d == [0.0; 3]));
    }

    #[test]
    fn fasteners_move_along_their_own_axis() {
        let items = joint();
        let p = plan(&items);
        for (k, it) in items.iter().enumerate().filter(|x| x.1.axis.is_some()) {
            let d = p.own[k];
            let z = z_of(&it.rows);
            let along = d[0] * z[0] + d[1] * z[1] + d[2] * z[2];
            // all of it along z, and the way the fastener says
            assert!((along.abs() - len(d)).abs() < 1e-3, "{k}: {d:?}");
            assert!(along * it.axis.unwrap() > 0.0, "{k}: {d:?}");
            // its whole length, out of its hole
            assert!(len(d) >= it.local.1[2] - it.local.0[2], "{k}: {d:?}");
        }
        // the bolt backs out upwards (its head's way), the nut drops off its end
        assert!(p.own[2][2] > 0.0 && p.own[4][2] < 0.0, "{:?}", p.own);
        // the fasteners ride with the plate they sit in: their unit is a plate's
        assert!(
            p.units[2] == p.units[0] || p.units[2] == p.units[1],
            "{:?}",
            p.units
        );
        assert!(
            p.units[4] == p.units[0] || p.units[4] == p.units[1],
            "{:?}",
            p.units
        );
    }

    #[test]
    fn units_go_first_then_fasteners() {
        let p = plan(&joint());
        let half = at(&p, 0.4);
        // by 0.4 the plates are well apart, the fasteners still home on them
        assert!((0..5).any(|k| len(p.unit[k]) > 0.0));
        for (k, h) in half.iter().enumerate().skip(2) {
            assert_eq!(*h, scale(p.unit[k], smooth(0.4 / 0.6)), "{k}");
        }
    }
}
