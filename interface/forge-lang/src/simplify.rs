//! Prepare a derived solid from the source's evaluated geometry. Writing a document or
//! publishing a new revision is the caller's job.
use crate::{Options, deps, drawing::Model, kept};
use forge_math::{Frame, Point2, Point3, Vec3, hash::hash128};
use forge_ops::{
    Profile, delete_faces, extrude,
    simplify::{planar_details, through_bores},
};
use forge_topo::{Body, Name, extent::exact_box};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rules {
    pub hole_diameter: Option<f64>,
    pub pocket_diagonal: Option<f64>,
    pub boss_diagonal: Option<f64>,
    pub component_diagonal: Option<f64>,
    pub keep: BTreeSet<String>,
    pub exclude: BTreeSet<String>,
    pub envelopes: BTreeSet<String>,
    pub keep_faces: BTreeMap<String, BTreeSet<Name>>,
    pub remove_faces: BTreeMap<String, BTreeSet<Name>>,
}

impl Rules {
    pub fn uses_picks(&self) -> bool {
        !self.keep.is_empty()
            || !self.exclude.is_empty()
            || !self.envelopes.is_empty()
            || !self.keep_faces.is_empty()
            || !self.remove_faces.is_empty()
    }
}

pub struct Source {
    parts: Vec<Part>,
    pub revision: u128,
    pub imported: bool,
    path: PathBuf,
    dependencies: deps::Deps,
}

impl Source {
    pub fn faces(&self) -> impl Iterator<Item = (&str, Name, &'static str)> + '_ {
        self.parts.iter().flat_map(|part| {
            part.body.face_ids().map(move |id| {
                let face = part.body.face(id);
                (part.id.as_str(), face.name, face.surface.kind())
            })
        })
    }

    pub fn current(&self) -> bool {
        dependencies_current(&self.dependencies, &self.path)
    }
}

fn dependencies_current(deps: &deps::Deps, source: &Path) -> bool {
    deps.current_with_limits(|p| {
        if p == source { forge_step::Limits::default().max_bytes as u64 } else { deps::LIMIT }
    })
}

struct Part {
    id: String,
    body: Arc<Body>,
    frame: Frame,
}

pub struct ResultModel {
    pub parts: Vec<(String, Body)>,
    pub changes: Vec<Change>,
}

#[derive(Debug, PartialEq)]
pub struct Change {
    pub part: String,
    pub action: &'static str,
    pub faces: Vec<Name>,
    pub size: Option<f64>,
}

/// Load the finished native part or all solved occurrences. For an exchange file, use
/// the reader's occurrence frames instead of concatenating unplaced product bodies.
pub fn load(path: &Path, root: Option<&Path>) -> Result<Source, String> {
    let bytes = deps::read_plain(path, forge_step::Limits::default().max_bytes as u64)
        .map_err(|e| format!("{}: {e:?}", path.display()))?;
    if path.extension().is_some_and(|ext| ext == "fg") {
        let text = std::str::from_utf8(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let options = Options {
            dir: path.parent().map(Path::to_path_buf),
            root: root.map(Path::to_path_buf),
            render: true,
            checks: false,
            overlap: false,
            expects: false,
            meshes: false,
            ..Options::default()
        };
        let (run, dependencies) =
            kept::watched(path, text, || crate::run(text, options, Default::default()).0);
        if !run.ok() {
            return Err(format!("{}: {}", path.display(), crate::report::json(&run, text)));
        }
        let model = Model::of(&run).ok_or("Source did not build a solid")?;
        if run
            .asm
            .as_ref()
            .is_some_and(|a| a.occurrences.len() != model.bodies.len())
        {
            return Err("Some assembly occurrences have no resolved solid".into());
        }
        if !dependencies_current(&dependencies, path) {
            return Err("Source dependencies changed while building".into());
        }
        let parts: Vec<_> = model
            .bodies
            .into_iter()
            .zip(model.paths)
            .map(|((body, frame), path)| Part {
                id: if path.is_empty() { "part".into() } else { path },
                body,
                frame,
            })
            .collect();
        // A native wrapper can still pick topology that came from an exchange file.
        let imported = parts.iter().any(|part| part.body.from_file)
            || dependencies.0.keys().any(|file| {
                file.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| crate::media::SOLID_KINDS.contains(&ext.to_ascii_lowercase().as_str()))
            });
        return Ok(Source {
            path: path.to_owned(),
            parts,
            revision: dependencies.digest(),
            imported,
            dependencies,
        });
    }

    let watch = deps::watch();
    deps::read(path, Some(&bytes));
    let dependencies = watch.done();
    let imported = forge_step::import(&bytes, &forge_step::Limits::default()).map_err(|e| e.to_string())?;
    if let Some(refusal) = imported.refusals.first() {
        return Err(format!(
            "Source is incomplete: entity #{} {}",
            refusal.entity, refusal.why
        ));
    }
    let parts = if imported.occurrences.is_empty() {
        if imported.stats.assembly > 0 {
            return Err("Source assembly placements could not be resolved".into());
        }
        imported
            .solids
            .into_iter()
            .map(|solid| Part {
                id: format!("#{}", solid.entity),
                body: Arc::new(solid.body),
                frame: Frame::WORLD,
            })
            .collect()
    } else {
        let solids: BTreeMap<_, _> = imported
            .solids
            .into_iter()
            .map(|s| (s.entity, Arc::new(s.body)))
            .collect();
        let referenced: BTreeSet<_> = imported.occurrences.iter().map(|o| o.solid).collect();
        if solids.keys().any(|id| !referenced.contains(id)) {
            return Err("Source contains solids with no resolved occurrence".into());
        }
        let ids = step_part_names(&imported.occurrences);
        imported
            .occurrences
            .into_iter()
            .zip(ids)
            .map(|(occ, id)| {
                let body = solids
                    .get(&occ.solid)
                    .ok_or_else(|| format!("Missing solid #{}", occ.solid))?
                    .clone();
                Ok(Part {
                    id,
                    body,
                    frame: occ.frame,
                })
            })
            .collect::<Result<_, String>>()?
    };
    Ok(Source {
        path: path.to_owned(),
        parts,
        revision: hash128(&bytes),
        imported: true,
        dependencies,
    })
}

fn step_part_names(occurrences: &[forge_step::Occurrence]) -> Vec<String> {
    let product_paths: Vec<_> = occurrences.iter().map(|occ| occ.path.join("/")).collect();
    let mut counts = BTreeMap::new();
    for path in &product_paths {
        *counts.entry(path.as_str()).or_insert(0) += 1;
    }
    let candidates: Vec<_> = occurrences
        .iter()
        .zip(&product_paths)
        .map(|(occ, path)| {
            if counts[path.as_str()] == 1 {
                path.clone()
            } else {
                occ.instance_path.join("/")
            }
        })
        .collect();
    let mut used: BTreeSet<_> = product_paths.clone().into_iter().collect();
    used.extend(candidates.iter().cloned());
    let mut assigned: BTreeSet<_> = product_paths
        .iter()
        .filter(|path| counts[path.as_str()] == 1)
        .cloned()
        .collect();
    let mut suffixes = BTreeMap::new();
    product_paths
        .iter()
        .zip(candidates)
        .map(|(product, candidate)| {
            if counts[product.as_str()] == 1 || assigned.insert(candidate.clone()) {
                return candidate;
            }
            let suffix = suffixes.entry(candidate.clone()).or_insert(1);
            loop {
                *suffix += 1;
                let name = format!("{candidate} [{suffix}]");
                if used.insert(name.clone()) {
                    return name;
                }
            }
        })
        .collect()
}

fn validate(source: &Source, rules: &Rules) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for part in &source.parts {
        if !ids.insert(part.id.as_str()) {
            return Err(format!("Ambiguous component name {:?}", part.id));
        }
    }
    for id in rules
        .keep
        .iter()
        .chain(&rules.exclude)
        .chain(&rules.envelopes)
        .chain(rules.keep_faces.keys())
        .chain(rules.remove_faces.keys())
    {
        if !ids.contains(id.as_str()) {
            return Err(format!("Component {id:?} is missing in the current source"));
        }
    }
    for limit in [
        rules.hole_diameter,
        rules.pocket_diagonal,
        rules.boss_diagonal,
        rules.component_diagonal,
    ]
    .into_iter()
    .flatten()
    {
        if !limit.is_finite() || limit < 0.0 {
            return Err("Size limits must be finite and nonnegative".into());
        }
    }
    Ok(())
}

fn selected_faces(body: &Body, part: &str, names: Option<&BTreeSet<Name>>) -> Result<BTreeSet<Name>, String> {
    let Some(names) = names else {
        return Ok(BTreeSet::new());
    };
    for &name in names {
        let mut matching = body.face_ids().filter(|&f| body.face(f).name == name);
        if matching.next().is_none() || matching.next().is_some() {
            return Err(format!("{part}: missing or ambiguous face {:016x}", name.0));
        }
    }
    Ok(names.clone())
}

fn checked(body: &Body, part: &str) -> Result<(), String> {
    if body.face_ids().next().is_none() {
        return Err(format!("{part}: simplification produced an empty solid"));
    }
    if let Some(problem) = forge_topo::check(body).first() {
        return Err(format!("{part}: {}", problem.reason()));
    }
    Ok(())
}

pub fn build(source: &Source, rules: &Rules) -> Result<ResultModel, String> {
    validate(source, rules)?;
    let mut result = ResultModel {
        parts: Vec::new(),
        changes: Vec::new(),
    };
    for part in &source.parts {
        let id = &part.id;
        let body = &part.body;
        let keep = selected_faces(body, id, rules.keep_faces.get(id))?;
        let mut remove = selected_faces(body, id, rules.remove_faces.get(id))?;
        let bounds = exact_box(body);
        let size = Vec3::new(bounds.x.width(), bounds.y.width(), bounds.z.width());
        let diagonal = size.length();
        let protected = rules.keep.contains(id) || !keep.is_empty();
        let exclude = rules.exclude.contains(id);
        let envelope = rules.envelopes.contains(id);
        if (exclude || envelope) && (protected || !remove.is_empty() || (exclude && envelope)) {
            return Err(format!("{id}: conflicting component and face rules"));
        }
        let small = source.parts.len() > 1 && rules.component_diagonal.is_some_and(|limit| diagonal <= limit);
        if !protected && (exclude || (small && !envelope && remove.is_empty())) {
            result.changes.push(Change {
                part: id.clone(),
                action: "exclude",
                faces: vec![],
                size: Some(diagonal),
            });
            continue;
        }
        let edited = if envelope {
            let plane = Frame::WORLD.with_origin(Point3::new(bounds.x.lo, bounds.y.lo, bounds.z.lo));
            let profile = Profile {
                plane,
                loops: vec![Profile::rectangle(Point2::ORIGIN, size.x, size.y, 0)],
            };
            let solid = extrude(&profile, size.z, Name::NONE.with_str(id).0)
                .map_err(|e| format!("{id}: cannot make envelope: {e:?}"))?;
            result.changes.push(Change {
                part: id.clone(),
                action: "envelope",
                faces: vec![],
                size: Some(diagonal),
            });
            Some(solid)
        } else {
            let mut protect = keep;
            if rules.keep.contains(id) {
                protect.extend(body.face_ids().map(|f| body.face(f).name));
            }
            if !remove.is_disjoint(&protect) {
                return Err(format!("{id}: a protected face was selected for removal"));
            }
            if !remove.is_empty() {
                result.changes.push(Change {
                    part: id.clone(),
                    action: "heal",
                    faces: remove.iter().copied().collect(),
                    size: None,
                });
            }
            if !rules.keep.contains(id)
                && let Some(limit) = rules.hole_diameter
            {
                for bore in through_bores(body) {
                    let held = bore.walls.iter().chain(&bore.ends).any(|n| protect.contains(n));
                    let action = if held || bore.diameter > limit {
                        "keep bore"
                    } else {
                        "fill bore"
                    };
                    if action == "fill bore" {
                        remove.extend(&bore.walls);
                    }
                    result.changes.push(Change {
                        part: id.clone(),
                        action,
                        faces: bore.walls,
                        size: Some(bore.diameter),
                    });
                }
            }
            if !rules.keep.contains(id) && (rules.pocket_diagonal.is_some() || rules.boss_diagonal.is_some())
            {
                for detail in planar_details(body) {
                    let (limit, action) = if detail.pocket {
                        (rules.pocket_diagonal, "fill pocket")
                    } else {
                        (rules.boss_diagonal, "remove boss")
                    };
                    if !limit.is_some_and(|limit| detail.diagonal <= limit) {
                        continue;
                    }
                    let held =
                        protect.contains(&detail.support) || detail.faces.iter().any(|n| protect.contains(n));
                    if !held {
                        remove.extend(&detail.faces);
                    }
                    result.changes.push(Change {
                        part: id.clone(),
                        action: if held { "keep detail" } else { action },
                        faces: detail.faces,
                        size: Some(detail.diagonal),
                    });
                }
            }
            if remove.is_empty() {
                None
            } else {
                let solid = delete_faces(body, &remove.into_iter().collect::<Vec<_>>())
                    .map_err(|e| format!("{id}: cannot heal selected faces: {e:?}"))?;
                for name in protect {
                    let before = body.face_by_name(name).ok_or("Protected face is missing")?;
                    let after = solid
                        .face_by_name(name)
                        .ok_or_else(|| format!("{id}: protected face {:016x} disappeared", name.0))?;
                    if body.face(before).surface != solid.face(after).surface
                        || forge_agent::facemesh::key(body, before)
                            != forge_agent::facemesh::key(&solid, after)
                    {
                        return Err(format!("{id}: protected face {:016x} changed", name.0));
                    }
                }
                Some(solid)
            }
        };
        let shaped = edited.as_ref().unwrap_or(body);
        checked(shaped, id)?;
        result
            .parts
            .push((id.clone(), forge_asm::check::rigid_copy(shaped, &part.frame)));
        result.changes.push(Change {
            part: id.clone(),
            action: "include",
            faces: vec![],
            size: Some(diagonal),
        });
    }
    if result.parts.is_empty() {
        return Err("The rules excluded every solid".into());
    }
    Ok(result)
}
