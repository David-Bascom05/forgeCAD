//! Standard holes for screws, as data: clearance diameters (close, normal, loose), tap drills,
//! and counterbores and countersinks for the screw's head, ISO metric and ANSI/UTS inch, each in
//! two conventions picked per hole, as CAD hole wizards offer them: ISO (DIN) and ANSI.
//! The published standards are paid; the values were checked against the reproductions cited
//! above the table (2026-10-30), and the DIN ones are marked where only widely published.
//!
//! Counterbores take a socket head cap screw (ISO 4762, ASME B18.3) as deep as its head is high
//! (k = d), so the head sits flush; countersinks a socket flat head (ISO 10642, ISO 2009 below
//! M3, ASME B18.3 Table 2A) as wide as its theoretical sharp head, 90 and 82 degrees.

use crate::hole::{Head, Thread};
use forge_math::dmath::PI;

/// One screw size and thread.
#[derive(Debug, PartialEq)]
pub struct Screw {
    /// As written: M6, M8x1, 1/4-20, 10-32 (a numbered size: no `#`, which starts a comment).
    pub name: &'static str,
    /// Major diameter, mm.
    pub size: f64,
    pub pitch: f64,
    /// Close, normal and loose clearance diameters (the same in both conventions).
    pub clearance: [f64; 3],
    /// Tap drill and counterbore (diameter, depth), by convention (ISO, ANSI).
    pub tap: [f64; 2],
    pub counterbore: [(f64, f64); 2],
    /// Diameter and included angle; None where no flat head screw is standard.
    pub countersink: Option<(f64, f64)>,
}

/// Whose tables a hole follows: ISO (with DIN 336 tap drills and DIN 974-1 counterbores), or
/// ANSI (ASME B18.2.8 clearances, tap drills by the thread, ASME B18.3 counterbores). Inch
/// sizes have only ANSI's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    Iso,
    Ansi,
}

/// How a hole fits its screw: a clearance hole, or drilled to tap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Close,
    Normal,
    Loose,
    Tap,
}

/// A hole sized from the tables: which screw, whose tables, and how it fits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Standard {
    pub screw: &'static Screw,
    pub convention: Convention,
    pub fit: Fit,
}

impl Standard {
    pub fn diameter(&self) -> f64 {
        match self.fit {
            Fit::Close => self.screw.clearance[0],
            Fit::Normal => self.screw.clearance[1],
            Fit::Loose => self.screw.clearance[2],
            Fit::Tap => self.screw.tap[self.convention as usize],
        }
    }

    /// The thread a tapped hole records.
    pub fn thread(&self) -> Option<Thread> {
        (self.fit == Fit::Tap).then_some(Thread {
            size: self.screw.size,
            pitch: self.screw.pitch,
        })
    }

    /// The seat for the screw's socket head.
    pub fn counterbore(&self) -> Head {
        let (diameter, depth) = self.screw.counterbore[self.convention as usize];
        Head::Counterbore { diameter, depth }
    }

    /// The seat for the screw's flat head, if one is standard.
    pub fn countersink(&self) -> Option<Head> {
        self.screw
            .countersink
            .map(|(diameter, angle)| Head::Countersink { diameter, angle })
    }
}

impl Screw {
    pub fn inch(&self) -> bool {
        !self.name.starts_with('M')
    }

    /// The convention a hole of this size follows unless told: ISO for metric, ANSI for inch.
    pub fn default_convention(&self) -> Convention {
        if self.inch() {
            Convention::Ansi
        } else {
            Convention::Iso
        }
    }
}

/// The screw a size names, written in full (M8x1, 1/4-20): as an expression an inch size would
/// be a negative diameter, so a size never shadows a hole's diameter.
pub fn screw(name: &str) -> Option<&'static Screw> {
    SCREWS.iter().find(|s| s.name == name)
}

/// A metric row: DIN 336 tap drill (ANSI's is the major diameter less the pitch), counterbores
/// per DIN 974-1 and ASME B18.3.1M, countersink (0: none).
const fn metric(
    name: &'static str,
    d: f64,
    p: f64,
    clear: [f64; 3],
    tap: f64,
    cb: [f64; 2],
    cs: f64,
) -> Screw {
    Screw {
        name,
        size: d,
        pitch: p,
        clearance: clear,
        tap: [tap, d - p],
        counterbore: [(cb[0], d), (cb[1], d)],
        countersink: if cs > 0.0 { Some((cs, PI / 2.0)) } else { None },
    }
}

const IN: f64 = 25.4;

/// An inch row, in inches; ANSI's tables in both places.
const fn inch(name: &'static str, d: f64, tpi: f64, clear: [f64; 3], tap: f64, cb: f64, cs: f64) -> Screw {
    Screw {
        name,
        size: d * IN,
        pitch: IN / tpi,
        clearance: [clear[0] * IN, clear[1] * IN, clear[2] * IN],
        tap: [tap * IN, tap * IN],
        counterbore: [(cb * IN, d * IN), (cb * IN, d * IN)],
        countersink: if cs > 0.0 {
            Some((cs * IN, 82.0 / 180.0 * PI))
        } else {
            None
        },
    }
}

// Metric, mm. Clearances: ISO 273 close/normal/loose (ASME B18.2.8's metric table is the same:
// amesweb.info/Screws/Metric-Clearance-Hole-Chart.aspx), as reproduced at
// albanycountyfasteners.com/media/6b/b8/g0/1764944339/clearance-hole-chart.pdf (M1.6-M30) and
// optimas.com/technical-resources/tapping-sizes (M36). Tap drills (ISO): DIN 336, the widely
// published values (6.8 for M8); the Optimas chart agrees except M8 6.75, M10x1.25 8.75,
// M12x1.25 10.75, which ANSI's rule gives. Counterbores: ISO, DIN 974-1 series for ISO 4762, the
// widely published values (the standard's own table could not be read); ANSI, ASME B18.3.1M /
// Machinery's Handbook, amesweb.info/Screws/Metric-Counterbore-Hole-Dimensions-Chart.aspx.
// Countersinks: ISO 10642 dk theoretical max, engineersedge.com/hardware/bs_en_iso_10642_14583.htm
// (none standard past M20); ISO 2009 dk theoretical max for M1.6-M2.5,
// globalfastener.com/standards/detail_133.html.
// Inch, in. Clearances: ASME B18.2.8-1999 minimum diameters, amesweb.info/screws/
// Clearance-Hole-Chart.aspx (the same at jandesupply.com/reference-clearance-holes.php). Tap
// drills: 75% thread, medesign.seas.upenn.edu/index.php/Guides.TapDrillChart (to 1/2) and
// faculty.etsu.edu/hemphill/tap_drill.htm, tameson.com/pages/tap-drill-chart (7/16 up).
// Counterbores: ASME B18.3 Appendix A, engineersedge.com/mechanical,045tolerances/
// drill_and_counterbore_sizes_14163.htm. Countersinks: ASME B18.3 Table 2A dk max theoretical
// sharp, fasten.it/en/norms/norm/asme_b18.3_table_2a.
pub static SCREWS: &[Screw] = &[
    metric("M1.6", 1.6, 0.35, [1.7, 1.8, 2.0], 1.25, [3.5, 3.5], 3.6),
    metric("M2", 2.0, 0.4, [2.2, 2.4, 2.6], 1.6, [4.4, 4.4], 4.4),
    metric("M2.5", 2.5, 0.45, [2.7, 2.9, 3.1], 2.05, [5.5, 5.4], 5.5),
    metric("M3", 3.0, 0.5, [3.2, 3.4, 3.6], 2.5, [6.5, 6.5], 6.72),
    metric("M4", 4.0, 0.7, [4.3, 4.5, 4.8], 3.3, [8.0, 8.25], 8.96),
    metric("M5", 5.0, 0.8, [5.3, 5.5, 5.8], 4.2, [10.0, 9.75], 11.2),
    metric("M6", 6.0, 1.0, [6.4, 6.6, 7.0], 5.0, [11.0, 11.25], 13.44),
    metric("M8", 8.0, 1.25, [8.4, 9.0, 10.0], 6.8, [15.0, 14.25], 17.92),
    metric("M8x1", 8.0, 1.0, [8.4, 9.0, 10.0], 7.0, [15.0, 14.25], 17.92),
    metric("M10", 10.0, 1.5, [10.5, 11.0, 12.0], 8.5, [18.0, 17.25], 22.4),
    metric(
        "M10x1.25",
        10.0,
        1.25,
        [10.5, 11.0, 12.0],
        8.8,
        [18.0, 17.25],
        22.4,
    ),
    metric("M10x1", 10.0, 1.0, [10.5, 11.0, 12.0], 9.0, [18.0, 17.25], 22.4),
    metric("M12", 12.0, 1.75, [13.0, 13.5, 14.5], 10.2, [20.0, 19.25], 26.88),
    metric(
        "M12x1.5",
        12.0,
        1.5,
        [13.0, 13.5, 14.5],
        10.5,
        [20.0, 19.25],
        26.88,
    ),
    metric(
        "M12x1.25",
        12.0,
        1.25,
        [13.0, 13.5, 14.5],
        10.8,
        [20.0, 19.25],
        26.88,
    ),
    metric("M14", 14.0, 2.0, [15.0, 15.5, 16.5], 12.0, [24.0, 22.25], 30.8),
    metric(
        "M14x1.5",
        14.0,
        1.5,
        [15.0, 15.5, 16.5],
        12.5,
        [24.0, 22.25],
        30.8,
    ),
    metric("M16", 16.0, 2.0, [17.0, 17.5, 18.5], 14.0, [26.0, 25.5], 33.6),
    metric("M16x1.5", 16.0, 1.5, [17.0, 17.5, 18.5], 14.5, [26.0, 25.5], 33.6),
    metric("M20", 20.0, 2.5, [21.0, 22.0, 24.0], 17.5, [33.0, 31.5], 40.32),
    metric(
        "M20x1.5",
        20.0,
        1.5,
        [21.0, 22.0, 24.0],
        18.5,
        [33.0, 31.5],
        40.32,
    ),
    metric("M24", 24.0, 3.0, [25.0, 26.0, 28.0], 21.0, [40.0, 37.5], 0.0),
    metric("M24x2", 24.0, 2.0, [25.0, 26.0, 28.0], 22.0, [40.0, 37.5], 0.0),
    metric("M30", 30.0, 3.5, [31.0, 33.0, 35.0], 26.5, [50.0, 47.5], 0.0),
    metric("M36", 36.0, 4.0, [37.0, 39.0, 42.0], 32.0, [58.0, 56.5], 0.0),
    inch("2-56", 0.086, 56.0, [0.094, 0.102, 0.116], 0.070, 0.1875, 0.197),
    inch("2-64", 0.086, 64.0, [0.094, 0.102, 0.116], 0.070, 0.1875, 0.197),
    inch("4-40", 0.112, 40.0, [0.120, 0.128, 0.144], 0.089, 0.21875, 0.255),
    inch("4-48", 0.112, 48.0, [0.120, 0.128, 0.144], 0.0935, 0.21875, 0.255),
    inch("6-32", 0.138, 32.0, [0.154, 0.170, 0.185], 0.1065, 0.28125, 0.307),
    inch("6-40", 0.138, 40.0, [0.154, 0.170, 0.185], 0.113, 0.28125, 0.307),
    inch("8-32", 0.164, 32.0, [0.180, 0.196, 0.213], 0.136, 0.3125, 0.359),
    inch("8-36", 0.164, 36.0, [0.180, 0.196, 0.213], 0.136, 0.3125, 0.359),
    inch("10-24", 0.190, 24.0, [0.206, 0.221, 0.238], 0.1495, 0.375, 0.411),
    inch("10-32", 0.190, 32.0, [0.206, 0.221, 0.238], 0.159, 0.375, 0.411),
    inch("1/4-20", 0.25, 20.0, [0.266, 0.281, 0.297], 0.201, 0.4375, 0.531),
    inch("1/4-28", 0.25, 28.0, [0.266, 0.281, 0.297], 0.213, 0.4375, 0.531),
    inch(
        "5/16-18",
        0.3125,
        18.0,
        [0.328, 0.344, 0.359],
        0.257,
        0.53125,
        0.656,
    ),
    inch(
        "5/16-24",
        0.3125,
        24.0,
        [0.328, 0.344, 0.359],
        0.272,
        0.53125,
        0.656,
    ),
    inch("3/8-16", 0.375, 16.0, [0.391, 0.406, 0.422], 0.3125, 0.625, 0.781),
    inch("3/8-24", 0.375, 24.0, [0.391, 0.406, 0.422], 0.332, 0.625, 0.781),
    inch(
        "7/16-14",
        0.4375,
        14.0,
        [0.453, 0.469, 0.484],
        0.368,
        0.71875,
        0.844,
    ),
    inch(
        "7/16-20",
        0.4375,
        20.0,
        [0.453, 0.469, 0.484],
        0.3906,
        0.71875,
        0.844,
    ),
    inch("1/2-13", 0.5, 13.0, [0.531, 0.562, 0.609], 0.4219, 0.8125, 0.938),
    inch("1/2-20", 0.5, 20.0, [0.531, 0.562, 0.609], 0.4531, 0.8125, 0.938),
    inch("5/8-11", 0.625, 11.0, [0.656, 0.688, 0.734], 0.53125, 1.0, 1.188),
    inch("5/8-18", 0.625, 18.0, [0.656, 0.688, 0.734], 0.5781, 1.0, 1.188),
    inch(
        "3/4-10",
        0.75,
        10.0,
        [0.781, 0.812, 0.906],
        0.65625,
        1.1875,
        1.438,
    ),
    inch("3/4-16", 0.75, 16.0, [0.781, 0.812, 0.906], 0.6875, 1.1875, 1.438),
    inch("1-8", 1.0, 8.0, [1.031, 1.094, 1.156], 0.875, 1.625, 1.938),
    inch("1-12", 1.0, 12.0, [1.031, 1.094, 1.156], 0.9219, 1.625, 1.938),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_resolve() {
        let m6 = screw("M6").expect("M6");
        let at = |convention, fit| Standard {
            screw: m6,
            convention,
            fit,
        };
        assert_eq!(at(Convention::Iso, Fit::Normal).diameter(), 6.6);
        assert_eq!(at(Convention::Ansi, Fit::Normal).diameter(), 6.6);
        assert_eq!(at(Convention::Iso, Fit::Normal).thread(), None);
        let tap = at(Convention::Iso, Fit::Tap);
        assert_eq!(tap.diameter(), 5.0);
        assert_eq!(
            tap.thread(),
            Some(Thread {
                size: 6.0,
                pitch: 1.0
            })
        );
        // DIN 336 and the pitch rule part ways for M8 and M12; counterbores differ throughout
        let m8 = screw("M8").expect("M8");
        assert_eq!(m8.tap, [6.8, 6.75]);
        assert_eq!(screw("M12").expect("M12").tap, [10.2, 10.25]);
        assert_eq!(m8.counterbore, [(15.0, 8.0), (14.25, 8.0)]);
        assert_eq!(screw("10-32").expect("10-32").tap[1], 0.159 * IN);
        assert!(screw("M7").is_none() && screw("1").is_none() && screw("1/4").is_none());
        assert_eq!(m8.default_convention(), Convention::Iso);
        assert_eq!(
            screw("1/4-20").expect("1/4-20").default_convention(),
            Convention::Ansi
        );
    }

    /// Every row is a hole the hole feature takes: clearances growing, wider than the screw;
    /// tap drills inside it, at least its minor diameter; heads wider than the clearance.
    #[test]
    fn rows_are_sane() {
        for s in SCREWS {
            let [c, n, l] = s.clearance;
            assert!(s.size < c && c < n && n < l, "{}", s.name);
            for (t, (d, depth)) in s.tap.into_iter().zip(s.counterbore) {
                assert!(t < s.size && t >= s.size - 1.3 * s.pitch, "{}", s.name);
                assert!(d > l && depth > 0.0, "{}", s.name);
            }
            if let Some((d, a)) = s.countersink {
                assert!(d > l && a > 0.0 && a < PI, "{}", s.name);
            }
            assert_eq!(SCREWS.iter().filter(|t| t.name == s.name).count(), 1);
        }
    }
}
