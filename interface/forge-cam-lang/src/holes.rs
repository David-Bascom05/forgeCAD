//! The part's hole features as CAM reads them: the design's own numbers, from its history.

/// A hole feature as CAM reads it: the design's own numbers, not a recognition (research/cam,
/// "feature-aware"). Positions are each place's mouth centre in the part's frame.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleInfo {
    /// The statement's name (`bolts`).
    pub name: String,
    pub places: Vec<[f64; 3]>,
    /// The drilling direction (into the material).
    pub dir: [f64; 3],
    pub d: f64,
    /// The wall's depth; None: through.
    pub depth: Option<f64>,
    /// The drill point's included angle (blind holes), degrees.
    pub point: Option<f64>,
    /// A thread: (major diameter, pitch).
    pub thread: Option<(f64, f64)>,
    /// The standard's screw and fit, when the hole came from one (`M6`, "close" | "normal" |
    /// "loose" | "tap").
    pub screw: Option<String>,
    pub fit: Option<&'static str>,
    /// A counterbore (d, depth) or countersink (d, included angle in degrees).
    pub counterbore: Option<(f64, f64)>,
    pub countersink: Option<(f64, f64)>,
    /// The ISO 286 hole class the part gives a plain hole (`fit: "H7"`): it is reamed.
    pub class: Option<&'static str>,
}

/// Every hole feature of an evaluation, in statement order.
pub fn hole_infos(ev: &forge_lang::eval::Eval) -> Vec<HoleInfo> {
    let mut out = Vec::new();
    for f in &ev.feats {
        if f.kind != "hole" {
            continue;
        }
        let Some(forge_model::Feature::Hole(h)) = forge_lang::holes::kernel_feature(ev, f.idx) else {
            continue;
        };
        let o = h.at.origin;
        let z = h.at.z().get();
        let mut places = vec![[o.x, o.y, o.z]];
        if let Some(forge_model::At::Sketch { at, .. }) = &h.places {
            for p in at.iter().skip(1) {
                let q = o + (*p - at[0]);
                places.push([q.x, q.y, q.z]);
            }
        }
        out.push(HoleInfo {
            name: f.id.clone(),
            places,
            dir: [z.x, z.y, z.z],
            d: h.diameter,
            depth: match h.end {
                forge_model::End::Depth(d) => Some(d),
                _ => None,
            },
            point: h.point.map(f64::to_degrees),
            thread: h.thread.as_ref().map(|t| (t.size, t.pitch)),
            screw: h.standard.map(|s| s.screw.name.to_string()),
            fit: h.standard.map(|s| match s.fit {
                forge_model::standard::Fit::Close => "close",
                forge_model::standard::Fit::Normal => "normal",
                forge_model::standard::Fit::Loose => "loose",
                forge_model::standard::Fit::Tap => "tap",
            }),
            counterbore: match h.head {
                forge_model::Head::Counterbore { diameter, depth } => Some((diameter, depth)),
                _ => None,
            },
            countersink: match h.head {
                forge_model::Head::Countersink { diameter, angle } => Some((diameter, angle.to_degrees())),
                _ => None,
            },
            class: ev.hole_fits.iter().find(|(id, _)| *id == f.id).map(|(_, c)| *c),
        });
    }
    out
}

/// The hole features of `src`, as CAM plans them (`holes::hole_infos`): an evaluation of its
/// own, so an ordinary build does no work for CAM (and forge-lang carries none of it).
pub fn hole_view(src: &str, opts: forge_lang::Options) -> Vec<HoleInfo> {
    let Ok(prog) = forge_lang::syntax::parse(src) else {
        return vec![];
    };
    let mut ev = forge_lang::eval::Eval::new(src, opts.rules, opts.build, Default::default());
    ev.dir = opts.dir.clone();
    ev.root = forge_lang::fence_of(&opts);
    ev.asm = Some(forge_lang::asm::Doc::default());
    ev.check_static(&prog);
    if !ev.errors.is_empty() {
        ev.b.building = false;
    }
    ev.run(&prog);
    hole_infos(&ev)
}
