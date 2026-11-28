//! One purchasing line per distinct part specification, with all its fitted occurrences.
use super::*;
use forge_agent::json::Json;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartSpec {
    pub symbol: String,
    pub value: String,
    pub footprint: Option<String>,
    pub datasheet: Option<String>,
    pub fields: Map<String, String>,
}
#[derive(Debug)]
pub struct BomLine {
    pub typed_fields: Json,
    pub part: PartSpec,
    pub references: Vec<String>,
}
pub(super) fn field_json(fields: &Map<String, String>) -> Json {
    Json::Obj(
        fields
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().into()))
            .collect(),
    )
}
impl BomLine {
    pub fn json(&self) -> Json {
        let p = &self.part;
        Json::obj(vec![
            ("symbol", p.symbol.clone().into()),
            ("value", p.value.clone().into()),
            ("footprint", p.footprint.clone().map_or(Json::Null, Into::into)),
            ("datasheet", p.datasheet.clone().map_or(Json::Null, Into::into)),
            ("fields", field_json(&p.fields)),
            ("typed_fields", self.typed_fields.clone()),
            ("quantity", self.references.len().into()),
            (
                "references",
                Json::Arr(self.references.iter().cloned().map(Into::into).collect()),
            ),
        ])
    }
}
pub fn bom(doc: &Document, c: &Circuit) -> Vec<BomLine> {
    bom_for(doc, c, &Map::new())
}
pub fn bom_for(doc: &Document, c: &Circuit, overrides: &Map<String, bool>) -> Vec<BomLine> {
    let mut lines: Map<PartSpec, Vec<String>> = Map::new();
    for (path, sheet) in &c.occurrences {
        for row in doc.rows.iter().filter(|r| r.sheet == *sheet) {
            if let Item::Place {
                symbol,
                value,
                footprint,
                fields,
                datasheet,
                reference,
                populated,
                in_bom: true,
                ..
            } = &row.item
            {
                let lib = &doc.symbols[symbol];
                if lib.power_net.is_some()
                    || !overrides.get(&format!("{path}/{}", row.id)).unwrap_or(populated)
                {
                    continue;
                }
                let mut merged = lib.fields.clone();
                merged.extend(fields.clone());
                let part = PartSpec {
                    symbol: symbol.clone(),
                    value: value.clone(),
                    footprint: footprint.clone(),
                    fields: merged,
                    datasheet: datasheet.clone().or_else(|| lib.datasheet.clone()),
                };
                lines.entry(part).or_default().push(format!(
                    "{path}/{}",
                    doc.reference(
                        &format!("{path}/{}", row.id),
                        reference.as_ref().unwrap_or(&row.id)
                    )
                ));
            }
        }
    }
    lines
        .into_iter()
        .map(|(part, references)| BomLine {
            typed_fields: doc.symbols[&part.symbol].typed_fields(doc, &part.fields),
            part,
            references,
        })
        .collect()
}
pub fn bom_csv(lines: &[BomLine]) -> String {
    let keys: Set<_> = lines.iter().flat_map(|l| l.part.fields.keys()).collect();
    let csv = |v: Vec<String>| {
        v.into_iter()
            .map(|x| format!("\"{}\"", x.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut rows = vec![csv([
        "Quantity",
        "References",
        "Symbol",
        "Value",
        "Footprint",
        "Datasheet",
    ]
    .map(str::to_owned)
    .into_iter()
    .chain(keys.iter().map(|s| (*s).clone()))
    .collect())];
    for l in lines {
        let p = &l.part;
        let mut row = vec![
            l.references.len().to_string(),
            l.references.join(" "),
            p.symbol.clone(),
            p.value.clone(),
            p.footprint.clone().unwrap_or_default(),
            p.datasheet.clone().unwrap_or_default(),
        ];
        row.extend(keys.iter().map(|k| p.fields.get(*k).cloned().unwrap_or_default()));
        rows.push(csv(row));
    }
    rows.join("\r\n") + "\r\n"
}
