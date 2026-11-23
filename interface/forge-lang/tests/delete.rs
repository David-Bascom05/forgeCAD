//! `delete(BODIES)`: the bodies those features made removed from the part, the others as they were;
//! a later name of a deleted body is the typed error Deleted; a delete of a body not there, or of
//! every body, is refused.
mod common;

const TWO: &str = "sketch sa(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
a = extrude(sa, 10)
sketch sb(on: xy) {
  r = rect(corner: (40, 0), size: (10, 20))
}
b = extrude(sb, 10, new: true)
";

#[test]
fn a_deleted_body_leaves_the_part() {
    let both = common::holds(TWO);
    assert_eq!(both.bodies, 2);
    let m = common::holds(&format!("{TWO}gone = delete(a)\n"));
    assert_eq!(m.bodies, 1);
    assert!((m.volume - 2000.0).abs() < 1e-6, "{}", m.volume);
    let m = common::holds(&format!("{TWO}gone = delete([b])\n"));
    assert_eq!(m.bodies, 1);
    assert!((m.volume - 1000.0).abs() < 1e-6, "{}", m.volume);
}

#[test]
fn a_deleted_body_cannot_be_named() {
    for later in [
        "c = movebody(of: a, moves: slide(along: X, by: 5))",
        "e = fillet(edges(of: a, crease: \"convex\"), 1)",
        "f = a.end",
        "g = delete(a)",
        "h = union(b, tools: a)",
    ] {
        let r = common::refused(&format!("{TWO}gone = delete(a)\n{later}\n"));
        assert!(
            r.contains("Deleted") && r.contains("deleted by gone"),
            "{later}: {r}"
        );
    }
    // a list bound before the delete holds it too
    let r = common::refused(&format!(
        "{TWO}both = [a, b]\ngone = delete(a)\nm = movebody(of: both, moves: slide(along: X, by: 5))\n"
    ));
    // a list bound before the delete still holds the feature: the kernel's NoBody
    assert!(r.contains("NoBody"), "{r}");
    // the body left is there to work on
    let m = common::holds(&format!(
        "{TWO}gone = delete(a)\nm = movebody(of: b, moves: slide(along: X, by: 5))\n"
    ));
    assert_eq!(m.bodies, 1);
}

#[test]
fn a_delete_of_every_body_or_none_is_refused() {
    let r = common::refused(&format!("{TWO}gone = delete([a, b])\n"));
    assert!(r.contains("no body") || r.contains("DeletesAll"), "{r}");
    let r = common::refused(&format!("{TWO}gone = delete([])\n"));
    assert!(r.contains("NotOne"), "{r}");
    // a feature that made no body of its own
    let r = common::refused(&format!(
        "{TWO}sketch sc(on: a.end) {{\n  c = circle((5, 5), 2)\n}}\nbore = cut(sc.c, 3)\ngone = delete(bore)\n"
    ));
    assert!(!r.is_empty(), "{r}");
}

/// `delete` is a new built-in name. research/forge-lang/lean/ForgeLang/BuiltinDelete.lean
/// (`delete_conservative`) proves adding it conservative for any document that binds no `delete`;
/// no `.fg` in the repository binds or reads it, so every part means what it meant before.
#[test]
fn delete_bound_nowhere() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut stack = vec![root];
    let mut files = 0;
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
                let code: String = line
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .split('"')
                    .step_by(2)
                    .collect();
                let mut at = 0;
                while let Some(i) = code[at..].find("delete") {
                    let s = at + i;
                    let e = s + "delete".len();
                    at = e;
                    let before = code[..s].chars().next_back();
                    let word = !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
                        && !code[e..]
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    assert!(!word, "{}:{}: uses the name `delete`: {line}", p.display(), n + 1);
                }
            }
        }
    }
    assert!(files > 100, "only {files} .fg files found");
}
