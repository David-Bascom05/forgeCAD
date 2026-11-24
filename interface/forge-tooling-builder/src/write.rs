//! forge-tooling's TOML tables written canonically: `[[kind]]`, `id` first, the other keys in
//! order, then each nested array of tables (`[[kind.cite]]`, `[[kind.preset]]`) after them.
//! Strings are written so the reader takes them back unchanged (it knows `\" \\ \n \t`); any
//! other control character is refused.
use crate::Rec;
use forge_tooling::toml::{Table, Val};
use std::fmt::Write;

/// A number as the reader reads it back: Rust's shortest round-trip decimal; −0 as 0.
pub fn num(x: f64) -> String {
    if x == 0.0 { "0".into() } else { format!("{x}") }
}

pub fn string(s: &str) -> Result<String, String> {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if c.is_control() => {
                return Err(format!(
                    "{s:?}: the character U+{:04X} cannot be written",
                    c as u32
                ));
            }
            c => o.push(c),
        }
    }
    o.push('"');
    Ok(o)
}

fn value(v: &Val) -> Result<String, String> {
    Ok(match v {
        Val::Str(s) => string(s)?,
        Val::Num(x) => num(*x),
        Val::Bool(b) => b.to_string(),
        Val::Arr(a) => format!(
            "[{}]",
            a.iter().map(value).collect::<Result<Vec<_>, _>>()?.join(", ")
        ),
        Val::Table(_) | Val::Tables(_) => return Err("a table is not a value".into()),
    })
}

fn key(k: &str) -> String {
    if !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        k.to_string()
    } else {
        format!("\"{k}\"")
    }
}

fn table(out: &mut String, path: &str, t: &Table) -> Result<(), String> {
    writeln!(out, "[[{path}]]").unwrap();
    if let Some(v) = t.get("id") {
        writeln!(out, "id = {}", value(v)?).unwrap();
    }
    for (k, v) in t {
        if k == "id" || matches!(v, Val::Table(_) | Val::Tables(_)) {
            continue;
        }
        writeln!(out, "{} = {}", key(k), value(v)?).unwrap();
    }
    for (k, v) in t {
        match v {
            Val::Tables(ts) => {
                for sub in ts {
                    table(out, &format!("{path}.{k}"), sub)?;
                }
            }
            Val::Table(_) => {
                return Err(format!(
                    "`{path}.{k}`: a plain table inside a record is not written"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// A file of `kind` records: a header comment, then each record, a blank line between.
pub fn file(kind: &str, header: &str, recs: &[Rec]) -> Result<String, String> {
    let mut s = String::new();
    for l in header.lines() {
        s.push_str("# ");
        s.push_str(l);
        s.push('\n');
    }
    for r in recs {
        s.push('\n');
        table(&mut s, kind, &r.table)?;
    }
    Ok(s)
}
