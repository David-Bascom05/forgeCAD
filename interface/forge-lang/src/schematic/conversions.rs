//! Library-owned reversible edges. Finite lookup equivalence classes are indexed once on load.
use super::*;
use crate::electronics::{fault, list, string};
use crate::syntax::{Expr, ExprKind};
use forge_agent::json::Json;
use std::{collections::VecDeque, sync::Arc};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConversionTable {
    pub name: String,
    pub from: String,
    pub to: String,
    pub rows: Vec<[String; 2]>,
}
impl ConversionTable {
    fn join(from: &str, to: &str) -> Self {
        Self {
            name: format!("{from} ↔ {to}"),
            from: from.into(),
            to: to.into(),
            rows: Vec::new(),
        }
    }
    pub fn json(&self) -> Json {
        Json::obj(vec![
            ("name", self.name.clone().into()),
            ("from", self.from.clone().into()),
            ("to", self.to.clone().into()),
            (
                "rows",
                Json::Arr(
                    self.rows
                        .iter()
                        .map(|row| Json::Arr(row.iter().cloned().map(Into::into).collect()))
                        .collect(),
                ),
            ),
        ])
    }
}
pub(super) fn read(e: &Expr) -> Result<Vec<ConversionTable>> {
    list(e)?
        .iter()
        .map(|row| {
            let ExprKind::Map(pairs) = &row.kind else {
                return Err(fault(
                    row,
                    "InvalidConversionTable",
                    "expected a table with from and to formats",
                ));
            };
            let mut fields = Map::new();
            for (k, v) in pairs {
                if !matches!(k.as_str(), "name" | "from" | "to" | "rows")
                    || fields.insert(k.as_str(), v).is_some()
                {
                    return Err(fault(v, "InvalidConversionTable", k));
                }
            }
            let req = |k| {
                fields
                    .get(k)
                    .copied()
                    .ok_or_else(|| fault(row, "InvalidConversionTable", format!("missing {k}")))
            };
            let mut table = ConversionTable::join(&string(req("from")?)?, &string(req("to")?)?);
            if let Some(name) = fields.get("name") {
                table.name = string(name)?;
            }
            if let Some(rows) = fields.get("rows") {
                for pair in list(rows)? {
                    let [a, b] = list(pair)? else {
                        return Err(fault(
                            pair,
                            "InvalidConversionPair",
                            "expected [source, destination]",
                        ));
                    };
                    table.rows.push([string(a)?, string(b)?]);
                }
            }
            Ok(table)
        })
        .collect()
}

type Atom = (String, String);
#[derive(Debug, Default, PartialEq)]
struct Index {
    tables: Vec<ConversionTable>,
    adj: Map<String, Set<String>>,
    classes: Vec<Map<String, String>>,
    class_of: Map<Atom, usize>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConversionGraph(Arc<Index>);
impl ConversionGraph {
    pub fn tables(&self) -> &[ConversionTable] {
        &self.0.tables
    }
    pub fn defaults(units: &[AttributeUnit]) -> Vec<ConversionTable> {
        units
            .first()
            .map(|base| {
                units
                    .iter()
                    .skip(1)
                    .map(|u| ConversionTable::join(&base.name, &u.name))
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn build(
        kind: StrapKind,
        units: &[AttributeUnit],
        tables: Vec<ConversionTable>,
    ) -> std::result::Result<Self, String> {
        let mut index = Index::default();
        let formats: Set<_> = units.iter().map(|u| u.name.as_str()).collect();
        let mut names = Set::new();
        let mut links: Map<Atom, Set<Atom>> = Map::new();
        for t in &tables {
            if t.name.trim().is_empty() || !names.insert(&t.name) {
                return Err("Give each conversion table a distinct name.".into());
            }
            if t.from == t.to || ![&t.from, &t.to].iter().all(|s| formats.contains(s.as_str())) {
                return Err(format!("{} needs two different declared formats.", t.name));
            }
            if (kind == StrapKind::Lookup) == t.rows.is_empty() {
                return Err(format!(
                    "{}: lookup tables need value pairs; numeric tables use their declared scales or encodings.",
                    t.name
                ));
            }
            index.adj.entry(t.from.clone()).or_default().insert(t.to.clone());
            index.adj.entry(t.to.clone()).or_default().insert(t.from.clone());
            for [a, b] in &t.rows {
                if a.is_empty() || b.is_empty() {
                    return Err(format!("{} has an empty value.", t.name));
                }
                let left = (t.from.clone(), a.clone());
                let right = (t.to.clone(), b.clone());
                links.entry(left.clone()).or_default().insert(right.clone());
                links.entry(right).or_default().insert(left);
            }
        }
        let first = units.first().ok_or("Declare the formats first.")?;
        let mut reached = Set::new();
        let mut pending = vec![first.name.as_str()];
        while let Some(u) = pending.pop() {
            if reached.insert(u) {
                pending.extend(index.adj.get(u).into_iter().flatten().map(String::as_str));
            }
        }
        if reached.len() != formats.len() {
            return Err(format!(
                "Connect every format to {} through a conversion table.",
                first.name
            ));
        }
        if kind == StrapKind::Lookup && links.is_empty() {
            return Err("Declare at least one value mapping.".into());
        }
        for start in links.keys() {
            if index.class_of.contains_key(start) {
                continue;
            }
            let mut values = Map::new();
            let mut pending = vec![start.clone()];
            while let Some(atom) = pending.pop() {
                if index.class_of.contains_key(&atom) {
                    continue;
                }
                if let Some(other) = values.insert(atom.0.clone(), atom.1.clone()) {
                    return Err(format!(
                        "Conflicting conversion paths give both {other} and {} in {}.",
                        atom.1, atom.0
                    ));
                }
                index.class_of.insert(atom.clone(), index.classes.len());
                pending.extend(links[&atom].iter().cloned());
            }
            if let Some(missing) = formats.iter().find(|name| !values.contains_key(**name)) {
                return Err(format!(
                    "{} in {} has no translation into {missing}.",
                    start.1, start.0
                ));
            }
            index.classes.push(values);
        }
        index.tables = tables;
        Ok(Self(Arc::new(index)))
    }
    fn lookup(&self, value: &str, from: &str, to: &str) -> Option<String> {
        let class = self.0.class_of.get(&(from.into(), value.into()))?;
        self.0.classes[*class].get(to).cloned()
    }
    pub fn value_count(&self) -> usize {
        self.0.classes.len()
    }
    pub fn typed(
        &self,
        kind: StrapKind,
        value: &str,
        units: &[AttributeUnit],
        from: Option<&str>,
    ) -> Option<Json> {
        if kind != StrapKind::Lookup {
            return kind.typed_with_units(value, units, from);
        }
        let base = &units.first()?.name;
        Some(Json::obj(vec![
            ("value", self.lookup(value, from.unwrap_or(base), base)?.into()),
            ("unit", base.clone().into()),
        ]))
    }
    pub fn convert(
        &self,
        kind: StrapKind,
        value: &str,
        units: &[AttributeUnit],
        from: Option<&str>,
        to: &str,
    ) -> Option<String> {
        let first = &units.first()?.name;
        if kind == StrapKind::Lookup {
            return self.lookup(value, from.unwrap_or(first), to);
        }
        let encode = |name: &str| kind.format_unit(value, units, from, name);
        let source = units
            .iter()
            .filter(|u| {
                u.radix.as_ref().map_or_else(
                    || value.ends_with(&u.name),
                    |r| {
                        !r.prefix.is_empty()
                            && value
                                .get(..r.prefix.len())
                                .is_some_and(|s| s.eq_ignore_ascii_case(&r.prefix))
                    },
                )
            })
            .max_by_key(|u| u.radix.as_ref().map_or(u.name.len(), |r| r.prefix.len()))
            .map(|u| u.name.as_str())
            .or(from)
            .unwrap_or(first);
        if source == to || self.0.tables.is_empty() {
            return encode(to);
        }
        let mut queue = VecDeque::from([source]);
        let mut seen = Set::from([source]);
        while let Some(u) = queue.pop_front() {
            for next in self.0.adj.get(u).into_iter().flatten() {
                if seen.insert(next.as_str())
                    && let Some(rendered) = encode(next)
                {
                    if next == to {
                        return Some(rendered);
                    }
                    queue.push_back(next);
                }
            }
        }
        None
    }
}
