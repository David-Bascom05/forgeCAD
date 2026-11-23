//! `scale(about: POINT, by: k)`, the uniform-scale motion of movebody and copybody: the part's
//! volume times k³, the same faces and edges, motions in order, and every bad factor refused with a
//! typed error.
mod common;

const PART: &str = "shared k = ratio(1.5)
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (40, 20))
}
base = extrude(plan, 10)
sketch top(on: base.end) {
  c = circle((20, 10), 4)
}
bore = cut(top.c, 10)
sketch far(on: xy) {
  p = pt(100, 0)
}
round = fillet(edges(of: base, crease: \"convex\", along: Z), 2)
";

fn plain() -> forge_lang::Measure {
    common::holds(PART)
}

#[test]
fn a_scaled_part_has_k_cubed_the_volume() {
    let m0 = plain();
    for (k, txt) in [(1.5, "k"), (0.25, "ratio(0.25)"), (3.0, "count(3)")] {
        let m = common::holds(&format!(
            "{PART}grown = movebody(of: base, moves: scale(about: top.c, by: {txt}))\n"
        ));
        let want = m0.volume * k * k * k;
        assert!(
            (m.volume - want).abs() <= 1e-9 * want,
            "k={k}: {} vs {want}",
            m.volume
        );
        assert_eq!(
            (m.faces, m.edges, m.bodies),
            (m0.faces, m0.edges, m0.bodies),
            "k={k}"
        );
    }
}

#[test]
fn scale_mixes_with_rigid_motions_and_copies() {
    let m0 = plain();
    let m = common::holds(&format!(
        "{PART}twin = copybody(of: base, moves: [slide(along: X, by: 100), scale(about: far.p, by: count(2)), spin(about: axis(far.p, top.c), by: 30deg)])\n"
    ));
    assert_eq!(m.bodies, 2);
    let want = m0.volume * 9.0;
    assert!((m.volume - want).abs() <= 1e-9 * want, "{} vs {want}", m.volume);
}

#[test]
fn bad_factors_are_refused() {
    for by in ["ratio(0)", "ratio(-2)", "k - k"] {
        let r = common::refused(&format!(
            "{PART}grown = movebody(of: base, moves: scale(about: top.c, by: {by}))\n"
        ));
        assert!(r.contains("scale factor"), "{by}: {r}");
    }
    let r = common::refused(&format!(
        "{PART}grown = movebody(of: base, moves: scale(about: top.c, by: 2mm))\n"
    ));
    assert!(!r.is_empty(), "a length is no factor");
}

/// `scale` is a new built-in name. research/forge-lang/lean/ForgeLang/Builtins.lean
/// (`scale_conservative`) proves adding it conservative for any document that binds no `scale`;
/// no `.fg` file in the repository binds it (its uses are a drawing view's `scale:` argument, a
/// label, not a binding), so every part means what it meant before.
#[test]
fn scale_bound_nowhere() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut stack = vec![root];
    let mut files = 0;
    let b = "scale";
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("a directory") {
            let p = e.expect("an entry").path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.is_dir() {
                if !name.starts_with('.') && name != "target" {
                    stack.push(p);
                }
                continue;
            }
            if !name.ends_with(".fg") {
                continue;
            }
            files += 1;
            let src = std::fs::read_to_string(&p).unwrap_or_default();
            for (n, line) in src.lines().enumerate() {
                // the code of the line, its comment and strings left out
                let code: String = line
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .split('"')
                    .step_by(2)
                    .collect();
                let mut at = 0;
                while let Some(i) = code[at..].find(b) {
                    let s = at + i;
                    let e = s + b.len();
                    at = e;
                    let before = code[..s].chars().next_back();
                    let word = !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
                        && !code[e..]
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    if !word {
                        continue;
                    }
                    let rest = code[e..].trim_start();
                    let lead = code[..s].split_whitespace().last().unwrap_or("");
                    let binds = (rest.starts_with('=') && !rest.starts_with("=="))
                        || matches!(
                            lead,
                            "shared" | "param" | "sketch" | "path" | "for" | "part" | "def"
                        );
                    assert!(!binds, "{}:{}: binds `scale`: {line}", p.display(), n + 1);
                }
            }
        }
    }
    assert!(files > 100, "only {files} .fg files found");
}
