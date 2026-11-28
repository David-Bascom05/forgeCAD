use super::*;
type Error = Box<dyn std::error::Error>;
const GAP: f64 = 0.55;
const MCU: &[(&str, XY, u8)] = &[
    ("U2", [0., 0.], 0),
    ("C4", [-6.6, 0.], 3),
    ("Y1", [-8., -3.55], 0),
    ("C9", [-11.65, -3.55], 3),
    ("C5", [-9.05, 0.22], 3),
    ("C6", [-6.6, 3.45], 3),
    ("C7", [-9.1, 4.3], 3),
    ("R9", [-6.7, 7.3], 0),
    ("C8", [-5., -6.7], 2),
    ("R10", [-8.6, -6.7], 0),
];
const MCU_GUIDES: &[(&str, &str)] = &[
    ("Y1.3", "U2.8"),
    ("C9.1", "Y1.4"),
    ("C9.2", "Y1.2"),
    ("C4.1", "U2.6"),
    ("C4.2", "U2.7"),
    ("C5.1", "U2.6"),
    ("C6.1", "U2.5"),
    ("C7.1", "U2.5"),
    ("R9.2", "U2.5"),
    ("C8.1", "U2.10"),
    ("R10.2", "U2.10"),
];

// Board-specific placement budgets in millimetres (not electrical DRC limits).
// Check every handoff separately. A short connection cannot compensate for a bad one.
const HANDOFFS: &[(&str, &str, f64)] = &[
    ("D2.3", "R1.1", 16.),
    ("Q1.3", "C31.1", 36.),
    ("C106.1", "U102.1", 25.),
    ("C109.1", "C4.1", 25.),
    ("Q3.2", "L1.1", 24.),
];
fn handoffs(board: &Board, model: &Model, seeds: &[Seed], poses: &[XY]) -> Vec<f64> {
    HANDOFFS
        .iter()
        .map(|&(a, z, _)| path_length(board, model, seeds, poses, &[(a, z)]))
        .collect()
}
fn handoff_excess(lengths: &[f64]) -> f64 {
    lengths
        .iter()
        .zip(HANDOFFS)
        .map(|(d, h)| (d / h.2 - 1.).max(0.))
        .fold(0., f64::max)
}

struct Template {
    members: Vec<usize>,
    poses: Vec<Seed>,
}
impl Template {
    fn at(&self, seeds: &mut [Seed], at: XY, turn: u8, fixed: Option<usize>) {
        for (&i, p) in self.members.iter().zip(&self.poses) {
            let d = rotate(p.at, turn);
            seeds[i] = Seed {
                at: [at[0] + d[0], at[1] + d[1]],
                turns: (turn + p.turns) % 4,
                fixed: fixed == Some(i),
            };
        }
    }
}
fn template(m: &Model, ids: Vec<usize>, pinned: &[(&str, XY, u8)], bounds: Rect) -> Result<Template, Error> {
    let mut seeds = vec![
        Seed {
            at: [0., 0.],
            turns: 0,
            fixed: false
        };
        m.ids.len()
    ];
    for &(name, at, turns) in pinned {
        let i = m
            .ids
            .binary_search_by(|v| v.as_str().cmp(name))
            .map_err(|_| format!("unknown {name}"))?;
        if !ids.contains(&i) {
            return Err(format!("unexpected template member {name}").into());
        }
        seeds[i] = Seed {
            at,
            turns,
            fixed: true,
        };
    }
    m.fit_template(&mut seeds, &ids, bounds, GAP)
        .map_err(|e| format!("circuit around {}: {e}", pinned[0].0))?;
    Ok(Template {
        poses: ids.iter().map(|&i| seeds[i]).collect(),
        members: ids,
    })
}
fn local_search(input: &Path, out: &Path) -> Result<(), Error> {
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err("choose an empty output directory".into());
    }
    fs::create_dir_all(out)?;
    let dir = input.parent().unwrap();
    for file in ["symbols.fg", "footprints.fg", "six-amp-schematic.fg"] {
        fs::copy(dir.join(file), out.join(file))?;
    }
    let src = fs::read_to_string(input)?;
    let b = pcb::compile(&src, Some(dir), None).map_err(|e| e.message)?;
    let mut m = Model::new(&b)?;
    for (pairs, weight) in [
        (&HOT[..], 15.),
        (&GATES[..], 16.),
        (MCU_GUIDES, 18.),
        (&BYPASS[..], 12.),
    ] {
        for &(a, z) in pairs {
            m.add_guide(&b, a, z, weight)?;
        }
    }
    for n in 11..=16 {
        for (a, z) in [
            (format!("D{n}.1"), format!("J{n}.3")),
            (format!("D{n}.2"), format!("J{n}.4")),
            (format!("U{n}.18"), format!("J{n}.2")),
        ] {
            m.add_guide(&b, &a, &z, 10.)?;
        }
    }
    let named = |names: &[&str]| {
        names
            .iter()
            .map(|n| m.ids.binary_search_by(|s| s.as_str().cmp(n)).unwrap())
            .collect::<Vec<_>>()
    };
    let scope = |suffix: &str| {
        m.ids
            .iter()
            .enumerate()
            .filter(|(_, id)| {
                b.layout.placements[*id]
                    .occurrence
                    .as_deref()
                    .and_then(|p| p.rsplit_once('/'))
                    .is_some_and(|(p, _)| p.ends_with(suffix))
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    };
    let rect = |lo, hi| Rect { lo, hi };
    let mcu = template(
        &m,
        named(&MCU.iter().map(|x| x.0).collect::<Vec<_>>()),
        MCU,
        rect([-15., -9.], [7., 9.]),
    )?;
    let can = template(
        &m,
        named(&["U3", "C10", "D5", "JP1", "R8", "J2"]),
        &[("J2", [0., 0.], 0), ("U3", [-1., 9.], 2), ("C10", [4., 9.5], 1)],
        rect([-10., -4.5], [11., 15.]),
    )?;
    let ldo_ids = named(&["U102", "C108", "C109"]);
    let ldo = template(
        &m,
        ldo_ids.clone(),
        &[
            ("U102", [0., 0.], 0),
            ("C108", [-3.6, 0.], 3),
            ("C109", [3.6, 0.], 3),
        ],
        rect([-6., -3.], [6., 3.]),
    )?;
    let aux = template(
        &m,
        scope("block_aux")
            .into_iter()
            .filter(|i| !ldo_ids.contains(i))
            .collect(),
        &[
            ("U101", [0., 0.], 0),
            ("L101", [9., 0.], 0),
            ("D101", [2., -7.0], 0),
            ("C106", [15.3, -2.9], 0),
            ("C107", [15.3, 2.9], 0),
        ],
        rect([-11., -10.], [17., 9.]),
    )?;
    let critical = named(&["Q2", "Q3", "C31", "R20", "U4"]);
    let output = named(&["L1", "C41", "C42", "C43", "C44", "C45"]);
    let control = scope("block_main")
        .into_iter()
        .filter(|i| !output.contains(i))
        .collect::<Vec<_>>();
    let mut best_core = None;
    for high in [0, 2] {
        for low in [0, 2] {
            let Ok(trial) = template(
                &m,
                critical.clone(),
                &[("Q2", [0., 5.6], high), ("Q3", [0., -5.6], low)],
                rect([-18., -14.], [10., 14.]),
            ) else {
                continue;
            };
            let mut seeds = vec![
                Seed {
                    at: [0., 0.],
                    turns: 0,
                    fixed: false
                };
                m.ids.len()
            ];
            trial.at(&mut seeds, [0., 0.], 0, None);
            let at: Vec<_> = seeds.iter().map(|s| s.at).collect();
            let cost = path_length(&b, &m, &seeds, &at, &HOT) + 2. * path_length(&b, &m, &seeds, &at, &GATES);
            if best_core.as_ref().is_none_or(|(k, _)| cost < *k) {
                best_core = Some((cost, trial));
            }
        }
    }
    let (_, core) = best_core.ok_or("no switch-cell arrangement")?;
    let pins = core
        .members
        .iter()
        .zip(&core.poses)
        .map(|(&i, p)| (m.ids[i].as_str(), p.at, p.turns))
        .collect::<Vec<_>>();
    let driver = template(&m, control, &pins, rect([-22., -15.], [10., 15.5]))?;
    let mut bucks = Vec::new();
    for folded in [false, true] {
        let (l, dir) = if folded {
            ([-1., -25.0], 2)
        } else {
            ([18.5, -3.], 0)
        };
        let mut pins = Vec::new();
        for (id, delta, q) in [
            ("L1", [0., 0.], 0),
            ("C41", [10., -5.7], 0),
            ("C42", [10., 0.], 0),
            ("C43", [10., 5.7], 0),
            ("C44", [18., -4.75], 0),
            ("C45", [18., 4.75], 0),
        ] {
            let d = rotate(delta, dir);
            pins.push((id, [l[0] + d[0], l[1] + d[1]], (dir + q) % 4));
        }
        let mut buck = template(&m, output.clone(), &pins, rect([-29., -49.], [44., 18.]))?;
        buck.members.extend_from_slice(&driver.members);
        buck.poses.extend_from_slice(&driver.poses);
        bucks.push(buck);
    }
    let input = template(
        &m,
        named(&["J1", "F1", "D1", "D2"]),
        &[
            ("J1", [0., 0.], 1),
            ("F1", [0., 6.5], 0),
            ("D1", [8., 9.7], 1),
            ("D2", [0., 11.3], 0),
        ],
        rect([-4.5, -4.3], [12., 15.]),
    )?;
    let input_ids = &input.members;
    let hotswap = template(
        &m,
        scope("front")
            .into_iter()
            .filter(|i| !input_ids.contains(i))
            .collect(),
        &[
            ("Q1", [0., 0.], 0),
            ("R1", [-1., 8.], 0),
            ("C3", [0., -8.7], 0),
            ("U1", [-9.5, 5.5], 0),
        ],
        rect([-16., -17.], [15., 12.]),
    )?;
    let mut sensors = Vec::new();
    for n in 1..=3 {
        let t = format!("TH{n}");
        let r = format!("R{}", n + 10);
        let c = format!("C{}", n + 10);
        sensors.push(template(
            &m,
            named(&[&t, &r, &c]),
            &[(&t, [0., 0.], 0)],
            rect([-5., -4.], [5., 4.]),
        )?);
    }
    let mut channels = Vec::new();
    for n in 1..=6 {
        let j = format!("J{}", 10 + n);
        let u = format!("U{}", 10 + n);
        let d = format!("D{}", 10 + n);
        channels.push(template(
            &m,
            scope(&format!("block_ch{n}")),
            &[(&j, [0., 0.], 0), (&u, [0., -11.5], 0), (&d, [6.7, -2.], 0)],
            rect([-8.6, -20.], [8.6, 4.5]),
        )?);
    }
    let templates = [&mcu, &can, &ldo, &aux, &bucks[0], &input, &hotswap];
    let names: Vec<Vec<String>> = templates
        .iter()
        .copied()
        .chain(channels.iter())
        .chain(sensors.iter())
        .map(|t| t.members.iter().map(|&i| m.ids[i].clone()).collect())
        .collect();
    let groups = m.circuit_groups(true, &names)?;
    let mut shortlist = BTreeMap::new();
    let mut search = String::from("variant\tstatus\n");
    for size in [
        [92., 68.],
        [100., 72.],
        [100., 78.],
        [104., 76.],
        [108., 78.],
        [112., 78.],
        [112., 82.],
        [116., 76.],
        [116., 82.],
    ] {
        let [w, h] = size;
        for (fan, fan_name) in FANS.iter().enumerate() {
            for fold in 0..2 {
                for orient in 0..4 {
                    for layout in 0..4 {
                        for arrangement in 0..3 {
                            let v = Variant {
                                size,
                                fan,
                                buck: fold,
                                hand: 0,
                                aux: 0,
                                bank: 0,
                            };
                            let id = format!(
                                "{}-{}-{w}x{h}-r{orient}-m{layout}-a{arrangement}",
                                fan_name,
                                ["inline", "folded"][fold]
                            );
                            let mut seeds: Vec<_> = m.ids.iter().map(|s| v.seed(&b, s)).collect();
                            let keep = corners(size).map(|[x, y]| rect([x - 3., y - 3.], [x + 3., y + 3.]));
                            for (n, t) in channels.iter().enumerate() {
                                let (mut p, q) = v.fan_port(n);
                                if fan == 2 && n < 3 {
                                    p[0] = [16., w * 0.36, w * 0.56][n];
                                }
                                if fan == 0 {
                                    p[0] = 16. + (w - 32.) * n as f64 / 5.;
                                }
                                if fan == 2 && n >= 3 {
                                    p[1] = [17., h * 0.49, h - 17.][n - 3];
                                }
                                if fan == 3 {
                                    p[1] = [17., h * 0.5, h - 17.][n % 3];
                                }
                                if fan == 1 && n >= 3 {
                                    p[0] = [w * 0.32, w * 0.50, w * 0.68][n - 3];
                                }
                                t.at(&mut seeds, p, q, Some(named(&[&format!("J{}", 11 + n)])[0]));
                            }
                            let can_x = if fan >= 2 { w * 0.62 } else { w - 18. };
                            can.at(&mut seeds, [can_x, 6.], 0, Some(named(&["J2"])[0]));
                            if fan == 3 {
                                seeds[named(&["J1"])[0]].at = [w * 0.35, 6.];
                            }
                            let inlet = seeds[named(&["J1"])[0]].at;
                            input.at(&mut seeds, inlet, 0, Some(named(&["J1"])[0]));
                            hotswap.at(&mut seeds, [inlet[0] + 10., h * 0.30], (layout + 1) % 4, None);
                            for (n, t) in sensors.iter().enumerate() {
                                let at = seeds[named(&[&format!("TH{}", n + 1)])[0]].at;
                                t.at(&mut seeds, at, 0, None);
                            }
                            let (power, center) = match arrangement {
                                0 => (
                                    [0.35 * w, 0.52 * h],
                                    if layout % 2 == 0 {
                                        [0.64 * w, 0.52 * h]
                                    } else {
                                        [0.48 * w, 0.58 * h]
                                    },
                                ),
                                1 => ([0.26 * w, 0.36 * h], [0.66 * w, 0.65 * h]),
                                _ => ([0.52 * w, 0.29 * h], [0.44 * w, 0.63 * h]),
                            };
                            mcu.at(&mut seeds, center, layout, None);
                            ldo.at(&mut seeds, [0.63 * w, 0.29 * h], orient, None);
                            aux.at(&mut seeds, [0.43 * w, 0.23 * h], layout, None);
                            bucks[fold].at(&mut seeds, power, orient, None);
                            let allowed = vec![false; seeds.len()];
                            match m.pack_circuits(
                                &mut seeds,
                                &groups,
                                rect([0.9, 0.9], [w - 0.9, h - 0.9]),
                                &keep,
                                GAP,
                                Some(&allowed),
                            ) {
                                Err(e) => search += &format!("{id}\t{e}\n"),
                                Ok(()) => {
                                    let poses: Vec<_> = seeds.iter().map(|p| p.at).collect();
                                    let estimate = m.estimate(&poses, &seeds);
                                    let excess = handoff_excess(&handoffs(&b, &m, &seeds, &poses));
                                    let hot = path_length(&b, &m, &seeds, &poses, &HOT);
                                    let trial = Trial {
                                        v,
                                        seeds,
                                        poses,
                                        estimate,
                                        hot,
                                    };
                                    search += &format!("{id}\tpacked\n");
                                    let entry = shortlist
                                        .entry((w as u32, h as u32, fan, fold))
                                        .or_insert_with(|| None);
                                    if entry.as_ref().is_none_or(|(_, e, t): &(String, f64, Trial)| {
                                        (excess, estimate) < (*e, t.estimate)
                                    }) {
                                        *entry = Some((id, excess, trial));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        fs::write(out.join("search.tsv"), &search)?;
    }
    let mut results = String::from(
        "variant\twidth\theight\tscore\tsense_mm\tswitch_mm\tgate_mm\tblockers\tinput_shunt_mm\tprotected_input_mm\tfive_volt_mm\tlogic_supply_mm\tswitch_inductor_mm\tmax_handoff_excess\n",
    );
    let mut best = None;
    for (id, excess, t) in shortlist.into_values().flatten() {
        let text = generate(&src, &b, &m, &t);
        let report = pcb::query(&text, Some(out), None, &["floorplan".into()]).map_err(|e| e.message)?;
        let (score, sense, _, blockers) = numbers(&report)?;
        let [w, h] = t.v.size;
        let gates = path_length(&b, &m, &t.seeds, &t.poses, &GATES);
        let distances = handoffs(&b, &m, &t.seeds, &t.poses)
            .iter()
            .map(|v| format!("{v:.2}"))
            .collect::<Vec<_>>()
            .join("\t");
        results += &format!(
            "{id}\t{w}\t{h}\t{score:.2}\t{sense:.2}\t{:.2}\t{gates:.2}\t{blockers}\t{distances}\t{excess:.5}\n",
            t.hot
        );
        fs::write(out.join(format!("{id}.fg")), text)?;
        fs::write(out.join(format!("{id}.json")), report.to_string())?;
        if blockers == 0
            && excess <= 1e-8
            && best
                .as_ref()
                .is_none_or(|(a, c, _)| w * h < *a || (w * h == *a && score < *c))
        {
            best = Some((w * h, score, id));
        }
    }
    fs::write(out.join("finalists.tsv"), results)?;
    let (_, _, id) =
        best.ok_or("no placement passed circuit reservations, interstage handoff budgets and DRC")?;
    fs::write(out.join("winner.txt"), id + "\n")?;
    Ok(())
}
pub(super) fn run(input: &Path, out: &Path) -> Result<(), Error> {
    local_search(input, out)
}
