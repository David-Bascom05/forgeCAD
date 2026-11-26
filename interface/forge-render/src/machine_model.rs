//! A model the user attached to a machine (forge_machine_builder::models): its file read to
//! triangles in its own coordinates (STEP, IGES and Parasolid through forge-step, meshed as the app
//! meshes a file's body; STL as it is), kept per file, and placed in the base frame at home.
use crate::machine_mesh::Tris;
use forge_machine_builder::models::{Format, Model};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Read files, by path and (length, modified time): a file replaced is read again.
type Read = Mutex<HashMap<(PathBuf, u64, Option<std::time::SystemTime>), Result<FileModel, String>>>;

#[derive(Clone)]
pub struct FileModel {
    pub triangles: Arc<Tris>,
    pub notes: Arc<[String]>,
}

/// A model file's triangles in its own coordinates and units.
pub fn file_tris(path: &Path) -> Result<Arc<Tris>, String> {
    file_model(path).map(|m| m.triangles)
}

pub fn file_model(path: &Path) -> Result<FileModel, String> {
    static READ: OnceLock<Read> = OnceLock::new();
    // through the app's filesystem: std's has no files in a page
    let meta = forge_dirs::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let key = (path.to_path_buf(), meta.len(), meta.modified().ok());
    let c = READ.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(r) = c.lock().expect("no panics while held").get(&key) {
        return r.clone();
    }
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let r = forge_dirs::fs::read(path)
        .map_err(|e| format!("{name}: {e}"))
        .and_then(|bytes| read_model(&name, &bytes));
    c.lock().expect("no panics while held").insert(key, r.clone());
    r
}

/// A model's bytes to triangles (`name` picks the format): forge_machine_builder::model_files
/// reads it (the CLI's reader too), flat-shaded here, a patch per part.
pub fn read(name: &str, bytes: &[u8]) -> Result<Tris, String> {
    let (tris, _) = read_noting(name, bytes)?;
    Ok(tris)
}

fn read_model(name: &str, bytes: &[u8]) -> Result<FileModel, String> {
    let (triangles, notes) = read_noting(name, bytes)?;
    Ok(FileModel {
        triangles: Arc::new(triangles),
        notes: notes.into(),
    })
}

fn read_noting(name: &str, bytes: &[u8]) -> Result<(Tris, Vec<String>), String> {
    let (pieces, notes) = forge_machine_builder::model_files::read_noting(name, bytes)?;
    let _ = Format::of(name)?;
    let mut t = Tris::default();
    for p in &pieces {
        push(&mut t, &p.pos, &p.tri);
    }
    Ok((t, notes))
}

/// A message's first line and the rest: what a panel shows, and what it shows on hover.
pub fn headline(msg: &str) -> (&str, &str) {
    msg.split_once('\n').unwrap_or((msg, ""))
}

/// Triangles flat-shaded, each with its own corners.
fn push(t: &mut Tris, pos: &[[f64; 3]], tris: &[[u32; 3]]) {
    use forge_machine::kin::{cross, norm, sub};
    let (t0, v0) = (t.tri.len(), t.pos.len());
    for tri in tris {
        let [a, b, c] = tri.map(|i| pos[i as usize]);
        let n = cross(sub(b, a), sub(c, a));
        let l = norm(n);
        let n = if l > 0.0 {
            n.map(|x| x / l)
        } else {
            [0.0, 0.0, 1.0]
        };
        let k = t.pos.len() as u32;
        t.pos.extend([a, b, c]);
        t.nrm.extend([n; 3]);
        t.tri.push([k, k + 1, k + 2]);
    }
    t.patches.push((
        t0 as u32,
        (t.tri.len() - t0) as u32,
        v0 as u32,
        (t.pos.len() - v0) as u32,
    ));
}

/// The model's triangles placed in the base frame at home.
pub fn placed(t: &Tris, m: &Model) -> Tris {
    let r = m.rigid();
    Tris {
        pos: t.pos.iter().map(|p| m.place(*p)).collect(),
        nrm: t.nrm.iter().map(|n| r.dir(*n)).collect(),
        tri: t.tri.clone(),
        edges: t
            .edges
            .iter()
            .map(|e| e.iter().map(|p| m.place(*p)).collect())
            .collect(),
        patches: t.patches.clone(),
    }
}

/// At most this many of a model's points are checked against its envelope (evenly spread).
pub const CHECKED: usize = 200_000;

/// The model's distinct-enough points, placed (a sample of at most `CHECKED`).
pub fn placed_points(t: &Tris, m: &Model) -> Vec<[f64; 3]> {
    let step = t.pos.len().div_ceil(CHECKED).max(1);
    t.pos.iter().step_by(step).map(|p| m.place(*p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_partial_geometry_keeps_its_warning() {
        use forge_math::{Frame, Point2};
        use forge_ops::{Profile, extrude};
        let plate = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::rectangle(Point2::new(-5., -3.), 10., 6., 0),
                Profile::rectangle(Point2::new(-1., -1.), 2., 2., 9),
            ],
        };
        let text = forge_step::export(&extrude(&plate, 2., 1).unwrap(), &Default::default())
            .unwrap()
            .text;
        let src = "#33=CARTESIAN_POINT('',(0.,0.,0.));";
        assert!(text.contains(src));
        let path =
            forge_dirs::temp_dir().join(format!("forge-partial-model-{}.step", forge_dirs::process_id()));
        forge_dirs::fs::write(&path, text.replace(src, "#33=CARTESIAN_POINT('',(0.,0.,-50.));")).unwrap();
        let first = file_model(&path).unwrap();
        let reopened = file_model(&path).unwrap();
        assert!(!first.triangles.tri.is_empty());
        assert!(
            first.notes.iter().any(|s| s.contains("50.0 mm")),
            "{:?}",
            first.notes
        );
        assert!(Arc::ptr_eq(&first.notes, &reopened.notes));
        forge_dirs::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_ascii_stl_reads_and_lands_where_its_model_says() {
        let stl = b"solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid x\n";
        let t = read("part.stl", stl).unwrap();
        assert_eq!(t.tri.len(), 1);
        assert_eq!(t.nrm[0], [0.0, 0.0, 1.0]);
        let mut m = Model::new(None, "part.stl".into());
        m.scale = 25.4;
        m.offset = [0.0, 0.0, 100.0];
        let p = placed(&t, &m);
        assert_eq!(p.pos[1], [25.4, 0.0, 100.0]);
        assert_eq!(placed_points(&t, &m).len(), 3);
        // an STL's bytes under a Parasolid name: read as what its name says, and refused
        let e = read("part.x_t", stl).unwrap_err();
        assert!(e.starts_with("part.x_t couldn't be read as a Parasolid model\n"), "{e}");
        let (head, why) = headline(&e);
        assert_eq!(head, "part.x_t couldn't be read as a Parasolid model");
        assert!(!why.is_empty());
        assert!(read("empty.stl", b"solid x\nendsolid x\n").is_err());
    }
}
