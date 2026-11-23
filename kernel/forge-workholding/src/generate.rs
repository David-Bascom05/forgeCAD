//! Generated workholding: a part you machine, with its own setup (as forge-cam's soft jaws are).
//!
//! - a fixture plate made from the part: located on two of the part's through holes by a round
//!   pin and a diamond pin (the diamond relieved across the line between them, so the pair
//!   locates without over-constraining), held by toe clamps at the middles of the outline's two
//!   long sides, the plate given as its own `.fg` part (its pin holes drilled through);
//! - step jaws: soft-jaw blanks with a ledge the stock seats on (no parallels);
//! - lathe soft jaws bored to the part's diameter (the boring region, the grip check).
//!
//! The pin choice is a rule, stated: the two holes farthest apart (the longest baseline turns the
//! part least for a pin's clearance); ties go to the earlier pair in the given order.
use crate::geom::Prim;
use crate::lathe::{Ring, Spin};
use crate::{Body, Box3, Kind, Mount, Placed, Record, Role, WhError};

fn refuse(device: &str, what: impl Into<String>, fix: impl Into<String>) -> WhError {
    WhError::Placement {
        device: device.to_string(),
        what: what.into(),
        fix: fix.into(),
    }
}

/// A hole of the part, through it, in the part frame (xy).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hole {
    pub at: [f64; 2],
    pub d: f64,
}

/// A generated fixture plate.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateFixture {
    pub placed: Placed,
    /// The round pin's hole and the diamond pin's (indices into the holes given).
    pub pins: [usize; 2],
    /// Toe-clamp places on the outline (xy), each with the push direction into the part.
    pub clamps: Vec<([f64; 2], [f64; 2])>,
    /// The plate as a part (`.fg`), its corner at the origin.
    pub fg: String,
    /// The plate's corner in the part frame (xy) and its top (z).
    pub corner: [f64; 2],
    pub top: f64,
}

/// A fixture plate for a part whose outline's box is `part` (its underside at `part.min[2]`):
/// the plate overhangs the outline by `margin` each side and is `t` thick; pins of `fit` less
/// than each hole's diameter stand `pin_h` into the holes.
pub fn plate_from_part(
    name: &str,
    part: &Box3,
    holes: &[Hole],
    margin: f64,
    t: f64,
    fit: f64,
    pin_h: f64,
) -> Result<PlateFixture, WhError> {
    if holes.len() < 2 {
        return Err(refuse(
            name,
            "a plate locates the part on two of its through holes",
            "a part with two holes, or a vise",
        ));
    }
    if !(margin >= 0.0 && t > 0.0 && fit >= 0.0 && pin_h > 0.0) {
        return Err(refuse(
            name,
            "margin ≥ 0, thickness > 0, fit ≥ 0, pin height > 0",
            "give them",
        ));
    }
    let mut best = (0.0, [0, 1]);
    for i in 0..holes.len() {
        for j in i + 1..holes.len() {
            let (a, b) = (holes[i].at, holes[j].at);
            let d = (a[0] - b[0]).hypot(a[1] - b[1]);
            if d > best.0 {
                best = (d, [i, j]);
            }
        }
    }
    let [i, j] = best.1;
    if best.0 <= 0.0 {
        return Err(refuse(name, "the holes coincide", "two holes apart"));
    }
    for k in [i, j] {
        if holes[k].d - fit <= 0.0 {
            return Err(refuse(
                name,
                format!("hole {k} ⌀{} is no larger than the fit", holes[k].d),
                "a smaller fit",
            ));
        }
    }
    let z = part.min[2];
    let (x0, y0) = (part.min[0] - margin, part.min[1] - margin);
    let (l, w) = (
        part.max[0] - part.min[0] + 2.0 * margin,
        part.max[1] - part.min[1] + 2.0 * margin,
    );
    let mut bodies = vec![Body::new(
        "plate",
        Role::Plate,
        vec![Prim::aabb([x0, y0, z - t], [x0 + l, y0 + w, z])],
    )];
    for (k, which) in [(i, "round pin"), (j, "diamond pin")] {
        let h = holes[k];
        bodies.push(Body::new(
            which,
            Role::Dowel,
            vec![Prim::post(h.at[0], h.at[1], 0.5 * (h.d - fit), z, z + pin_h)],
        ));
    }
    // toe clamps at the middles of the two long sides, pushing in
    let m = part.mid();
    let clamps = if part.size()[0] >= part.size()[1] {
        vec![
            ([m[0], part.min[1]], [0.0, 1.0]),
            ([m[0], part.max[1]], [0.0, -1.0]),
        ]
    } else {
        vec![
            ([part.min[0], m[1]], [1.0, 0.0]),
            ([part.max[0], m[1]], [-1.0, 0.0]),
        ]
    };
    // the plate as a part: corner at the origin, pin holes through
    let mut fg = format!(
        "# {name}: a fixture plate generated for a part (forge-workholding generate.rs): round pin\n\
         # in hole {i}, diamond pin in hole {j}; corner at the origin, the part's frame shifted by\n\
         # ({x0}, {y0}, {}).\n\
         shared L = {l}\nshared W = {w}\nshared T = {t}\n",
        z - t
    );
    for (n, k) in [("round", i), ("diamond", j)] {
        let h = holes[k];
        fg += &format!(
            "shared {n}_x = {}\nshared {n}_y = {}\nshared {n}_d = {}\n",
            h.at[0] - x0,
            h.at[1] - y0,
            h.d - fit
        );
    }
    fg += "\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (L, W))\n}\nplate = extrude(plan, T)\n\
           sketch pins(on: plate.end) {\n  r = pt(round_x, round_y)\n  d = pt(diamond_x, diamond_y)\n}\n\
           round = hole(at: pins.r, d: round_d, depth: thru)\ndiamond = hole(at: pins.d, d: diamond_d, depth: thru)\n";
    Ok(PlateFixture {
        placed: Placed {
            name: name.to_string(),
            kind: Kind::FixturePlate,
            bodies,
            mount: Mount::Table { z: z - t },
            held: vec![],
            notes: vec![format!(
                "generated plate {l:.1} × {w:.1} × {t}: round pin in hole {i}, diamond pin in hole {j} ({:.1} apart), toe clamps at the long sides' middles",
                best.0
            )],
        },
        pins: [i, j],
        clamps,
        fg,
        corner: [x0, y0],
        top: z,
    })
}

/// Step jaws: soft-jaw blanks on a vise, each with a ledge `ledge_d` deep under the stock's
/// edge, the stock seated on the ledges and gripped `clamp` above them (no parallels).
pub fn step_jaws(
    name: &str,
    v: &Record,
    blank: &Record,
    stock: &Box3,
    axis: usize,
    clamp: f64,
    ledge_d: f64,
) -> Result<Placed, WhError> {
    crate::mill::vise_with(name, v, stock, axis, clamp, None, Some(blank), Some(ledge_d))
}

/// Lathe soft jaws bored to grip ⌀`grip_d` over `depth` behind the jaws' face at `z_face`: the
/// chuck's jaws as soft jaws (the chuck record's jaw sizes), the region the boring op cuts out of
/// them (ρ, z), and the check that the bore fits in the jaws (a wall of at least `wall` left).
pub struct BoredJaws {
    pub spin: Spin,
    /// The region to bore: ρ from the jaws' inner face as closed to grip_d / 2, z over the depth.
    pub bore: Ring,
}

pub fn bored_jaws(
    name: &str,
    chuck: &Record,
    closed_d: f64,
    grip_d: f64,
    z_face: f64,
    depth: f64,
    wall: f64,
) -> Result<BoredJaws, WhError> {
    if chuck.kind != Kind::Chuck {
        return Err(refuse(
            &chuck.name,
            format!("is a {}", chuck.kind.name()),
            "name a chuck",
        ));
    }
    let (jh, jl) = (chuck.get("jaw_height")?, chuck.get("jaw_len")?);
    let r0 = 0.5 * closed_d;
    let r1 = 0.5 * grip_d;
    if !(grip_d > closed_d) {
        return Err(refuse(
            name,
            format!("the bore ⌀{grip_d:.2} is not larger than the jaws as closed (⌀{closed_d:.2})"),
            "close the jaws further before boring (on a plug or ring)",
        ));
    }
    if r1 > r0 + jh - wall {
        return Err(refuse(
            name,
            format!("the bore ⌀{grip_d:.2} leaves less than {wall:.2} of jaw"),
            "taller soft jaws, or grip the part elsewhere",
        ));
    }
    if !(depth > 0.0 && depth <= jl) {
        return Err(refuse(
            name,
            format!("bore depth {depth:.2} within the jaws' {jl:.2}"),
            "SetArg depth",
        ));
    }
    // gripping the part, the jaws open to the bore: their envelope from r1 outward
    let mut spin = crate::lathe::chuck_jaws(name, chuck, grip_d, z_face, depth, false)?;
    spin.notes.push(format!(
        "soft jaws bored to ⌀{grip_d:.2} over {depth:.2} (closed to ⌀{closed_d:.2} to bore)"
    ));
    Ok(BoredJaws {
        spin,
        bore: Ring {
            name: "bore".into(),
            role: Role::Jaw,
            poly: vec![
                [r0, z_face - depth],
                [r1, z_face - depth],
                [r1, z_face],
                [r0, z_face],
            ],
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Library;

    #[test]
    fn a_plate_from_a_parts_holes() {
        let part = Box3 {
            min: [0.0, 0.0, 0.0],
            max: [120.0, 60.0, 15.0],
        };
        let holes = [
            Hole {
                at: [10.0, 10.0],
                d: 8.0,
            },
            Hole {
                at: [60.0, 30.0],
                d: 10.0,
            },
            Hole {
                at: [110.0, 50.0],
                d: 8.0,
            },
        ];
        let p = plate_from_part("plate", &part, &holes, 20.0, 20.0, 0.02, 8.0).unwrap();
        assert_eq!(p.pins, [0, 2]);
        assert_eq!(p.clamps[0].0, [60.0, 0.0]);
        assert!(p.fg.contains("round = hole(") && p.fg.contains("shared round_d = 7.98"));
        assert!(plate_from_part("p", &part, &holes[..1], 20.0, 20.0, 0.02, 8.0).is_err());
    }

    #[test]
    fn step_jaws_and_bored_jaws() {
        let l = Library::builtin();
        let s = Box3 {
            min: [0.0; 3],
            max: [80.0, 60.0, 30.0],
        };
        let p = step_jaws(
            "v",
            l.get("standin-vise-150").unwrap(),
            l.get("standin-soft-al-150").unwrap(),
            &s,
            0,
            10.0,
            4.0,
        )
        .unwrap();
        assert_eq!(
            p.bodies.iter().filter(|b| b.name.starts_with("step jaw")).count(),
            2
        );
        assert!(!p.bodies.iter().any(|b| b.role == Role::Parallel));
        let b = bored_jaws(
            "c",
            l.get("standin-chuck-200").unwrap(),
            40.0,
            50.0,
            0.0,
            15.0,
            3.0,
        )
        .unwrap();
        assert_eq!(b.bore.poly[1], [25.0, -15.0]);
        assert!(
            bored_jaws(
                "c",
                l.get("standin-chuck-200").unwrap(),
                40.0,
                110.0,
                0.0,
                15.0,
                3.0
            )
            .is_err()
        );
        assert!(
            bored_jaws(
                "c",
                l.get("standin-chuck-200").unwrap(),
                40.0,
                30.0,
                0.0,
                15.0,
                3.0
            )
            .is_err()
        );
    }
}
