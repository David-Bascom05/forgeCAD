//! Tag names for versions and releases: a file's path is one ref component, percent-encoded
//! where git's ref rules (git-check-ref-format) or a `/` would get in the way, so `parts/a` and
//! `parts/a/b.fg` never clash as directory and file.

const VERSIONS: &str = "forgecad/v/";
const RELEASES: &str = "forgecad/rel/";

pub fn enc(path: &str) -> String {
    let b = path.as_bytes();
    let mut out = String::new();
    let lock_dot = path.ends_with(".lock").then(|| path.len() - 5);
    for (i, &c) in b.iter().enumerate() {
        let dot_bad = c == b'.'
            && (i == 0
                || i + 1 == b.len()
                || b.get(i + 1) == Some(&b'.')
                || (i > 0 && b[i - 1] == b'.')
                || Some(i) == lock_dot);
        let bad = !(0x21..0x7f).contains(&c) || b"/%~^:?*[\\@{}".contains(&c) || dot_bad;
        if bad {
            out.push_str(&format!("%{c:02X}"));
        } else {
            out.push(c as char);
        }
    }
    out
}

pub fn dec(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn version_dir(path: &str) -> String {
    format!("{VERSIONS}{}", enc(path))
}

/// The tag (under `refs/tags/`) of a file's named version n.
pub fn version_tag(path: &str, n: u32) -> String {
    format!("{}/{n}", version_dir(path))
}

/// A full ref `refs/tags/forgecad/v/<path>/<n>`: its path and n.
pub fn parse_version_ref(full: &str) -> Option<(String, u32)> {
    let rest = full.strip_prefix("refs/tags/")?.strip_prefix(VERSIONS)?;
    let (p, n) = rest.split_once('/')?;
    Some((dec(p)?, n.parse().ok()?))
}

pub fn release_dir(path: &str) -> String {
    format!("{RELEASES}{}", enc(path))
}

pub fn release_tag(path: &str, rev: &str) -> String {
    format!("{}/{rev}", release_dir(path))
}

pub fn parse_release_ref(full: &str) -> Option<(String, String)> {
    let rest = full.strip_prefix("refs/tags/")?.strip_prefix(RELEASES)?;
    let (p, r) = rest.split_once('/')?;
    Some((dec(p)?, r.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_paths_are_valid_refs_and_round_trip() {
        for p in [
            "bracket.fg",
            "parts/bracket.fg",
            "parts/a",
            ".hidden/x.fg",
            "a..b.fg",
            "x.lock",
            "with space/and~tilde^:?*[\\@{x}.fg",
            "dir.",
            "ü/日本.fg",
        ] {
            let t = format!("refs/tags/{}", version_tag(p, 3));
            assert!(forge_git::refs::valid_name(&t), "{t}");
            assert_eq!(parse_version_ref(&t), Some((p.to_string(), 3)));
        }
        assert_eq!(enc("parts/bracket.fg"), "parts%2Fbracket.fg");
    }
}
