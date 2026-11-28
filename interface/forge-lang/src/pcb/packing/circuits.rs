//! Coarsen the placement graph before legalization so that unrelated packages cannot
//! occupy the space between members of a pin-local circuit.
use super::*;

impl Model {
    /// Connected components of populated linked supply/return pairs and typed clocks,
    /// optionally extended with template membership supplied by the caller.
    pub fn circuit_groups(
        &self,
        inferred: bool,
        explicit: &[Vec<String>],
    ) -> Result<Vec<Vec<usize>>, String> {
        let mut owner: Vec<usize> = (0..self.ids.len()).collect();
        let mut join = |names: &[String]| -> Result<(), String> {
            if names.len() < 2 {
                return Err("a local circuit needs at least two placements".into());
            }
            let ids = names
                .iter()
                .map(|name| {
                    self.ids
                        .binary_search(name)
                        .map_err(|_| format!("unknown circuit member {name}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let root = owner[ids[0]];
            for i in ids {
                let old = owner[i];
                if old != root {
                    for v in &mut owner {
                        if *v == old {
                            *v = root;
                        }
                    }
                }
            }
            Ok(())
        };
        if inferred {
            for (a, b) in &self.local_pairs {
                join(&[a.clone(), b.clone()])?;
            }
        }
        for group in explicit {
            join(group)?;
        }
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        for (i, g) in owner.into_iter().enumerate() {
            groups.entry(g).or_default().push(i);
        }
        Ok(groups.into_values().filter(|g| g.len() > 1).collect())
    }

    /// Legalize one local circuit in a caller-supplied working area before packing
    /// it on the board. Only guides with both endpoints in the circuit participate.
    pub fn fit_template(
        &self,
        seeds: &mut [Seed],
        members: &[usize],
        room: Rect,
        gap: f64,
    ) -> Result<(), String> {
        self.validate_domain(seeds, room, &[], gap)?;
        let mut subset = Model {
            ids: Vec::new(),
            boxes: Vec::new(),
            guides: Vec::new(),
            local_pairs: Vec::new(),
        };
        let mut map = BTreeMap::new();
        let mut poses = Vec::new();
        for &i in members {
            if i >= self.ids.len() || map.insert(i, poses.len()).is_some() {
                return Err("invalid template member".into());
            }
            subset.ids.push(self.ids[i].clone());
            subset.boxes.push(self.boxes[i]);
            poses.push(seeds[i]);
        }
        for &i in members {
            subset.guides.push(
                self.guides[i]
                    .iter()
                    .filter_map(|e| {
                        Some(Contact {
                            peer: *map.get(&e.peer)?,
                            local: e.local,
                            remote: e.remote,
                            weight: e.weight,
                        })
                    })
                    .collect(),
            );
        }
        let positions = subset.pack(&poses, room, &[], gap)?;
        for (p, at) in poses.iter_mut().zip(positions) {
            p.at = at;
        }
        let allowed = poses.iter().map(|p| !p.fixed).collect::<Vec<_>>();
        subset.polish(&mut poses, room, &[], gap, &allowed)?;
        for (&i, p) in members.iter().zip(poses) {
            seeds[i] = p;
        }
        Ok(())
    }

    pub(super) fn intrusions(&self, seeds: &[Seed], groups: &[Vec<usize>]) -> Vec<String> {
        let packages: Vec<_> = self
            .boxes
            .iter()
            .zip(seeds)
            .map(|(b, p)| b.turn(p.turns).shift(p.at))
            .collect();
        let mut violations = Vec::new();
        for group in groups {
            let mut envelope = packages[group[0]];
            for &i in group {
                for k in 0..2 {
                    envelope.lo[k] = envelope.lo[k].min(packages[i].lo[k]);
                    envelope.hi[k] = envelope.hi[k].max(packages[i].hi[k]);
                }
            }
            let outsiders: Vec<_> = packages
                .iter()
                .enumerate()
                .filter(|(i, b)| !group.contains(i) && b.overlap(envelope, 0.))
                .map(|(i, _)| self.ids[i].as_str())
                .collect();
            if !outsiders.is_empty() {
                let members = group.iter().map(|&i| self.ids[i].as_str()).collect::<Vec<_>>();
                violations.push(format!(
                    "LocalCircuitInterleaved {} contains {}",
                    members.join(","),
                    outsiders.join(",")
                ));
            }
        }
        violations
    }

    /// Synthesize a short initial template around the largest footprint. Shares
    /// just the pad guides internal to this circuit, independent of surrounding nets.
    pub(super) fn compact_inferred(
        &self,
        seeds: &mut [Seed],
        group: &[usize],
        gap: f64,
    ) -> Result<(), String> {
        let fixed: Vec<_> = group.iter().copied().filter(|&i| seeds[i].fixed).collect();
        if fixed.len() > 1 {
            return Ok(());
        }
        let root = fixed.first().copied().unwrap_or_else(|| {
            *group
                .iter()
                .max_by(|&&i, &&j| self.boxes[i].area().total_cmp(&self.boxes[j].area()))
                .unwrap()
        });
        let origin = seeds[root].at;
        let reach = group
            .iter()
            .map(|&i| {
                let b = self.boxes[i];
                (b.hi[0] - b.lo[0]).max(b.hi[1] - b.lo[1]) + gap
            })
            .sum::<f64>();
        let locked = seeds[root].fixed;
        // Keep an anchored template on the package origin using a local coordinate frame.
        for &i in group {
            seeds[i].at = [0., 0.];
        }
        seeds[root].fixed = true;
        self.fit_template(
            seeds,
            group,
            Rect {
                lo: [-reach; 2],
                hi: [reach; 2],
            },
            gap,
        )?;
        seeds[root].fixed = locked;
        for &i in group {
            for (x, d) in seeds[i].at.iter_mut().zip(origin) {
                *x += d;
            }
        }
        Ok(())
    }

    /// Pack each local template as one movable envelope. Member offsets and headings
    /// come from the supplied seeds. Electrical guides retain their actual pad offsets.
    /// A locked member locks its entire template. Empty groups use ordinary packing.
    pub fn pack_circuits(
        &self,
        seeds: &mut [Seed],
        groups: &[Vec<usize>],
        room: Rect,
        keepouts: &[Rect],
        gap: f64,
        rotate_free: Option<&[bool]>,
    ) -> Result<(), String> {
        self.validate_domain(seeds, room, keepouts, gap)?;
        if rotate_free.is_some_and(|r| r.len() != seeds.len()) {
            return Err("missing rotation permissions".into());
        }
        let mut membership = vec![None; seeds.len()];
        let mut bodies = groups.to_vec();
        for (g, group) in groups.iter().enumerate() {
            if group.len() < 2 {
                return Err("a circuit needs at least two placements".into());
            }
            for &i in group {
                if i >= seeds.len() || membership[i].replace(g).is_some() {
                    return Err("invalid or overlapping circuit membership".into());
                }
            }
        }
        for (i, m) in membership.iter_mut().enumerate() {
            if m.is_none() {
                *m = Some(bodies.len());
                bodies.push(vec![i]);
            }
        }
        let mut coarse = Model {
            ids: Vec::new(),
            boxes: Vec::new(),
            guides: (0..bodies.len()).map(|_| Vec::new()).collect(),
            local_pairs: Vec::new(),
        };
        let mut anchors = Vec::new();
        let mut allowed = Vec::new();
        let mut offsets = vec![[0., 0.]; seeds.len()];
        for group in &bodies {
            let origin = seeds[group[0]].at;
            let mut envelopes = Vec::<(usize, Rect)>::new();
            for &i in group {
                offsets[i] = [seeds[i].at[0] - origin[0], seeds[i].at[1] - origin[1]];
                let r = self.boxes[i].turn(seeds[i].turns).shift(offsets[i]);
                if let Some((j, _)) = envelopes.iter().find(|(_, b)| b.overlap(r, gap - 1e-7)) {
                    return Err(format!(
                        "local template overlaps {} and {}",
                        self.ids[i], self.ids[*j]
                    ));
                }
                envelopes.push((i, r));
            }
            let mut envelope = envelopes[0].1;
            for (_, r) in envelopes {
                for k in 0..2 {
                    envelope.lo[k] = envelope.lo[k].min(r.lo[k]);
                    envelope.hi[k] = envelope.hi[k].max(r.hi[k]);
                }
            }
            coarse.ids.push(self.ids[group[0]].clone());
            coarse.boxes.push(envelope);
            anchors.push(Seed {
                at: origin,
                turns: 0,
                fixed: group.iter().any(|&i| seeds[i].fixed),
            });
            allowed.push(rotate_free.is_some_and(|r| group.iter().all(|&i| r[i])));
        }
        let local = |i: usize, xy: XY| {
            let p = rotate(xy, seeds[i].turns);
            [p[0] + offsets[i][0], p[1] + offsets[i][1]]
        };
        for (i, edges) in self.guides.iter().enumerate() {
            let g = membership[i].unwrap();
            for e in edges {
                let peer = membership[e.peer].unwrap();
                if g != peer {
                    coarse.guides[g].push(Contact {
                        peer,
                        local: local(i, e.local),
                        remote: local(e.peer, e.remote),
                        weight: e.weight,
                    });
                }
            }
        }
        let positions = coarse.pack(&anchors, room, keepouts, gap)?;
        for (p, at) in anchors.iter_mut().zip(positions) {
            p.at = at;
        }
        if rotate_free.is_some() {
            coarse.polish(&mut anchors, room, keepouts, gap, &allowed)?;
        }
        for (i, s) in seeds.iter_mut().enumerate() {
            let body = anchors[membership[i].unwrap()];
            let d = rotate(offsets[i], body.turns);
            s.at = [body.at[0] + d[0], body.at[1] + d[1]];
            s.turns = (s.turns % 4 + body.turns) % 4;
        }
        Ok(())
    }
}
