//! Replay from the preceding six-amp study. Write independent boards alongside copies
//! of their shared schematic/libraries in a supplied output directory.
#[path = "compact_fans/circuits.rs"]
mod circuits;
use forge_agent::json::Json;
use forge_lang::pcb::{
    self, Board,
    packing::{Model, Rect, Seed, XY, rotate},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
const HOT: [(&str, &str); 4] = [
    ("C31.1", "Q2.2"),
    ("Q2.3", "Q3.2"),
    ("Q3.3", "R20.2"),
    ("R20.1", "C31.2"),
];
const FANS: [&str; 4] = ["one-edge", "opposed", "corner", "side-banks"];
const BUCK: [&str; 3] = ["inline", "folded", "stacked"];

#[derive(Clone, Copy)]
struct Variant {
    size: XY,
    fan: usize,
    buck: usize,
    hand: u8,
    aux: u8,
    bank: u8,
}
impl Variant {
    fn id(self) -> String {
        format!(
            "{}-{}-{}x{}-{}{}{}",
            FANS[self.fan], BUCK[self.buck], self.size[0], self.size[1], self.hand, self.aux, self.bank
        )
    }
    fn fan_port(self, n: usize) -> (XY, u8) {
        let [w, h] = self.size;
        let k = n % 3;
        match self.fan {
            0 => ([13. + (w - 26.) * n as f64 / 5., h - 6.], 0),
            1 if n >= 3 => ([w * [0.32, 0.50, 0.67][k], 6.], 2),
            1 => ([w * [0.20, 0.49, 0.78][k], h - 6.], 0),
            2 if n >= 3 => ([w - 6., h * [0.23, 0.48, 0.73][k]], 3),
            2 => ([w * [0.16, 0.41, 0.66][k], h - 6.], 0),
            _ if n >= 3 => ([w - 6., h * [0.25, 0.52, 0.79][k]], 3),
            _ => ([6., h * [0.25, 0.52, 0.79][k]], 1),
        }
    }
    fn seed(self, b: &Board, id: &str) -> Seed {
        let [w, h] = self.size;
        let p = &b.layout.placements[id];
        let old = p.pose.at.map(|x| x as f64 / 1e6);
        let occurrence = p.occurrence.as_ref().unwrap();
        let group = occurrence.rsplit_once('/').unwrap().0;
        let pose = |at, turns, fixed| Seed { at, turns, fixed };
        let relative = |origin: XY, target: XY, scale: XY, q: u8| {
            let d = rotate(
                [(old[0] - origin[0]) * scale[0], (old[1] - origin[1]) * scale[1]],
                q,
            );
            pose([target[0] + d[0], target[1] + d[1]], q, false)
        };
        if let Some(channel) = group.strip_prefix("front/block_ch") {
            let n = channel.parse::<usize>().unwrap() - 1;
            let (port, q) = self.fan_port(n);
            if id.starts_with('J') {
                return pose(port, q, true);
            }
            let hand = if (n as u8 + self.hand).is_multiple_of(2) {
                1.
            } else {
                -1.
            };
            return relative([22. + 27. * n as f64, 99.], port, [0.57 * hand, 0.77], q);
        }
        let input = [12.5, 6.];
        let can = [w - 18., 6.];
        if id == "J1" {
            return pose(input, 1, true);
        }
        if id == "J2" {
            return pose(can, 0, true);
        }
        let power = if self.fan == 3 {
            [w * 0.42, h * 0.53]
        } else {
            [w * 0.34, h * 0.50]
        };
        let mcu = if self.fan == 3 {
            [w * 0.62, h * 0.62]
        } else {
            [w * 0.65, h * 0.53]
        };
        let offset = |d: XY| [power[0] + d[0], power[1] + d[1]];
        let sw = self.buck;
        if group == "front" {
            // Separate connector/fuse, pass FET and supervisor/divider neighbourhoods.
            return match id {
                "F1" | "D1" | "D2" => relative([18., 24.], [w * 0.17, 13.], [0.70, 0.68], self.aux),
                "Q1" | "R1" | "C3" | "D4" => {
                    relative([40., 22.], [w * 0.25, h * 0.28], [0.62, 0.62], self.hand)
                }
                _ => relative([27., 13.], [w * 0.32, h * 0.19], [0.67, 0.67], 0),
            };
        }
        if group.ends_with("block_main") {
            // Independently turn the switch pair, driver group, input ceramics and output bank.
            let q = [0, 2, 1][sw];
            return match id {
                "Q2" => pose(offset([0., 3.]), q, false),
                "Q3" => pose(
                    offset([[0., -8.], [1., -8.], [12., 2.]][sw]),
                    [2, 0, 3][sw],
                    false,
                ),
                "R20" => pose(offset([[-6., -9.], [6., -9.], [13., -5.]][sw]), q, false),
                "L1" => pose(
                    offset([[17., 0.], [-12., -4.], [8., 12.]][sw]),
                    [0, 2, 1][sw],
                    false,
                ),
                "C31" | "C32" | "C33" | "C34" | "C35" | "C36" | "C47" | "C48" => relative(
                    [44., 65.],
                    offset([[0., 11.], [7., 4.], [7., -8.]][sw]),
                    [0.54, 0.60],
                    q,
                ),
                "C41" | "C42" | "C43" | "C44" | "C45" => relative(
                    [70., 54.],
                    offset(if self.bank == 0 {
                        [[25., 3.], [-13., 7.], [20., 11.]][sw]
                    } else {
                        [[17., -11.], [-22., -7.], [-1., 18.]][sw]
                    }),
                    [0.64, 0.74],
                    ([1, 2, 3][sw] + self.bank) % 4,
                ),
                _ => relative(
                    [26., 60.],
                    offset([[-11., 8.], [11., 1.], [0., -8.]][sw]),
                    [0.68, 0.7],
                    q,
                ),
            };
        }
        if group.ends_with("block_aux") {
            return match id {
                "U102" | "C108" | "C109" => {
                    relative([95., 23.], [mcu[0] + 3., mcu[1] - 9.], [0.74, 0.75], self.aux)
                }
                "L101" | "C106" | "C107" => {
                    relative([80., 21.5], [w * 0.51, h * 0.25], [0.62, 0.7], self.aux)
                }
                _ => relative([70., 17.], [w * 0.46, h * 0.19], [0.71, 0.7], self.aux),
            };
        }
        if group.ends_with("block_logic") {
            return match id {
                "D5" | "JP1" | "R8" | "U3" | "C10" => relative([147., 7.], can, [0.62, 0.72], 0),
                "J3" => pose([mcu[0] - 3., mcu[1] + 10.], self.hand, false),
                _ => relative([101., 49.], mcu, [0.70, 0.7], self.hand),
            };
        }
        let targets = [offset([-5., 10.]), [w * 0.55, h * 0.74], [w * 0.88, h * 0.15]];
        let n = match id {
            "TH1" | "R11" | "C11" => 0,
            "TH2" | "R12" | "C12" => 1,
            _ => 2,
        };
        relative(
            [[32., 70.], [117., 70.], [161., 36.]][n],
            targets[n],
            [0.77, 0.77],
            0,
        )
    }
}
struct Trial {
    v: Variant,
    seeds: Vec<Seed>,
    poses: Vec<XY>,
    estimate: f64,
    hot: f64,
}
fn generate(src: &str, b: &Board, m: &Model, t: &Trial) -> String {
    let [w, h] = t.v.size;
    let mut result = String::new();
    for line in src.lines() {
        let id = line.split('=').next().unwrap_or("").trim();
        let rewritten = if let Ok(i) = m.ids.binary_search_by(|s| s.as_str().cmp(id)) {
            let p = &b.layout.placements[id];
            let angle = p.pose.angle + 90. * f64::from(t.seeds[i].turns);
            let begin = line.find("at:").unwrap();
            let x = line[begin..].find('(').unwrap() + begin;
            let end = line[x..].find(')').unwrap() + x + 1;
            // All placements in this particular input use the default angle.
            assert!(!line.contains("angle:"));
            format!(
                "{}({:.3},{:.3}), angle:{}deg{}",
                &line[..x],
                t.poses[i][0],
                t.poses[i][1],
                angle,
                &line[end..]
            )
        } else if id == "outline" {
            format!(
                "outline = edge(points:[(2,0),({},0),({},2),({},{}),({},{}),(2,{}),(0,{}),(0,2)])",
                w - 2.,
                w,
                w,
                h - 2.,
                w - 2.,
                h,
                h,
                h - 2.
            )
        } else if id == "ground" {
            format!(
                "ground = zone(net:GND, layer:gnd, clearance:0.36mm, points:[(2,2),({},2),({},{}),(2,{})])",
                w - 2.,
                w - 2.,
                h - 2.,
                h - 2.
            )
        } else if let Some(n) = id.strip_prefix("mount").and_then(|v| v.parse::<usize>().ok()) {
            let [x, y] = corners(t.v.size)[n];
            format!("{id} = npth(at:({x},{y}), drill:3.2mm)")
        } else {
            line.into()
        };
        result.push_str(&rewritten);
        result.push('\n');
    }
    result
}
fn corners([w, h]: XY) -> [XY; 4] {
    [[3.6, 3.6], [w - 3.6, 3.6], [3.6, h - 3.6], [w - 3.6, h - 3.6]]
}
fn path_length(b: &Board, m: &Model, seeds: &[Seed], poses: &[XY], pairs: &[(&str, &str)]) -> f64 {
    let pad = |k: &str| {
        let p = &b.layout.pads[k];
        let i = m.ids.binary_search(&p.placement).unwrap();
        let center = b.layout.placements[&p.placement].pose.at;
        let offset = rotate(
            std::array::from_fn(|k| (p.at[k] - center[k]) as f64 / 1e6),
            seeds[i].turns,
        );
        [poses[i][0] + offset[0], poses[i][1] + offset[1]]
    };
    pairs
        .iter()
        .map(|(a, z)| {
            let a = pad(a);
            let z = pad(z);
            (a[0] - z[0]).abs() + (a[1] - z[1]).abs()
        })
        .sum()
}
const GATES: [(&str, &str); 2] = [("U4.18", "Q2.1"), ("U4.13", "Q3.1")];
const BYPASS: [(&str, &str); 4] = [
    ("C4.1", "U2.6"),
    ("C4.2", "U2.7"),
    ("C10.1", "U3.3"),
    ("C10.2", "U3.2"),
];

/// Read back only the literal poses emitted by generate(). The exact reviewer
/// below independently recompiles the entire .fg including its imported files.
fn read_poses(path: &Path, model: &Model) -> Result<Vec<Seed>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let mut poses = BTreeMap::new();
    for line in text.lines().filter(|s| s.contains("= place(")) {
        let id = line.split_once('=').ok_or("missing id")?.0.trim();
        let at = line
            .split_once("at:")
            .ok_or("missing position")?
            .1
            .trim()
            .strip_prefix('(')
            .ok_or("expected point")?;
        let (x, y) = at
            .split_once(')')
            .ok_or("expected point")?
            .0
            .split_once(',')
            .ok_or("expected coordinates")?;
        let angle = line
            .split_once("angle:")
            .ok_or("missing angle")?
            .1
            .trim()
            .split_once("deg")
            .ok_or("expected degrees")?
            .0
            .parse::<u16>()?;
        poses.insert(
            id,
            Seed {
                at: [x.parse()?, y.parse()?],
                turns: ((angle / 90) % 4) as u8,
                fixed: matches!(id, "J1" | "J2" | "J11" | "J12" | "J13" | "J14" | "J15" | "J16"),
            },
        );
    }
    model
        .ids
        .iter()
        .map(|k| {
            poses
                .get(k.as_str())
                .copied()
                .ok_or_else(|| format!("missing {k}").into())
        })
        .collect()
}
fn finish(input: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(input)?;
    let base = pcb::compile(&src, input.parent(), None).map_err(|e| e.message)?;
    let mut model = Model::new(&base)?;
    for (pairs, w) in [(&HOT[..], 12.), (&GATES[..], 24.), (&BYPASS[..], 18.)] {
        for &(a, z) in pairs {
            model.add_guide(&base, a, z, w)?;
        }
    }
    for n in 11..=16 {
        for (a, z, w) in [
            (format!("D{n}.1"), format!("J{n}.3"), 5.),
            (format!("D{n}.2"), format!("J{n}.4"), 5.),
            (format!("U{n}.18"), format!("J{n}.2"), 6.),
        ] {
            model.add_guide(&base, &a, &z, w)?;
        }
    }
    let baseline: Vec<_> = model
        .ids
        .iter()
        .map(|id| Seed {
            at: base.layout.placements[id].pose.at.map(|v| v as f64 / 1e6),
            turns: 0,
            fixed: false,
        })
        .collect();
    let at: Vec<_> = baseline.iter().map(|p| p.at).collect();
    let prior_hot = path_length(&base, &model, &baseline, &at, &HOT);
    let prior_gate = path_length(&base, &model, &baseline, &at, &GATES);
    let prior_bypass = path_length(&base, &model, &baseline, &at, &BYPASS);
    let baseline_report =
        pcb::query(&src, input.parent(), None, &["floorplan".into()]).map_err(|e| e.message)?;
    let (prior_score, prior_sense, _, _) = numbers(&baseline_report)?;
    let coarse = fs::read_to_string(out.join("finalists.tsv"))?;
    let mut finalists = BTreeMap::new();
    for line in coarse.lines().skip(1) {
        let f: Vec<_> = line.split('\t').collect();
        if f.len() != 9 || f[8] != "0" {
            continue;
        }
        let id = f[0];
        let size: XY = [f[1].parse()?, f[2].parse()?];
        let fan = FANS
            .iter()
            .position(|s| id.starts_with(s))
            .ok_or("unknown family")?;
        let buck = BUCK
            .iter()
            .position(|s| id.contains(&format!("-{s}-")))
            .ok_or("unknown switch cell")?;
        let v = Variant {
            size,
            fan,
            buck,
            hand: 0,
            aux: 0,
            bank: 0,
        };
        let mut seeds = read_poses(&out.join(format!("{id}.fg")), &model)?;
        let allow: Vec<_> = seeds.iter().map(|s| !s.fixed).collect();
        let keep = corners(size).map(|p| Rect {
            lo: [p[0] - 3., p[1] - 3.],
            hi: [p[0] + 3., p[1] + 3.],
        });
        model.polish(
            &mut seeds,
            Rect {
                lo: [0.9, 0.9],
                hi: [size[0] - 0.9, size[1] - 0.9],
            },
            &keep,
            0.55,
            &allow,
        )?;
        let poses: Vec<_> = seeds.iter().map(|s| s.at).collect();
        let hot = path_length(&base, &model, &seeds, &poses, &HOT);
        let gates = path_length(&base, &model, &seeds, &poses, &GATES);
        let bypass = path_length(&base, &model, &seeds, &poses, &BYPASS);
        let estimate = model.estimate(&poses, &seeds);
        let trial = Trial {
            v,
            seeds,
            poses,
            estimate,
            hot,
        };
        // Retain one electrically competitive version of every harness family on every outline.
        let key = (size[0] as u32, fan);
        let key_cost = estimate + 20. * (gates - prior_gate).max(0.) + 20. * (bypass - prior_bypass).max(0.);
        if finalists
            .get(&key)
            .is_none_or(|(cost, _, _): &(f64, String, Trial)| key_cost < *cost)
        {
            finalists.insert(key, (key_cost, format!("{id}-refined"), trial));
        }
    }
    let mut results = String::from(
        "variant\twidth\theight\tarea\tscore\tsense_mm\tworst_sense_mm\thot_loop_mm\tblockers\n",
    );
    let mut winner: Option<((bool, u32), f64, String)> = None;
    for (_, name, t) in finalists.into_values() {
        let text = generate(&src, &base, &model, &t);
        let j = pcb::query(&text, Some(out), None, &["floorplan".into()]).map_err(|e| e.message)?;
        let (score, sense, worst, blocked) = numbers(&j)?;
        let [w, h] = t.v.size;
        let gates = path_length(&base, &model, &t.seeds, &t.poses, &GATES);
        let bypass = path_length(&base, &model, &t.seeds, &t.poses, &BYPASS);
        results += &format!(
            "{name}\t{w}\t{h}\t{}\t{score:.3}\t{sense:.3}\t{worst:.3}\t{:.3}\t{blocked}\n",
            w * h - 8.,
            t.hot
        );
        fs::write(out.join(format!("{name}.fg")), &text)?;
        fs::write(out.join(format!("{name}.json")), j.to_string())?;
        if blocked == 0 && score < prior_score && sense < prior_sense && t.hot < prior_hot {
            let key = (gates > prior_gate || bypass > prior_bypass, (w * h) as u32);
            let cost = score + 9. * t.hot;
            if winner
                .as_ref()
                .is_none_or(|(k, v, _)| key < *k || (key == *k && cost < *v))
            {
                winner = Some((key, cost, name));
            }
        }
        fs::write(out.join("refined.tsv"), &results)?;
    }
    let (_, _, winner) = winner.ok_or("no refined board beats the previous area-study electrical targets")?;
    fs::write(out.join("winner.txt"), format!("{winner}\n"))?;
    Ok(())
}
fn numbers(j: &Json) -> Result<(f64, f64, f64, usize), Box<dyn std::error::Error>> {
    let r = j.get("result").ok_or("missing result")?;
    let Some(Json::Arr(rows)) = r.get("candidates") else {
        return Err("missing candidates".into());
    };
    let row = &rows[0];
    let Some(Json::Arr(blocked)) = row.get("blockers") else {
        return Err("missing geometry checks".into());
    };
    let Some(Json::Arr(terms)) = row.get("terms") else {
        return Err("missing terms".into());
    };
    let sense: Vec<_> = terms
        .iter()
        .filter(|t| {
            t.get("id")
                .and_then(Json::str)
                .is_some_and(|s| s.contains("/load/"))
        })
        .filter_map(|t| t.get("evidence")?.get("length_mm")?.num())
        .collect();
    if sense.len() != 6 {
        return Err("expected six sense guides".into());
    }
    Ok((
        row.get("score").and_then(Json::num).ok_or("missing score")?,
        sense.iter().sum(),
        sense.into_iter().fold(0., f64::max),
        blocked.len(),
    ))
}
fn run(input: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dir = input.parent().unwrap();
    let src = fs::read_to_string(input)?;
    if out.exists() && fs::read_dir(out)?.next().is_some() {
        return Err("choose an empty output directory".into());
    }
    fs::create_dir_all(out)?;
    for name in ["symbols.fg", "footprints.fg", "six-amp-schematic.fg"] {
        fs::copy(dir.join(name), out.join(name))?;
    }
    let b = pcb::compile(&src, Some(dir), None).map_err(|e| e.message)?;
    let mut model = Model::new(&b)?;
    for (a, z) in HOT {
        model.add_guide(&b, a, z, 9.)?
    }
    for (a, z) in [
        ("U4.13", "Q3.1"),
        ("U4.18", "Q2.1"),
        ("Q3.2", "L1.1"),
        ("U2.6", "C4.1"),
        ("U3.3", "C10.1"),
    ] {
        model.add_guide(&b, a, z, 5.)?
    }
    let mut best = BTreeMap::new();
    let mut log = String::from("variant\tarea_mm2\tproposal_cost\thot_loop_mm\tstatus\n");
    for (wi, sz) in [[86., 64.], [92., 68.], [100., 72.]].into_iter().enumerate() {
        let [w, h] = sz;
        let area = w * h - 8.;
        let keep = corners(sz).map(|p| Rect {
            lo: [p[0] - 3., p[1] - 3.],
            hi: [p[0] + 3., p[1] + 3.],
        });
        for fan in 0..FANS.len() {
            for buck in 0..BUCK.len() {
                for hand in 0..2 {
                    for aux in 0..2 {
                        for bank in 0..2 {
                            let v = Variant {
                                size: sz,
                                fan,
                                buck,
                                hand,
                                aux,
                                bank,
                            };
                            let seeds: Vec<_> = model.ids.iter().map(|id| v.seed(&b, id)).collect();
                            match model.pack(
                                &seeds,
                                Rect {
                                    lo: [0.9, 0.9],
                                    hi: [w - 0.9, h - 0.9],
                                },
                                &keep,
                                0.55,
                            ) {
                                Ok(poses) => {
                                    let estimate = model.estimate(&poses, &seeds);
                                    let hot = path_length(&b, &model, &seeds, &poses, &HOT);
                                    log += &format!("{}\t{area}\t{estimate:.3}\t{hot:.3}\tpacked\n", v.id());
                                    let key = (wi, fan, buck);
                                    let trial = Trial {
                                        v,
                                        seeds,
                                        poses,
                                        estimate,
                                        hot,
                                    };
                                    if best.get(&key).is_none_or(|t: &Trial| estimate < t.estimate) {
                                        best.insert(key, trial);
                                    }
                                }
                                Err(e) => log += &format!("{}\t{area}\t\t\t{e}\n", v.id()),
                            }
                        }
                    }
                }
            }
        }
    }
    fs::write(out.join("search.tsv"), log)?;
    let mut results = String::from(
        "variant\twidth\theight\tarea\tscore\tsense_mm\tworst_sense_mm\thot_loop_mm\tblockers\n",
    );
    for t in best.into_values() {
        let name = t.v.id();
        let text = generate(&src, &b, &model, &t);
        let [w, h] = t.v.size;
        match pcb::query(&text, Some(out), None, &["floorplan".into()]) {
            Ok(j) => {
                let (score, sense, worst, blocked) = numbers(&j)?;
                results += &format!(
                    "{name}\t{w}\t{h}\t{}\t{score:.3}\t{sense:.3}\t{worst:.3}\t{:.3}\t{blocked}\n",
                    w * h - 8.,
                    t.hot
                );
                fs::write(out.join(format!("{name}.fg")), text)?;
                fs::write(out.join(format!("{name}.json")), j.to_string())?;
            }
            Err(e) => {
                results += &format!(
                    "{name}\t{w}\t{h}\t{}\t\t\t\t{:.3}\t{}\n",
                    w * h - 8.,
                    t.hot,
                    e.message
                )
            }
        }
        fs::write(out.join("finalists.tsv"), &results)?;
    }
    finish(input, out)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if args.len() == 3 && args[0] == Path::new("--finish") {
        return finish(&args[1], &args[2]);
    }
    if args.len() == 3 && args[0] == Path::new("--legacy") {
        return run(&args[1], &args[2]);
    }
    if args.len() != 2 {
        return Err("usage: compact_fans INPUT.fg OUTPUT_DIRECTORY".into());
    }
    circuits::run(&args[0], &args[1])
}
