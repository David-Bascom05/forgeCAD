//! Bill of materials: counts by (document, body), rolled up through subassemblies. Reference
//! instances (an envelope, a floor) and suppressed ones are not counted. Nothing here needs a solve.

use crate::model::{Assembly, Component, DocId, Parts};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BomItem {
    Part { doc: DocId, body: u64 },
    Assembly(DocId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BomLine {
    pub item: BomItem,
    pub qty: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bom {
    /// This level's direct children: parts and subassemblies.
    pub top: Vec<BomLine>,
    /// Each subassembly document used anywhere below, with its own direct children.
    pub subs: BTreeMap<DocId, Vec<BomLine>>,
    /// Every part, all levels multiplied out.
    pub flat: Vec<BomLine>,
}

fn level(asm: &Assembly) -> Vec<BomLine> {
    let mut m: BTreeMap<BomItem, usize> = BTreeMap::new();
    for i in asm.instances.iter().filter(|i| !i.suppressed && !i.reference) {
        let item = match &i.of {
            Component::Part(p) => BomItem::Part {
                doc: p.doc.clone(),
                body: p.body,
            },
            Component::Assembly(d) => BomItem::Assembly(d.clone()),
        };
        *m.entry(item).or_default() += 1;
    }
    m.into_iter().map(|(item, qty)| BomLine { item, qty }).collect()
}

/// The BOM of an assembly. A subassembly that is missing, or that contains itself, counts as a
/// line of its own and adds no parts.
pub fn bom<P: Parts>(asm: &Assembly, parts: &P) -> Bom {
    let mut out = Bom {
        top: level(asm),
        ..Bom::default()
    };
    // parts per document, multiplied out, memoised; a cycle gives none
    fn flat<P: Parts>(
        doc: &DocId,
        parts: &P,
        memo: &mut BTreeMap<DocId, BTreeMap<(DocId, u64), usize>>,
        subs: &mut BTreeMap<DocId, Vec<BomLine>>,
        stack: &mut Vec<DocId>,
    ) -> BTreeMap<(DocId, u64), usize> {
        if let Some(m) = memo.get(doc) {
            return m.clone();
        }
        let Some(a) = parts.assembly(doc) else {
            return BTreeMap::new();
        };
        if stack.contains(doc) {
            return BTreeMap::new();
        }
        stack.push(doc.clone());
        let lines = level(a);
        let m = roll(&lines, parts, memo, subs, stack);
        stack.pop();
        subs.insert(doc.clone(), lines);
        memo.insert(doc.clone(), m.clone());
        m
    }
    fn roll<P: Parts>(
        lines: &[BomLine],
        parts: &P,
        memo: &mut BTreeMap<DocId, BTreeMap<(DocId, u64), usize>>,
        subs: &mut BTreeMap<DocId, Vec<BomLine>>,
        stack: &mut Vec<DocId>,
    ) -> BTreeMap<(DocId, u64), usize> {
        let mut m: BTreeMap<(DocId, u64), usize> = BTreeMap::new();
        for l in lines {
            match &l.item {
                BomItem::Part { doc, body } => *m.entry((doc.clone(), *body)).or_default() += l.qty,
                BomItem::Assembly(d) => {
                    for (k, q) in flat(d, parts, memo, subs, stack) {
                        *m.entry(k).or_default() += q * l.qty;
                    }
                }
            }
        }
        m
    }
    let mut memo = BTreeMap::new();
    let all = roll(&out.top, parts, &mut memo, &mut out.subs, &mut Vec::new());
    out.flat = all
        .into_iter()
        .map(|((doc, body), qty)| BomLine {
            item: BomItem::Part { doc, body },
            qty,
        })
        .collect();
    out
}
