//! The standard tooling sets, in our own round numbers (no catalogue's figures): a European
//! (Promecam-style) set and an American set, each with straight, gooseneck and acute punches, a
//! range of V dies and a press. Ids are `SET/TOOL`.
use crate::tooling::{
    Backgauge, Beam, Die, Finger, Press, Punch, PunchKind, Relief, Segment, Springback, Style, Tooling,
};
use forge_math::dmath;

fn deg(d: f64) -> f64 {
    d * dmath::PI / 180.0
}

/// Sectioned lengths: a split set totalling 835 mm and two whole 835s (1670 + 835 = 2505 mm).
fn eu_segments() -> Vec<Segment> {
    [
        (10.0, 2),
        (15.0, 1),
        (20.0, 2),
        (40.0, 1),
        (50.0, 2),
        (100.0, 1),
        (200.0, 1),
        (300.0, 1),
        (835.0, 2),
    ]
    .into_iter()
    .map(|(length, count)| Segment { length, count })
    .collect()
}

/// Inch sections: 1, 1.5, 2, 3, 4, 6 inches, two each, and three 24-inch bars.
fn us_segments() -> Vec<Segment> {
    [
        (25.4, 2),
        (38.1, 2),
        (50.8, 2),
        (76.2, 2),
        (101.6, 2),
        (152.4, 2),
        (609.6, 3),
    ]
    .into_iter()
    .map(|(length, count)| Segment { length, count })
    .collect()
}

#[allow(clippy::too_many_arguments)]
fn punch(
    set: &str,
    name: &str,
    style: Style,
    kind: PunchKind,
    height: f64,
    tip_radius: f64,
    tip_angle_deg: f64,
    tip_height: f64,
    width: f64,
    relief: Option<Relief>,
    max_load: f64,
) -> Punch {
    Punch {
        id: format!("{set}/{name}"),
        style,
        kind,
        height,
        tip_radius,
        tip_angle: deg(tip_angle_deg),
        tip_height,
        width,
        relief,
        profile: None,
        segments: if style == Style::Eu {
            eu_segments()
        } else {
            us_segments()
        },
        max_load: Some(max_load),
    }
}

#[allow(clippy::too_many_arguments)]
fn die(
    set: &str,
    style: Style,
    v: f64,
    angle_deg: f64,
    rs: f64,
    height: f64,
    width: f64,
    max_load: f64,
) -> Die {
    Die {
        id: format!("{set}/V{}-{}", fmt_len(v), angle_deg as i64),
        style,
        v,
        angle: deg(angle_deg),
        shoulder_radius: rs,
        height,
        width,
        floor: None,
        profile: None,
        segments: if style == Style::Eu {
            eu_segments()
        } else {
            us_segments()
        },
        max_load: Some(max_load),
    }
}

fn fmt_len(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

/// Springback rows every set starts with: mild steel, stainless and aluminium, degrees of
/// overbend at a 90 degree turn (stated as round shop numbers: a stand-in, not a maker's data).
pub fn springback_standin() -> Vec<Springback> {
    let row = |m: &str, a: f64, o: f64| Springback {
        material: m.into(),
        angle: deg(a),
        overbend: deg(o),
    };
    vec![
        row("steel", 30.0, 0.5),
        row("steel", 90.0, 1.5),
        row("stainless", 30.0, 1.0),
        row("stainless", 90.0, 3.0),
        row("aluminium", 30.0, 0.5),
        row("aluminium", 90.0, 2.0),
    ]
}

pub fn std_eu() -> Tooling {
    let s = "std-eu";
    let e = Style::Eu;
    let p = |n, k, h, r, a, ht, w, rel, l| punch(s, n, e, k, h, r, a, ht, w, rel, l);
    let punches = vec![
        p(
            "P88-R08",
            PunchKind::Straight,
            100.0,
            0.8,
            88.0,
            12.0,
            26.0,
            None,
            1000.0,
        ),
        p(
            "P88-R3",
            PunchKind::Straight,
            100.0,
            3.0,
            88.0,
            14.0,
            26.0,
            None,
            1000.0,
        ),
        p(
            "P85-R08-H120",
            PunchKind::Straight,
            120.0,
            0.8,
            85.0,
            12.0,
            20.0,
            None,
            800.0,
        ),
        p(
            "G88-R08",
            PunchKind::Gooseneck,
            135.0,
            0.8,
            88.0,
            12.0,
            26.0,
            Some(Relief {
                depth: 40.0,
                z0: 25.0,
                z1: 105.0,
            }),
            500.0,
        ),
        p(
            "A30-R04",
            PunchKind::Acute,
            100.0,
            0.4,
            30.0,
            25.0,
            20.0,
            None,
            400.0,
        ),
    ];
    let d = |v, a, rs, h, w, l| die(s, e, v, a, rs, h, w, l);
    let dies = vec![
        d(6.0, 88.0, 0.6, 60.0, 30.0, 400.0),
        d(8.0, 88.0, 0.8, 60.0, 30.0, 500.0),
        d(10.0, 88.0, 1.0, 60.0, 40.0, 600.0),
        d(12.0, 88.0, 1.5, 60.0, 40.0, 700.0),
        d(16.0, 88.0, 2.0, 60.0, 50.0, 800.0),
        d(20.0, 88.0, 2.5, 60.0, 60.0, 900.0),
        d(25.0, 88.0, 3.0, 70.0, 70.0, 1000.0),
        d(32.0, 88.0, 4.0, 80.0, 80.0, 1200.0),
        d(8.0, 30.0, 0.6, 60.0, 30.0, 300.0),
        d(12.0, 30.0, 1.0, 60.0, 40.0, 400.0),
    ];
    Tooling {
        id: s.into(),
        punches,
        dies,
        press: Press {
            id: format!("{s}/press-100t-3m"),
            open_height: 360.0,
            stroke: 220.0,
            bed_length: 3000.0,
            housing_gap: 2600.0,
            ram: Beam {
                front: 60.0,
                back: 60.0,
            },
            bed: Beam {
                front: 60.0,
                back: 60.0,
            },
            max_force: Some(1000.0),
            backgauge: Backgauge {
                fingers: 2,
                x: (5.0, 600.0),
                r: (0.0, 150.0),
                finger: Finger {
                    depth: 25.0,
                    height: 20.0,
                    width: 15.0,
                },
                retract: 5.0,
                z: None,
            },
            frame: None,
            unchecked: Vec::new(),
            estimated: Vec::new(),
        },
        clearance: 0.5,
        self_clearance: 0.0,
        short_end: None,
        springback: springback_standin(),
        v_rule: 8.0,
        stations: Vec::new(),
    }
}

pub fn std_us() -> Tooling {
    let s = "std-us";
    let u = Style::Us;
    let inch = 25.4;
    let p = |n, k, h, r, a, ht, w, rel, l| punch(s, n, u, k, h, r, a, ht, w, rel, l);
    let punches = vec![
        p(
            "S85-R1",
            PunchKind::Straight,
            4.0 * inch,
            1.0,
            85.0,
            12.0,
            0.75 * inch,
            None,
            900.0,
        ),
        p(
            "S90-R1",
            PunchKind::Straight,
            4.0 * inch,
            1.0,
            90.0,
            12.0,
            0.75 * inch,
            None,
            900.0,
        ),
        p(
            "G85-R1",
            PunchKind::Gooseneck,
            5.5 * inch,
            1.0,
            85.0,
            12.0,
            0.75 * inch,
            Some(Relief {
                depth: 1.5 * inch,
                z0: 1.0 * inch,
                z1: 4.0 * inch,
            }),
            450.0,
        ),
        p(
            "A30-R05",
            PunchKind::Acute,
            4.0 * inch,
            0.5,
            30.0,
            25.0,
            0.75 * inch,
            None,
            350.0,
        ),
    ];
    let d = |v: f64, a, rs, h, w, l| die(s, u, v * inch, a, rs, h, w, l);
    let dies = vec![
        d(0.25, 85.0, 0.8, 2.5 * inch, 1.25 * inch, 450.0),
        d(0.375, 85.0, 1.0, 2.5 * inch, 1.25 * inch, 550.0),
        d(0.5, 85.0, 1.5, 2.5 * inch, 1.5 * inch, 650.0),
        d(0.625, 85.0, 2.0, 2.5 * inch, 1.75 * inch, 750.0),
        d(0.75, 85.0, 2.5, 2.5 * inch, 2.0 * inch, 850.0),
        d(1.0, 85.0, 3.0, 3.0 * inch, 2.5 * inch, 1000.0),
        d(1.25, 85.0, 4.0, 3.0 * inch, 3.0 * inch, 1100.0),
        d(0.5, 90.0, 1.5, 2.5 * inch, 1.5 * inch, 650.0),
        d(0.375, 30.0, 0.8, 2.5 * inch, 1.25 * inch, 300.0),
        d(0.5, 30.0, 1.0, 2.5 * inch, 1.5 * inch, 350.0),
    ];
    Tooling {
        id: s.into(),
        punches,
        dies,
        press: Press {
            id: format!("{s}/press-90t-10ft"),
            open_height: 15.0 * inch,
            stroke: 9.0 * inch,
            bed_length: 120.0 * inch,
            housing_gap: 102.0 * inch,
            ram: Beam {
                front: 2.5 * inch,
                back: 2.5 * inch,
            },
            bed: Beam {
                front: 2.5 * inch,
                back: 2.5 * inch,
            },
            max_force: Some(800.0),
            backgauge: Backgauge {
                fingers: 2,
                x: (0.25 * inch, 24.0 * inch),
                r: (0.0, 6.0 * inch),
                finger: Finger {
                    depth: 1.0 * inch,
                    height: 0.75 * inch,
                    width: 0.625 * inch,
                },
                retract: 0.25 * inch,
                z: None,
            },
            frame: None,
            unchecked: Vec::new(),
            estimated: Vec::new(),
        },
        clearance: 0.5,
        self_clearance: 0.0,
        short_end: None,
        springback: springback_standin(),
        v_rule: 8.0,
        stations: Vec::new(),
    }
}

/// Every standard set.
pub fn sets() -> Vec<Tooling> {
    vec![std_eu(), std_us()]
}

/// A standard set by id.
pub fn set(id: &str) -> Option<Tooling> {
    sets().into_iter().find(|t| t.id == id)
}

pub const IDS: [&str; 2] = ["std-eu", "std-us"];
