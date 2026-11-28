//! Reuse electrical results when the only source change is a complete placement coordinate.
//! Dependencies have already been checked by Cache. A failed guard leaves the revision untouched.
use super::*;

/// Expressions are bracketed by their original parentheses, with identical bytes on both sides.
/// This keeps the surrounding tokens unchanged. All other edits go through the complete reader.
pub(super) fn update(last: &mut Compiled, src: &str) -> bool {
    if last.doc.packages.custom_erc() {
        return false;
    }
    let start = last
        .src
        .bytes()
        .zip(src.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let rows = &last.doc.rows;
    let idx = rows.partition_point(|row| row.span.hi as usize <= start);
    let Some(span) = rows.get(idx).and_then(|row| row.pose_span) else {
        return false;
    };
    let lo = span.lo as usize;
    let old_end = span.hi as usize;
    if start <= lo || start >= old_end || src.len() > u32::MAX as usize {
        return false;
    }
    let delta = src.len() as i64 - last.src.len() as i64;
    let Ok(end) = usize::try_from(old_end as i64 + delta) else {
        return false;
    };
    let Some(replacement) = src.get(lo..end) else {
        return false;
    };
    if src.get(end..) != last.src.get(old_end..) {
        return false;
    }
    let Some(pos) = reader::isolated_point(replacement) else {
        return false;
    };
    let row = &mut last.doc.rows[idx];
    let Item::Place {
        symbol,
        unit,
        at,
        angle,
        mirror,
        ..
    } = &mut row.item
    else {
        return false;
    };
    *at = pos;
    let def = &last.doc.symbols[symbol.as_str()];
    let c = &mut last.circuit;
    let scopes: Set<String> = c
        .occurrences
        .iter()
        .filter(|(_, sheet)| **sheet == row.sheet)
        .map(|(k, _)| k.clone())
        .collect();
    let mut changed = Set::new();
    c.issues.retain(|i| {
        !matches!(i.kind, "TextBoundsOverlap" | "TextGeometryOverlap")
            || !i.objects[0]
                .rsplit_once('/')
                .is_some_and(|(path, _)| scopes.contains(path))
    });
    for scope in &scopes {
        for (name, pin) in def.pins.iter().filter(|(_, p)| p.unit == *unit) {
            let key = format!("{scope}/{}.{name}", row.id);
            let location = graph::transform(pin.at, pos, *angle, *mirror);
            c.terminals.get_mut(&key).expect("placed pin").at = location;
            c.locations.insert(key.clone(), location);
            changed.insert(key);
        }
    }
    let moved_wires: Vec<_> = c
        .wires
        .iter()
        .filter(|(_, (a, b, _))| changed.contains(a) || changed.contains(b))
        .map(|(id, _)| id.clone())
        .collect();
    changed.extend(moved_wires);
    c.issues.retain(|i| {
        !matches!(i.kind, "WireNearPin" | "WireOverlap") || !i.objects.iter().any(|id| changed.contains(id))
    });
    let sheet = row.sheet.clone();
    let rows = last
        .doc
        .rows
        .iter()
        .filter(|r| r.sheet == sheet)
        .collect::<Vec<_>>();
    for scope in &scopes {
        let findings = drawing::readability(&last.doc, c, scope, &rows);
        c.issues.extend(findings);
        c.issues
            .extend(super::wire_readability::findings(c, scope, Some(&changed)));
    }
    c.sort_findings();
    if delta != 0 {
        let shift = |span: &mut Span| {
            for n in [&mut span.lo, &mut span.hi] {
                if *n as usize >= old_end {
                    *n = (*n as i64 + delta) as u32;
                }
            }
        };
        for row in &mut last.doc.rows[idx..] {
            shift(&mut row.span);
            if let Some(span) = &mut row.pose_span {
                shift(span);
            }
        }
        for origin in c.origins.values_mut() {
            shift(origin);
        }
    }
    last.src.clear();
    last.src.push_str(src);
    true
}
