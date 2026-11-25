//! File workflow for Simplify. A generated .fg document records the source and rules. Its
//! import points to a checked snapshot, so opening it is independent of a refresh.
use forge_agent::json::Json;
use forge_lang::simplify::{self, Rules, Source};
use forge_math::hash::hash128;
use forge_topo::Name;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

const HEADER: &str = "# forgecad-simplify-v1 ";

struct Recipe {
    source: PathBuf,
    revision: u128,
    assets: Vec<(String, String)>,
    rules: Rules,
    paused: bool,
}

impl Recipe {
    fn text(&self) -> Result<String, String> {
        let source = self.source.to_str().ok_or("Source path must be UTF-8")?;
        let data = Json::obj(vec![
            ("source", source.into()),
            ("revision", format!("{:032x}", self.revision).into()),
            (
                "assets",
                Json::Arr(
                    self.assets
                        .iter()
                        .map(|(name, file)| Json::Arr(vec![name.as_str().into(), file.as_str().into()]))
                        .collect(),
                ),
            ),
            ("paused", self.paused.into()),
            ("rules", rules_json(&self.rules)),
        ]);
        Ok(format!("{HEADER}{data}\n{}", self.part_text()))
    }

    fn part_text(&self) -> String {
        use std::fmt::Write as _;
        let mut text = String::new();
        for (name, asset) in &self.assets {
            let binding = if name == "part" && self.assets.len() == 1 {
                "body".to_string()
            } else {
                let label: String = name
                    .chars()
                    .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                    .take(48)
                    .collect();
                format!("body_{label}_{:016x}", Name::NONE.with_str(name).0)
            };
            let _ = writeln!(text, "{binding} = import(\"{asset}\")");
        }
        text
    }

    fn read(text: &str) -> Result<Self, String> {
        let header = text
            .lines()
            .next()
            .and_then(|s| s.strip_prefix(HEADER))
            .ok_or("Expected a linked Simplify document")?;
        let value = Json::parse(header)?;
        let string = |key| {
            value
                .get(key)
                .and_then(Json::str)
                .ok_or_else(|| format!("Recipe needs {key}"))
        };
        let Some(Json::Arr(entries)) = value.get("assets") else {
            return Err("Recipe needs assets".into());
        };
        let assets = entries
            .iter()
            .map(|entry| {
                let Json::Arr(pair) = entry else {
                    return Err("Invalid snapshot entry");
                };
                let [Json::Str(name), Json::Str(file)] = pair.as_slice() else {
                    return Err("Invalid snapshot entry");
                };
                let hash = file
                    .strip_prefix("simplified-")
                    .and_then(|s| s.strip_suffix(".step"))
                    .ok_or("Invalid snapshot name")?;
                if hash.len() != 32 || u128::from_str_radix(hash, 16).is_err() {
                    return Err("Invalid snapshot name");
                }
                Ok((name.clone(), file.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if assets.is_empty() {
            return Err("Recipe needs at least one snapshot".into());
        }
        let recipe = Self {
            source: PathBuf::from(string("source")?),
            revision: u128::from_str_radix(string("revision")?, 16).map_err(|_| "Invalid source revision")?,
            assets,
            rules: read_rules(value.get("rules").ok_or("Recipe needs rules")?)?,
            paused: match value.get("paused") {
                Some(Json::Bool(b)) => *b,
                _ => return Err("Recipe needs a link state".into()),
            },
        };
        if recipe.text()? != text {
            return Err("Simplify document was edited. Save it as a separate part before refreshing".into());
        }
        Ok(recipe)
    }
}

fn strings(values: &BTreeSet<String>) -> Json {
    Json::Arr(values.iter().map(|v| v.as_str().into()).collect())
}

fn face_picks(values: &BTreeMap<String, BTreeSet<Name>>) -> Json {
    Json::Arr(
        values
            .iter()
            .flat_map(|(id, ns)| ns.iter().map(move |n| Json::Str(format!("{id}={:016x}", n.0))))
            .collect(),
    )
}

// The JSON report rounds measurements for display. A recipe must replay the exact limits.
fn limit_json(limit: Option<f64>) -> Json {
    limit.map_or(Json::Null, |value| Json::Str(value.to_string()))
}

fn rules_json(r: &Rules) -> Json {
    Json::obj(vec![
        ("hole_diameter", limit_json(r.hole_diameter)),
        ("pocket_diagonal", limit_json(r.pocket_diagonal)),
        ("boss_diagonal", limit_json(r.boss_diagonal)),
        ("component_diagonal", limit_json(r.component_diagonal)),
        ("keep", strings(&r.keep)),
        ("exclude", strings(&r.exclude)),
        ("envelopes", strings(&r.envelopes)),
        ("keep_faces", face_picks(&r.keep_faces)),
        ("remove_faces", face_picks(&r.remove_faces)),
    ])
}

fn array<'a>(
    j: &'a Json,
    key: &'static str,
) -> Result<impl Iterator<Item = Result<&'a str, String>>, String> {
    let Some(Json::Arr(items)) = j.get(key) else {
        return Err(format!("Recipe needs {key}"));
    };
    Ok(items
        .iter()
        .map(move |v| v.str().ok_or_else(|| format!("{key} must contain strings"))))
}

fn read_rules(j: &Json) -> Result<Rules, String> {
    let limit = |key| match j.get(key) {
        Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) => s
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .map(Some)
            .ok_or_else(|| format!("Invalid limit {key}")),
        _ => Err(format!("Invalid limit {key}")),
    };
    let set = |key| {
        array(j, key)?
            .map(|s| s.map(str::to_owned))
            .collect::<Result<BTreeSet<_>, _>>()
    };
    let faces = |key| {
        let mut out = BTreeMap::new();
        for item in array(j, key)? {
            add_face(&mut out, item?)?;
        }
        Ok::<_, String>(out)
    };
    Ok(Rules {
        hole_diameter: limit("hole_diameter")?,
        pocket_diagonal: limit("pocket_diagonal")?,
        boss_diagonal: limit("boss_diagonal")?,
        component_diagonal: limit("component_diagonal")?,
        keep: set("keep")?,
        exclude: set("exclude")?,
        envelopes: set("envelopes")?,
        keep_faces: faces("keep_faces")?,
        remove_faces: faces("remove_faces")?,
    })
}

fn add_face(out: &mut BTreeMap<String, BTreeSet<Name>>, value: &str) -> Result<(), String> {
    let (part, name) = value.rsplit_once('=').ok_or("Face picks use COMPONENT=HEX")?;
    let name = Name(u64::from_str_radix(name, 16).map_err(|_| "Invalid face name")?);
    out.entry(part.into()).or_default().insert(name);
    Ok(())
}

#[derive(Default)]
struct Args {
    out: Option<PathBuf>,
    json: bool,
    replace_rules: bool,
    faces: bool,
    rules: Rules,
}

fn args(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut words = args.iter().map(String::as_str);
    while let Some(word) = words.next() {
        match word {
            "--json" => out.json = true,
            "--faces" => out.faces = true,
            "--replace-rules" => out.replace_rules = true,
            "-o" => out.out = Some(PathBuf::from(words.next().ok_or("Missing output path")?)),
            "--holes" | "--pockets" | "--bosses" | "--components" => {
                let value: f64 = words
                    .next()
                    .ok_or("Missing size limit")?
                    .parse()
                    .map_err(|_| "Expected millimetres")?;
                if !value.is_finite() || value < 0.0 {
                    return Err("Size limits must be finite and nonnegative".into());
                }
                match word {
                    "--holes" => out.rules.hole_diameter = Some(value),
                    "--pockets" => out.rules.pocket_diagonal = Some(value),
                    "--bosses" => out.rules.boss_diagonal = Some(value),
                    _ => out.rules.component_diagonal = Some(value),
                }
            }
            "--keep" | "--exclude" | "--envelope" => {
                let set = match word {
                    "--keep" => &mut out.rules.keep,
                    "--exclude" => &mut out.rules.exclude,
                    _ => &mut out.rules.envelopes,
                };
                set.insert(words.next().ok_or("Missing component name")?.into());
            }
            "--keep-face" | "--remove-face" => {
                let map = if word == "--keep-face" {
                    &mut out.rules.keep_faces
                } else {
                    &mut out.rules.remove_faces
                };
                add_face(map, words.next().ok_or("Missing face name")?)?;
            }
            _ => return Err(format!("Unknown Simplify option {word}")),
        }
    }
    Ok(out)
}

/// Place a complete file without overwriting an existing snapshot. On refresh, replace the
/// document last, after the snapshot exists. An interrupted refresh leaves the previous valid
/// document pointing to the previous snapshot.
fn publish(path: &Path, bytes: &[u8], old: Option<&[u8]>) -> Result<(), String> {
    let io = || -> std::io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("File needs a parent"))?;
        let mut attempt = 0;
        let (tmp, mut file) = loop {
            let tmp = parent.join(format!(".forge-simplify-{}-{attempt}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&tmp) {
                Ok(file) => break (tmp, file),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => attempt += 1,
                Err(e) => return Err(e),
            }
        };
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            if let Some(old) = old {
                if fs::symlink_metadata(path)?.file_type().is_symlink() || fs::read(path)? != old {
                    return Err(std::io::Error::other("Document changed during Simplify"));
                }
                fs::rename(&tmp, path)
            } else {
                fs::hard_link(&tmp, path)
            }
        })();
        let _ = fs::remove_file(tmp);
        result
    };
    io().map_err(|e| format!("{}: {e}", path.display()))
}

fn relative(from: &Path, to: &Path) -> PathBuf {
    let shared = from
        .components()
        .zip(to.components())
        .take_while(|(a, b)| a == b)
        .count();
    if shared == 0 {
        return to.to_path_buf();
    }
    let mut relative = PathBuf::new();
    for _ in from.components().skip(shared) {
        relative.push("..");
    }
    relative.extend(to.components().skip(shared));
    relative
}

fn output_path(path: &Path) -> Result<PathBuf, String> {
    if path.extension().is_none_or(|x| x != "fg") {
        return Err("Simplify output must be a .fg file".into());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let dir = fs::canonicalize(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    Ok(dir.join(path.file_name().ok_or("Missing document filename")?))
}

fn report(model: &simplify::ResultModel, source: &Source, document: Option<&Path>) -> Json {
    let changes = model
        .changes
        .iter()
        .map(|c| {
            Json::obj(vec![
                ("part", c.part.as_str().into()),
                ("action", c.action.into()),
                (
                    "faces",
                    Json::Arr(
                        c.faces
                            .iter()
                            .map(|n| Json::Str(format!("{:016x}", n.0)))
                            .collect(),
                    ),
                ),
                ("size_mm", c.size.map_or(Json::Null, Json::Num)),
            ])
        })
        .collect();
    Json::obj(vec![
        ("ok", true.into()),
        ("revision", format!("{:032x}", source.revision).into()),
        (
            "faces",
            model
                .parts
                .iter()
                .map(|(_, body)| body.face_ids().count())
                .sum::<usize>()
                .into(),
        ),
        ("changes", Json::Arr(changes)),
        (
            "document",
            document.map_or(Json::Null, |p| Json::Str(p.display().to_string())),
        ),
    ])
}

fn save(
    path: &Path,
    recipe: &mut Recipe,
    model: &simplify::ResultModel,
    source: &Source,
    old: Option<&[u8]>,
) -> Result<(), String> {
    recipe.assets.clear();
    for (name, body) in &model.parts {
        let asset = save_asset(path, body)?;
        recipe.assets.push((name.clone(), asset));
    }
    let text = recipe.text()?;
    if !source.current() {
        return Err("Source changed during Simplify. Refresh again with the new revision".into());
    }
    publish(path, text.as_bytes(), old)
}

fn save_asset(path: &Path, body: &forge_topo::Body) -> Result<String, String> {
    let step = forge_step::export(body, &forge_step::Options::default()).map_err(|e| e.to_string())?;
    let read = forge_step::import(step.text.as_bytes(), &forge_step::Limits::default())
        .map_err(|e| e.to_string())?;
    if !read.refusals.is_empty()
        || read.solids.len() != step.solids
        || read.solids.iter().any(|s| !forge_topo::check(&s.body).is_empty())
    {
        return Err("The simplified snapshot did not reopen as valid solids".into());
    }
    let asset_name = format!("simplified-{:032x}.step", hash128(step.text.as_bytes()));
    let asset = path.with_file_name(&asset_name);
    let bytes = step.text.as_bytes();
    match fs::symlink_metadata(&asset) {
        Ok(m) if m.is_file() && fs::read(&asset).is_ok_and(|saved| saved == bytes) => {}
        Ok(_) => return Err(format!("{}: snapshot name is occupied", asset.display())),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            if let Err(e) = publish(&asset, bytes, None)
                && !(fs::symlink_metadata(&asset).is_ok_and(|m| m.is_file())
                    && fs::read(&asset).is_ok_and(|saved| saved == bytes))
            {
                return Err(e);
            }
        }
        Err(e) => return Err(format!("{}: {e}", asset.display())),
    }
    Ok(asset_name)
}

fn under_root(path: PathBuf, root: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(root) = root {
        let git = path.strip_prefix(root).ok().map(|relative| {
            relative
                .components()
                .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case(".git"))
        });
        if git != Some(false) {
            return Err("Simplify path must be under the project root and outside .git".into());
        }
    }
    Ok(path)
}

fn operation(argv: &[String], root: Option<&Path>) -> Result<(Json, String), String> {
    let [action, input, rest @ ..] = argv else {
        return Err(
            "Use simplify preview|create SOURCE or simplify refresh|pause|resume|detach DERIVED.fg".into(),
        );
    };
    let args = args(rest)?;
    if action == "preview" || action == "create" {
        if args.replace_rules
            || (action == "preview" && args.out.is_some())
            || (action == "create" && args.faces)
        {
            return Err("Unexpected option for this command".into());
        }
        let source_path = under_root(
            fs::canonicalize(input).map_err(|e| format!("{input}: {e}"))?,
            root,
        )?;
        let source = simplify::load(&source_path, root)?;
        let model = simplify::build(&source, &args.rules)?;
        if action == "preview" {
            let mut j = report(&model, &source, None);
            if args.faces
                && let Json::Obj(fields) = &mut j
            {
                fields.push((
                    "source_faces".into(),
                    Json::Arr(
                        source
                            .faces()
                            .map(|(part, name, surface)| {
                                Json::obj(vec![
                                    ("part", part.into()),
                                    ("face", format!("{:016x}", name.0).into()),
                                    ("surface", surface.into()),
                                ])
                            })
                            .collect(),
                    ),
                ));
            }
            return Ok((j.clone(), j.to_string()));
        }
        let path = under_root(output_path(&args.out.ok_or("Create needs -o DERIVED.fg")?)?, root)?;
        if fs::symlink_metadata(&path).is_ok() {
            return Err(format!("{} already exists", path.display()));
        }
        let mut recipe = Recipe {
            source: relative(path.parent().expect("absolute document path"), &source_path),
            revision: source.revision,
            assets: Vec::new(),
            rules: args.rules,
            paused: false,
        };
        save(&path, &mut recipe, &model, &source, None)?;
        return Ok((
            report(&model, &source, Some(&path)),
            format!("Created {}", path.display()),
        ));
    }
    if !matches!(action.as_str(), "refresh" | "pause" | "resume" | "detach")
        || args.out.is_some()
        || args.faces
    {
        return Err("Expected refresh, pause, resume or detach with a derived .fg document".into());
    }
    if (args.rules != Rules::default() && !args.replace_rules) || (args.replace_rules && action != "refresh")
    {
        return Err("Use refresh --replace-rules to replace the saved rules".into());
    }
    let path = under_root(output_path(Path::new(input))?, root)?;
    if fs::symlink_metadata(&path)
        .map_err(|e| format!("{input}: {e}"))?
        .file_type()
        .is_symlink()
    {
        return Err("Select the actual derived file rather than a symbolic link".into());
    }
    let mut lock = fs::File::open(&path).map_err(|e| format!("{input}: {e}"))?;
    lock.try_lock()
        .map_err(|e| format!("{input}: could not lock the derived document: {e}"))?;
    let mut old = String::new();
    (&mut lock)
        .take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut old)
        .map_err(|e| format!("{input}: {e}"))?;
    if old.len() > 4 * 1024 * 1024 {
        return Err("Simplify document is too large".into());
    }
    let mut recipe = Recipe::read(&old)?;
    if action != "refresh" {
        recipe.paused = action == "pause";
        let text = if action == "detach" {
            recipe.part_text()
        } else {
            recipe.text()?
        };
        publish(&path, text.as_bytes(), Some(old.as_bytes()))?;
        let line = format!("{action}: {}", path.display());
        return Ok((
            Json::obj(vec![
                ("ok", true.into()),
                ("document", path.display().to_string().into()),
            ]),
            line,
        ));
    }
    if recipe.paused {
        return Err("The source link is paused".into());
    }
    let source_path = path
        .parent()
        .expect("absolute document path")
        .join(&recipe.source);
    let source_path = under_root(
        fs::canonicalize(source_path).map_err(|e| format!("Source: {e}"))?,
        root,
    )?;
    if source_path == path {
        return Err("A derived document cannot be its own source".into());
    }
    let source = simplify::load(&source_path, root)?;
    if args.replace_rules {
        recipe.rules = args.rules;
    } else if source.imported && recipe.revision != source.revision && recipe.rules.uses_picks() {
        return Err("Imported source changed. Review the new component and face picks before refreshing with --replace-rules".into());
    }
    let model = simplify::build(&source, &recipe.rules)?;
    recipe.revision = source.revision;
    save(&path, &mut recipe, &model, &source, Some(old.as_bytes()))?;
    Ok((
        report(&model, &source, Some(&path)),
        format!("Refreshed {}", path.display()),
    ))
}

pub fn run(argv: &[String]) -> std::process::ExitCode {
    super::said(operation(argv, None), argv.iter().any(|a| a == "--json"))
}

pub(crate) fn rpc(root: &Path, writable: bool, params: &Json) -> Result<(Json, String), String> {
    let action = params
        .get("action")
        .and_then(Json::str)
        .ok_or("Simplify needs an action")?;
    if !writable && action != "preview" {
        return Err("Only preview is available in a read-only session".into());
    }
    let file = params
        .get("file")
        .and_then(Json::str)
        .ok_or("Simplify needs a file")?;
    let root_path = |name: &str| {
        root.join(name)
            .to_str()
            .map(str::to_owned)
            .ok_or("Path is not UTF-8")
    };
    let mut words = vec![action.into(), root_path(file)?];
    if let Some(options) = params.get("rules") {
        let Json::Arr(options) = options else {
            return Err("rules must be an array of option strings".into());
        };
        for option in options {
            let word = option.str().ok_or("rules must contain option strings")?;
            if word == "-o" {
                return Err("Supply the output as the out parameter".into());
            }
            words.push(word.into());
        }
    }
    if let Some(out) = params.get("out").and_then(Json::str) {
        words.extend(["-o".into(), root_path(out)?]);
    }
    operation(&words, Some(root))
}
