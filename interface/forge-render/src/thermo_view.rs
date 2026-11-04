//! A thermoform's wall thickness as world triangles over its formed part: the modelled sheet,
//! inside the trim, lifted off the tool by the sheet's thickness (just clear of the part's face),
//! each corner coloured by the thickness there.
use crate::origin::WorldVert;
use crate::shell::thermo::{ThermoRow, colour};
use forge_lang::thermo::ThermoOut;
use forge_math::{Point2, Vec3};

fn f3(p: forge_math::Point3) -> [f32; 3] {
    [p.x as f32, p.y as f32, p.z as f32]
}

/// The panel's row and the map's triangles.
pub fn view(o: &ThermoOut) -> (ThermoRow, Vec<WorldVert>) {
    let r = &o.formed.report;
    let mut facts = vec![
        (
            "material".to_string(),
            format!("{} at {:.0} C", o.material, o.temperature),
        ),
        ("sheet".into(), format!("{:.2} mm", o.t)),
        ("depth".into(), format!("{:.1} mm", r.depth)),
        ("areal draw ratio".into(), format!("{:.2}", r.adr)),
        ("mean wall".into(), format!("{:.2} mm", r.mean_wall)),
        ("vents".into(), format!("{}", r.vents.len())),
    ];
    let Some(w) = &o.walls else {
        return (
            ThermoRow {
                id: o.id.clone(),
                facts,
                ..ThermoRow::default()
            },
            vec![],
        );
    };
    facts.push(("unformed".into(), format!("{}", w.unformed.len())));
    let row = ThermoRow {
        id: o.id.clone(),
        facts,
        min: (w.min.0 * o.t, f3(w.min.1)),
        max: (w.max.0 * o.t, f3(w.max.1)),
        model: format!(
            "estimate: membrane, sticking contact, isothermal; law: {}{}",
            o.law.source,
            if o.law.stand_in { " (stand-in)" } else { "" }
        ),
    };
    let f = &o.formed;
    let poly = forge_thermo::build::polygon(&f.trim);
    let inside = |p: forge_math::Point3| {
        let l = f.frame.point_to_local(p);
        forge_thermo::build::inside(&poly, Point2::new(l.x, l.y)).0
    };
    // each node's normal, off the tool (the sheet's triangles run counter-clockwise seen from it)
    let mut n = vec![Vec3::ZERO; w.nodes.len()];
    for t in &w.tris {
        let [a, b, c] = t.map(|k| w.nodes[k as usize]);
        let v = (b - a).cross(c - a);
        for &k in t {
            n[k as usize] += v;
        }
    }
    let (lo, hi) = (w.min.0, w.max.0);
    let lift = o.t * 1.05 + 0.1;
    let mut out = vec![];
    for t in &w.tris {
        let [a, b, c] = t.map(|k| w.nodes[k as usize]);
        let mid = forge_math::Point3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );
        if !inside(mid) {
            continue;
        }
        for &k in t {
            let k = k as usize;
            let up = n[k].normalize().map_or(Vec3::ZERO, |u| u.get());
            let p = w.nodes[k] + up * lift;
            let v = if hi > lo {
                ((w.node_ratio[k] - lo) / (hi - lo)) as f32
            } else {
                0.5
            };
            let c = colour(v);
            out.push(WorldVert {
                at: f3(p),
                colour: [c[0], c[1], c[2], 1.0],
            });
        }
    }
    (row, out)
}
