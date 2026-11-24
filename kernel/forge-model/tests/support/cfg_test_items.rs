//! Skipping `#[cfg(test)]` items in a Rust source, for the source scans in tests
//! (forge-model tests/hidden_inputs.rs, forge-render tests/glyphs.rs, by `#[path]`): a test item
//! mid-file hides only itself.

/// The lines of `text` outside `#[cfg(test)]` items. Each such item (a fn, a mod, a use, ...) is
/// skipped from its attribute to its matching closing brace (or its `;`, for one without a body),
/// and the scan goes on after it: a test item mid-file hides only itself. Braces inside strings,
/// raw strings, char literals and comments are not counted.
pub fn live_lines(text: &str) -> Vec<&str> {
    let mut out = vec![];
    let mut starts = vec![];
    let mut at = 0;
    for l in text.split_inclusive('\n') {
        starts.push(at);
        at += l.len();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim_start();
        if let Some(rest) = t.strip_prefix("#[cfg(test)]") {
            let from = starts[i] + (lines[i].len() - rest.len());
            let end = item_end(text, from);
            // resume on the line after the one the item ends on
            i = starts.partition_point(|&s| s <= end);
            continue;
        }
        out.push(lines[i]);
        i += 1;
    }
    out
}

/// The byte offset of the end of the item starting at or after `from`: its body's closing brace,
/// or a `;` before any brace. The end of the text if neither comes.
fn item_end(text: &str, from: usize) -> usize {
    let b = text.as_bytes();
    let mut i = from;
    let mut depth = 0usize;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut nest = 1;
                i += 2;
                while i < b.len() && nest > 0 {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        nest += 1;
                        i += 1;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        nest -= 1;
                        i += 1;
                    }
                    i += 1;
                }
                continue;
            }
            b'r' if (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
                && matches!(b.get(i + 1), Some(b'"') | Some(b'#')) =>
            {
                // a raw string r"..", r#".."#: its end is `"` and as many `#`s
                let mut j = i + 1;
                let mut hashes = 0;
                while b.get(j) == Some(&b'#') {
                    hashes += 1;
                    j += 1;
                }
                if b.get(j) == Some(&b'"') {
                    j += 1;
                    loop {
                        if j >= b.len() {
                            return b.len();
                        }
                        if b[j] == b'"'
                            && b[j + 1..].iter().take(hashes).filter(|&&c| c == b'#').count() == hashes
                        {
                            i = j + 1 + hashes;
                            break;
                        }
                        j += 1;
                    }
                    continue;
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'\'' => {
                // a char literal ('x', '\n', '\u{7b}', '{'); otherwise a lifetime
                if b.get(i + 1) == Some(&b'\\') {
                    i += 2;
                    while i < b.len() && b[i] != b'\'' {
                        i += 1;
                    }
                } else if let Some(c) = text[i + 1..].chars().next() {
                    let n = c.len_utf8();
                    if b.get(i + 1 + n) == Some(&b'\'') {
                        i += 1 + n;
                    }
                }
            }
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i;
                }
            }
            b';' if depth == 0 => return i,
            _ => {}
        }
        i += 1;
    }
    b.len()
}
