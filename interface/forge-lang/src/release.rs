//! Releases: a named version frozen as a revision of its part number (ASME Y14.35 letters, or
//! numbers before the first letter), with its exports (STEP; an assembly's BOM too) written
//! beside the project in `releases/`, never in history. The release record is an annotated tag
//! `forgecad/rel/<path>/<rev>` on the version's save: part number, revision, reason, approver,
//! the kernel, and each export's SHA-256. The kernel builds byte for byte the same, so
//! Regenerate rebuilds a release from its tag and checks every hash; a newer kernel that would
//! change a released part is drift, shown, never a silent change.

use crate::compare;
use forge_agent::json::Json;
use forge_git::{HashKind, Oid};
use forge_history::{History, names};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Export {
    pub kind: String,
    pub sha256: String,
    pub file: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub path: String,
    pub part_number: String,
    pub revision: String,
    pub version: u32,
    pub reason: String,
    pub approver: String,
    pub kernel: String,
    pub exports: Vec<Export>,
    pub save: Oid,
    pub time: i64,
    pub author: String,
}

/// Revision letters as ASME Y14.35 has them: no I, O, Q, S, X or Z.
const LETTERS: &[u8] = b"ABCDEFGHJKLMNPRTUVWY";

fn letter_index(r: &str) -> Option<usize> {
    let mut n = 0usize;
    for c in r.bytes() {
        let k = LETTERS.iter().position(|&l| l == c)?;
        n = n * LETTERS.len() + k + 1;
    }
    Some(n)
}

fn letters_of(mut n: usize) -> String {
    let mut out = vec![];
    while n > 0 {
        n -= 1;
        out.push(LETTERS[n % LETTERS.len()]);
        n /= LETTERS.len();
    }
    out.reverse();
    String::from_utf8(out).expect("ascii")
}

/// The revision after `prev`: the next letter (A after none, AA after Y), or with `numeric`
/// and no letter yet, the next number. Letters never go back to numbers.
pub fn next_revision(prev: Option<&str>, numeric: bool) -> String {
    match prev {
        None if numeric => "1".into(),
        None => "A".into(),
        Some(p) if p.bytes().all(|c| c.is_ascii_digit()) => {
            if numeric {
                (p.parse::<u64>().unwrap_or(0) + 1).to_string()
            } else {
                "A".into()
            }
        }
        Some(p) => letters_of(letter_index(p).unwrap_or(0) + 1),
    }
}

/// Which of two revisions is later (numbers before letters).
fn order(r: &str) -> (u8, usize, u64) {
    if r.bytes().all(|c| c.is_ascii_digit()) {
        (0, 0, r.parse().unwrap_or(0))
    } else {
        (1, letter_index(r).unwrap_or(0), 0)
    }
}

fn parse(h: &History, path: &str, rev: &str, msg: &str, save: Oid, time: i64, author: String) -> Record {
    let t = forge_history::trailers(msg);
    let get = |k: &str| {
        t.iter()
            .find(|x| x.0 == k)
            .map(|x| x.1.clone())
            .unwrap_or_default()
    };
    let body = msg.trim_end();
    let body = if t.is_empty() {
        body
    } else {
        body.rsplit_once("\n\n").map_or("", |x| x.0)
    };
    let reason = body.split_once("\n\n").map_or("", |x| x.1).trim().to_string();
    let exports = t
        .iter()
        .filter(|x| x.0 == "ForgeCAD-Export")
        .filter_map(|x| {
            let mut it = x.1.split_whitespace();
            let kind = it.next()?.to_string();
            let sha = it.next()?.trim_start_matches("sha256:").to_string();
            let file = it.collect::<Vec<_>>().join(" ");
            Some(Export {
                kind,
                sha256: sha,
                file,
            })
        })
        .collect();
    let _ = h;
    Record {
        path: path.to_string(),
        part_number: get("ForgeCAD-Part-Number"),
        revision: rev.to_string(),
        version: get("ForgeCAD-Version").parse().unwrap_or(0),
        reason,
        approver: get("ForgeCAD-Approver"),
        kernel: get("ForgeCAD-Kernel"),
        exports,
        save,
        time,
        author,
    }
}

/// A file's releases (None: every file's), oldest revision first.
pub fn records(h: &History, path: Option<&str>) -> Result<Vec<Record>, String> {
    let prefix = match path {
        Some(p) => format!("refs/tags/{}/", names::release_dir(p)),
        None => "refs/tags/forgecad/rel/".into(),
    };
    let mut out = vec![];
    for (full, id) in h.repo.refs.list(&prefix).map_err(|e| e.to_string())? {
        let Some((p, rev)) = names::parse_release_ref(&full) else {
            continue;
        };
        let Ok(Some(tag)) = h.repo.read_tag(&id) else {
            continue;
        };
        let (time, author) = tag
            .tagger
            .as_ref()
            .map_or((0, String::new()), |t| (t.time, t.name.clone()));
        out.push(parse(h, &p, &rev, &tag.message, tag.object, time, author));
    }
    out.sort_by(|a, b| (a.path.as_str(), order(&a.revision)).cmp(&(b.path.as_str(), order(&b.revision))));
    Ok(out)
}

/// The newest released revision of a file on disk, if its folder keeps history and it has one.
pub fn revision_of(file: &Path) -> Option<String> {
    if !file.is_file() {
        return None;
    }
    let h = crate::pins::history(file.parent()?)?;
    let rel = h.rel(file).ok()?;
    current(&h, &rel).map(|r| r.revision)
}

/// The newest released revision of a file.
pub fn current(h: &History, path: &str) -> Option<Record> {
    records(h, Some(path)).ok()?.into_iter().last()
}

fn stem(rel: &str) -> String {
    Path::new(rel)
        .file_stem()
        .map_or_else(|| rel.to_string(), |s| s.to_string_lossy().to_string())
}

/// An export: its kind, file name and bytes.
type File = (String, String, Vec<u8>);

/// The exports of a document at a save, and its part number.
fn exports(h: &History, rel: &str, at: &Oid, name: &str) -> Result<(Vec<File>, String), String> {
    let side = compare::build_at_step(h, rel, Some(at), "release", Some(name.to_string()))?;
    if let Some(e) = side.run.errors.first() {
        return Err(format!("{rel} does not build: {}", e.text()));
    }
    let pn = side.run.props.get("part_number").unwrap_or("").to_string();
    let mut out = vec![];
    for s in crate::export::step(&side.run, name)? {
        out.push((
            "step".to_string(),
            format!("{name}{}.step", s.suffix),
            s.text.into_bytes(),
        ));
    }
    if let Some(a) = &side.run.asm {
        let mut csv = String::from("part,quantity\n");
        for (p, n) in &a.bom {
            let p = if p.contains(',') || p.contains('"') {
                format!("\"{}\"", p.replace('"', "\"\""))
            } else {
                p.clone()
            };
            csv.push_str(&format!("{p},{n}\n"));
        }
        out.push(("bom".to_string(), format!("{name}-bom.csv"), csv.into_bytes()));
    }
    Ok((out, pn))
}

fn release_dir(h: &History, folder: &str, rev: &str) -> PathBuf {
    h.work().join("releases").join(folder).join(rev)
}

fn write_all(dir: &Path, files: &[File]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for (_, name, data) in files {
        let p = dir.join(name);
        let mut l = forge_git::fsutil::Lock::take(&p).map_err(|e| e.to_string())?;
        l.write(data).map_err(|e| e.to_string())?;
        l.commit().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn folder_of(pn: &str, rel: &str) -> String {
    let f: String = if pn.trim().is_empty() {
        stem(rel)
    } else {
        pn.trim().to_string()
    };
    f.chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Releases the file as it is (saved first): the next revision, a named version, the exports
/// and the release record.
pub fn release(
    file: &Path,
    reason: &str,
    approver: &str,
    numeric: bool,
    agent: Option<&str>,
) -> Result<Record, String> {
    if reason.trim().is_empty() {
        return Err("a release needs a reason".into());
    }
    let h = History::open(file).ok_or_else(|| format!("{} keeps no history", file.display()))?;
    let rel = h.rel(file).map_err(|e| e.to_string())?;
    crate::vtools::save(file, "saved", agent)?;
    let prev = current(&h, &rel);
    let rev = next_revision(prev.as_ref().map(|p| p.revision.as_str()), numeric);
    let head = h
        .head()
        .map_err(|e| e.to_string())?
        .ok_or("nothing saved to release")?;
    let name = format!("{}-{rev}", stem(&rel));
    let (files, pn) = exports(&h, &rel, &head, &name)?;
    let folder = folder_of(&pn, &rel);
    let v = h
        .create_version(&rel, &format!("{} rev {rev}", stem(&rel)), reason, agent)
        .map_err(|e| e.to_string())?;
    let mut msg = format!("{} rev {rev}\n\n{}\n\n", stem(&rel), reason.trim());
    msg.push_str(&format!("ForgeCAD-Path: {rel}\n"));
    if !pn.is_empty() {
        msg.push_str(&format!("ForgeCAD-Part-Number: {pn}\n"));
    }
    msg.push_str(&format!("ForgeCAD-Revision: {rev}\nForgeCAD-Version: {}\n", v.n));
    if !approver.trim().is_empty() {
        msg.push_str(&format!("ForgeCAD-Approver: {}\n", approver.trim()));
    }
    msg.push_str(&format!("ForgeCAD-Kernel: {}\n", crate::store::KERNEL));
    for (kind, fname, data) in &files {
        msg.push_str(&format!(
            "ForgeCAD-Export: {kind} sha256:{} {fname}\n",
            HashKind::Sha256.of(data).hex()
        ));
    }
    if let Some(a) = agent {
        msg.push_str(&format!("ForgeCAD-Agent: {a}\n"));
    }
    let who = h
        .who()
        .ok_or("no user.name and user.email to sign the release with")?;
    h.repo
        .tag(&names::release_tag(&rel, &rev), &v.save, &msg, &who)
        .map_err(|e| e.to_string())?;
    write_all(&release_dir(&h, &folder, &rev), &files)?;
    let tag = h
        .repo
        .refs
        .resolve(&format!("refs/tags/{}", names::release_tag(&rel, &rev)))
        .map_err(|e| e.to_string())?
        .ok_or("the release record vanished")?;
    let t = h
        .repo
        .read_tag(&tag)
        .map_err(|e| e.to_string())?
        .ok_or("not a tag")?;
    Ok(parse(&h, &rel, &rev, &t.message, v.save, who.time, who.name))
}

/// What Regenerate found: each export's hash as rebuilt against the record's.
#[derive(Clone, Debug, PartialEq)]
pub struct Regenerated {
    pub record: Record,
    pub files: Vec<(String, bool)>,
    /// The record's kernel is not this one: a mismatch is drift, not an error.
    pub other_kernel: bool,
    pub dir: PathBuf,
}

/// Rebuilds a release from its record with this kernel, writes its files, and checks them.
pub fn regenerate(file: &Path, rev: &str) -> Result<Regenerated, String> {
    let h = History::open(file).ok_or_else(|| format!("{} keeps no history", file.display()))?;
    let rel = h.rel(file).map_err(|e| e.to_string())?;
    let r = records(&h, Some(&rel))?
        .into_iter()
        .find(|r| r.revision == rev)
        .ok_or_else(|| format!("{rel} has no revision {rev}"))?;
    let name = format!("{}-{rev}", stem(&rel));
    let (files, pn) = exports(&h, &rel, &r.save, &name)?;
    let dir = release_dir(&h, &folder_of(&pn, &rel), rev);
    write_all(&dir, &files)?;
    let checked = r
        .exports
        .iter()
        .map(|e| {
            let now = files
                .iter()
                .find(|f| f.1 == e.file)
                .map(|f| HashKind::Sha256.of(&f.2).hex());
            (e.file.clone(), now.as_deref() == Some(e.sha256.as_str()))
        })
        .collect();
    Ok(Regenerated {
        other_kernel: r.kernel != crate::store::KERNEL,
        record: r,
        files: checked,
        dir,
    })
}

pub fn record_json(r: &Record) -> Json {
    Json::obj(vec![
        ("file", Json::Str(r.path.clone())),
        ("part_number", Json::Str(r.part_number.clone())),
        ("revision", Json::Str(r.revision.clone())),
        ("version", Json::Num(r.version as f64)),
        ("reason", Json::Str(r.reason.clone())),
        ("approver", Json::Str(r.approver.clone())),
        ("kernel", Json::Str(r.kernel.clone())),
        ("save", Json::Str(r.save.hex()[..10].to_string())),
        (
            "exports",
            Json::Arr(
                r.exports
                    .iter()
                    .map(|e| {
                        Json::obj(vec![
                            ("kind", Json::Str(e.kind.clone())),
                            ("file", Json::Str(e.file.clone())),
                            ("sha256", Json::Str(e.sha256.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

pub fn regenerated_json(g: &Regenerated) -> Json {
    let all_same = g.files.iter().all(|f| f.1);
    Json::obj(vec![
        ("release", record_json(&g.record)),
        ("dir", Json::Str(g.dir.display().to_string())),
        (
            "files",
            Json::Arr(
                g.files
                    .iter()
                    .map(|(f, ok)| Json::obj(vec![("file", Json::Str(f.clone())), ("same", Json::Bool(*ok))]))
                    .collect(),
            ),
        ),
        ("same_kernel", Json::Bool(!g.other_kernel)),
        (
            "result",
            Json::Str(
                match (all_same, g.other_kernel) {
                    (true, _) => "identical",
                    (false, true) => {
                        "drift: this kernel changes the released files; a change needs a new revision"
                    }
                    (false, false) => "mismatch: the same kernel built different files",
                }
                .into(),
            ),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asme_revisions() {
        assert_eq!(next_revision(None, false), "A");
        assert_eq!(next_revision(Some("H"), false), "J");
        assert_eq!(next_revision(Some("N"), false), "P");
        assert_eq!(next_revision(Some("R"), false), "T");
        assert_eq!(next_revision(Some("W"), false), "Y");
        assert_eq!(next_revision(Some("Y"), false), "AA");
        assert_eq!(next_revision(Some("AY"), false), "BA");
        assert_eq!(next_revision(None, true), "1");
        assert_eq!(next_revision(Some("2"), true), "3");
        assert_eq!(next_revision(Some("2"), false), "A");
        assert_eq!(next_revision(Some("C"), true), "D");
        let mut r = "A".to_string();
        for _ in 0..500 {
            let n = next_revision(Some(&r), false);
            assert!(order(&n) > order(&r));
            assert!(!n.bytes().any(|c| b"IOQSXZ".contains(&c)));
            r = n;
        }
    }
}
