//! The simulation's tools drawn as what they are, for display only: the cutter's fluted length
//! from its record (`nof` flutes at the record's `helix`, straight when it gives none; a tap's
//! thread at its pitch; the tip's shape as the tool's own profile has it), then its neck and
//! shank, then the holder's turned profile. The flutes are the Library Builder's
//! (`library_mesh::fluted`), the rest `machine_mesh::revolve`: one mesh code for both.
//!
//! The certified checks never see any of this: they keep the tool's revolved profiles
//! (`forge_cam_geom::tool::Shape`). Each tool is meshed once, in its own frame (tip at the
//! origin, axis +z); a frame poses it and turns it about its axis.

use super::draw::{look, profile, shade_pub};
use crate::machine_mesh::Tris;
use crate::origin::WorldVert;
use forge_cam::ops::ToolUse;
use forge_cam_geom::tool::Part;
use forge_tooling::Kind;

/// One part of a tool and its colour.
pub struct Piece {
    pub tris: Tris,
    pub colour: [f32; 3],
    /// Turns with the spindle (the cutter, neck and shank; not the holder's nut, which turns
    /// too but shows nothing turning).
    pub spins: bool,
    /// Per vertex, a factor on the colour: the flutes' valleys dark, their lands light, so a
    /// fluted cutter reads as one at a glance (empty: 1 everywhere).
    pub shade: Vec<f32>,
}

/// A tool as drawn.
pub struct ToolLook {
    pub pieces: Vec<Piece>,
    /// Triangles in all (what a frame transforms).
    pub tris: usize,
    /// No holder in the document: its place is said, not drawn as a guess.
    pub no_holder: bool,
    /// What the drawing leaves out, said at the tool's top (above the holder's gauge line): the
    /// interface the record names (its taper and flange are in no record of the library yet), or
    /// that the document gives no holder.
    /// Where it points: (radius, height) from the tip, on the holder's lower outer edge (or the
    /// shank's top).
    pub note: Option<([f64; 2], String)>,
    /// The tool and its holder in people's words, the note's label beside the tool ("Ø10 flat
    /// end mill · BT40 holder, ER32 nut"); the note itself (record keys, sources) is its hover.
    pub label: String,
    /// The drawing's widest radius (the holder's flange, the nut): the label stands clear of it.
    pub reach: f64,
    /// A generic holder interface (taper and flange) above the gauge line, drawn see-through and
    /// labelled "generic shape": no record of the library holds the interface's dimensions.
    pub ghost: Option<Tris>,
}

/// The colour a tool's cutting part shows: its coating's, else its material's.
fn cutting_colour(tu: &ToolUse) -> [f32; 3] {
    let c = tu.tool.coating.to_ascii_lowercase();
    let m = tu.tool.material.to_ascii_lowercase();
    if c == "tin" {
        look::COAT_TIN
    } else if c.contains("altin") || c.contains("tialn") || c.contains("alcrn") || c.contains("ticn") {
        look::COAT_ALTIN
    } else if m.contains("hss") || m.contains("cobalt") || m.contains("pm") {
        look::HSS
    } else {
        look::CUTTER
    }
}

/// The colour of a tool's shank and neck: carbide or tool steel.
fn shank_colour(tu: &ToolUse) -> [f32; 3] {
    let m = tu.tool.material.to_ascii_lowercase();
    if m.contains("hss") || m.contains("cobalt") || m.contains("pm") {
        look::HSS
    } else {
        look::SHANK
    }
}

/// A (ρ, z) profile as the builder's outline (z, r): its outer side, z rising, the largest radius
/// at each height.
fn outline(prof: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = prof.iter().map(|q| [q[1], q[0].max(0.0)]).collect();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(b[1].total_cmp(&a[1])));
    let mut out: Vec<[f64; 2]> = vec![];
    for q in pts {
        match out.last_mut() {
            Some(l) if (q[0] - l[0]).abs() < 1e-9 => l[1] = l[1].max(q[1]),
            _ => out.push(q),
        }
    }
    // the tip on the axis (a flat end's face is the fan `fluted` draws)
    if out.first().is_some_and(|q| q[0] > 1e-9 || q[1] > 1e-9) && out[0][0] <= 1e-9 {
        out.insert(0, [0.0, 0.0]);
    }
    out
}

/// A tap's cutting length: the outline with its thread cut in, a 60° form `pitch` apart (the
/// root 0.6134 P below the crest), sampled finely enough for the fluted mesh to show it.
fn threaded(o: &[[f64; 2]], lcf: f64, pitch: f64) -> Vec<[f64; 2]> {
    let depth = 0.6134 * pitch;
    let r_at = |z: f64| {
        let mut r = 0.0;
        for w in o.windows(2) {
            if (w[0][0]..=w[1][0]).contains(&z) && w[1][0] > w[0][0] {
                r = w[0][1] + (w[1][1] - w[0][1]) * (z - w[0][0]) / (w[1][0] - w[0][0]);
                break;
            }
        }
        r
    };
    let n = ((lcf / pitch) * 8.0).ceil().clamp(8.0, 512.0) as usize;
    let mut out: Vec<[f64; 2]> = (0..=n)
        .map(|i| {
            let z = lcf * i as f64 / n as f64;
            let u = (z / pitch).rem_euclid(1.0);
            let tri = 1.0 - (2.0 * u - 1.0).abs();
            [z, (r_at(z) - depth * (1.0 - tri)).max(0.0)]
        })
        .collect();
    out.extend(o.iter().copied().filter(|q| q[0] > lcf));
    out
}

/// A tool's look, meshed once: `helix` the record's (None: straight flutes, as the builder
/// draws a record that gives none).
pub fn build(tu: &ToolUse, helix: Option<f64>) -> ToolLook {
    let bodies = &tu.shape.bodies;
    let holder_at = (tu.holder.is_some() && bodies.len() > 2).then(|| bodies.len() - 1);
    let mut pieces = vec![];
    for (bi, b) in bodies.iter().enumerate() {
        let o = outline(&profile(b));
        if o.len() < 2 {
            continue;
        }
        if Some(bi) == holder_at {
            pieces.push(Piece {
                tris: crate::machine_mesh::revolve(&o),
                colour: look::HOLDER,
                spins: false,
                shade: vec![],
            });
            continue;
        }
        let z0 = o[0][0];
        let local: Vec<[f64; 2]> = o.iter().map(|q| [q[0] - z0, q[1]]).collect();
        let lift = |mut t: Tris| {
            for p in &mut t.pos {
                p[2] += z0;
            }
            for e in &mut t.edges {
                for p in e {
                    p[2] += z0;
                }
            }
            t
        };
        if b.part == Part::Cutter && tu.tool.nof > 0 && !tu.probe {
            let top = local.last().map_or(0.0, |q| q[0]);
            let lcf = if tu.tool.lcf > 0.0 {
                tu.tool.lcf.min(top)
            } else {
                top
            };
            let (cut, h) = match tu.tool.kind {
                Kind::Tap if tu.tool.tp > 0.0 => (threaded(&local, lcf, tu.tool.tp), 0.0),
                // a reamer's and a drill's flutes as the record gives them; a tap's straight
                Kind::Tap => (local.clone(), 0.0),
                _ => (
                    local.clone(),
                    helix.unwrap_or_else(|| typical_helix(tu.tool.kind)),
                ),
            };
            let (tris, shade) = gashed(
                lift(crate::library_mesh::fluted(&cut, lcf, tu.tool.nof, h)),
                &cut,
                z0,
            );
            pieces.push(Piece {
                tris,
                colour: cutting_colour(tu),
                spins: true,
                shade,
            });
            // the cutter body above its flutes (to its top): plain
            let rest: Vec<[f64; 2]> = std::iter::once([lcf, 0.0])
                .chain(local.iter().copied().filter(|q| q[0] >= lcf - 1e-9))
                .collect();
            if rest.len() > 2 {
                pieces.push(Piece {
                    tris: lift(crate::machine_mesh::revolve(&rest)),
                    colour: shank_colour(tu),
                    spins: true,
                    shade: vec![],
                });
            }
        } else {
            pieces.push(Piece {
                tris: lift(crate::machine_mesh::revolve(&local)),
                colour: if b.part == Part::Cutter {
                    cutting_colour(tu)
                } else {
                    shank_colour(tu)
                },
                spins: true,
                shade: vec![],
            });
        }
    }
    // where the shank meets the holder (in view in a close-up), else the shank's top
    let shank_r = 0.5 * tu.tool.dcon.max(tu.tool.dc);
    let at = match holder_at.and_then(|k| bodies.get(k)) {
        Some(h) => [shank_r, h.z0],
        None => bodies.last().map_or([0.0; 2], |b| [b.rmax, b.z1]),
    };
    let top = at;
    let note = match &tu.holder {
        None => Some((top, "No holder given: the tool's shank only".to_string())),
        Some(id) => {
            static LIB: std::sync::OnceLock<forge_tooling::Library> = std::sync::OnceLock::new();
            let lib = LIB.get_or_init(forge_tooling::Library::builtin);
            let iface = lib
                .holder(id)
                .ok()
                .map(|h| h.interface.clone())
                .filter(|i| !i.is_empty());
            Some((
                top,
                match iface {
                    Some(i) => format!("{id}: {i} taper and flange a generic shape (not in the library)"),
                    None => format!("{id}: its interface is not in the library"),
                },
            ))
        }
    };
    // a helix the record does not give: drawn at the kind's usual one, and said
    let note = match (
        helix,
        tu.tool.nof > 0 && !tu.probe && !matches!(tu.tool.kind, Kind::Tap | Kind::Reamer),
    ) {
        (None, true) => note.map(|(a, t)| {
            (
                a,
                format!(
                    "{t}\n{}: helix not in its record, drawn at {:.0}°",
                    tu.tool.id,
                    typical_helix(tu.tool.kind)
                ),
            )
        }),
        _ => note,
    };
    let mut note = note;
    // the holder's interface, from the cited table: a solid where it has one
    let ghost = None;
    if let Some((body, said)) = holder_at
        .and_then(|k| bodies.get(k))
        .zip(tu.holder.as_deref())
        .and_then(|(b, id)| interface_body(id, b.z1))
    {
        pieces.push(Piece {
            tris: body,
            colour: look::HOLDER,
            spins: false,
            shade: vec![],
        });
        note = note.map(|(a, t)| {
            let rest: Vec<&str> = t.lines().skip(1).collect();
            (
                a,
                std::iter::once(said.as_str())
                    .chain(rest)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        });
    }
    // the holder's collet nut, from its record: the holder's own, else its clamp system's largest
    if let Some((h, nut)) = holder_at
        .and_then(|k| bodies.get(k))
        .zip(tu.holder.as_deref())
        .and_then(|(b, id)| {
            let n = collet_nut(id)?;
            Some((b, n))
        })
    {
        let (r, z0, z1) = (0.5 * nut.0, h.z0, h.z0 + nut.1);
        pieces.push(Piece {
            tris: crate::machine_mesh::revolve(&[
                [z0, 0.0],
                [z0, r - 0.6],
                [z0 + 0.6, r],
                [z1, r],
                [z1, 0.0],
            ]),
            colour: look::NUT,
            spins: true,
            shade: vec![],
        });
        note = note.map(|(a, t)| {
            let mut lines: Vec<String> = t.lines().map(str::to_string).collect();
            if let Some(l) = lines.first_mut() {
                *l = format!("{l}; nut {}", nut.2);
            }
            (a, lines.join("\n"))
        });
    }
    let tris = pieces.iter().map(|p| p.tris.tri.len()).sum();
    let label = format!("{} \u{b7} {}", tool_words(tu), holder_words(tu.holder.as_deref()));
    let reach = pieces
        .iter()
        .flat_map(|p| p.tris.pos.iter())
        .map(|q| (q[0] * q[0] + q[1] * q[1]).sqrt())
        .fold(0.0, f64::max);
    ToolLook {
        pieces,
        tris,
        no_holder: tu.holder.is_none(),
        note,
        label,
        reach,
        ghost,
    }
}

/// A tool in people's words: its diameter and kind ("Ø10 flat end mill", "Ø6.8 drill").
pub fn tool_words(tu: &ToolUse) -> String {
    let kind = match tu.tool.kind {
        _ if tu.probe => "probe",
        Kind::Flat => "flat end mill",
        Kind::Ball => "ball end mill",
        Kind::Bull => "bull-nose end mill",
        Kind::Chamfer => "chamfer mill",
        Kind::Spot => "spot drill",
        Kind::Drill => "drill",
        Kind::Tap => "tap",
        Kind::ThreadMill => "thread mill",
        Kind::Reamer => "reamer",
        Kind::Bore => "boring bar",
        Kind::Counterbore => "counterbore",
        Kind::Countersink => "countersink",
        Kind::Slot => "slitting saw",
        Kind::Engrave => "engraver",
    };
    let d = format!("{:.2}", tu.tool.dc);
    let d = d.trim_end_matches('0').trim_end_matches('.');
    format!("\u{d8}{d} {kind}")
}

/// A holder in people's words: its interface and collet nut ("BT40 holder, ER32 nut"), from the
/// library's records; the record key is the note's.
pub fn holder_words(holder: Option<&str>) -> String {
    let Some(id) = holder else {
        return "no holder given".into();
    };
    static LIB: std::sync::OnceLock<forge_tooling::Library> = std::sync::OnceLock::new();
    let lib = LIB.get_or_init(forge_tooling::Library::builtin);
    let iface = lib
        .holder(id)
        .ok()
        .map(|h| h.interface.clone())
        .unwrap_or_default();
    let holder = match iface.as_str() {
        "" => "holder".to_string(),
        "straight" => "straight-shank holder".to_string(),
        i => format!("{i} holder"),
    };
    match collet_nut(id) {
        Some((_, _, nut)) => {
            let system = nut.split(" from ").next().unwrap_or(&nut).to_string();
            format!("{holder}, {system} nut")
        }
        None => holder,
    }
}

/// The records' helix angles (degrees) by tool id, from the tool library the document reads
/// (built in, the user's), read once: a record that gives none is absent.
pub fn helices() -> std::collections::HashMap<String, f64> {
    let user = forge_tooling_builder::user_cam();
    let (entries, _) = forge_tooling_builder::dynamic::catalog("tool", None, user.as_deref());
    entries
        .iter()
        .filter_map(|e| {
            let h = forge_tooling::toml::num(&e.def.table, "helix")?;
            (h > 0.0 && h < 80.0).then(|| (e.def.id.clone(), h))
        })
        .collect()
}

/// Draws a tool with its tip at `p`, turned `spin` radians about its axis; red where it is in a
/// failed check now.
pub fn draw(t: &ToolLook, p: [f64; 3], spin: f64, fail: bool, out: &mut Vec<WorldVert>) {
    let (s, c) = forge_math::dmath::sin_cos(spin);
    out.reserve(t.tris * 3);
    for pc in &t.pieces {
        let (s, c) = if pc.spins { (s, c) } else { (0.0, 1.0) };
        let base = if fail { look::FAIL } else { pc.colour };
        let tr = &pc.tris;
        for tri in &tr.tri {
            for &k in tri {
                let (q, n) = (tr.pos[k as usize], tr.nrm[k as usize]);
                let f = pc.shade.get(k as usize).copied().unwrap_or(1.0);
                let base = [base[0] * f, base[1] * f, base[2] * f];
                let at = [
                    (p[0] + q[0] * c - q[1] * s) as f32,
                    (p[1] + q[0] * s + q[1] * c) as f32,
                    (p[2] + q[2]) as f32,
                ];
                let nn = [
                    (n[0] * c - n[1] * s) as f32,
                    (n[0] * s + n[1] * c) as f32,
                    n[2] as f32,
                ];
                out.push(WorldVert {
                    at,
                    colour: shade_pub(base, nn),
                });
            }
        }
    }
}

/// How fast the drawn flutes turn, rev/s, for a spindle at `rpm` and the playback at `speed`
/// (sim seconds per second): slow enough to read as turning, never faster than 3 rev/s (a real
/// spindle's 200 rev/s would only flicker), still when paused.
pub fn turns_per_s(rpm: f64, speed: f64, playing: bool) -> f64 {
    if !playing || rpm <= 0.0 {
        return 0.0;
    }
    (rpm / 60.0 * speed / 400.0).clamp(0.4, 3.0)
}

/// The cutting edges (each flute's lip, the builder's `Tris::edges`) as thin dark lines over the
/// flutes, so a fluted cutter reads as one at any size; they turn with it.
pub fn edges(t: &ToolLook, p: [f64; 3], spin: f64, eye: &super::draw::Eye, out: &mut Vec<WorldVert>) {
    let (s, c) = forge_math::dmath::sin_cos(spin);
    // a little proud of the surface, so the depth test keeps them at any distance
    let at = |q: [f64; 3]| {
        let (x, y) = (q[0] * 1.03, q[1] * 1.03);
        [
            (p[0] + x * c - y * s) as f32,
            (p[1] + x * s + y * c) as f32,
            (p[2] + q[2]) as f32,
        ]
    };
    for pc in t.pieces.iter().filter(|pc| pc.spins) {
        for e in &pc.tris.edges {
            for w in e.windows(2) {
                super::draw::ribbon(eye, at(w[0]), at(w[1]), 2.0, [0.04, 0.04, 0.05, 0.9], out);
            }
        }
    }
}

/// The helix a cutter of this kind usually has, for drawing a record that gives none (said in
/// the tool's note): end mills and twist drills 30°, taps and reamers straight.
pub fn typical_helix(k: Kind) -> f64 {
    match k {
        Kind::Tap | Kind::Reamer | Kind::Spot | Kind::Countersink | Kind::Counterbore | Kind::Engrave => 0.0,
        _ => 30.0,
    }
}

/// The fluted mesh with its gashes deepened (the builder's groove taken 1.8 times as deep, for
/// the simulation's distance) and each vertex shaded by how deep it lies: a valley at 0.45, a
/// land at 1.15 of the colour. Normals are taken again from the moved surface.
fn gashed(mut t: Tris, outline: &[[f64; 2]], z0: f64) -> (Tris, Vec<f32>) {
    let r_at = |z: f64| {
        let mut r = 0.0;
        for w in outline.windows(2) {
            if (w[0][0]..=w[1][0]).contains(&z) && w[1][0] > w[0][0] {
                r = w[0][1] + (w[1][1] - w[0][1]) * (z - w[0][0]) / (w[1][0] - w[0][0]);
                break;
            }
        }
        r
    };
    let mut shade = Vec::with_capacity(t.pos.len());
    for p in &mut t.pos {
        let rr = (p[0] * p[0] + p[1] * p[1]).sqrt();
        let full = r_at(p[2] - z0);
        if full <= 1e-6 || rr <= 1e-9 {
            shade.push(1.0);
            continue;
        }
        let deficit = (full - rr).max(0.0) / full;
        let deep = (deficit * 1.8).min(0.45);
        let k = (full * (1.0 - deep)) / rr;
        p[0] *= k;
        p[1] *= k;
        // 0 on a land, 1 at the bottom of a valley
        let u = (deep / 0.4).min(1.0) as f32;
        shade.push(1.15 - 0.7 * u);
    }
    // normals from the moved surface, a triangle's to its corners
    for tri in &t.tri {
        let [a, b, c] = tri.map(|k| t.pos[k as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if l > 0.0 {
            for &k in tri {
                t.nrm[k as usize] = [n[0] / l, n[1] / l, n[2] / l];
            }
        }
    }
    (t, shade)
}

/// A holder's spindle interface above its gauge line, from the library's cited interface table
/// (forge-tooling `[[interface]]`, CAM-HAAS: Sumitomo, ISCAR, Maritool): the flange between its
/// faces and the taper from the gauge diameter. The taper's small end no maker prints: it is
/// narrowed by the cited ratio (7/24 on the diameter for BT and CAT, 1/10 for HSK); CAT's flange
/// thickness is its faces' difference, BT50's front face its back plus its thickness. The
/// V-groove is not published and not drawn. Display only: the certified checks never see it.
/// Returns the solid and its note; None where the interface has no table (TTS, straight).
fn interface_body(holder: &str, z_gauge: f64) -> Option<(Tris, String)> {
    static LIB: std::sync::OnceLock<forge_tooling::Library> = std::sync::OnceLock::new();
    let lib = LIB.get_or_init(forge_tooling::Library::builtin);
    let iface = lib.interface(&lib.holder(holder).ok()?.interface)?;
    let d = |k: &str| iface.dim(k);
    let (gd, tl, fd) = (d("gauge_d")?, d("taper_length")?, d("flange_d")?);
    let o: Vec<[f64; 2]> = if iface.kind == "hsk" {
        // the collar (from the contact face toward the tool), then the hollow taper up
        let ct = d("collar_t")?;
        vec![
            [z_gauge - ct, 0.0],
            [z_gauge - ct, 0.5 * fd],
            [z_gauge, 0.5 * fd],
            [z_gauge, 0.5 * gd],
            [z_gauge + tl, 0.5 * (gd - tl / 10.0)],
            [z_gauge + tl, 0.0],
        ]
    } else {
        let back = d("gauge_to_flange_back")?;
        let front = d("gauge_to_flange_front").or_else(|| Some(back + d("flange_t")?))?;
        let at_back = gd + back * 7.0 / 24.0;
        vec![
            [z_gauge - front, 0.0],
            [z_gauge - front, 0.5 * fd],
            [z_gauge - back, 0.5 * fd],
            [z_gauge - back, 0.5 * at_back],
            [z_gauge + tl, 0.5 * (gd - tl * 7.0 / 24.0)],
            [z_gauge + tl, 0.0],
        ]
    };
    let note = format!(
        "{holder}: {} taper and flange from the interface table; V-groove not published, not drawn",
        iface.id
    );
    Some((crate::machine_mesh::revolve(&o), note))
}

/// The generic interface, see-through, posed with the tool (it does not turn: nothing on it shows
/// turning).
pub fn ghost(t: &ToolLook, p: [f64; 3], out: &mut Vec<WorldVert>) {
    let Some(g) = &t.ghost else { return };
    for tri in &g.tri {
        for &k in tri {
            let (q, n) = (g.pos[k as usize], g.nrm[k as usize]);
            let c = shade_pub(look::HOLDER, [n[0] as f32, n[1] as f32, n[2] as f32]);
            out.push(WorldVert {
                at: [(p[0] + q[0]) as f32, (p[1] + q[1]) as f32, (p[2] + q[2]) as f32],
                colour: [c[0], c[1], c[2], 0.3],
            });
        }
    }
}

/// A holder's collet nut: (outside diameter, length, its maker and id) from the nut's record,
/// the holder's own `nut` where it names one, else the largest of its clamp's system.
fn collet_nut(holder: &str) -> Option<(f64, f64, String)> {
    static LIB: std::sync::OnceLock<forge_tooling::Library> = std::sync::OnceLock::new();
    let lib = LIB.get_or_init(forge_tooling::Library::builtin);
    let h = lib.holder(holder).ok()?;
    let n = match &h.nut {
        Some(id) => lib.collet_nuts.iter().find(|n| &n.id == id)?,
        None => lib
            .collet_nuts
            .iter()
            .filter(|n| n.system.eq_ignore_ascii_case(&h.clamp))
            .max_by(|a, b| a.od.total_cmp(&b.od))?,
    };
    let who = if n.vendor.is_empty() {
        n.id.clone()
    } else {
        format!("{} {}", n.vendor, n.product_id)
    };
    Some((n.od, n.length, format!("{} from {who}", n.system)))
}
