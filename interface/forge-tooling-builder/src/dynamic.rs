//! The tooling kinds by their word, for a UI that switches between them at run time, and each
//! record's outline as a revolved profile for its preview.
use crate::{ColletNuts, Collets, Holders, Inserts, PullStuds, Rec, Tools, TurnHolders};
use forge_library::{Catalog, Entry, Kind, Level};
use forge_tooling::build::Record;
use forge_tooling::toml::{Table, Val};
use std::path::{Path, PathBuf};

/// The catalog of `word`'s records (project, user `cam/` folder): entries and load errors.
/// A kind's catalog: its entries and the files that did not load.
pub type Listed = (Vec<Entry<Rec>>, Vec<(Level, String, String)>);

pub fn catalog(word: &str, project: Option<&Path>, user_cam: Option<&Path>) -> Listed {
    fn go<K: Kind<Record = Rec>>(p: Option<&Path>, u: Option<&Path>) -> Listed {
        let user = u.map(|u| u.join(K::FOLDER));
        let c = Catalog::<K>::load(p, user.as_deref());
        (c.entries, c.errors)
    }
    match word {
        "tool" => go::<Tools>(project, user_cam),
        "holder" => go::<Holders>(project, user_cam),
        "collet" => go::<Collets>(project, user_cam),
        "collet_nut" => go::<ColletNuts>(project, user_cam),
        "pull_stud" => go::<PullStuds>(project, user_cam),
        "insert" => go::<Inserts>(project, user_cam),
        "turn_holder" => go::<TurnHolders>(project, user_cam),
        _ => (Vec::new(), Vec::new()),
    }
}

/// The folder `word`'s records are kept in at a level.
pub fn level_dir(
    word: &str,
    level: Level,
    project: Option<&Path>,
    user_cam: Option<&Path>,
) -> Option<PathBuf> {
    let folder = crate::KINDS.iter().find(|(w, _, _)| *w == word)?.1;
    match level {
        Level::Project => Some(project?.join("cam").join(folder)),
        Level::User => Some(user_cam?.join(folder)),
        Level::Builtin => None,
    }
}

/// Saves a record of `word` (forge_library::save: read back before writing).
pub fn save(word: &str, rec: &Rec, dir: &Path, level: Level, overwrite: bool) -> Result<PathBuf, String> {
    match word {
        "tool" => forge_library::save::<Tools>(rec, &[], dir, level, overwrite),
        "holder" => forge_library::save::<Holders>(rec, &[], dir, level, overwrite),
        "collet" => forge_library::save::<Collets>(rec, &[], dir, level, overwrite),
        "collet_nut" => forge_library::save::<ColletNuts>(rec, &[], dir, level, overwrite),
        "pull_stud" => forge_library::save::<PullStuds>(rec, &[], dir, level, overwrite),
        "insert" => forge_library::save::<Inserts>(rec, &[], dir, level, overwrite),
        "turn_holder" => forge_library::save::<TurnHolders>(rec, &[], dir, level, overwrite),
        _ => Err(format!("no kind `{word}`")),
    }
}

/// A record's table read again as `word` (after an edit): the record, or why it does not read.
pub fn reread(word: &str, table: &Table) -> Result<Rec, String> {
    let text = crate::write::table_file(word, table)?;
    let mut v = crate::parse_kind(word, &text)?;
    v.pop().ok_or_else(|| format!("no {word} read"))
}

/// A new record of `word` derived from `from` under `id` (forge_tooling::build::derive, the
/// levels applied), with no overrides: edited afterwards in the form.
pub fn derive(
    word: &str,
    from: &str,
    id: &str,
    project: Option<&Path>,
    user_cam: Option<&Path>,
) -> Result<Rec, String> {
    forge_library::check_id(id)?;
    let (lib, _) = crate::library(project, user_cam);
    let typed = forge_tooling::build::derive(&lib, word, from, id, &[]).map_err(|e| format!("{e:?}"))?;
    let table = crate::cli::derived_table_pub(&lib, word, from, id, &[])?;
    Ok(Rec {
        id: id.to_string(),
        table,
        typed,
    })
}

/// Sets field `key` to `v` as the user's value: the field's cites are replaced by one `user`.
pub fn set_user(table: &mut Table, key: &str, v: Val) {
    table.insert(key.to_string(), v);
    let mut cites: Vec<Table> = match table.remove("cite") {
        Some(Val::Tables(c)) => c,
        _ => Vec::new(),
    };
    cites.retain(|c| forge_tooling::toml::text(c, "path") != Some(key));
    let mut c = Table::new();
    c.insert("path".into(), Val::Str(key.into()));
    c.insert("status".into(), Val::Str("user".into()));
    cites.push(c);
    table.insert("cite".into(), Val::Tables(cites));
}

/// A field's provenance status (its first cite), or None.
pub fn status_of(table: &Table, key: &str) -> Option<String> {
    match table.get("cite") {
        Some(Val::Tables(c)) => c
            .iter()
            .find(|c| forge_tooling::toml::text(c, "path") == Some(key))
            .and_then(|c| forge_tooling::toml::text(c, "status"))
            .map(str::to_string),
        _ => None,
    }
}

/// The record's outline as (z along its axis from the tip or nose, radius) points, to revolve
/// about the axis for a preview. None for records with no turned shape (inserts, turn holders).
pub fn outline(r: &Rec) -> Option<Vec<[f64; 2]>> {
    use forge_tooling::Kind as K;
    match &r.typed {
        Record::Tool(t) => {
            let (rc, rs) = (t.dc / 2.0, t.dcon / 2.0);
            let mut p = vec![[0.0, 0.0]];
            match t.kind {
                K::Ball => {
                    for k in 1..=8 {
                        let th = std::f64::consts::FRAC_PI_2 * k as f64 / 8.0;
                        let (sn, cs) = forge_math::dmath::sin_cos(th);
                        p.push([rc * (1.0 - cs), rc * sn]);
                    }
                }
                K::Bull if t.re > 0.0 => {
                    p.push([0.0, (rc - t.re).max(0.0)]);
                    for k in 1..=6 {
                        let th = std::f64::consts::FRAC_PI_2 * k as f64 / 6.0;
                        let (sn, cs) = forge_math::dmath::sin_cos(th);
                        p.push([t.re * (1.0 - cs), rc - t.re + t.re * sn]);
                    }
                }
                _ if t.angle > 0.0 && t.angle < 180.0 => {
                    let r0 = t.tip_d / 2.0;
                    let (sn, cs) = forge_math::dmath::sin_cos(t.angle.to_radians() / 2.0);
                    let h = (rc - r0) * cs / sn;
                    p.push([0.0, r0]);
                    p.push([h.max(0.0), rc]);
                }
                _ => p.push([0.0, rc]),
            }
            p.push([t.lcf, rc]);
            // a necked tool: the neck from the flutes to its reach, then the shank
            let neck = forge_tooling::toml::num(&r.table, "neck_d");
            let reach = forge_tooling::toml::num(&r.table, "reach");
            match (neck, reach) {
                (Some(nd), Some(lu)) if nd > 0.0 && lu > t.lcf && lu < t.oal => {
                    p.push([t.lcf, nd / 2.0]);
                    p.push([lu, nd / 2.0]);
                    p.push([lu, rs]);
                }
                _ => p.push([t.lcf, rs]),
            }
            p.push([t.oal, rs]);
            p.push([t.oal, 0.0]);
            Some(p)
        }
        Record::Holder(h) => {
            let mut p = vec![[h.profile[0][0], 0.0]];
            p.extend(h.profile.iter().map(|[z, d]| [*z, d / 2.0]));
            p.push([h.profile[h.profile.len() - 1][0], 0.0]);
            Some(p)
        }
        Record::Collet(c) => {
            let (len, od) = (c.length?, c.od?);
            Some(vec![
                [0.0, c.bore / 2.0],
                [0.0, od / 2.0],
                [len, od / 2.0],
                [len, c.bore / 2.0],
            ])
        }
        Record::ColletNut(n) => Some(vec![
            [0.0, n.od / 2.0 * 0.6],
            [0.0, n.od / 2.0],
            [n.length, n.od / 2.0],
            [n.length, n.od / 2.0 * 0.6],
        ]),
        Record::PullStud(s) => Some(vec![[0.0, 0.0], [0.0, 6.0], [s.length, 6.0], [s.length, 0.0]]),
        _ => None,
    }
}
