//! Every character the app's strings show is in the bundled fonts: one missing draws as a box.
//! Strings and char literals in the sources, outside their test modules; ASCII is in any font.

use forge_render::shell::theme;

/// The text of every string and char literal in a Rust source, roughly: what's between quotes,
/// escapes taken as they come, comments skipped.
fn literals(src: &str) -> String {
    let b: Vec<char> = src.chars().collect();
    let (mut out, mut k) = (String::new(), 0);
    while k < b.len() {
        match b[k] {
            '/' if b.get(k + 1) == Some(&'/') => {
                while k < b.len() && b[k] != '\n' {
                    k += 1;
                }
            }
            '"' => {
                k += 1;
                while k < b.len() && b[k] != '"' {
                    if b[k] == '\\' {
                        k += 1;
                    } else {
                        out.push(b[k]);
                    }
                    k += 1;
                }
            }
            // a char literal, not a lifetime: 'x' or '\n'
            '\'' if b.get(k + 2) == Some(&'\'') => {
                out.push(b[k + 1]);
                k += 2;
            }
            _ => {}
        }
        k += 1;
    }
    out
}

#[test]
fn every_character_the_app_shows_is_in_its_fonts() {
    let ctx = egui::Context::default();
    ctx.set_fonts(theme::fonts());
    // fonts are ready after a pass
    let mut out = ctx.run_pass(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    let src = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut files = vec![src.to_path_buf()];
    let mut missing: Vec<(char, String)> = vec![];
    while let Some(p) = files.pop() {
        if p.is_dir() {
            files.extend(std::fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
            continue;
        }
        if p.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        // the app's own strings, not its tests'
        let app = text.split("#[cfg(test)]").next().unwrap_or("");
        for c in literals(app).chars().filter(|c| !c.is_ascii()) {
            // every family falls back to Public Sans: what's shown in it is what must be there
            let there = ctx.fonts_mut(|f| f.has_glyph(&egui::FontId::proportional(12.0), c));
            if !there && !missing.iter().any(|m| m.0 == c) {
                let at = p.strip_prefix(src).unwrap().display().to_string();
                missing.push((c, at));
            }
        }
    }
    assert!(missing.is_empty(), "drawn as boxes: {missing:?}");
}
