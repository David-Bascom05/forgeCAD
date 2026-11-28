//! Native packages are installed explicitly and pinned by version. Reading the
//! manifest does not start the worker. One process serves successive rule evaluations.
use super::*;
use crate::electronics_library::worker::{Diagnostic, Fact, Input};
use std::{
    io::{BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

#[derive(Debug)]
pub(crate) struct Leaf {
    runtime: Arc<Runtime>,
    pub(super) properties: BTreeMap<String, sch::AttributeType>,
    pub(super) rules: Vec<(RuleSpec, Input)>,
}
#[derive(Debug)]
struct Runtime {
    exe: PathBuf,
    session: Mutex<Option<Session>>,
}
#[derive(Debug)]
struct Session {
    child: Child,
    requests: mpsc::Sender<Option<String>>,
    replies: mpsc::Receiver<Result<Json, String>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Session {
    fn start(exe: &Path, arg: &str) -> Result<Self, String> {
        let mut child = Command::new(exe)
            .arg(arg)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let (requests, rx) = mpsc::channel::<Option<String>>();
        let (tx, replies) = mpsc::channel();
        std::thread::spawn(move || {
            for req in rx {
                let result = (|| {
                    if let Some(line) = req {
                        writeln!(input, "{line}")
                            .and_then(|_| input.flush())
                            .map_err(|e| e.to_string())?;
                    }
                    worker::read_frame(&mut output)?.ok_or_else(|| "Electronics worker exited".into())
                })();
                let failed = result.is_err();
                if tx.send(result).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            requests,
            replies,
        })
    }
    fn request(&self, req: Option<String>) -> Result<Json, String> {
        if req.as_ref().is_some_and(|s| s.len() + 1 > worker::LIMIT) {
            return Err("Electronics worker request exceeds limit".into());
        }
        self.requests.send(req).map_err(|e| e.to_string())?;
        self.replies
            .recv_timeout(Duration::from_secs(5))
            .map_err(|e| format!("Electronics worker unavailable: {e}"))?
    }
}
fn safe(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}
fn worker_file() -> &'static str {
    if cfg!(windows) { "worker.exe" } else { "worker" }
}
fn base() -> Result<PathBuf, String> {
    forge_dirs::config()
        .map(|p| p.join("forgecad/electronics"))
        .ok_or("No user config directory".into())
}
fn get<'a>(j: &'a Json, k: &str) -> Result<&'a str, String> {
    j.get(k)
        .and_then(Json::str)
        .ok_or_else(|| format!("Missing worker field {k}"))
}
fn array<'a>(j: &'a Json, k: &str) -> Result<&'a [Json], String> {
    match j.get(k) {
        Some(Json::Arr(xs)) => Ok(xs),
        _ => Err(format!("Invalid worker array {k}")),
    }
}
struct Node {
    name: String,
    properties: BTreeMap<String, sch::AttributeType>,
    rules: Vec<(String, Severity, Input)>,
    children: Vec<Node>,
    requirements: Vec<(String, String, Option<String>)>,
}
impl Node {
    fn read(j: &Json, depth: usize, ids: &mut BTreeSet<String>) -> Result<Self, String> {
        if depth > 32 {
            return Err("Package nesting exceeds 32 levels".into());
        }
        let name = get(j, "name")?.to_string();
        if !safe(&name) {
            return Err("Invalid package name".into());
        }
        let src = get(j, "properties")?;
        for stmt in crate::syntax::parse(src).map_err(|e| e.text())? {
            use crate::syntax::{ExprKind, StmtKind};
            if !matches!(stmt.kind,StmtKind::Assign{value,..} if matches!(&value.kind,ExprKind::Call(head,_) if matches!(&head.kind,ExprKind::Name(n) if n=="attribute_type")))
            {
                return Err("Worker properties must contain attribute types".into());
            }
        }
        let properties = sch::read(&format!("symbol_library\n{src}"), None, None)
            .map_err(|e| e.message)?
            .attribute_types
            .into_iter()
            .filter(|(id, _)| !id.starts_with("std."))
            .collect();
        let mut rules = vec![];
        let mut requirements = vec![];
        for r in array(j, "checks")? {
            let id = get(r, "id")?;
            if !safe(id) || !ids.insert(id.to_owned()) {
                return Err(format!("Invalid or duplicate rule {id}"));
            }
            let level = match get(r, "severity")? {
                "error" => Severity::Error,
                "warning" => Severity::Warning,
                "info" => Severity::Info,
                _ => return Err("Invalid severity".into()),
            };
            if r.get("requires").is_some() {
                for need in array(r, "requires")? {
                    let field = get(need, "field")?;
                    let symbol = match need.get("symbol") {
                        None | Some(Json::Null) => None,
                        Some(Json::Str(s)) => Some(s.clone()),
                        _ => return Err("Invalid symbol selector".into()),
                    };
                    requirements.push((id.into(), field.into(), symbol));
                }
            }
            rules.push((
                id.to_owned(),
                level,
                Input::parse(get(r, "input")?).ok_or("Invalid input kind")?,
            ));
        }
        let mut children = Vec::new();
        let mut names = BTreeSet::new();
        for child in array(j, "children")? {
            let n = Self::read(child, depth + 1, ids)?;
            if !names.insert(n.name.clone()) {
                return Err(format!("Duplicate child {}", n.name));
            }
            children.push(n);
        }
        Ok(Self {
            name,
            properties,
            rules,
            children,
            requirements,
        })
    }
    fn activate(mut self, rt: &Arc<Runtime>, uri: String) -> &'static Library {
        // Installed, version-pinned descriptors live for the application lifetime and
        // are interned only once per package. Findings can borrow their stable IDs.
        let mut lib = Library::new(Box::leak(self.name.into_boxed_str()));
        for (name, property) in &mut self.properties {
            property.origin = Some(Box::leak(format!("{uri}#{name}").into_boxed_str()));
        }
        lib.children = Box::leak(
            self.children
                .into_iter()
                .map(|n| {
                    let path = format!("{uri}/{}", n.name);
                    n.activate(rt, path)
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let intern = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
        lib.requirements = Box::leak(
            self.requirements
                .into_iter()
                .map(|(rule, field, symbol)| worker::RequiredField {
                    rule: intern(rule),
                    field: intern(field),
                    symbol: symbol.map(intern),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        lib.external = Some(Box::leak(Box::new(Leaf {
            runtime: rt.clone(),
            properties: self.properties,
            rules: self
                .rules
                .into_iter()
                .map(|(id, severity, i)| {
                    (
                        RuleSpec {
                            id: Box::leak(id.into_boxed_str()),
                            severity,
                        },
                        i,
                    )
                })
                .collect(),
        })));
        Box::leak(Box::new(lib))
    }
}
fn manifest(j: &Json) -> Result<(String, Node), String> {
    if get(j, "protocol")? != worker::PROTOCOL {
        return Err("Unsupported electronics protocol".into());
    }
    let version = get(j, "version")?;
    if !safe(version) {
        return Err("Invalid package version".into());
    }
    Ok((
        version.into(),
        Node::read(
            j.get("library").ok_or("Missing library")?,
            0,
            &mut BTreeSet::new(),
        )?,
    ))
}
/// Install a trusted executable for this user. Each name/version is immutable.
pub fn install(exe: &Path) -> Result<String, String> {
    let exe = std::fs::canonicalize(exe).map_err(|e| e.to_string())?;
    let base = base()?;
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let stage = base.join(format!(
        ".install-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    std::fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let result = (|| {
        let binary = stage.join(worker_file());
        std::fs::copy(&exe, &binary).map_err(|e| e.to_string())?;
        let data = Session::start(&binary, "--manifest")?.request(None)?;
        let (v, n) = manifest(&data)?;
        let pkg = base.join(&n.name);
        std::fs::create_dir_all(&pkg).map_err(|e| e.to_string())?;
        std::fs::write(stage.join("manifest.json"), data.to_string()).map_err(|e| e.to_string())?;
        std::fs::rename(&stage, pkg.join(&v)).map_err(|e| format!("Cannot install {}@{v}: {e}", n.name))?;
        Ok(format!("rust:{}@{v}", n.name))
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}
pub(crate) fn resolve(uri: &str) -> Result<Option<&'static Library>, String> {
    let Some(path) = uri.strip_prefix("rust:") else {
        return Ok(super::lookup(uri));
    };
    let mut seg = path.split('/');
    let first = seg.next().unwrap_or("");
    let Some((name, v)) = first.split_once('@') else {
        return super::lookup(uri)
            .map(Some)
            .ok_or_else(|| format!("Missing Rust library {uri}. Installed libraries use rust:name@version"));
    };
    if !safe(name) || !safe(v) {
        return Err("Invalid installed package ID".into());
    }
    static CACHE: OnceLock<Mutex<BTreeMap<String, &'static Library>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    let mut lib = if let Some(&lib) = cache.get(first) {
        lib
    } else {
        let dir = base()?.join(name).join(v);
        let bytes = crate::deps::read_plain(&dir.join("manifest.json"), worker::LIMIT as u64)
            .map_err(|_| format!("Install {first} to open this project"))?;
        let (version, node) = manifest(&Json::parse(
            std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
        )?)?;
        if version != v || node.name != name {
            return Err(format!("Manifest identity differs from {first}"));
        }
        let lib = node.activate(
            &Arc::new(Runtime {
                exe: dir.join(worker_file()),
                session: Mutex::new(None),
            }),
            format!("rust:{first}"),
        );
        cache.insert(first.to_owned(), lib);
        lib
    };
    for part in seg {
        lib = lib
            .children
            .iter()
            .copied()
            .find(|c| c.name == part)
            .ok_or_else(|| format!("Unknown child library {part} in {uri}"))?;
    }
    Ok(Some(lib))
}
fn run(leaf: &Leaf, id: &str, input: Input, facts: &[Fact]) -> Result<Vec<Diagnostic>, String> {
    let req = Json::obj(vec![
        ("rule", id.into()),
        ("input", input.word().into()),
        ("facts", Json::Arr(facts.iter().map(Fact::json).collect())),
    ]);
    let mut lock = leaf.runtime.session.lock().unwrap();
    if lock.is_none() {
        *lock = Some(Session::start(&leaf.runtime.exe, "--worker")?);
    }
    let reply = lock
        .as_ref()
        .unwrap()
        .request(Some(req.to_string()))
        .inspect_err(|_| {
            lock.take();
        })?;
    let rows = if let Json::Obj(_) = &reply {
        for skipped in array(&reply, "not_run")? {
            let object = get(skipped, "object")?;
            if get(skipped, "rule")? != id || object.is_empty() {
                return Err("Invalid worker coverage witness".into());
            }
            for entry in array(skipped, "missing")? {
                evidence::missing(id, object, entry.str().ok_or("Invalid missing field")?);
            }
        }
        reply.get("findings").ok_or("Missing findings")?
    } else {
        &reply
    };
    let Json::Arr(rows) = rows else {
        return Err("Invalid worker response".into());
    };
    rows.iter()
        .map(|r| {
            let objects = array(r, "objects")?
                .iter()
                .map(|j| j.str().map(String::from).ok_or("Invalid witness".to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            if objects.is_empty() {
                return Err("Worker finding needs a witness".into());
            }
            Ok(Diagnostic {
                objects,
                message: get(r, "message")?.into(),
            })
        })
        .collect()
}
fn checks(
    libs: &[&Library],
    input: Input,
    facts: impl FnOnce() -> Vec<Fact>,
    mut visit: impl FnMut(RuleSpec, Result<Vec<Diagnostic>, String>),
) {
    let mut make = Some(facts);
    let mut cache = None;
    for lib in libs {
        for (r, i, hook) in lib
            .worker_checks
            .iter()
            .map(|r| (r.rule, r.input, Some(r.evaluate)))
            .chain(
                lib.external
                    .into_iter()
                    .flat_map(|leaf| leaf.rules.iter().map(|&(r, i)| (r, i, None))),
            )
        {
            if i == input {
                let facts = cache.get_or_insert_with(|| make.take().unwrap()());
                let required: Vec<_> = lib.requirements.iter().filter(|need| need.rule == r.id).collect();
                let eligible;
                let facts = if required.is_empty() {
                    facts.as_slice()
                } else {
                    eligible = facts
                        .iter()
                        .filter(|f| {
                            let mut ready = true;
                            for need in &required {
                                if let Some(object) = need.absent(f) {
                                    evidence::missing(r.id, object, need.field);
                                    ready = false;
                                }
                            }
                            ready
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    if eligible.is_empty() && !facts.is_empty() {
                        continue;
                    }
                    eligible.as_slice()
                };
                visit(
                    r,
                    match hook {
                        Some(f) => Ok(f(facts)),
                        None => run(lib.external.unwrap(), r.id, input, facts),
                    },
                );
            }
        }
    }
}
pub(super) fn erc(libs: &[&Library], doc: &sch::Document, c: &sch::Circuit, out: &mut Vec<sch::Issue>) {
    let mut emit = |r: RuleSpec, ds: Result<Vec<Diagnostic>, String>| match ds {
        Ok(ds) => {
            for d in ds {
                out.push(sch::Issue {
                    kind: r.id,
                    unknown: false,
                    objects: d.objects,
                    message: d.message,
                })
            }
        }
        Err(e) => out.push(sch::Issue {
            kind: "LibraryCheckFailed",
            unknown: true,
            objects: vec![],
            message: format!("{}: {e}", r.id),
        }),
    };
    checks(
        libs,
        Input::PinSpecifications,
        || {
            let records = |specs: &[sch::characteristics::Specification]| {
                Json::Arr(specs.iter().map(|s| s.json()).collect())
            };
            c.terminals
                .iter()
                .filter(|(_, p)| p.populated)
                .map(|(id, p)| {
                    let sym = &doc.symbols[&p.symbol];
                    let pin = &sym.pins[&p.pin];
                    let active = p
                        .function
                        .as_ref()
                        .and_then(|f| pin.functions.get(f))
                        .map_or(&[][..], |f| &f.specs);
                    Fact::PinSpecifications {
                        pin: id.clone(),
                        net: c.net_of[id].clone(),
                        symbol: p.symbol.clone(),
                        function: p.function.clone(),
                        device: records(&sym.specs),
                        base: records(&pin.specs),
                        active: records(active),
                    }
                })
                .collect()
        },
        &mut emit,
    );
    checks(libs, Input::Addresses, || c.addresses.clone(), &mut emit);
    checks(
        libs,
        Input::Components,
        || {
            let mut facts = vec![];
            for (scope, sheet) in &c.occurrences {
                for row in doc.rows.iter().filter(|r| &r.sheet == sheet) {
                    if let sch::Item::Place {
                        symbol,
                        fields,
                        populated,
                        ..
                    } = &row.item
                    {
                        let id = format!("{scope}/{}", row.id);
                        let sym = &doc.symbols[symbol];
                        if c.is_fitted(&id, *populated) && sym.power_net.is_none() {
                            let mut values = sym.fields.clone();
                            values.extend(fields.clone());
                            facts.push(Fact::Component {
                                id,
                                symbol: symbol.clone(),
                                typed: sym.typed_fields(doc, &values),
                                fields: values,
                            });
                        }
                    }
                }
            }
            facts
        },
        &mut emit,
    );
    checks(
        libs,
        Input::Supplies,
        || {
            let mut facts = vec![];
            for (id, pin) in &c.terminals {
                if pin.electrical != sch::Electrical::Supply || !pin.populated {
                    continue;
                }
                for provider in &c.nets[&c.net_of[id]] {
                    if let Some(t) = c.terminals.get(provider).filter(|p| p.populated) {
                        for role in &t.gives {
                            if let Some(range) = doc.roles[role].provided_voltage() {
                                facts.push(Fact::Supply {
                                    pin: id.clone(),
                                    provider: provider.clone(),
                                    minimum_v: range.min,
                                    maximum_v: range.max,
                                });
                            }
                        }
                    }
                }
            }
            facts
        },
        emit,
    );
}
pub(super) fn drc<'a>(libs: &[&Library], b: &'a pcb::Board, out: &mut pcb::Check<'a>) {
    checks(
        libs,
        Input::Tracks,
        || {
            b.tracks
                .iter()
                .map(|(id, t)| Fact::Track {
                    id: id.clone(),
                    net: b.nets[&t.net].name.clone(),
                    layer: t.layer.clone(),
                    width_nm: t.width_nm,
                })
                .collect()
        },
        |r, ds| {
            out.rules.push(pcb::Rule::Library(r.id));
            let checked = ds.and_then(|ds| {
                let mut findings = Vec::new();
                for d in ds {
                    let (id, t) = d
                        .objects
                        .first()
                        .and_then(|id| b.tracks.get_key_value(id))
                        .ok_or("Unknown track witness")?;
                    findings.push(pcb::Finding {
                        rule: pcb::Rule::Library(r.id),
                        object: id,
                        span: t.span,
                        witness: pcb::Witness::Library {
                            message: d.message,
                            objects: d.objects,
                        },
                    });
                }
                Ok(findings)
            });
            match checked {
                Ok(fs) => out.findings.extend(fs),
                Err(e) => out.findings.push(pcb::Finding {
                    rule: pcb::Rule::Library("LibraryCheckFailed"),
                    object: r.id,
                    span: Default::default(),
                    witness: pcb::Witness::Library {
                        message: e,
                        objects: vec![],
                    },
                }),
            }
        },
    );
}
