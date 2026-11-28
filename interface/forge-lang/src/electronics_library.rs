//! Rust electronics library SDK. A package may export any combination of properties,
//! ERC passes, DRC passes and child libraries. Rule IDs survive import aliases.
use crate::{pcb, schematic as sch};
use forge_agent::json::Json;
pub mod evidence;
mod installed;
pub mod worker;
pub use installed::install;
pub(crate) use installed::resolve as resolve_import;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}
impl Severity {
    pub fn word(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct RuleSpec {
    pub id: &'static str,
    pub severity: Severity,
}
pub type Sheets<'a> = BTreeMap<&'a str, Vec<&'a sch::Row>>;
pub type Erc = fn(&sch::Document, &sch::Circuit, &mut Vec<sch::Issue>);
pub type Drc = for<'a> fn(&'a pcb::Board, &mut pcb::Check<'a>);
type Resolve = fn(&sch::Document, &mut sch::Circuit, &Sheets<'_>, &BTreeMap<String, bool>);

#[derive(Debug)]
pub struct Library {
    pub name: &'static str,
    pub children: &'static [&'static Library],
    pub properties: Option<fn() -> BTreeMap<String, sch::AttributeType>>,
    pub erc_rules: &'static [RuleSpec],
    pub drc_rules: &'static [RuleSpec],
    pub erc: Option<Erc>,
    pub drc: Option<Drc>,
    pub worker_checks: &'static [worker::Check],
    pub requirements: &'static [worker::RequiredField],
    pub(crate) external: Option<&'static installed::Leaf>,
    pub(crate) resolve: Option<Resolve>,
}
impl Library {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            children: &[],
            properties: None,
            erc_rules: &[],
            drc_rules: &[],
            erc: None,
            drc: None,
            worker_checks: &[],
            requirements: &[],
            external: None,
            resolve: None,
        }
    }
    pub const fn with_erc(mut self, rules: &'static [RuleSpec], run: Erc) -> Self {
        self.erc_rules = rules;
        self.erc = Some(run);
        self
    }
    pub const fn with_drc(mut self, rules: &'static [RuleSpec], run: Drc) -> Self {
        self.drc_rules = rules;
        self.drc = Some(run);
        self
    }
    pub const fn with_requirements(mut self, requirements: &'static [worker::RequiredField]) -> Self {
        self.requirements = requirements;
        self
    }
    pub const fn with_checks(mut self, checks: &'static [worker::Check]) -> Self {
        self.worker_checks = checks;
        self
    }
    pub const fn with_children(mut self, children: &'static [&'static Library]) -> Self {
        self.children = children;
        self
    }
    pub const fn with_properties(mut self, properties: fn() -> BTreeMap<String, sch::AttributeType>) -> Self {
        self.properties = Some(properties);
        self
    }
    pub fn exported_types(&self) -> BTreeMap<String, sch::AttributeType> {
        let mut defs = self
            .external
            .map(|leaf| leaf.properties.clone())
            .or_else(|| self.properties.map(|get| get()))
            .unwrap_or_default();
        for child in self.children {
            for (key, t) in child.exported_types() {
                defs.insert(format!("{}.{key}", child.name), t);
            }
        }
        defs
    }
}
fn installed() -> &'static RwLock<BTreeMap<&'static str, &'static Library>> {
    static INSTALLED: OnceLock<RwLock<BTreeMap<&'static str, &'static Library>>> = OnceLock::new();
    INSTALLED.get_or_init(|| RwLock::new(BTreeMap::from([("standard", &crate::standard_library::STANDARD)])))
}
/// Register a statically linked Rust package once at application startup.
pub fn register(lib: &'static Library) -> Result<(), String> {
    let mut checked = Loaded::default();
    checked.add(lib)?;
    let mut installed = installed().write().unwrap();
    if let Some(other) = installed.get(lib.name) {
        if std::ptr::eq(*other, lib) {
            return Ok(());
        }
        return Err(format!("Library {} already registered", lib.name));
    }
    installed.insert(lib.name, lib);
    Ok(())
}
pub fn lookup(uri: &str) -> Option<&'static Library> {
    let path = uri.strip_prefix("rust:").or_else(|| uri.strip_prefix("forge:"))?;
    let mut segments = path.split('/');
    let mut lib = *installed().read().unwrap().get(segments.next()?)?;
    for name in segments {
        lib = lib.children.iter().copied().find(|l| l.name == name)?;
    }
    Some(lib)
}
#[derive(Default, Clone, Debug)]
pub struct Loaded {
    libs: Vec<&'static Library>,
    pub(crate) erc: BTreeMap<&'static str, Severity>,
    pub(crate) drc: BTreeMap<&'static str, Severity>,
}
impl Loaded {
    pub fn standard() -> Self {
        static STD: OnceLock<Loaded> = OnceLock::new();
        STD.get_or_init(|| {
            let mut s = Self::default();
            s.add(&crate::standard_library::STANDARD)
                .expect("standard library");
            s
        })
        .clone()
    }
    fn visit(&mut self, lib: &'static Library, active: &mut Vec<&'static Library>) -> Result<(), String> {
        if active.iter().any(|x| std::ptr::eq(*x, lib)) {
            return Err(format!("Library cycle at {}", lib.name));
        }
        if self.libs.iter().any(|x| std::ptr::eq(*x, lib)) {
            return Ok(());
        }
        if lib.name.is_empty()
            || !lib
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(format!("Invalid library name {}", lib.name));
        }
        let mut children = BTreeSet::new();
        active.push(lib);
        for child in lib.children {
            if !children.insert(child.name) {
                return Err(format!("Duplicate child {} in {}", child.name, lib.name));
            }
            self.visit(child, active)?;
        }
        active.pop();
        for (rules, target) in [(lib.erc_rules, &mut self.erc), (lib.drc_rules, &mut self.drc)] {
            for rule in rules {
                if rule.id.trim().is_empty()
                    || rule.id == "LibraryCheckFailed"
                    || target.insert(rule.id, rule.severity).is_some()
                {
                    return Err(format!(
                        "Duplicate or empty rule ID '{}' in {}",
                        rule.id, lib.name
                    ));
                }
            }
        }
        for (rule, input) in lib.worker_checks.iter().map(|r| (r.rule, r.input)).chain(
            lib.external
                .into_iter()
                .flat_map(|leaf| leaf.rules.iter().copied()),
        ) {
            let target = if input == worker::Input::Tracks {
                &mut self.drc
            } else {
                &mut self.erc
            };
            if rule.id.trim().is_empty()
                || rule.id == "LibraryCheckFailed"
                || target.insert(rule.id, rule.severity).is_some()
            {
                return Err(format!("Duplicate rule {}", rule.id));
            }
        }
        for need in lib.requirements {
            let supported = lib
                .worker_checks
                .iter()
                .map(|c| (c.rule, c.input))
                .chain(lib.external.into_iter().flat_map(|l| l.rules.iter().copied()))
                .any(|(r, input)| {
                    r.id == need.rule
                        && matches!(
                            input,
                            worker::Input::Components | worker::Input::PinSpecifications
                        )
                });
            if !supported || need.field.trim().is_empty() {
                return Err(format!("Unsupported field dependency on {}", need.rule));
            }
        }
        self.libs.push(lib);
        Ok(())
    }
    pub fn add(&mut self, lib: &'static Library) -> Result<(), String> {
        let mut next = self.clone();
        next.visit(lib, &mut Vec::new())?;
        *self = next;
        Ok(())
    }
    pub(crate) fn merge(&mut self, other: &Self) -> Result<(), String> {
        let mut next = self.clone();
        for lib in &other.libs {
            next.visit(lib, &mut Vec::new())?;
        }
        *self = next;
        Ok(())
    }
    pub fn erc_rules(&self) -> impl Iterator<Item = (&'static str, Severity)> + '_ {
        self.erc.iter().map(|(&k, &v)| (k, v))
    }
    pub fn drc_rules(&self) -> impl Iterator<Item = (&'static str, Severity)> + '_ {
        self.drc.iter().map(|(&k, &v)| (k, v))
    }
    pub(crate) fn resolve(
        &self,
        doc: &sch::Document,
        c: &mut sch::Circuit,
        sheets: &Sheets<'_>,
        fit: &BTreeMap<String, bool>,
    ) {
        for lib in &self.libs {
            if let Some(run) = lib.resolve {
                run(doc, c, sheets, fit);
            }
        }
        let mut out = Vec::new();
        for lib in &self.libs {
            if let Some(run) = lib.erc {
                run(doc, c, &mut out);
            }
        }
        installed::erc(&self.libs, doc, c, &mut out);
        c.issues.extend(out);
    }
    pub(crate) fn board<'a>(&self, b: &'a pcb::Board, report: &mut pcb::Check<'a>) {
        for lib in &self.libs {
            if let Some(run) = lib.drc {
                run(b, report);
            }
        }
        installed::drc(&self.libs, b, report);
    }
    pub(crate) fn custom_erc(&self) -> bool {
        self.libs.iter().any(|l| {
            !crate::standard_library::ERC
                .children
                .iter()
                .any(|c| std::ptr::eq(*c, *l))
                && l.erc.is_some()
        })
    }
    pub fn json(&self) -> Json {
        Json::Arr(
            self.libs
                .iter()
                .map(|lib| {
                    let mut erc: Vec<_> = lib.erc_rules.iter().map(|r| r.id.into()).collect();
                    let mut drc: Vec<_> = lib.drc_rules.iter().map(|r| r.id.into()).collect();
                    for (rule, input) in lib
                        .worker_checks
                        .iter()
                        .map(|c| (c.rule, c.input))
                        .chain(lib.external.into_iter().flat_map(|l| l.rules.iter().copied()))
                    {
                        if input == worker::Input::Tracks {
                            drc.push(rule.id.into());
                        } else {
                            erc.push(rule.id.into());
                        }
                    }
                    let properties = lib
                        .external
                        .map(|l| l.properties.keys().cloned().collect::<Vec<_>>())
                        .or_else(|| lib.properties.map(|f| f().into_keys().collect()))
                        .unwrap_or_default();
                    Json::obj(vec![
                        ("name", lib.name.into()),
                        (
                            "children",
                            Json::Arr(lib.children.iter().map(|c| c.name.into()).collect()),
                        ),
                        ("erc", Json::Arr(erc)),
                        ("drc", Json::Arr(drc)),
                        (
                            "properties",
                            Json::Arr(properties.into_iter().map(Into::into).collect()),
                        ),
                    ])
                })
                .collect(),
        )
    }
}
