//! A feature kind in the words and the icon a person sees: the browser's rows, the dialogs'
//! titles, what a value field measures.

/// What a field of this name measures, if it's a number.
pub fn field_kind(label: &str) -> Option<forge_doc::units::Kind> {
    use forge_doc::units::Kind;
    match label {
        "Distance" | "Diameter" | "Radius" | "Thickness" | "Depth" | "Length" | "Offset" | "Height"
        | "Pitch" | "Width" | "Value" | "Spacing" | "Seat diameter" | "Seat depth" | "Distance 1"
        | "Distance 2" => Some(Kind::Length),
        "Angle" | "Angle 2" | "Sweep" | "Taper" => Some(Kind::Angle),
        _ => None,
    }
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
        "sketch" => "sketch",
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
        k if super::sheet::is_sheet_kind(k) => {
            super::sheet::icon(super::sheet::cmd_of_kind(k).expect("a sheet kind"))
        }
        _ => "point",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let kinds = [
            "extrude", "revolve", "ball", "sweep", "loft", "coil", "rib", "hole", "fillet", "chamfer",
            "shell", "draft", "thread", "pattern", "sketch", "plane", "axis", "point", "ucs", "offset",
            "thicken", "delete", "replace", "combine", "move", "text", "split", "unknown",
        ];
        for k in kinds {
            let name = icon(k);
            assert!(
                all.lines().any(|l| l.split(' ').next() == Some(name)),
                "{k}: {name}"
            );
        }
    }
}
