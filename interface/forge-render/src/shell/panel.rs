//! A feature's line in the words a person reads: what each argument of each kind is called, and
//! which icon the kind has. Reads the arguments as the part file writes them; changes nothing.

const OPS: [&str; 4] = ["add", "cut", "keep", "new"];
/// Words that begin a loop of a profile.
const LOOPS: [&str; 6] = ["rect", "circle", "rrect", "poly", "loop", "slot"];

fn is_frame(w: &str) -> bool {
    let w = w.strip_prefix('-').unwrap_or(w);
    matches!(w, "xy" | "xz" | "yz" | "yx" | "zx" | "zy")
}

fn is_ref(w: &str) -> bool {
    w.starts_with('(') || w.starts_with('@')
}

fn is_number(w: &str) -> bool {
    w.parse::<f64>().is_ok()
        || w.contains('(') && !is_ref(w)
        || w.chars().all(|c| c.is_ascii_digit() || ".+-*/".contains(c))
}

/// What a feature's main number is.
fn main_number(kind: &str) -> &'static str {
    match kind {
        "extrude" => "Distance",
        "revolve" => "Angle",
        "ball" | "fillet" | "facefillet" => "Radius",
        "chamfer" | "offset" => "Distance",
        "shell" | "rib" | "thicken" => "Thickness",
        "draft" => "Angle",
        "hole" => "Diameter",
        "plane" => "Offset",
        _ => "Value",
    }
}

/// What a feature's references are.
fn refs_name(kind: &str) -> &'static str {
    match kind {
        "fillet" | "chamfer" => "Edges",
        "shell" | "draft" | "facefillet" | "fullround" | "offset" | "thicken" | "delete" | "splitface" => {
            "Faces"
        }
        "move" | "combine" => "Bodies",
        _ => "Reference",
    }
}

/// A keyword's name, and how many words it takes (none: up to the next keyword).
fn keyword(w: &str) -> Option<(&'static str, Option<usize>)> {
    Some(match w {
        "at" => ("At", Some(3)),
        "thru" => ("Depth", Some(0)),
        "pitch" => ("Pitch", Some(1)),
        "turns" => ("Turns", Some(1)),
        "height" => ("Height", Some(1)),
        "axis" => ("Axis", None),
        "path" => ("Path", None),
        "to" => ("To", None),
        "from" => ("From", None),
        "cb" => ("Counterbore", Some(2)),
        "cs" | "csink" => ("Countersink", None),
        "cbore" => ("Counterbore", Some(0)),
        "lin" => ("Linear", None),
        "circ" => ("Circular", None),
        "mirror" => ("Mirror", None),
        "by" => ("By", Some(3)),
        "turn" | "rot" => ("Turn", None),
        "font" => ("Font", Some(1)),
        "draft" => ("Draft", Some(1)),
        "setback" => ("Setback", None),
        "extend" => ("Extend", Some(2)),
        "with" => ("With", None),
        "face" => ("Face", None),
        "wrap" => ("Wrap", Some(1)),
        "off" => ("Offset", Some(1)),
        "pts" => ("Through", None),
        "mid" => ("Middle of", None),
        "meet" => ("Meeting", None),
        "norm" => ("Normal to", None),
        "ang" => ("Angle to", None),
        "tan" => ("Tangent to", None),
        "on" => ("On", None),
        "profile" => ("Profile", None),
        "rail" => ("Rail", None),
        "centre" => ("Centre path", None),
        "smooth" => ("Smooth", Some(0)),
        "left" => ("Hand", Some(0)),
        "flip" => ("Flip", Some(0)),
        _ => return None,
    })
}

/// The feature's arguments as label and value, in order.
pub fn fields(kind: &str, args: &[&str]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut k = 0;
    let mut main_done = false;
    let mut push = |label: &str, value: String| {
        // a repeat of a label continues it
        match out.last_mut().filter(|l| l.0 == label) {
            Some(l) => {
                l.1.push(' ');
                l.1.push_str(&value);
            }
            None => out.push((label.to_string(), value)),
        }
    };
    if kind == "pattern" {
        // the first words up to its kind are what's copied
        while k < args.len() && !matches!(args[k], "lin" | "circ" | "mirror" | "pat" | "path") {
            push("Features", args[k].to_string());
            k += 1;
        }
    }
    while k < args.len() {
        let w = args[k];
        if OPS.contains(&w) && k == 0 {
            push("Operation", w.to_string());
            k += 1;
        } else if is_frame(w) {
            push("Plane", w.to_string());
            k += 1;
        } else if LOOPS.contains(&w) {
            let start = k;
            k += 1;
            while k < args.len()
                && !LOOPS.contains(&args[k])
                && keyword(args[k]).is_none()
                && !is_frame(args[k])
            {
                k += 1;
            }
            push("Profile", args[start..k].join(" "));
        } else if is_ref(w) {
            push(refs_name(kind), w.to_string());
            k += 1;
        } else if let Some((label, count)) = keyword(w) {
            k += 1;
            let start = k;
            match count {
                Some(n) => k = (k + n).min(args.len()),
                None => {
                    while k < args.len() && keyword(args[k]).is_none() && !LOOPS.contains(&args[k]) {
                        k += 1;
                    }
                }
            }
            let value = match (w, count) {
                ("thru", _) => "through".to_string(),
                ("left", _) => "left".to_string(),
                (_, Some(0)) => "yes".to_string(),
                _ => args[start..k].join(" "),
            };
            // "at X Y Z" belongs with its plane
            push(
                if label == "At" { "Plane" } else { label },
                if label == "At" {
                    format!("at {value}")
                } else {
                    value
                },
            );
        } else if is_number(w) && !main_done {
            push(main_number(kind), w.to_string());
            main_done = true;
            k += 1;
        } else if is_number(w) && kind == "hole" {
            push("Depth", w.to_string());
            k += 1;
        } else {
            push(refs_name(kind), w.to_string());
            k += 1;
        }
    }
    out
}

/// A sketch row as label and value.
pub fn sketch_row(args: &[String]) -> Option<(String, String)> {
    let (head, tail) = args.split_first()?;
    let label = match head.as_str() {
        "pt" => "Point",
        "line" => "Line",
        "poly" => "Polyline",
        "rect" => "Rectangle",
        "circle" => "Circle",
        "arc" => "Arc",
        "fix" => "Fixed",
        "co" => "Coincident",
        "hor" => "Horizontal",
        "ver" => "Vertical",
        "par" => "Parallel",
        "perp" => "Perpendicular",
        "tan" => "Tangent",
        "eq" => "Equal",
        "conc" => "Concentric",
        "mid" => "Midpoint",
        "sym" => "Symmetric",
        "dist" => "Distance",
        "hdist" => "Horizontal distance",
        "vdist" => "Vertical distance",
        "rad" => "Radius",
        "dia" => "Diameter",
        "ang" => "Angle",
        other => other,
    };
    Some((label.to_string(), tail.join(" ")))
}

/// A feature kind's word, as the ribbon has it: what the tree counts by.
pub fn kind_word(kind: &str) -> String {
    match kind {
        "facefillet" | "fullround" => "Fillet".to_string(),
        "ucs" => "UCS".to_string(),
        "splitface" => "Split".to_string(),
        "movebody" | "copybody" => "Move".to_string(),
        k => {
            let mut c = k.chars();
            c.next()
                .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
        }
    }
}

/// How a tree row of this feature looks.
pub fn icon(kind: &str) -> &'static str {
    match kind {
        "extrude" => "extrude",
        "revolve" => "revolve",
        "ball" => "ball",
        "sweep" => "sweep",
        "loft" => "loft",
        "coil" => "coil",
        "rib" => "rib",
        "hole" => "hole",
        "fillet" | "facefillet" | "fullround" => "fillet",
        "chamfer" => "chamfer",
        "shell" => "shell",
        "draft" => "draft",
        "thread" => "thread",
        "pattern" => "rect",
        "plane" => "plane",
        "axis" => "axis",
        "point" => "point",
        "ucs" => "ucs",
        "offset" => "offset",
        "thicken" => "thicken",
        "delete" => "delete",
        "replace" | "combine" => "combine",
        "move" | "movebody" | "copybody" => "move",
        "text" => "text",
        "split" | "splitface" => "split",
        _ => "point",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn got(kind: &str, line: &str) -> Vec<(String, String)> {
        let words: Vec<&str> = line.split(' ').collect();
        fields(kind, &words)
    }

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn the_usual_features_read_in_words() {
        assert_eq!(
            got("extrude", "cut 2 -xy at 0 5 6 circle 0 0 4"),
            pairs(&[
                ("Operation", "cut"),
                ("Distance", "2"),
                ("Plane", "-xy at 0 5 6"),
                ("Profile", "circle 0 0 4")
            ])
        );
        assert_eq!(
            got("hole", "5 -xy at -20 -10 6 thru"),
            pairs(&[
                ("Diameter", "5"),
                ("Plane", "-xy at -20 -10 6"),
                ("Depth", "through")
            ])
        );
        assert_eq!(
            got("fillet", "3 @e1 @e2"),
            pairs(&[("Radius", "3"), ("Edges", "@e1 @e2")])
        );
        assert_eq!(
            fields("axis", &["boss", "(faces of boss cyl)"]),
            pairs(&[("Reference", "boss (faces of boss cyl)")])
        );
        assert_eq!(
            fields("fullround", &["(f1)", "(f2)", "(f3)"]),
            pairs(&[("Faces", "(f1) (f2) (f3)")])
        );
        assert_eq!(
            got("pattern", "hole1 lin 1 0 0 3 20"),
            pairs(&[("Features", "hole1"), ("Linear", "1 0 0 3 20")])
        );
    }

    #[test]
    fn a_rows_word_is_the_ribbons() {
        assert_eq!(kind_word("extrude"), "Extrude");
        assert_eq!(kind_word("fullround"), "Fillet");
        assert_eq!(kind_word("facefillet"), "Fillet");
        assert_eq!(kind_word("sketch"), "Sketch");
    }

    #[test]
    fn every_kind_has_an_icon_on_file() {
        let all = include_str!("icons.txt");
        for k in forge_doc::text::KINDS {
            let name = icon(k.name);
            assert!(
                all.lines().any(|l| l.split(' ').next() == Some(name)),
                "{}: {name}",
                k.name
            );
        }
    }
}
