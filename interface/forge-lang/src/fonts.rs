//! The fonts shipped with ForgeCAD, by family and style: Open Sans 1.10 (TrueType outlines; the
//! version Onshape sets sketch text in; Apache License 2.0) and Source Sans 3 (CFF outlines; SIL
//! Open Font License 1.1). Both licences allow them to be embedded and redistributed with
//! software (fonts/README.md records where each came from, its licence and its hash).
//! A part names one by its family (`font: "Open Sans"`); a font file beside the part
//! (`font: "block.ttf"`) is read as `media::font` reads it.

use forge_font::Font;
use std::sync::{Arc, OnceLock};

/// A shipped face: its family, its style as its name table gives it, its bytes.
struct Face {
    family: &'static str,
    style: &'static str,
    bytes: &'static [u8],
}

const FACES: &[Face] = &[
    Face {
        family: "Open Sans",
        style: "Regular",
        bytes: include_bytes!("../fonts/OpenSans-Regular.ttf"),
    },
    Face {
        family: "Open Sans",
        style: "Bold",
        bytes: include_bytes!("../fonts/OpenSans-Bold.ttf"),
    },
    Face {
        family: "Open Sans",
        style: "Italic",
        bytes: include_bytes!("../fonts/OpenSans-Italic.ttf"),
    },
    Face {
        family: "Open Sans",
        style: "Bold Italic",
        bytes: include_bytes!("../fonts/OpenSans-BoldItalic.ttf"),
    },
    Face {
        family: "Source Sans 3",
        style: "Regular",
        bytes: include_bytes!("../fonts/SourceSans3-Regular.otf"),
    },
];

/// The family a sketch text takes when it names none.
pub const DEFAULT: &str = "Open Sans";

/// The style a bold and italic choice names.
pub fn style(bold: bool, italic: bool) -> &'static str {
    match (bold, italic) {
        (false, false) => "Regular",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (true, true) => "Bold Italic",
    }
}

/// Whether a family is shipped (in any style), its name matched without regard to case.
pub fn is_family(family: &str) -> bool {
    FACES.iter().any(|f| f.family.eq_ignore_ascii_case(family))
}

/// Every shipped family and style.
pub fn faces() -> impl Iterator<Item = (&'static str, &'static str)> {
    FACES.iter().map(|f| (f.family, f.style))
}

/// The shipped face of the family in that style, parsed once; None if the family has no such
/// face.
pub fn shipped(family: &str, bold: bool, italic: bool) -> Option<Arc<Font>> {
    static PARSED: OnceLock<Vec<OnceLock<Arc<Font>>>> = OnceLock::new();
    let parsed = PARSED.get_or_init(|| FACES.iter().map(|_| OnceLock::new()).collect());
    let want = style(bold, italic);
    let k = FACES
        .iter()
        .position(|f| f.family.eq_ignore_ascii_case(family) && f.style == want)?;
    Some(
        parsed[k]
            .get_or_init(|| Arc::new(Font::parse(FACES[k].bytes.to_vec()).expect("a shipped font reads")))
            .clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped face reads, says the family and style it is filed under, and holds an
    /// outline for every printable character of Latin-1.
    #[test]
    fn shipped_faces_read() {
        for (family, style) in faces() {
            let (bold, italic) = (style.contains("Bold"), style.contains("Italic"));
            let f = shipped(family, bold, italic).expect("shipped");
            assert_eq!((f.family.as_str(), f.style.as_str()), (family, style));
            for c in (0x21u32..0x7f).chain(0xa1..0x100).filter_map(char::from_u32) {
                if c == '\u{ad}' {
                    continue;
                }
                let g = f.glyph(c);
                assert!(g != 0, "{family} {style}: no glyph for {c:?}");
                f.outline(g)
                    .unwrap_or_else(|e| panic!("{family} {style} {c:?}: {e:?}"));
            }
        }
        assert!(shipped("open sans", false, false).is_some());
        assert!(shipped("Source Sans 3", true, false).is_none());
    }
}
