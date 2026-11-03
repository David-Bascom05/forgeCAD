//! Why a part is invalid: forge-topo's check problems, each on its body (named by the feature
//! that made the body) and at the faces, edges and corners it names, described by what made
//! them: a sheet's wall, bend, cut or trim where the sheet's roles know the name, else the
//! features the face came from.

use crate::eval::Eval;
use forge_sheet as fs;
use forge_sheet::error::{Strip, TrimRef};
use forge_sheet::roles::Skin;
use forge_topo::Body;
use forge_topo::check::{Problem, check};
use forge_topo::{EdgeId, FaceId, Name};

/// One check problem of the part.
#[derive(Clone, Debug, PartialEq)]
pub struct Invalid {
    /// The feature that made the body.
    pub body: String,
    /// The problem, as forge-topo names it (`LoopsCross`, `FaceWrongSide`, ...).
    pub kind: &'static str,
    /// What is wrong, in words, with the problem's numbers.
    pub reason: String,
    /// The faces, edges and corners it is at.
    pub at: Vec<String>,
}

/// The problems of each body on its own (bodies may touch or overlap; the joined view's skins
/// then overlap without anything being wrong).
pub fn problems(ev: &mut Eval) -> Vec<Invalid> {
    let Some(solids) = ev.b.stages.last().map(|s| s.solids.clone()) else {
        return vec![];
    };
    let mut out = vec![];
    if solids.len() > 1 {
        for s in &solids {
            let body = forge_model::finish(s.body.clone());
            for p in check(&body) {
                out.push(invalid(ev, Some(s.id), &body, &p));
            }
        }
    } else if let Some(body) = ev.b.body().cloned() {
        for p in check(&body) {
            out.push(invalid(ev, solids.first().map(|s| s.id), &body, &p));
        }
    }
    out
}

fn kind(p: &Problem) -> &'static str {
    match p {
        Problem::LoopsCross { .. } => "LoopsCross",
        Problem::EdgeUses { .. } => "EdgeUses",
        Problem::EdgeUsesSameWay { .. } => "EdgeUsesSameWay",
        Problem::LoopEmpty { .. } => "LoopEmpty",
        Problem::LoopBroken { .. } => "LoopBroken",
        Problem::VertexOffEdge { .. } => "VertexOffEdge",
        Problem::VertexTolerance { .. } => "VertexTolerance",
        Problem::EdgeOffFace { .. } => "EdgeOffFace",
        Problem::EdgeDegenerate { .. } => "EdgeDegenerate",
        Problem::FaceWrongSide { .. } => "FaceWrongSide",
        Problem::DuplicateName { .. } => "DuplicateName",
        Problem::Euler { .. } => "Euler",
        Problem::FaceNoLoops { .. } => "FaceNoLoops",
        Problem::OuterNotFirst { .. } => "OuterNotFirst",
        Problem::ShellsOverlap { .. } => "ShellsOverlap",
        Problem::ShellEuler { .. } => "ShellEuler",
        Problem::VertexFans { .. } => "VertexFans",
    }
}

fn invalid(ev: &mut Eval, solid: Option<usize>, body: &Body, p: &Problem) -> Invalid {
    let owner = solid
        .and_then(|i| ev.feats.iter().find(|f| f.idx == i))
        .map_or_else(|| "the part".to_string(), |f| f.id.clone());
    let mut at = vec![];
    let face = |ev: &mut Eval, f: FaceId| face_text(ev, solid, body, f);
    let edge = |ev: &mut Eval, e: EdgeId| edge_text(ev, solid, body, e);
    match *p {
        Problem::LoopsCross { face: f }
        | Problem::FaceNoLoops { face: f }
        | Problem::OuterNotFirst { face: f }
        | Problem::ShellsOverlap { face: f } => at.push(face(ev, f)),
        Problem::FaceWrongSide { face: f, coedge } => {
            at.push(face(ev, f));
            at.push(edge(ev, body.coedge(coedge).edge));
        }
        Problem::EdgeOffFace { edge: e, face: f, .. } => {
            at.push(edge(ev, e));
            at.push(face(ev, f));
        }
        Problem::EdgeUses { edge: e, .. }
        | Problem::EdgeUsesSameWay { edge: e }
        | Problem::EdgeDegenerate { edge: e } => at.push(edge(ev, e)),
        Problem::VertexOffEdge { edge: e, vertex, .. } => {
            at.push(edge(ev, e));
            at.push(format!("corner {:?}", body.vertex(vertex).name));
        }
        Problem::LoopEmpty { lp } | Problem::LoopBroken { lp, .. } => at.push(face(ev, body.lp(lp).face)),
        Problem::VertexTolerance { vertex, .. } | Problem::VertexFans { vertex, .. } => {
            at.push(format!("corner {:?}", body.vertex(vertex).name))
        }
        Problem::DuplicateName { name } => at.push(format!("{name:?}")),
        Problem::ShellEuler { shell, .. } => {
            if let Some(&f) = body.shell(shell).faces.first() {
                at.push(face(ev, f));
            }
        }
        Problem::Euler { .. } => {}
    }
    Invalid {
        body: owner,
        kind: kind(p),
        reason: p.reason(),
        at,
    }
}

/// `name`, and the names merged into it.
fn merged(body: &Body, name: Name) -> Vec<Name> {
    let mut out = vec![name];
    out.extend(body.joins.aliases.iter().filter(|a| a.1 == name).map(|a| a.0));
    out
}

/// A sheet's built map on this body, with the salt its names carry there.
fn sheet_maps<'e>(ev: &'e Eval<'_>, solid: Option<usize>) -> Vec<(usize, &'e fs::Built, Option<Name>)> {
    let mut out = vec![];
    for (si, st) in ev.sheets.list.iter().enumerate() {
        if let Some(b) = &st.folded
            && Some(st.idx) == solid
        {
            out.push((si, b, None));
        }
        for u in &st.unfolds {
            let idx = ev.feats.iter().find(|f| f.id == u.id).map(|f| f.idx);
            if let Some(b) = &u.built
                && idx.is_some()
                && idx == solid
            {
                out.push((si, b, Some(Name::feature(u.hid))));
            }
        }
    }
    out
}

fn skin_text(s: Skin) -> &'static str {
    match s {
        Skin::Reference => "reference",
        Skin::Back => "back",
    }
}

fn strip_text(ev: &Eval, si: usize, s: Strip) -> String {
    let st = &ev.sheets.list[si];
    match s {
        Strip::Wall(w) => {
            let n = st.walls.iter().find(|x| x.0 == w).map_or("?", |x| x.1.as_str());
            format!("wall {n}")
        }
        Strip::Bend(j) => {
            let n = st.joints.iter().find(|x| x.0 == j).map_or("?", |x| x.1.as_str());
            format!("bend {n}")
        }
    }
}

fn trim_text(ev: &Eval, si: usize, t: TrimRef) -> String {
    let st = &ev.sheets.list[si];
    let wall = |w| st.walls.iter().find(|x| x.0 == w).map_or("?", |x| x.1.as_str());
    let joint = |j| st.joints.iter().find(|x| x.0 == j).map_or("?", |x| x.1.as_str());
    match t {
        TrimRef::Outline { wall: w, segment } => format!("outline edge {segment} of wall {}", wall(w)),
        TrimRef::Cut { cut, segment } => {
            let n = st.cuts.iter().find(|x| x.0 == cut).map_or("?", |x| x.1.as_str());
            format!("edge {segment} of cut {n}")
        }
        TrimRef::TangentLine { joint: j, .. } => format!("tangent line of bend {}", joint(j)),
        TrimRef::BendEnd { joint: j, .. } => format!("end of bend {}", joint(j)),
        TrimRef::MoldLine { joint: j, .. } => format!("mold line of bend {}", joint(j)),
        TrimRef::Relief { joint: j, .. } => format!("relief of bend {}", joint(j)),
        TrimRef::Corner { a, b, .. } => format!("corner relief of bends {} and {}", joint(a), joint(b)),
    }
}

fn face_text(ev: &mut Eval, solid: Option<usize>, body: &Body, f: FaceId) -> String {
    let names = merged(body, body.face(f).name);
    for (si, built, salt) in sheet_maps(ev, solid) {
        let on = |n: Name| salt.map_or(n, |s| n.salted(s));
        let at = |n: Name| names.iter().any(|&x| x == on(n) || x.is_piece_of(on(n)));
        if let Some(s) = built.roles.skins.iter().find(|s| at(s.2)) {
            return format!("{} skin of {}", skin_text(s.1), strip_text(ev, si, s.0));
        }
        if let Some(s) = built.roles.sides.iter().find(|s| at(s.1)) {
            return format!("side face at {}", trim_text(ev, si, s.0));
        }
    }
    let n = ev.b.built();
    let origin = ev.b.origin(n);
    let mut by: Vec<usize> = names
        .iter()
        .filter_map(|x| origin.get(x))
        .flatten()
        .copied()
        .collect();
    by.sort_unstable();
    by.dedup();
    let ids: Vec<String> = by
        .iter()
        .filter_map(|&i| ev.feats.iter().find(|f| f.idx == i).map(|f| f.id.clone()))
        .collect();
    if ids.is_empty() {
        format!("face {:?}", body.face(f).name)
    } else {
        format!("a face made by {}", ids.join(", "))
    }
}

fn edge_text(ev: &mut Eval, solid: Option<usize>, body: &Body, e: EdgeId) -> String {
    let names = merged(body, body.edge(e).name);
    for (si, built, salt) in sheet_maps(ev, solid) {
        let on = |n: Name| salt.map_or(n, |s| n.salted(s));
        if let Some(s) = built
            .roles
            .edges
            .iter()
            .find(|s| names.iter().any(|&x| x == on(s.2) || x.is_piece_of(on(s.2))))
        {
            return format!("{} edge at {}", skin_text(s.1), trim_text(ev, si, s.0));
        }
    }
    // the faces either side of it
    let faces: Vec<FaceId> = body
        .face_ids()
        .filter(|&f| {
            body.face(f)
                .loops
                .iter()
                .any(|&l| body.lp(l).coedges().any(|c| body.coedge(c).edge == e))
        })
        .collect();
    let sides: Vec<String> = faces
        .into_iter()
        .take(2)
        .map(|f| face_text(ev, solid, body, f))
        .collect();
    if sides.is_empty() {
        format!("edge {:?}", body.edge(e).name)
    } else {
        format!("the edge between {}", sides.join(" and "))
    }
}
