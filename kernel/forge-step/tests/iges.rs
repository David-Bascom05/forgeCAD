//! IGES files: a block with a hole, written once by OpenCascade from a shape made for this test and
//! kept beside it, and every way of damaging it.
use forge_math::dmath;
use forge_step::{Limits, import};
use forge_topo::{check, mass::mass};

const BLOCK: &str = include_str!("data/block_with_hole.igs");

fn block_is_right(text: &str, scale: f64) {
    let got = import(text.as_bytes(), &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    assert_eq!(got.solids.len(), 1);
    let b = &got.solids[0].body;
    assert!(check(b).is_empty(), "{:?}", check(b));
    let m = mass(b).unwrap();
    let hole = dmath::PI * 4.0 * 6.0;
    let v = (20.0 * 10.0 * 6.0 - hole) * scale * scale * scale;
    let a = (2.0 * (200.0 + 120.0 + 60.0) - 2.0 * dmath::PI * 4.0 + dmath::TAU * 2.0 * 6.0) * scale * scale;
    assert!((m.volume - v).abs() < 1e-6 * v, "{} {v}", m.volume);
    assert!((m.area - a).abs() < 1e-6 * a, "{} {a}", m.area);
}

#[test]
fn a_block_with_a_hole_is_read_exactly() {
    block_is_right(BLOCK, 1.0);
}

#[test]
fn units_come_from_the_global_section() {
    let inches = BLOCK.replacen(",1.,2,2HMM,", ",1.,1,2HIN,", 1);
    assert_ne!(inches, BLOCK);
    block_is_right(&inches, 25.4);
}

/// Trimmed surfaces hold no topology: refused by name, not read as something else.
#[test]
fn trimmed_surfaces_are_refused_by_name() {
    let line = |text: &str, sect: char, n: usize| format!("{text:<72}{sect}{n:07}\n");
    let text = line("", 'S', 1)
        + &line(
            ",,1Hx,1Hx,1Hx,1Hx,32,308,15,308,15,,1.,2,2HMM,1,0.01,1Hx,1E-07,20.,",
            'G',
            1,
        )
        + &line("1Hx,,11,0,1Hx,;", 'G', 2)
        + &line(
            "     144       1       0       0       0       0       0       00000000",
            'D',
            1,
        )
        + &line("     144       0       0       1       0", 'D', 2)
        + &line(
            "144,0,1,0,0;                                                   0000001",
            'P',
            1,
        );
    let got = import(text.as_bytes(), &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert!(got.solids.is_empty());
    assert!(got.refusals[0].why.contains("sewn"), "{:?}", got.refusals);
}

#[test]
fn cut_short_or_damaged_it_never_panics() {
    let at: Vec<usize> = BLOCK.char_indices().map(|x| x.0).step_by(97).collect();
    for &k in &at {
        let _ = import(&BLOCK.as_bytes()[..k], &Limits::default());
    }
    let lines: Vec<&str> = BLOCK.lines().collect();
    for i in (0..lines.len()).step_by(3) {
        for bad in ["-1", "99999999999", "1e400", "x"] {
            let mut l: Vec<String> = lines.iter().map(|x| x.to_string()).collect();
            l[i] = l[i].replacen(',', &format!(",{bad},"), 1);
            let _ = import(l.join("\n").as_bytes(), &Limits::default());
        }
    }
}
