//! Lathe tools: ISO inserts and holders as exact outlines, and the model of their solid that the
//! checker bounds.
//!
//! **The outline.** Every body of a lathe tool is, in the (ρ, z) plane through the cutting edge,
//! K ⊕ B_r: a convex polygon K (the core) grown by a disc of radius r. An insert with every corner
//! rounded by its nose radius r_ε is exactly that, K the insert shrunk by r_ε (its nose corner is
//! the nose centre); a grooving insert is its rectangle shrunk by its corner radius; a round
//! insert is a point grown by its radius; a drill's section is a polygon with r = 0. Corners other
//! than the nose are taken sharp (a superset). Coordinates are relative to the tool's reference
//! point, the centre of its nose (of its left corner for a groove or part-off blade).
//!
//! **The solid.** The tool lies at and below the centre height: the rake face at y = 0, the body
//! down to y = −d1. Below the edge the flanks fall away by the clearance angle α, so at depth
//! |y| the section is the outline shrunk by |y|·tan α. For a body that is
//! {(x, y, z): |y| ∈ [d0, d1], (x, z) ∈ (K ⊕ B_r) ⊖ B_{(|y|−d0) tan α}}, which is inside
//! K ⊕ B_{max(0, r − (|y| − d0) tan α)}. This is the tool model, data like a milling cutter's
//! profile: the library's figures are generic and restated from catalogues (verify with the
//! vendor).
//!
//! **Why 2D suffices.** A part of revolution is π⁻¹(H), H its half-profile, π(x, y, z) =
//! (√(x² + y²), z). π is 1-Lipschitz, so dist(B, π⁻¹H) ≥ dist(π(B), H) for any body B. A slab
//! of the body at depths [a, b] has, when its section S lies in x ≥ 0, π(S × [a, b]) ⊆ S ⊕
//! [0, e]·ρ̂ with e = √(xm² + b²) − xm, xm the least x of S (√(x² + y²) − x falls as x grows). A
//! section reaching x < 0 is mirrored too (check.rs). So the checker bounds dist(π(B), H) by the
//! distance of a polygon swept along the move and along [0, e]·ρ̂, less the slab's radius.
use forge_math::dmath;
use forge_tooling::toml::{self, Table};

/// One body of a lathe tool (see the module doc).
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub name: String,
    /// The insert's cutting part (held to the part's leave), or a non-cutting body (held to the
    /// clearance).
    pub cutter: bool,
    /// The convex core K, counter-clockwise, in (ρ, z) relative to the reference point.
    pub core: Vec<[f64; 2]>,
    pub r: f64,
    /// [d0, d1]: the depths below the centre height the body spans (0 ≤ d0 ≤ d1).
    pub depth: [f64; 2],
    /// tan α: how fast the outline shrinks below d0 (0 for a holder's straight sides).
    pub relief: f64,
    /// A solid of revolution about the spindle axis (a drill on the centre line): its section is
    /// its own image under π, no smear.
    pub on_axis: bool,
    /// A cylinder along z (a boring bar's shank): its axis `dx` from the reference point in x, at
    /// centre height, radius `r`, from z0 to z1 relative to the reference point. Its σ-image is
    /// a rectangle the checker works out per move (check.rs); the core is not used.
    pub cyl: Option<Cyl>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cyl {
    pub dx: f64,
    pub r: f64,
    pub z: [f64; 2],
}

/// What the tool does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// OD turning and facing (a right-hand holder: it feeds toward the chuck, −z, and in, −ρ).
    Turn,
    /// OD grooving: plunges −ρ; width along z.
    Groove,
    /// Part-off: a grooving blade with the reach to the centre.
    Part,
    /// OD threading: a 60° profile.
    Thread,
    /// A drill on the centre line.
    Drill,
    /// ID turning and facing with a boring bar (right hand: it feeds toward the chuck, −z, and
    /// out, +ρ); the insert points to +ρ.
    Bore,
    /// ID grooving: a blade on a bar, plunging +ρ; width along z.
    IdGroove,
    /// ID threading: a 60° insert on a bar, pointing +ρ.
    IdThread,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnTool {
    pub id: String,
    pub kind: Kind,
    /// The insert's ISO code (or the drill's name).
    pub code: String,
    /// The nose (corner) radius; a drill's radius.
    pub re: f64,
    /// The directions of the two edges leaving the nose, as angles from +ρ (degrees): the lead
    /// edge and the trailing edge; the insert lies between them.
    pub edges: [f64; 2],
    /// The insert's cutting edge length (or a groove blade's depth of reach).
    pub l: f64,
    /// A groove blade's width; a thread insert's pitch range.
    pub width: f64,
    pub pitch: [f64; 2],
    /// The imaginary tip, relative to the reference point: what the control's tool offset is set
    /// to, so what the program states (G40).
    pub tip: [f64; 2],
    /// The tip direction number of the control's nose-radius compensation (1–9; 3 for an OD
    /// right-hand tool).
    pub tip_number: u32,
    /// A left-hand tool: the right-hand one mirrored across z (it feeds +z, away from the chuck).
    pub left: bool,
    pub bodies: Vec<Body>,
    /// Cutting data: m/min, mm/rev, mm depth of cut.
    pub vc: f64,
    pub f: f64,
    pub ap: f64,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToolError {
    /// A code this library cannot decode: what, which character, the fix.
    Code {
        code: String,
        why: String,
    },
    Missing {
        record: String,
        field: &'static str,
    },
    Bad {
        record: String,
        why: String,
    },
    Unknown {
        id: String,
        near: Vec<String>,
    },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ToolError::Code { code, why } => write!(f, "`{code}`: {why}"),
            ToolError::Missing { record, field } => write!(f, "{record}: missing `{field}`"),
            ToolError::Bad { record, why } => write!(f, "{record}: {why}"),
            ToolError::Unknown { id, near } => write!(f, "no lathe tool `{id}`; near: {}", near.join(", ")),
        }
    }
}

/// An ISO 1832 insert, decoded.
#[derive(Clone, Debug, PartialEq)]
pub struct InsertSpec {
    pub shape: char,
    /// The nose's included angle (degrees); 360 for a round insert.
    pub eps: f64,
    /// The clearance angle of the insert itself (degrees).
    pub clearance: f64,
    /// Inscribed circle diameter.
    pub ic: f64,
    /// Cutting edge length (the side of the rhombus or triangle).
    pub l: f64,
    pub thickness: f64,
    pub re: f64,
}

fn code_err(code: &str, why: impl Into<String>) -> ToolError {
    ToolError::Code {
        code: code.to_string(),
        why: why.into(),
    }
}

/// Decodes an ISO 1832 turning insert code: shape, clearance, tolerance, type, size, thickness,
/// nose radius (`CNMG120408`, `DCMT11T304`, `VBMT160404`, `TNMG160408`, `SNMG120408`,
/// `RCMT1204M0`). The sizes are the standard IC series; the rest of the code (tolerance, chip
/// breaker, hand) does not change the outline and is kept as text.
pub fn decode_insert(code: &str) -> Result<InsertSpec, ToolError> {
    let c: Vec<char> = code.trim().chars().collect();
    if c.len() < 8 {
        return Err(code_err(
            code,
            "too short for an ISO 1832 insert code (e.g. CNMG120408)",
        ));
    }
    let shape = c[0];
    let eps: f64 = match shape {
        'C' => 80.0,
        'D' => 55.0,
        'V' => 35.0,
        'T' => 60.0,
        'S' => 90.0,
        'R' => 360.0,
        // the trigon: three 80° corners and three 160° ones, every side tangent to the IC
        'W' => 80.0,
        x => {
            return Err(code_err(
                code,
                format!("insert shape `{x}` not in this library (C D V T S W R)"),
            ));
        }
    };
    let clearance = match c[1] {
        'N' => 0.0,
        'A' => 3.0,
        'B' => 5.0,
        'C' => 7.0,
        'P' => 11.0,
        'D' => 15.0,
        'E' => 20.0,
        x => return Err(code_err(code, format!("clearance letter `{x}` not known"))),
    };
    let size: String = c[4..6].iter().collect();
    let thick: String = c[6..8].iter().collect();
    // the IC series per shape and size code (cutting edge length in whole mm)
    let ic = match (shape, size.as_str()) {
        ('C', "06") | ('D', "07") | ('V', "11") | ('T', "11") | ('S', "06") => 6.35,
        ('C', "09") | ('D', "11") | ('V', "16") | ('T', "16") | ('S', "09") => 9.525,
        ('C', "12") | ('D', "15") | ('V', "22") | ('T', "22") | ('S', "12") => 12.7,
        ('C', "16") | ('S', "15") | ('T', "27") => 15.875,
        ('C', "19") | ('S', "19") => 19.05,
        ('W', "06") => 9.525,
        ('W', "08") => 12.7,
        ('R', s) => s
            .parse::<f64>()
            .map_err(|_| code_err(code, "round insert size: two digits (mm)"))?,
        _ => {
            return Err(code_err(
                code,
                format!("size code `{size}` not in the {shape} series"),
            ));
        }
    };
    let thickness = match thick.as_str() {
        "01" => 1.59,
        "02" => 2.38,
        "03" => 3.18,
        "T3" => 3.97,
        "04" => 4.76,
        "05" => 5.56,
        "06" => 6.35,
        "07" => 7.94,
        "09" => 9.52,
        x => return Err(code_err(code, format!("thickness code `{x}` not known"))),
    };
    let (re, l) = if shape == 'R' {
        (0.5 * ic, 0.0)
    } else {
        let rc: String = c[8..10.min(c.len())].iter().collect();
        if rc.len() != 2 {
            return Err(code_err(code, "nose radius: two digits (04 = 0.4 mm)"));
        }
        let re = rc
            .parse::<f64>()
            .map_err(|_| code_err(code, format!("nose radius `{rc}`: two digits (04 = 0.4 mm)")))?
            / 10.0;
        let e = eps.to_radians();
        let l = match shape {
            'T' => ic * 3f64.sqrt(),
            'S' => ic,
            // a side between an 80° and a 160° corner: the tangent lengths r·cot 40° + r·cot 80°
            'W' => 0.5 * ic * (1.0 / tan(40f64.to_radians()) + 1.0 / tan(80f64.to_radians())),
            _ => ic / dmath::sin(e),
        };
        (re, l)
    };
    Ok(InsertSpec {
        shape,
        eps,
        clearance,
        ic,
        l,
        thickness,
        re,
    })
}

/// An ISO 5608 external toolholder, decoded: clamping, insert shape, style (the approach angle
/// κr), insert clearance, hand, shank height and width, length.
#[derive(Clone, Debug, PartialEq)]
pub struct HolderSpec {
    pub shape: char,
    pub kapr: f64,
    pub hand: char,
    pub h: f64,
    pub b: f64,
    pub length: f64,
}

/// `PCLNR2525M12`, `SVJBR2020K16`, `PDJNR2525M15`, `MTJNR2525M16`.
pub fn decode_holder(code: &str) -> Result<HolderSpec, ToolError> {
    let c: Vec<char> = code.trim().chars().collect();
    if c.len() < 10 {
        return Err(code_err(
            code,
            "too short for an ISO 5608 holder code (e.g. PCLNR2525M12)",
        ));
    }
    // the approach angle of each style letter (common catalogue practice)
    let kapr = match c[2] {
        'A' | 'F' | 'G' => 90.0,
        'B' | 'K' => 75.0,
        'D' | 'S' => 45.0,
        'J' => 93.0,
        'L' => 95.0,
        'N' => 62.5,
        'T' => 60.0,
        'V' => 72.5,
        x => return Err(code_err(code, format!("holder style `{x}` not known"))),
    };
    let hand = c[4];
    if hand != 'R' && hand != 'L' {
        return Err(code_err(
            code,
            "the hand: right (R) or left (L); neutral (N) holders are not in this library",
        ));
    }
    let h: f64 = c[5..7]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| code_err(code, "shank height: two digits"))?;
    let b: f64 = c[7..9]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| code_err(code, "shank width: two digits"))?;
    let length = match c[9] {
        'H' => 100.0,
        'K' => 125.0,
        'M' => 150.0,
        'P' => 170.0,
        'Q' => 180.0,
        'R' => 200.0,
        x => return Err(code_err(code, format!("length letter `{x}` not known"))),
    };
    Ok(HolderSpec {
        shape: c[1],
        kapr,
        hand,
        h,
        b,
        length,
    })
}

/// The insert's core (its outline less the nose radius, a convex polygon with its nose corner at
/// the origin, counter-clockwise), the lead edge leaving the nose at `phi1` degrees from +ρ.
fn insert_core(ins: &InsertSpec, phi1: f64) -> Vec<[f64; 2]> {
    let re = ins.re;
    let d1 = dir(phi1);
    let d2 = dir(phi1 + ins.eps);
    match ins.shape {
        'T' => {
            let ls = ins.l - 2.0 * re / tan(30f64.to_radians());
            vec![[0.0, 0.0], sc(d1, ls), sc(d2, ls)]
        }
        // the trigon less re: six equal sides (each the side less re·cot 40° and re·cot 80°),
        // turning left by 20° at a 160° corner and 100° at an 80° one
        'W' => {
            let e = ins.l - re * (1.0 / tan(40f64.to_radians()) + 1.0 / tan(80f64.to_radians()));
            let mut p = [0.0, 0.0];
            let mut v = vec![p];
            for t in [0.0, 20.0, 120.0, 140.0, 240.0] {
                p = add(p, sc(dir(phi1 + t), e));
                v.push(p);
            }
            v
        }
        _ => {
            let ls = ins.l - 2.0 * re / tan(0.5 * ins.eps.to_radians()).max(1e-9);
            let a = sc(d1, ls);
            let b = sc(d2, ls);
            vec![[0.0, 0.0], a, add(a, b), b]
        }
    }
}

/// tan through forge_math::dmath (the platform's libm differs between targets).
pub(crate) fn tan(x: f64) -> f64 {
    let (s, c) = dmath::sin_cos(x);
    s / c
}

fn dir(deg: f64) -> [f64; 2] {
    let t = deg.to_radians();
    [dmath::cos(t), dmath::sin(t)]
}

fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sc(a: [f64; 2], k: f64) -> [f64; 2] {
    [a[0] * k, a[1] * k]
}

/// Counter-clockwise convex hull (Andrew's monotone chain), for the generous holder outline.
fn hull(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cr =
        |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut lo: Vec<[f64; 2]> = Vec::new();
    for &q in &p {
        while lo.len() >= 2 && cr(lo[lo.len() - 2], lo[lo.len() - 1], q) <= 0.0 {
            lo.pop();
        }
        lo.push(q);
    }
    let mut hi: Vec<[f64; 2]> = Vec::new();
    for &q in p.iter().rev() {
        while hi.len() >= 2 && cr(hi[hi.len() - 2], hi[hi.len() - 1], q) <= 0.0 {
            hi.pop();
        }
        hi.push(q);
    }
    lo.pop();
    hi.pop();
    lo.extend(hi);
    lo
}

/// The rounding radius every generous (holder) outline is grown by: its hull is computed in f64,
/// so growing it covers the hull's rounding many times over.
const HOLDER_PAD: f64 = 0.05;

/// How far a generic seat's corners stand in from the insert's.
const SEAT_INSET: f64 = 0.5;

/// An OD turning tool from an insert and a holder (a left-hand one is its right-hand twin
/// mirrored across z, `mirrored`): the insert's core (its nose
/// corner at the nose centre, the reference point), its body to its thickness below centre
/// height with the effective clearance `clear_deg` (a negative insert in a negatively inclined
/// seat: about 6°), and the holder's head and shank.
pub fn turning_tool(
    id: &str,
    insert: &str,
    holder: &str,
    seat_deg: Option<f64>,
) -> Result<TurnTool, ToolError> {
    let ins = decode_insert(insert)?;
    let hol = decode_holder(holder)?;
    // a negative insert sits in a seat inclined about 6°, a positive one flat
    let clear_deg = seat_deg.unwrap_or(if ins.clearance == 0.0 { 6.0 } else { 0.0 });
    if !matches!(holder.trim().chars().next(), Some('P') | Some('S')) {
        return Err(code_err(
            holder,
            "only lever (P) and screw (S) clamping in this phase: a top clamp stands above the rake face",
        ));
    }
    if ins.shape != hol.shape {
        return Err(code_err(
            holder,
            format!("the holder takes `{}` inserts, not `{}`", hol.shape, ins.shape),
        ));
    }
    if ins.shape == 'R' {
        return Err(code_err(insert, "round inserts in a holder: next"));
    }
    // the lead edge leaves the nose at κr − 90° from +ρ, the trailing edge ε further round
    let phi1 = hol.kapr - 90.0;
    let phi2 = phi1 + ins.eps;
    if phi2 > 180.0 || phi1 < -90.0 {
        return Err(code_err(
            holder,
            "this style and insert leave no clearance at the face",
        ));
    }
    let re = ins.re;
    // the shrunk insert: same angles, sides shorter by the corner allowances; its nose corner
    // is at the nose centre (0, 0)
    let core = insert_core(&ins, phi1);
    let relief = tan((ins.clearance + clear_deg).max(0.0).to_radians());
    let insert_body = Body {
        name: "insert".into(),
        cutter: true,
        core: ccw(core.clone()),
        r: re,
        depth: [0.0, ins.thickness],
        relief,
        on_axis: false,
        cyl: None,
    };
    // the holder: its head holds the insert's far corners and the shank's front, the shank runs
    // out along +ρ for its length, b wide along z, h deep below centre height. The nose is f from
    // the shank's far side (generic: f = b + 0.28 b, as catalogues give for these styles).
    let far: Vec<[f64; 2]> = core.iter().skip(1).copied().collect();
    let f = hol.b * 1.28;
    let zb = f - hol.b; // the shank's near side above the nose centre
    let rho0 = far.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max) + re;
    let shank = vec![
        [rho0, zb],
        [rho0 + hol.length, zb],
        [rho0 + hol.length, zb + hol.b],
        [rho0, zb + hol.b],
    ];
    // the head under the insert: the seat is smaller than the insert (its corners SEAT_INSET in
    // toward the insert's middle) and its sides fall away below the insert at the insert's
    // clearance, continuing the insert's own relief; it reaches back to the shank's front
    let mid = {
        let n = core.len() as f64;
        [
            core.iter().map(|p| p[0]).sum::<f64>() / n,
            core.iter().map(|p| p[1]).sum::<f64>() / n,
        ]
    };
    let mut head_pts: Vec<[f64; 2]> = far
        .iter()
        .map(|p| {
            let v = [mid[0] - p[0], mid[1] - p[1]];
            let l = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-9);
            [p[0] + SEAT_INSET * v[0] / l, p[1] + SEAT_INSET * v[1] / l]
        })
        .collect();
    head_pts.push([rho0, zb]);
    head_pts.push([rho0, zb + hol.b]);
    let head = Body {
        name: "holder head".into(),
        cutter: false,
        core: hull(head_pts),
        r: (re - ins.thickness * relief).max(0.0) + HOLDER_PAD,
        depth: [ins.thickness, hol.h],
        relief,
        on_axis: false,
        cyl: None,
    };
    let shank_body = Body {
        name: "shank".into(),
        cutter: false,
        core: shank,
        r: HOLDER_PAD,
        depth: [0.0, hol.h],
        relief: 0.0,
        on_axis: false,
        cyl: None,
    };
    let t = TurnTool {
        id: id.into(),
        kind: Kind::Turn,
        code: insert.into(),
        re,
        edges: [phi1, phi2],
        l: ins.l,
        width: 0.0,
        pitch: [0.0, 0.0],
        tip: [-re, -re],
        tip_number: 3,
        left: false,
        bodies: vec![insert_body, head, shank_body],
        vc: 200.0,
        f: 0.2,
        ap: 2.0,
        source: "decoded from the ISO codes; holder head and f generic: verify with the vendor".into(),
    };
    Ok(if hol.hand == 'L' { mirrored(t) } else { t })
}

/// The tool mirrored across z (ρ, z) → (ρ, −z): a left-hand tool from its right-hand twin. A
/// mirror maps a convex core to a convex core and the clearances with it; the edges' angles from
/// +ρ change sign; the imaginary tip mirrors; the control's tip number with it (3 ↔ 4, 2 ↔ 1).
pub fn mirrored(mut t: TurnTool) -> TurnTool {
    for b in &mut t.bodies {
        // mirrored, then the vertices after the first reversed: counter-clockwise again, the
        // first vertex (the nose, which the checker's inner point starts from) still first
        let m: Vec<[f64; 2]> = b.core.iter().map(|p| [p[0], -p[1]]).collect();
        b.core = m[..1.min(m.len())]
            .iter()
            .chain(m[1..].iter().rev())
            .copied()
            .collect();
        if let Some(c) = &mut b.cyl {
            c.z = [-c.z[1], -c.z[0]];
        }
    }
    t.edges = [-t.edges[0], -t.edges[1]];
    t.tip = [t.tip[0], -t.tip[1]];
    t.tip_number = match t.tip_number {
        3 => 4,
        4 => 3,
        2 => 1,
        1 => 2,
        n => n,
    };
    t.left = !t.left;
    t
}

/// An ISO 6261 boring bar, decoded: shank diameter, length, insert shape, style (κr), hand.
#[derive(Clone, Debug, PartialEq)]
pub struct BarSpec {
    pub d: f64,
    pub length: f64,
    pub shape: char,
    pub kapr: f64,
    /// R or L: a left-hand bar is its right-hand twin mirrored across z.
    pub hand: char,
}

/// `S20R-SCLCR09`, `A16R-SDUCR07`: shank material and diameter, length letter, then clamping,
/// insert shape, style, clearance, hand, edge length.
pub fn decode_bar(code: &str) -> Result<BarSpec, ToolError> {
    let c = code.trim();
    let (head, tail) = c
        .split_once('-')
        .ok_or_else(|| code_err(code, "an ISO 6261 bar code has a dash (e.g. S20R-SCLCR09)"))?;
    let h: Vec<char> = head.chars().collect();
    if h.len() < 4 || !matches!(h[0], 'S' | 'A' | 'E' | 'C') {
        return Err(code_err(
            code,
            "shank: a material letter (S A E C), a diameter, a length letter",
        ));
    }
    let d: f64 = h[1..h.len() - 1]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| code_err(code, "the shank diameter"))?;
    let length = match h[h.len() - 1] {
        'H' => 100.0,
        'K' => 125.0,
        'M' => 150.0,
        'P' => 170.0,
        'Q' => 180.0,
        'R' => 200.0,
        'S' => 250.0,
        'T' => 300.0,
        x => return Err(code_err(code, format!("length letter `{x}` not known"))),
    };
    let t: Vec<char> = tail.chars().collect();
    if t.len() < 5 {
        return Err(code_err(
            code,
            "after the dash: clamping, shape, style, clearance, hand",
        ));
    }
    if !matches!(t[0], 'S' | 'P') {
        return Err(code_err(
            code,
            "only screw (S) and lever (P) clamping: a top clamp stands above the rake face",
        ));
    }
    let kapr = match t[2] {
        'L' => 95.0,
        'J' => 93.0,
        'U' => 93.0,
        'F' => 90.0,
        'Q' => 107.5,
        x => return Err(code_err(code, format!("bar style `{x}` not known (L J U F Q)"))),
    };
    if t[4] != 'R' && t[4] != 'L' {
        return Err(code_err(code, "the hand: right (R) or left (L)"));
    }
    Ok(BarSpec {
        hand: t[4],
        d,
        length,
        shape: t[1],
        kapr,
    })
}

/// A boring tool: an insert in a right-hand boring bar. The insert is the OD tool's mirrored
/// across the reference point (it points to +ρ, its edges at 180° − φ); the bar is a cylinder
/// along z, its axis f − r_ε toward the axis from the nose centre (f = D/2 + 3, generic, as
/// catalogues give for S16–S25 bars: verify with the vendor), from the nose's lowest point for
/// its length (the head and seat under the insert lie inside its cylinder). The insert's
/// clearance under the edge is its own plus a generic 6° inclination in the bar.
pub fn boring_tool(id: &str, insert: &str, bar: &str, seat_deg: Option<f64>) -> Result<TurnTool, ToolError> {
    let ins = decode_insert(insert)?;
    let b = decode_bar(bar)?;
    if ins.shape != b.shape {
        return Err(code_err(
            bar,
            format!("the bar takes `{}` inserts, not `{}`", b.shape, ins.shape),
        ));
    }
    // boring bars incline the insert for radial clearance under the edge: generic 6°
    let clear_deg = seat_deg.unwrap_or(6.0);
    let phi1 = b.kapr - 90.0;
    let phi2 = phi1 + ins.eps;
    let re = ins.re;
    let od = insert_core(&ins, phi1);
    let core: Vec<[f64; 2]> = od.iter().map(|p| [-p[0], p[1]]).collect();
    let relief = tan((ins.clearance + clear_deg).max(0.0).to_radians());
    let f = 0.5 * b.d + 3.0;
    let insert_body = Body {
        name: "insert".into(),
        cutter: true,
        core: ccw(core),
        r: re,
        depth: [0.0, ins.thickness],
        relief,
        on_axis: false,
        cyl: None,
    };
    let bar_body = Body {
        name: "bar".into(),
        cutter: false,
        core: vec![[0.0, 0.0]],
        r: 0.0,
        depth: [0.0, 0.0],
        relief: 0.0,
        on_axis: false,
        cyl: Some(Cyl {
            dx: -(f - re),
            r: 0.5 * b.d,
            z: [-re, b.length],
        }),
    };
    let t = TurnTool {
        id: id.into(),
        kind: Kind::Bore,
        code: insert.into(),
        re,
        edges: [180.0 - phi1, 180.0 - phi2],
        l: ins.l,
        width: b.d,
        pitch: [0.0, 0.0],
        tip: [re, -re],
        tip_number: 2,
        left: false,
        bodies: vec![insert_body, bar_body],
        vc: 180.0,
        f: 0.15,
        ap: 1.0,
        source: "decoded from the ISO codes; bar f and seat generic: verify with the vendor".into(),
    };
    Ok(if b.hand == 'L' { mirrored(t) } else { t })
}

/// A grooving or part-off blade: width w, corner radius rc, reach (depth from the tip to the
/// blade's holder), side clearance per side in degrees. The reference point is the centre of
/// the corner on the −z side; the blade plunges along −ρ.
pub fn groove_tool(
    id: &str,
    kind: Kind,
    w: f64,
    rc: f64,
    reach: f64,
    side_deg: f64,
    h: f64,
) -> Result<TurnTool, ToolError> {
    if !(w > 2.0 * rc && rc >= 0.0 && reach > 0.0) {
        return Err(ToolError::Bad {
            record: id.into(),
            why: format!("a blade {w} wide with corners {rc} and reach {reach}"),
        });
    }
    let core_w = w - 2.0 * rc;
    let taper = tan(side_deg.to_radians()) * reach;
    // the blade narrows by the side clearance on both sides as it runs back (+ρ) from the tip
    let core = vec![[0.0, 0.0], [reach, taper], [reach, core_w - taper], [0.0, core_w]];
    if core_w - 2.0 * taper <= 0.0 {
        return Err(ToolError::Bad {
            record: id.into(),
            why: "the side clearance closes the blade".into(),
        });
    }
    let blade = Body {
        name: "blade".into(),
        cutter: true,
        core: ccw(core),
        r: rc,
        depth: [0.0, 3.0],
        relief: tan(7f64.to_radians()),
        on_axis: false,
        cyl: None,
    };
    let holder = Body {
        name: "blade holder".into(),
        cutter: false,
        core: vec![
            [reach + rc, -6.0],
            [reach + rc + 60.0, -6.0],
            [reach + rc + 60.0, w + 6.0],
            [reach + rc, w + 6.0],
        ],
        r: HOLDER_PAD,
        depth: [0.0, h],
        relief: 0.0,
        on_axis: false,
        cyl: None,
    };
    Ok(TurnTool {
        id: id.into(),
        kind,
        code: format!("blade {w} x {reach}"),
        re: rc,
        edges: [-90.0, 90.0],
        l: reach,
        width: w,
        pitch: [0.0, 0.0],
        tip: [-rc, -rc],
        tip_number: 3,
        left: false,
        bodies: vec![blade, holder],
        vc: 120.0,
        f: 0.06,
        ap: reach,
        source: "generic blade: verify with the vendor".into(),
    })
}

/// An OD threading insert: 60° flank angle, tip radius rt, profile height hp (the insert's
/// reach along −ρ beyond what it cuts), pitch range. The reference point is the tip radius's
/// centre; the flanks leave it at 60° and 120° from +ρ.
pub fn thread_tool(id: &str, rt: f64, pitch: [f64; 2], h: f64) -> Result<TurnTool, ToolError> {
    let half = 30f64.to_radians();
    let reach = 4.0;
    // the core: an isoceles triangle, apex at the tip centre, flanks at ±30° from +ρ (the
    // insert's 60° V), cut off at the reach
    let side = reach * tan(half);
    let core = vec![[0.0, 0.0], [reach, -side], [reach, side]];
    let tip = Body {
        name: "thread insert".into(),
        cutter: true,
        core: ccw(core),
        r: rt,
        depth: [0.0, 3.5],
        relief: tan(8f64.to_radians()),
        on_axis: false,
        cyl: None,
    };
    let holder = Body {
        name: "thread holder".into(),
        cutter: false,
        core: vec![
            [reach + rt, -2.5],
            [reach + rt + 60.0, -2.5],
            [reach + rt + 60.0, 22.5],
            [reach + rt, 22.5],
        ],
        r: HOLDER_PAD,
        depth: [0.0, h],
        relief: 0.0,
        on_axis: false,
        cyl: None,
    };
    Ok(TurnTool {
        id: id.into(),
        kind: Kind::Thread,
        code: format!("60° thread, tip r {rt}"),
        re: rt,
        edges: [-30.0, 30.0],
        l: reach,
        width: 2.0 * side,
        pitch,
        tip: [-rt, 0.0],
        tip_number: 8,
        left: false,
        bodies: vec![tip, holder],
        vc: 100.0,
        f: 0.0,
        ap: 0.0,
        source: "generic 60° insert: verify with the vendor".into(),
    })
}

/// An internal grooving tool: a blade w wide with corners rc, reaching `reach` (the deepest
/// groove it cuts, from the bar's side) above a bar of diameter `bar_d`. The reference point is
/// the centre of the corner on the −z side; the blade points +ρ and narrows by the side
/// clearance as it runs back toward the bar. The bar is a cylinder along z (checked as one, its
/// exact image per move), its side `reach + rc` below the reference point, from the blade's −z
/// side for `length`. Generic: verify with the vendor.
pub fn id_groove_tool(
    id: &str,
    w: f64,
    rc: f64,
    reach: f64,
    side_deg: f64,
    bar_d: f64,
    length: f64,
) -> Result<TurnTool, ToolError> {
    if !(w > 2.0 * rc && rc >= 0.0 && reach > 0.0 && bar_d > 0.0 && length > 0.0) {
        return Err(ToolError::Bad {
            record: id.into(),
            why: format!("a blade {w} wide with corners {rc} and reach {reach} on a bar {bar_d}"),
        });
    }
    let core_w = w - 2.0 * rc;
    let taper = tan(side_deg.to_radians()) * reach;
    if core_w - 2.0 * taper <= 0.0 {
        return Err(ToolError::Bad {
            record: id.into(),
            why: "the side clearance closes the blade".into(),
        });
    }
    let core = vec![
        [0.0, 0.0],
        [0.0, core_w],
        [-reach, core_w - taper],
        [-reach, taper],
    ];
    let blade = Body {
        name: "blade".into(),
        cutter: true,
        core: ccw(core),
        r: rc,
        depth: [0.0, 3.0],
        relief: tan(7f64.to_radians()),
        on_axis: false,
        cyl: None,
    };
    let bar = Body {
        name: "bar".into(),
        cutter: false,
        core: vec![[0.0, 0.0]],
        r: 0.0,
        depth: [0.0, 0.0],
        relief: 0.0,
        on_axis: false,
        cyl: Some(Cyl {
            dx: -(reach + rc + 0.5 * bar_d),
            r: 0.5 * bar_d,
            z: [-rc, length],
        }),
    };
    Ok(TurnTool {
        id: id.into(),
        kind: Kind::IdGroove,
        code: format!("ID blade {w} x {reach} on Ø{bar_d}"),
        re: rc,
        edges: [90.0, -90.0],
        l: reach,
        width: w,
        pitch: [0.0, 0.0],
        tip: [rc, -rc],
        tip_number: 2,
        left: false,
        bodies: vec![blade, bar],
        vc: 100.0,
        f: 0.04,
        ap: reach,
        source: "generic ID grooving bar: verify with the vendor".into(),
    })
}

/// An internal threading tool: a 60° insert with tip radius rt pointing +ρ, reaching 4 mm above
/// a bar of diameter `bar_d` (a cylinder along z, from the insert's −z side for `length`). The
/// reference point is the tip radius's centre. Generic: verify with the vendor.
pub fn id_thread_tool(
    id: &str,
    rt: f64,
    pitch: [f64; 2],
    bar_d: f64,
    length: f64,
) -> Result<TurnTool, ToolError> {
    if !(rt >= 0.0 && bar_d > 0.0 && length > 0.0 && pitch[0] > 0.0 && pitch[0] <= pitch[1]) {
        return Err(ToolError::Bad {
            record: id.into(),
            why: format!("an ID thread insert r {rt}, pitch {pitch:?}, on a bar {bar_d}"),
        });
    }
    let reach = 4.0;
    let side = reach * tan(30f64.to_radians());
    let core = vec![[0.0, 0.0], [-reach, side], [-reach, -side]];
    let tip = Body {
        name: "thread insert".into(),
        cutter: true,
        core: ccw(core),
        r: rt,
        depth: [0.0, 3.5],
        relief: tan(8f64.to_radians()),
        on_axis: false,
        cyl: None,
    };
    let bar = Body {
        name: "bar".into(),
        cutter: false,
        core: vec![[0.0, 0.0]],
        r: 0.0,
        depth: [0.0, 0.0],
        relief: 0.0,
        on_axis: false,
        cyl: Some(Cyl {
            dx: -(reach + rt + 0.5 * bar_d),
            r: 0.5 * bar_d,
            z: [-side - rt, length],
        }),
    };
    Ok(TurnTool {
        id: id.into(),
        kind: Kind::IdThread,
        code: format!("ID 60° thread, tip r {rt}, on Ø{bar_d}"),
        re: rt,
        edges: [150.0, -150.0],
        l: reach,
        width: 2.0 * side,
        pitch,
        tip: [rt, 0.0],
        tip_number: 6,
        left: false,
        bodies: vec![tip, bar],
        vc: 80.0,
        f: 0.0,
        ap: 0.0,
        source: "generic 60° ID insert and bar: verify with the vendor".into(),
    })
}

/// A drill on the centre line: diameter d, point angle, flute length, shank diameter.
pub fn drill_tool(id: &str, d: f64, point_deg: f64, flute: f64, shank: f64) -> Result<TurnTool, ToolError> {
    let r = 0.5 * d;
    let tip_h = r / tan(0.5 * point_deg.to_radians());
    let sr = 0.5 * shank;
    // the cutting part: the point and the fluted cylinder (convex); the shank above it its own
    // body (a hull of both would invent material between them, wider than the flutes)
    let flutes = Body {
        name: "drill".into(),
        cutter: true,
        core: ccw(vec![[0.0, 0.0], [r, tip_h], [r, flute], [-r, flute], [-r, tip_h]]),
        r: 0.0,
        depth: [0.0, 0.0],
        relief: 0.0,
        on_axis: true,
        cyl: None,
    };
    let shank_body = Body {
        name: "drill shank".into(),
        cutter: false,
        core: ccw(vec![
            [-sr, flute],
            [sr, flute],
            [sr, flute + 60.0],
            [-sr, flute + 60.0],
        ]),
        r: 0.0,
        depth: [0.0, 0.0],
        relief: 0.0,
        on_axis: true,
        cyl: None,
    };
    Ok(TurnTool {
        id: id.into(),
        kind: Kind::Drill,
        code: format!("drill {d}"),
        re: 0.0,
        edges: [0.0, 0.0],
        l: flute,
        width: d,
        pitch: [0.0, 0.0],
        tip: [0.0, 0.0],
        tip_number: 0,
        left: false,
        bodies: vec![flutes, shank_body],
        vc: 25.0,
        f: 0.1,
        ap: 0.0,
        source: "generic drill".into(),
    })
}

/// The polygon counter-clockwise.
fn ccw(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut a = 0.0;
    for i in 0..p.len() {
        let (u, v) = (p[i], p[(i + 1) % p.len()]);
        a += u[0] * v[1] - v[0] * u[1];
    }
    if a < 0.0 {
        p.reverse();
    }
    p
}

/// The built-in lathe tool library (data/turning.toml).
pub const BUILTIN: &str = include_str!("../data/turning.toml");

/// The library's tools, each built from its record.
pub fn library(src: &str) -> Result<Vec<TurnTool>, ToolError> {
    let t = toml::parse(src).map_err(|e| ToolError::Bad {
        record: format!("line {}", e.line),
        why: e.what,
    })?;
    let mut out = Vec::new();
    for r in toml::tables(&t, "lathe_tool") {
        out.push(from_table(r)?);
    }
    Ok(out)
}

fn from_table(r: &Table) -> Result<TurnTool, ToolError> {
    let id = toml::text(r, "id")
        .ok_or(ToolError::Missing {
            record: "[[lathe_tool]]".into(),
            field: "id",
        })?
        .to_string();
    let rec = format!("lathe tool `{id}`");
    let need = |k: &'static str| {
        toml::num(r, k).ok_or(ToolError::Missing {
            record: rec.clone(),
            field: k,
        })
    };
    let kind = toml::text(r, "kind").unwrap_or("turn");
    let mut t = match kind {
        "turn" => {
            let ins = toml::text(r, "insert").ok_or(ToolError::Missing {
                record: rec.clone(),
                field: "insert",
            })?;
            let hol = toml::text(r, "holder").ok_or(ToolError::Missing {
                record: rec.clone(),
                field: "holder",
            })?;
            turning_tool(&id, ins, hol, toml::num(r, "seat_clearance"))?
        }
        "groove" | "part" => groove_tool(
            &id,
            if kind == "groove" {
                Kind::Groove
            } else {
                Kind::Part
            },
            need("width")?,
            need("corner")?,
            need("reach")?,
            toml::num(r, "side_clearance").unwrap_or(2.0),
            toml::num(r, "shank").unwrap_or(25.0),
        )?,
        "thread" => {
            let p = toml::nums(r, "pitch")
                .filter(|v| v.len() == 2)
                .ok_or(ToolError::Missing {
                    record: rec.clone(),
                    field: "pitch",
                })?;
            thread_tool(
                &id,
                need("tip_radius")?,
                [p[0], p[1]],
                toml::num(r, "shank").unwrap_or(25.0),
            )?
        }
        "bore" => {
            let ins = toml::text(r, "insert").ok_or(ToolError::Missing {
                record: rec.clone(),
                field: "insert",
            })?;
            let bar = toml::text(r, "bar").ok_or(ToolError::Missing {
                record: rec.clone(),
                field: "bar",
            })?;
            boring_tool(&id, ins, bar, toml::num(r, "seat_clearance"))?
        }
        "id_groove" => id_groove_tool(
            &id,
            need("width")?,
            need("corner")?,
            need("reach")?,
            toml::num(r, "side_clearance").unwrap_or(2.0),
            need("bar_diameter")?,
            toml::num(r, "length").unwrap_or(150.0),
        )?,
        "id_thread" => {
            let p = toml::nums(r, "pitch")
                .filter(|v| v.len() == 2)
                .ok_or(ToolError::Missing {
                    record: rec.clone(),
                    field: "pitch",
                })?;
            id_thread_tool(
                &id,
                need("tip_radius")?,
                [p[0], p[1]],
                need("bar_diameter")?,
                toml::num(r, "length").unwrap_or(150.0),
            )?
        }
        "drill" => drill_tool(
            &id,
            need("diameter")?,
            toml::num(r, "point").unwrap_or(118.0),
            need("flute")?,
            toml::num(r, "shank_diameter").unwrap_or(need("diameter")?),
        )?,
        x => {
            return Err(ToolError::Bad {
                record: rec,
                why: format!("kind `{x}`: turn, bore, groove, part, thread, id_groove, id_thread or drill"),
            });
        }
    };
    if let Some(v) = toml::num(r, "vc") {
        t.vc = v;
    }
    if let Some(v) = toml::num(r, "f") {
        t.f = v;
    }
    if let Some(v) = toml::num(r, "ap") {
        t.ap = v;
    }
    if let Some(s) = toml::text(r, "source") {
        t.source = s.to_string();
    }
    Ok(t)
}

/// A tool of the built-in library by id.
pub fn builtin(id: &str) -> Result<TurnTool, ToolError> {
    let lib = library(BUILTIN)?;
    let near: Vec<String> = lib.iter().map(|t| t.id.clone()).collect();
    lib.into_iter()
        .find(|t| t.id == id)
        .ok_or(ToolError::Unknown { id: id.into(), near })
}
