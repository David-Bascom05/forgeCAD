//! Text (`text`) and imported solids (`import`, `entity`): the files they read are found beside
//! the part file (`Options::dir`), read each time and parsed again only when their bytes change.

use crate::builtins::{Args, LEN, dim, not_in_sketch, plane_of, point_of, positive};
use crate::error::{Error, Role};
use crate::eval::{Eval, Faces, Feat, R, Value};
use crate::feat_error::{Because, FeatError, FileProblem};
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Ty};
use crate::wired::{feat, word};
use forge_math::Frame;
use forge_model::{Combine, Feature, FontRef, Import, Text, TextOn};
use forge_topo::{Body, Name};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
/// Files read, by path: the bytes read and what they came to. A file is known by its bytes alone,
/// never by its size and time: a file replaced with its time put back (`cp -p`, `rsync -t`, a
/// checkout) is read anew.
type Cache<T> = Mutex<Option<HashMap<PathBuf, (Arc<[u8]>, Arc<T>)>>>;

static FONTS: Cache<forge_font::Font> = Mutex::new(None);
static SOLIDS: Cache<Body> = Mutex::new(None);

const FONT_KINDS: &[&str] = &["ttf", "otf"];
const SOLID_KINDS: &[&str] = &["step", "stp", "brep", "igs", "iges"];
/// The most a file may be: larger ones are refused unread.
const LIMIT: u64 = 64 << 20;

/// A file beside the part file. Under a session root (`root`, canonical), nothing outside it is
/// found: every such refusal is the same `Missing`.
fn find(dir: Option<&Path>, root: Option<&Path>, file: &str, kinds: &[&str], span: Span) -> R<PathBuf> {
    let err = |problem| -> Error {
        FeatError::File {
            span,
            file: file.into(),
            problem,
        }
        .into()
    };
    let ext = Path::new(file)
        .extension()
        .and_then(|x| x.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !kinds.contains(&ext.as_str()) {
        return Err(err(FileProblem::WrongKind));
    }
    let dir = dir.ok_or_else(|| err(FileProblem::Missing))?;
    let found = find_in(dir, root, file, span);
    // read only once the fence, kind and size let it through; else only whether it is there
    match &found {
        Ok(p) => crate::deps::file(p),
        Err(_) => crate::deps::looked(&dir.join(file)),
    }
    found
}

fn find_in(dir: &Path, root: Option<&Path>, file: &str, span: Span) -> R<PathBuf> {
    let err = |problem| -> Error {
        FeatError::File {
            span,
            file: file.into(),
            problem,
        }
        .into()
    };
    if crate::outside(Some(dir), root, file) {
        return Err(err(FileProblem::Outside));
    }
    let p = std::fs::canonicalize(dir.join(file)).map_err(|_| err(FileProblem::Missing))?;
    if root.is_some_and(|r| !p.starts_with(r)) {
        return Err(err(FileProblem::Outside));
    }
    let meta = std::fs::metadata(&p).map_err(|_| err(FileProblem::Missing))?;
    // a device, a fifo, a socket, a folder: no file of its kind, never opened
    if !meta.is_file() {
        return Err(err(FileProblem::WrongKind));
    }
    if meta.len() > LIMIT {
        return Err(err(FileProblem::Unreadable(format!(
            "larger than {} MB",
            LIMIT >> 20
        ))));
    }
    Ok(p)
}

fn cached<T>(
    cache: &Cache<T>,
    path: PathBuf,
    parse: impl FnOnce(&[u8]) -> Result<T, FileProblem>,
) -> Result<Arc<T>, FileProblem> {
    // a refusal says nothing of what the file holds
    let data = crate::deps::read_plain(&path, LIMIT).map_err(|r| match r {
        crate::deps::Refused::NotPlain => FileProblem::WrongKind,
        crate::deps::Refused::TooBig(_) => FileProblem::Unreadable(format!("larger than {} MB", LIMIT >> 20)),
        crate::deps::Refused::Io(_) => FileProblem::Unreadable("could not be read".into()),
    })?;
    if let Ok(c) = cache.lock()
        && let Some((b, v)) = c.as_ref().and_then(|m| m.get(&path))
        && **b == data[..]
    {
        return Ok(v.clone());
    }
    let v = Arc::new(parse(&data)?);
    if let Ok(mut c) = cache.lock() {
        c.get_or_insert_with(HashMap::new)
            .insert(path, (data.into(), v.clone()));
    }
    Ok(v)
}

/// Whether a file is a TrueType or OpenType font's: its tag, and a table directory
/// whose every table lies inside the file (`len`). A collection: its tag and fonts' offsets.
fn is_font(head: &[u8], len: u64) -> bool {
    let u16_at = |k: usize| head.get(k..k + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let u32_at = |k: usize| {
        head.get(k..k + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    match head.get(..4) {
        Some(b"ttcf") => {
            let Some(n) = u32_at(8).filter(|n| (1..=64).contains(n)) else {
                return false;
            };
            (0..n as usize).all(|k| u32_at(12 + 4 * k).is_some_and(|o| (o as u64) + 12 <= len))
        }
        Some([0, 1, 0, 0]) | Some(b"OTTO") | Some(b"true") => {
            let Some(n) = u16_at(4).filter(|n| (1..=256).contains(n)) else {
                return false;
            };
            (0..n as usize).all(|k| {
                let r = 12 + 16 * k;
                let tag_ok = head
                    .get(r..r + 4)
                    .is_some_and(|t| t.iter().all(|c| (0x20..0x7f).contains(c)));
                match (u32_at(r + 8), u32_at(r + 12)) {
                    (Some(o), Some(l)) => tag_ok && o as u64 + l as u64 <= len,
                    _ => false,
                }
            })
        }
        _ => false,
    }
}

/// Whether a file is a model of the kind its extension says: STEP's header, an
/// IGES start line, an OpenCascade BREP's banner.
fn is_model(ext: &str, head: &[u8]) -> bool {
    match ext {
        "step" | "stp" => {
            let t = head.strip_prefix(b"\xef\xbb\xbf").unwrap_or(head);
            let k = t.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(t.len());
            t[k..].starts_with(b"ISO-10303-21")
        }
        "igs" | "iges" => {
            let line = head.split(|&c| c == b'\n').next().unwrap_or(&[]);
            line.len() >= 73
                && line[72] == b'S'
                && line
                    .get(73..80)
                    .is_some_and(|n| n.iter().all(|c| c.is_ascii_digit() || *c == b' '))
        }
        "brep" => head.windows(16).take(256).any(|w| w == b"CASCADE Topology"),
        _ => false,
    }
}

fn ext_of(file: &str) -> String {
    Path::new(file)
        .extension()
        .and_then(|x| x.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

pub fn font(dir: Option<&Path>, root: Option<&Path>, file: &str, span: Span) -> R<Arc<forge_font::Font>> {
    let p = find(dir, root, file, FONT_KINDS, span)?;
    cached(&FONTS, p, |data| {
        // its kind by what it holds (a font's tag and table directory), else never parsed
        if !is_font(data, data.len() as u64) {
            return Err(FileProblem::WrongKind);
        }
        // what the font reader says of it would be its bytes: only that it is no font it reads
        forge_font::Font::parse(data.to_vec()).map_err(|_| FileProblem::WrongKind)
    })
    .map_err(|problem| {
        FeatError::File {
            span,
            file: file.into(),
            problem,
        }
        .into()
    })
}

pub fn solid(dir: Option<&Path>, root: Option<&Path>, file: &str, span: Span) -> R<Arc<Body>> {
    let p = find(dir, root, file, SOLID_KINDS, span)?;
    let ext = ext_of(file);
    cached(&SOLIDS, p, |data| {
        // its kind by what it holds (the format's header), else never parsed
        if !is_model(&ext, data) {
            return Err(FileProblem::WrongKind);
        }
        // the reader's words would quote the file: only how far it got
        let imp = forge_step::import(data, &forge_step::Limits::default())
            .map_err(|_| FileProblem::Unreadable("not a model this reads".into()))?;
        if imp.solids.is_empty() {
            return Err(FileProblem::Unreadable(format!(
                "no solid this reads ({} refused)",
                imp.refusals.len()
            )));
        }
        let mut body = Body::new();
        for s in &imp.solids {
            body.append(&s.body);
        }
        Ok(body)
    })
    .map_err(|problem| {
        FeatError::File {
            span,
            file: file.into(),
            problem,
        }
        .into()
    })
}

fn string(a: &Args, k: &'static str) -> R<String> {
    match a.req(k)? {
        Value::Str(s) => Ok(s.to_string()),
        o => Err(Error::WrongType {
            span: a.span(k),
            what: k,
            want: Ty::Str,
            found: o.ty(),
        }),
    }
}

pub fn text(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "text", span)?;
    let id = ev.take_id("text", span)?;
    let a = Args::bind(
        ev,
        "text",
        &[
            "text", "font", "size", "depth", "on", "at", "cut", "wrap", "bodies",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let text = string(&a, "text")?;
    let file = string(&a, "font")?;
    // `check` with no part file to look beside reads nothing
    let font = match (ev.dir.is_some() || ev.b.building)
        .then(|| font(ev.dir.as_deref(), ev.root.as_deref(), &file, a.span("font")))
    {
        Some(f) => Some(f?),
        None => None,
    };
    let size = a.req_num("size", LEN)?;
    let depth = a.req_num("depth", LEN)?;
    positive(&size, a.span("size"), "a text's size")?;
    positive(&depth, a.span("depth"), "a text's depth")?;
    let mut dims = vec![dim("size", &size), dim("depth", &depth)];
    let plane = plane_of(ev, a.req("on")?, a.span("on"), "on")?;
    let frame = match a.opt("at") {
        Some(v) => {
            let p = point_of(ev, v, a.span("at"))?;
            plane.with_origin(p - plane.z().get() * (p - plane.origin).dot(plane.z().get()))
        }
        None => plane,
    };
    let on = match a.num("wrap", &[Dim::Length])? {
        Some(r) => {
            positive(&r, a.span("wrap"), "a wrap radius")?;
            dims.push(dim("wrap radius", &r));
            TextOn::Cylinder {
                axis: frame,
                radius: r.v,
            }
        }
        None => TextOn::Plane(frame),
    };
    let t = Text {
        text,
        font: FontRef { name: file, font },
        size: size.v,
        on,
        depth: depth.v,
        op: if a.flag("cut")? {
            Combine::Cut
        } else {
            Combine::Add
        },
    };
    let f = crate::bodies::wrap_in(ev, &a, Feature::Text(Box::new(t)))?;
    let (idx, hid) = ev.push_feature(&id, "text", f, span, dims)?;
    Ok(feat(id, "text", idx, hid, None))
}

/// A text's letter tops (embossed) or floors (engraved), each glyph region's cap.
pub fn text_member(ev: &mut Eval, f: &std::rc::Rc<Feat>, name: &str, span: Span) -> Option<R<Value>> {
    let Some(Feature::Text(t)) = crate::holes::kernel_feature(ev, f.idx) else {
        return None;
    };
    let (role, top) = match (name, t.op) {
        ("top", Combine::Add) => (Role::Top, true),
        ("floor", Combine::Cut) => (Role::Floor, false),
        _ => return None,
    };
    let regions = t
        .font
        .font
        .as_ref()
        .and_then(|fo| forge_model::text::layout(fo, &t.text, t.size).ok())
        .map_or(0, |r| r.len());
    let names: Vec<Name> = (0..regions)
        .map(|k| {
            if top {
                forge_model::text::roles::end(f.hid, k)
            } else {
                forge_model::text::roles::start(f.hid, k)
            }
        })
        .collect();
    if !ev.b.building || ev.b.built() <= f.idx {
        return Some(Ok(Value::Unresolved));
    }
    let body = ev.b.body().expect("built");
    let faces: Vec<Name> = names
        .iter()
        .flat_map(|&n| crate::build::role_faces(body, n))
        .map(|x| body.face(x).name)
        .collect();
    if faces.is_empty() {
        return Some(Err(Error::NoFace {
            span,
            id: f.id.clone(),
            role,
        }));
    }
    let plane = match &t.on {
        TextOn::Plane(fr) => Some(Frame::from_zx(fr.origin, fr.z(), fr.x().get()).unwrap_or(*fr)),
        _ => None,
    };
    Some(Ok(Value::Faces(std::rc::Rc::new(Faces {
        names: faces,
        owner: f.id.clone(),
        role,
        plane,
    }))))
}

pub fn import(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "import", span)?;
    let id = ev.take_id("import", span)?;
    let a = Args::bind(ev, "import", &["file", "op", "bodies"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let file = string(&a, "file")?;
    let body = match (ev.dir.is_some() || ev.b.building)
        .then(|| solid(ev.dir.as_deref(), ev.root.as_deref(), &file, a.span("file")))
    {
        Some(b) => Some(b?),
        None => None,
    };
    let op = match word(&a, "op", &["new", "add", "cut"])? {
        Some("add") => Combine::Add,
        Some("cut") => Combine::Cut,
        _ => Combine::New,
    };
    if op != Combine::New && !crate::builtins::has_solid(ev) {
        return Err(FeatError::NotApplicable {
            span: a.span("op"),
            func: "import",
            arg: "op",
            because: Because::NotSolid,
        }
        .into());
    }
    let f = Feature::Import(Box::new(Import { file, op, body }));
    let f = crate::bodies::wrap_in(ev, &a, f)?;
    let (idx, hid) = ev.push_feature(&id, "import", f, span, vec![])?;
    Ok(feat(id, "import", idx, hid, None))
}

/// `entity(of: IMPORT, solid: N, id: N)`: the face or edge a STEP file's entity `#id` (of its
/// solid `#solid`) became, by the name the reader gave it.
pub fn entity(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, "entity", &["of", "solid", "id"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let f = match a.req("of")? {
        Value::Feature(f) if f.kind == "import" => f,
        o => {
            return Err(Error::WrongType {
                span: a.span("of"),
                what: "an import",
                want: Ty::Feature,
                found: o.ty(),
            });
        }
    };
    let whole = |k: &'static str| -> R<u64> {
        let n = crate::builtins::num_of(a.req(k)?, &[Dim::Count], a.span(k), k)?;
        Ok(n.v.round() as u64)
    };
    let (solid, entity) = (whole("solid")?, whole("id")?);
    let salt = Name::feature(f.hid).with_str("import");
    let n = Name::feature(solid).with_str("step").with(entity).salted(salt);
    if !ev.b.building || ev.b.built() <= f.idx {
        return Ok(Value::Unresolved);
    }
    let body = ev.b.body().expect("built");
    let faces: Vec<Name> = crate::build::role_faces(body, n)
        .into_iter()
        .map(|x| body.face(x).name)
        .collect();
    if !faces.is_empty() {
        let plane = match &faces[..] {
            [one] => body
                .face_ids()
                .find(|&x| body.face(x).name == *one)
                .and_then(|x| crate::builtins::face_plane(body, x)),
            _ => None,
        };
        return Ok(Value::Faces(std::rc::Rc::new(Faces {
            names: faces,
            owner: f.id.clone(),
            role: Role::Faces,
            plane,
        })));
    }
    let edges: Vec<Name> = body
        .edge_ids()
        .map(|e| body.edge(e).name)
        .filter(|m| *m == n || m.is_piece_of(n))
        .collect();
    if edges.is_empty() {
        return Err(Error::NoFace {
            span,
            id: f.id.clone(),
            role: Role::Faces,
        });
    }
    Ok(Value::Edges(std::rc::Rc::new(edges)))
}
