use super::*;
#[path = "../standard_library/erc/connectivity.rs"]
pub(crate) mod standard;
use std::collections::VecDeque;
#[derive(Clone, Debug)]
pub struct Terminal {
    pub at: Point,
    pub symbol: String,
    pub pin: String,
    pub power_net: Option<String>,
    pub populated: bool,
    pub number: String,
    pub function: Option<String>,
    pub(super) instance_wants: Set<RoleId>,
    pub electrical: Electrical,
    pub wants: Set<RoleId>,
    pub gives: Set<RoleId>,
}
#[derive(Clone, Debug)]
pub struct Link {
    pub id: String,
    pub want: String,
    pub give: String,
    pub role: RoleId,
    pub returns: Option<(String, String)>,
    pub prefer: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct Issue {
    pub unknown: bool,
    pub kind: &'static str,
    pub objects: Vec<String>,
    pub message: String,
}
impl Issue {
    pub fn waiver<'a>(&self, d: &'a Document) -> Option<&'a str> {
        if d.waivers.is_empty() {
            return None;
        }
        let mut objects = self.objects.clone();
        objects.sort();
        objects.dedup();
        d.waivers
            .get(&(self.kind.to_owned(), objects))
            .map(String::as_str)
    }
    pub fn priority<'a>(&self, d: &'a Document) -> &'a str {
        d.severities.get(self.kind).map_or_else(
            || {
                d.packages
                    .erc
                    .get(self.kind)
                    .map_or(self.severity(), |s| s.word())
            },
            String::as_str,
        )
    }
    pub fn status(&self) -> &'static str {
        if self.unknown { "unknown" } else { "failed" }
    }
    pub fn severity(&self) -> &'static str {
        Self::default_severity(self.kind)
    }
    pub fn default_severity(kind: &str) -> &'static str {
        crate::standard_library::default_erc(kind)
    }
}
#[derive(Debug)]
pub struct VoltageProvider {
    pub pin: String,
    pub role: RoleId,
    pub range: VoltageRange,
}
#[derive(Debug)]
pub struct VoltageCheck {
    pub required: VoltageRange,
    pub providers: Vec<VoltageProvider>,
}
impl VoltageCheck {
    pub fn status(&self) -> &'static str {
        if self.providers.is_empty() {
            "unknown"
        } else if self
            .providers
            .iter()
            .all(|source| self.required.accepts(source.range))
        {
            "satisfied"
        } else {
            "out_of_range"
        }
    }
}
#[derive(Default, Debug)]
pub struct Circuit {
    pub(crate) addresses: Vec<crate::electronics_library::worker::Fact>,
    pub(super) fitted: Map<String, bool>,
    pub straps: Map<String, Map<String, StrapResult>>,
    pub(super) signals: super::signal::Signals,
    pub connections: Map<String, Map<RoleId, ConnectionCheck>>,
    pub voltages: Map<String, Map<RoleId, VoltageCheck>>,
    pub origins: Map<String, Span>,
    attached: Set<String>,
    pub terminals: Map<String, Terminal>,
    pub locations: Map<String, Point>,
    pub occurrences: Map<String, String>,
    pub adjacency: Map<String, Set<String>>,
    pub nets: Map<String, Vec<String>>,
    pub net_of: Map<String, String>,
    pub links: Vec<Link>,
    pub issues: Vec<Issue>,
    pub not_run: Vec<crate::electronics_library::evidence::NotRun>,
    // Incidence by both directions is for navigation only. Link direction is retained in Link.
    pub relations: Map<String, Vec<usize>>,
    pub diff_pairs: Map<String, [String; 2]>,
    pub buses: Map<String, Map<String, String>>,
    pub flows: Map<String, (String, String)>,
    junctions: Set<String>,
    ports: Set<String>,
    nc: Set<String>,
    labels: Map<String, String>,
    authored_labels: Map<String, String>,
    pub wires: Map<String, (String, String, Vec<Point>)>,
}
pub(super) fn transform(local: Point, at: Point, angle: f64, mirror: bool) -> Point {
    let [mut x, y] = local;
    if mirror {
        x = -x;
    }
    let [dx, dy] = match angle as i32 {
        90 => [-y, x],
        180 => [-x, -y],
        270 => [y, -x],
        _ => [x, y],
    };
    [at[0] + dx, at[1] + dy]
}
impl Circuit {
    pub fn is_fitted(&self, owner: &str, default: bool) -> bool {
        self.fitted.get(owner).copied().unwrap_or(default)
    }
    pub fn build(doc: &Document) -> Result<Self> {
        Self::build_fitted(doc, &Map::new())
    }
    pub fn build_fitted(doc: &Document, overrides: &Map<String, bool>) -> Result<Self> {
        let mut c = Self {
            fitted: overrides.clone(),
            ..Self::default()
        };
        if doc.library {
            c.issues = super::drawing::library_readability(doc);
            c.sort_findings();
            return Ok(c);
        }
        let mut by_sheet: Map<&str, Vec<&Row>> = Map::new();
        let mut children: Set<&str> = Set::new();
        for row in &doc.rows {
            if !doc.sheets.contains_key(&row.sheet) {
                return Err(Fault::new("UnknownSheet", row.span, &row.sheet));
            }
            if let Item::SheetInstance { of, .. } = &row.item {
                if !doc.sheets.contains_key(of) {
                    return Err(Fault::new("UnknownSheet", row.span, of));
                }
                children.insert(of);
            }
            by_sheet.entry(&row.sheet).or_default().push(row);
        }
        let mut seen = Set::new();
        for root in doc.sheets.keys().filter(|k| !children.contains(k.as_str())) {
            c.expand(doc, &by_sheet, root, root, &mut Set::new(), &mut seen)?;
        }
        if let Some(k) = doc.sheets.keys().find(|k| !seen.contains(*k)) {
            return Err(Fault::new("SheetCycle", Span::default(), k));
        }
        for (pin, t) in &mut c.terminals {
            if let Some(fit) = pin.rsplit_once('.').and_then(|(owner, _)| overrides.get(owner)) {
                t.populated = *fit;
            }
        }
        c.partition();
        let (_, coverage) = crate::electronics_library::evidence::capture(|| {
            doc.packages.resolve(doc, &mut c, &by_sheet, overrides)
        });
        c.not_run = coverage
            .into_iter()
            .filter(|r| doc.packages.erc.contains_key(r.rule.as_str()))
            .collect();
        let mut placed_paths = Set::new();
        for (scope, sheet) in &c.occurrences {
            let mut refs = Set::new();
            for row in by_sheet.get(sheet.as_str()).into_iter().flatten() {
                if let Item::Place {
                    symbol, reference, ..
                } = &row.item
                {
                    let path = format!("{scope}/{}", row.id);
                    let display = doc.reference(&path, reference.as_ref().unwrap_or(&row.id));
                    if doc.symbols[symbol].power_net.is_none() {
                        placed_paths.insert(path);
                    }
                    if !refs.insert(display) {
                        return Err(Fault::new(
                            "DuplicateReference",
                            row.span,
                            format!("{scope}/{display}"),
                        ));
                    }
                }
            }
        }
        for path in doc.annotations.keys() {
            if !placed_paths.contains(path) {
                return Err(Fault::new("UnknownAnnotationPart", Span::default(), path));
            }
        }
        for path in doc.variants.values().flat_map(|changes| changes.keys()) {
            if !placed_paths.contains(path) {
                return Err(Fault::new("UnknownVariantPart", Span::default(), path));
            }
        }
        c.sort_findings();
        Ok(c)
    }
    fn expand(
        &mut self,
        doc: &Document,
        by_sheet: &Map<&str, Vec<&Row>>,
        sheet: &str,
        scope: &str,
        active: &mut Set<String>,
        seen: &mut Set<String>,
    ) -> Result<()> {
        if !active.insert(sheet.into()) {
            return Err(Fault::new("SheetCycle", Span::default(), sheet));
        }
        if active.len() > 64 || self.occurrences.len() >= 32_768 || self.adjacency.len() > 500_000 {
            return Err(Fault::new(
                "HierarchyLimit",
                Span::default(),
                "schematic expansion limit reached",
            ));
        }
        seen.insert(sheet.to_string());
        self.occurrences.insert(scope.into(), sheet.into());
        let rows = by_sheet.get(sheet).map(Vec::as_slice).unwrap_or_default();

        let placed: Map<_, _> = rows
            .iter()
            .filter_map(|r| {
                if let Item::Place {
                    symbol,
                    populated,
                    pins,
                    ..
                } = &r.item
                {
                    let wants = &pins.wants;
                    let gives = &pins.gives;
                    let functions = &pins.functions;
                    Some((r.id.as_str(), (symbol, *populated, wants, gives, functions)))
                } else {
                    None
                }
            })
            .collect();
        let mut units = Set::new();
        for &row in rows {
            let id = format!("{scope}/{}", row.id);
            self.origins.insert(id.clone(), row.span);
            let gate = match &row.item {
                Item::Place {
                    unit,
                    at,
                    angle,
                    mirror,
                    ..
                } => Some((&row.id, unit, *at, *angle, *mirror)),
                Item::Gate {
                    of,
                    unit,
                    at,
                    angle,
                    mirror,
                } => Some((of, unit, *at, *angle, *mirror)),
                _ => None,
            };
            if let Some((owner, unit, at, angle, mirror)) = gate {
                let &(symbol, populated, wants, gives, functions) = placed
                    .get(owner.as_str())
                    .ok_or_else(|| Fault::new("UnknownOccurrence", row.span, owner))?;
                let def = &doc.symbols[symbol];
                if !units.insert((owner.as_str(), unit.as_str())) {
                    return Err(Fault::new("DuplicateGate", row.span, format!("{owner}.{unit}")));
                }
                if !def.pins.is_empty() && !def.pins.values().any(|p| p.unit == *unit) {
                    return Err(Fault::new("UnknownUnit", row.span, unit));
                }
                for (name, pin) in def.pins.iter().filter(|(_, p)| p.unit == *unit) {
                    let key = format!("{scope}/{owner}.{name}");
                    self.origins.insert(key.clone(), row.span);
                    let pos = transform(pin.at, at, angle, mirror);

                    self.node(key.clone(), pos);
                    if let Some(net) = &def.power_net {
                        let rail = format!("@global:{net}");
                        self.labels.insert(rail.clone(), net.clone());
                        self.origins.entry(rail.clone()).or_insert(row.span);
                        self.edge(&key, &rail);
                    }
                    let mode = functions.get(name).map(|f| &pin.functions[f]);
                    self.terminals.insert(
                        key,
                        Terminal {
                            populated,
                            at: pos,
                            symbol: (*symbol).clone(),
                            pin: name.clone(),
                            power_net: def.power_net.clone(),
                            number: pin.number.clone(),
                            function: functions.get(name).cloned(),
                            instance_wants: wants.get(name).cloned().unwrap_or_default(),
                            electrical: mode.map_or(pin.electrical, |f| f.electrical),
                            wants: mode
                                .map_or(&pin.wants, |f| &f.wants)
                                .iter()
                                .chain(wants.get(name).into_iter().flatten())
                                .cloned()
                                .collect(),
                            gives: mode
                                .map_or(&pin.gives, |f| &f.gives)
                                .iter()
                                .chain(gives.get(name).into_iter().flatten())
                                .cloned()
                                .collect(),
                        },
                    );
                }
            }
            match &row.item {
                Item::Junction(p) => {
                    self.junctions.insert(id.clone());
                    self.node(id, *p);
                }
                Item::Port(p) => {
                    self.ports.insert(id.clone());
                    self.node(id, *p);
                }
                Item::SheetInstance { of, at, anchors } => {
                    self.expand(doc, by_sheet, of, &id, active, seen)?;
                    let ports: Vec<_> = by_sheet
                        .get(of.as_str())
                        .into_iter()
                        .flatten()
                        .filter(|r| matches!(r.item, Item::Port(_)))
                        .map(|r| &r.id)
                        .collect();
                    if let Some(k) = anchors.keys().find(|k| !ports.contains(k)) {
                        return Err(Fault::new("UnknownPort", row.span, k));
                    }
                    for (i, name) in ports.into_iter().enumerate() {
                        let p = anchors.get(name).copied().unwrap_or([0., -(i as f64) * 2.54]);
                        let outside = format!("{id}.{name}");
                        let port = format!("{id}/{name}");
                        self.node(outside.clone(), [at[0] + p[0], at[1] + p[1]]);
                        self.edge(&outside, &port);
                    }
                }
                _ => {}
            }
        }
        for (owner, (symbol, ..)) in placed {
            let missing: Set<_> = doc.symbols[symbol]
                .pins
                .values()
                .map(|p| &p.unit)
                .filter(|unit| !units.contains(&(owner, unit.as_str())))
                .collect();
            standard::missing_units(self, scope, owner, missing);
        }
        for &row in rows {
            let id = format!("{scope}/{}", row.id);
            let endpoint = |c: &Circuit, name: &str| {
                let key = format!("{scope}/{name}");
                if c.locations.contains_key(&key) {
                    return Ok(key);
                }
                if let Some((sub, port)) = name.split_once('.') {
                    let key = format!("{scope}/{sub}/{port}");
                    if c.ports.contains(&key) {
                        return Ok(key);
                    }
                }
                Err(Fault::new(
                    "UnknownEndpoint",
                    row.span,
                    format!("{name} on sheet {sheet}"),
                ))
            };
            match &row.item {
                Item::Flow { from, to } => {
                    let (a, b) = (endpoint(self, from)?, endpoint(self, to)?);
                    self.flows.insert(id, (a, b));
                }
                Item::DiffPair { positive, negative } => {
                    let ends = [endpoint(self, positive)?, endpoint(self, negative)?];
                    self.diff_pairs.insert(id, ends);
                }
                Item::Bus(members) => {
                    let mut bus = Map::new();
                    for (k, v) in members {
                        bus.insert(k.clone(), endpoint(self, v)?);
                    }
                    self.buses.insert(id, bus);
                }
                Item::Wire { from, to, via } => {
                    let (a, b) = (endpoint(self, from)?, endpoint(self, to)?);
                    if a == b {
                        return Err(Fault::new("SelfWire", row.span, from));
                    }
                    self.attached.extend([a.clone(), b.clone()]);
                    self.edge(&a, &b);
                    self.wires.insert(id, (a, b, via.clone()));
                }
                Item::Label {
                    target, text, global, ..
                } => {
                    let a = endpoint(self, target)?;
                    let key = if *global {
                        format!("@global:{text}")
                    } else {
                        format!("@local:{scope}:{text}")
                    };
                    self.origins.entry(key.clone()).or_insert(row.span);
                    self.authored_labels.entry(key.clone()).or_insert(id);
                    self.labels.insert(key.clone(), text.clone());
                    self.attached.insert(a.clone());
                    self.edge(&a, &key);
                }
                Item::NoConnect(target) => {
                    let a = endpoint(self, target)?;
                    if !self.terminals.contains_key(&a) {
                        return Err(Fault::new("ExpectedPin", row.span, target));
                    }
                    self.nc.insert(a);
                }
                Item::Fulfill {
                    want,
                    give,
                    role,
                    returns,
                    prefer,
                } => {
                    let (w, g) = (endpoint(self, want)?, endpoint(self, give)?);
                    if w == g
                        || !self.terminals.get(&w).is_some_and(|p| p.wants.contains(role))
                        || !self.terminals.get(&g).is_some_and(|p| p.gives.contains(role))
                    {
                        return Err(Fault::new(
                            "RoleMismatch",
                            row.span,
                            format!("{w} must want {} and a distinct {g} must give it", role.0),
                        ));
                    }
                    let returns = if let Some((a, b)) = returns {
                        Some((endpoint(self, a)?, endpoint(self, b)?))
                    } else {
                        None
                    };
                    self.links.push(Link {
                        id,
                        want: w,
                        give: g,
                        role: role.clone(),
                        returns,
                        prefer: *prefer,
                    });
                }
                _ => {}
            }
        }

        active.remove(sheet);
        Ok(())
    }
    pub(super) fn strap_is_open(&self, net: &str, terminal: &str) -> bool {
        self.nets[net]
            .iter()
            .all(|id| id == terminal || self.inactive_strap_member(id))
    }
    pub(super) fn inactive_strap_member(&self, id: &str) -> bool {
        self.junctions.contains(id) || self.terminals.get(id).is_some_and(|p| !p.populated)
    }
    pub fn is_attached(&self, endpoint: &str) -> bool {
        self.attached.contains(endpoint) || self.nc.contains(endpoint)
    }
    fn node(&mut self, id: String, at: Point) {
        self.locations.insert(id.clone(), at);
        self.adjacency.entry(id).or_default();
    }
    fn edge(&mut self, a: &str, b: &str) {
        self.adjacency.entry(a.into()).or_default().insert(b.into());
        self.adjacency.entry(b.into()).or_default().insert(a.into());
    }
    pub(super) fn erc_ok(&self, doc: &Document) -> bool {
        !self
            .issues
            .iter()
            .any(|i| i.priority(doc) == "error" && i.waiver(doc).is_none())
    }
    pub(super) fn sort_findings(&mut self) {
        self.issues
            .sort_by(|a, b| (a.kind, &a.objects).cmp(&(b.kind, &b.objects)));
    }
    fn partition(&mut self) {
        for start in self.adjacency.keys() {
            if self.net_of.contains_key(start) {
                continue;
            }
            let mut todo = vec![start.clone()];
            self.net_of.insert(start.clone(), start.clone());
            let mut members = Vec::new();
            while let Some(node) = todo.pop() {
                for n in &self.adjacency[&node] {
                    if !self.net_of.contains_key(n) {
                        self.net_of.insert(n.clone(), start.clone());
                        todo.push(n.clone());
                    }
                }
                members.push(node);
            }
            members.sort();
            self.nets.insert(start.clone(), members);
        }
    }
    /// Count drawn segments from a pin through explicit junctions. Stop at other pins,
    /// ports and label joins so a remote provider on a named rail has weaker evidence.
    pub(super) fn drawn_branch<'a>(&'a self, root: &'a str) -> Map<&'a str, usize> {
        let mut distance = Map::from([(root, 0)]);
        let mut todo = VecDeque::from([root]);
        while let Some(a) = todo.pop_front() {
            let next = distance[a] + 1;
            for b in &self.adjacency[a] {
                if self.terminals.contains_key(b) || self.junctions.contains(b) {
                    if distance.contains_key(b.as_str()) {
                        continue;
                    }
                    distance.insert(b.as_str(), next);
                    if self.junctions.contains(b) {
                        todo.push_back(b);
                    }
                }
            }
        }
        distance
    }

    /// Outgoing functional stages and wants are directional. Wires and label joins are
    /// reversible. Sort peers by stable IDs, and emit feedback/shared nodes as references.
    pub fn rooted<'a>(&'a self, root: &'a str) -> Result<Vec<Step>> {
        if !self.terminals.contains_key(root) {
            return Err(Fault::new("UnknownPin", Span::default(), root));
        }
        let mut stages: Map<&str, Vec<(&str, &str)>> = Map::new();
        for (id, (a, b)) in &self.flows {
            stages.entry(a).or_default().push((b, id));
        }
        let mut seen = Set::from([root]);
        let mut wires = Set::new();
        let mut todo = VecDeque::from([root]);
        let mut out = Vec::new();
        while let Some(a) = todo.pop_front() {
            let mut emit = |b: &'a str, kind: &'static str, id: Option<&str>| {
                let first = seen.insert(b);
                out.push(Step {
                    from: a.into(),
                    to: b.into(),
                    kind,
                    id: id.map(str::to_string),
                    revisit: !first,
                });
                if first {
                    todo.push_back(b);
                }
            };
            for (b, id) in stages.get(a).into_iter().flatten() {
                emit(b, "flow", Some(id));
            }
            for &i in self.relations.get(a).into_iter().flatten() {
                let link = &self.links[i];
                if link.want == a {
                    emit(&link.give, "fulfillment", Some(&link.id));
                }
            }
            for b in &self.adjacency[a] {
                let b = b.as_str();
                if wires.insert((a.min(b), a.max(b))) {
                    emit(b, "connection", None);
                }
            }
        }
        Ok(out)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Step {
    pub from: String,
    pub to: String,
    pub kind: &'static str,
    pub id: Option<String>,
    pub revisit: bool,
}
