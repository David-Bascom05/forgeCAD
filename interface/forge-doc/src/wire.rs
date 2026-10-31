//! The binary form of document ops: a few bytes for "item 17, argument 2 = 12.5". The op log is a
//! file of these and the protocol will carry them. All integers are LEB128, strings have a length.

use crate::doc::{Arg, Body, Item, ItemId, Op, Row};
use forge_model::Unit;

/// What a log entry is to the undo stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Do,
    Undo,
    Redo,
}

/// One committed change: atomic, one version of the part.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub kind: Kind,
    /// Who made it: the protocol's identity slot, 0 until there are accounts.
    pub who: u64,
    pub ops: Vec<Op>,
    /// What undoes it, applied in order. Empty: nothing can (a hand edit).
    pub undo: Vec<Op>,
    /// FNV-1a of the file it leaves.
    pub hash: u64,
    /// What it left, for "what changed since": faces and exact volume.
    pub faces: u32,
    pub volume: f64,
}

/// FNV-1a over 64 bits: enough to tell a file was edited, or that two logs agree.
pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn int(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push(v as u8 | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn string(out: &mut Vec<u8>, s: &str) {
    int(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

/// 0 for none, else the id plus one.
fn opt_id(out: &mut Vec<u8>, id: Option<ItemId>) {
    int(out, id.map_or(0, |i| i.0.wrapping_add(1)));
}

fn opt_str(out: &mut Vec<u8>, s: &Option<String>) {
    match s {
        // length plus one, 0 for none
        Some(s) => {
            int(out, s.len() as u64 + 1);
            out.extend_from_slice(s.as_bytes());
        }
        None => out.push(0),
    }
}

fn args(out: &mut Vec<u8>, a: &[Arg]) {
    int(out, a.len() as u64);
    for x in a {
        string(out, &x.text);
        opt_id(out, x.to);
    }
}

fn item(out: &mut Vec<u8>, it: &Item) {
    int(out, it.id.0);
    opt_str(out, &it.label);
    opt_str(out, &it.note);
    match &it.body {
        Body::Blank => out.push(0),
        Body::Param { name, expr, unit } => {
            out.push(1 + *unit as u8);
            string(out, name);
            string(out, expr);
        }
        Body::Sketch { head, rows } => {
            out.push(4);
            args(out, head);
            int(out, rows.len() as u64);
            for r in rows {
                int(out, r.args.len() as u64);
                r.args.iter().for_each(|a| string(out, a));
                opt_str(out, &r.note);
            }
        }
        Body::Feature { kind, args: a } => {
            out.push(5);
            string(out, kind);
            args(out, a);
        }
    }
}

pub fn put_op(out: &mut Vec<u8>, op: &Op) {
    match op {
        Op::SetParam { item, expr } => {
            out.push(0);
            int(out, item.0);
            string(out, expr);
        }
        Op::SetDimension { item, row, value } => {
            out.push(1);
            int(out, item.0);
            int(out, *row as u64);
            string(out, value);
        }
        Op::SetArg { item, arg, text } => {
            out.push(2);
            int(out, item.0);
            int(out, *arg as u64);
            string(out, text);
        }
        Op::SetItem { item: it } => {
            out.push(3);
            item(out, it);
        }
        Op::Insert { before, items } => {
            out.push(4);
            opt_id(out, *before);
            int(out, items.len() as u64);
            items.iter().for_each(|it| item(out, it));
        }
        Op::Delete { item } => {
            out.push(5);
            int(out, item.0);
        }
        Op::Move { item, before } => {
            out.push(6);
            int(out, item.0);
            opt_id(out, *before);
        }
        Op::Text { src } => {
            out.push(7);
            string(out, src);
        }
    }
}

/// Reads the forms above; `None` where the bytes are not one.
pub struct Reader<'a>(pub &'a [u8]);

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let (a, b) = self.0.split_first()?;
        self.0 = b;
        Some(*a)
    }

    fn int(&mut self) -> Option<u64> {
        let (mut v, mut shift) = (0u64, 0);
        loop {
            let b = self.byte()?;
            if shift > 63 {
                return None;
            }
            v |= ((b & 0x7f) as u64) << shift;
            if b < 0x80 {
                return Some(v);
            }
            shift += 7;
        }
    }

    fn len(&mut self) -> Option<usize> {
        usize::try_from(self.int()?).ok()
    }

    fn take(&mut self, n: usize) -> Option<String> {
        let (a, b) = self.0.split_at_checked(n)?;
        self.0 = b;
        String::from_utf8(a.to_vec()).ok()
    }

    fn string(&mut self) -> Option<String> {
        let n = self.len()?;
        self.take(n)
    }

    fn opt_str(&mut self) -> Option<Option<String>> {
        match self.len()? {
            0 => Some(None),
            n => self.take(n - 1).map(Some),
        }
    }

    fn opt_id(&mut self) -> Option<Option<ItemId>> {
        Some(match self.int()? {
            0 => None,
            v => Some(ItemId(v - 1)),
        })
    }

    /// `n` of something, each at least a byte: a count the bytes cannot hold is refused unread.
    fn many<T>(&mut self, mut one: impl FnMut(&mut Self) -> Option<T>) -> Option<Vec<T>> {
        let n = self.len()?;
        if n > self.0.len() {
            return None;
        }
        (0..n).map(|_| one(self)).collect()
    }

    fn args(&mut self) -> Option<Vec<Arg>> {
        self.many(|r| {
            Some(Arg {
                text: r.string()?,
                to: r.opt_id()?,
            })
        })
    }

    fn item(&mut self) -> Option<Item> {
        let id = ItemId(self.int()?);
        let label = self.opt_str()?;
        let note = self.opt_str()?;
        let body = match self.byte()? {
            0 => Body::Blank,
            u @ 1..=3 => Body::Param {
                unit: [Unit::None, Unit::Mm, Unit::Deg][u as usize - 1],
                name: self.string()?,
                expr: self.string()?,
            },
            4 => Body::Sketch {
                head: self.args()?,
                rows: self.many(|r| {
                    Some(Row {
                        args: r.many(Self::string)?,
                        note: r.opt_str()?,
                    })
                })?,
            },
            5 => Body::Feature {
                kind: self.string()?,
                args: self.args()?,
            },
            _ => return None,
        };
        Some(Item {
            id,
            label,
            body,
            note,
        })
    }

    pub fn op(&mut self) -> Option<Op> {
        Some(match self.byte()? {
            0 => Op::SetParam {
                item: ItemId(self.int()?),
                expr: self.string()?,
            },
            1 => Op::SetDimension {
                item: ItemId(self.int()?),
                row: self.len()?,
                value: self.string()?,
            },
            2 => Op::SetArg {
                item: ItemId(self.int()?),
                arg: self.len()?,
                text: self.string()?,
            },
            3 => Op::SetItem { item: self.item()? },
            4 => Op::Insert {
                before: self.opt_id()?,
                items: self.many(Self::item)?,
            },
            5 => Op::Delete {
                item: ItemId(self.int()?),
            },
            6 => Op::Move {
                item: ItemId(self.int()?),
                before: self.opt_id()?,
            },
            7 => Op::Text { src: self.string()? },
            _ => return None,
        })
    }

    fn ops(&mut self) -> Option<Vec<Op>> {
        self.many(Self::op)
    }
}

const MAGIC: &[u8; 8] = b"FCOPLOG1";

/// An entry as the log file holds it: its length first, so a torn tail is seen.
pub fn put_entry(out: &mut Vec<u8>, e: &Entry) {
    let mut b = Vec::new();
    b.push(e.kind as u8);
    int(&mut b, e.who);
    b.extend_from_slice(&e.hash.to_le_bytes());
    int(&mut b, e.faces as u64);
    b.extend_from_slice(&e.volume.to_bits().to_le_bytes());
    for list in [&e.ops, &e.undo] {
        int(&mut b, list.len() as u64);
        list.iter().for_each(|op| put_op(&mut b, op));
    }
    int(out, b.len() as u64);
    out.extend_from_slice(&b);
}

impl Reader<'_> {
    fn entry(&mut self) -> Option<Entry> {
        let kind = match self.byte()? {
            0 => Kind::Do,
            1 => Kind::Undo,
            2 => Kind::Redo,
            _ => return None,
        };
        let who = self.int()?;
        let (h, rest) = self.0.split_first_chunk::<8>()?;
        self.0 = rest;
        let hash = u64::from_le_bytes(*h);
        let faces = u32::try_from(self.int()?).ok()?;
        let (v, rest) = self.0.split_first_chunk::<8>()?;
        self.0 = rest;
        Some(Entry {
            kind,
            who,
            hash,
            faces,
            volume: f64::from_bits(u64::from_le_bytes(*v)),
            ops: self.ops()?,
            undo: self.ops()?,
        })
    }
}

/// A new log file's first bytes.
pub fn log_header() -> Vec<u8> {
    MAGIC.to_vec()
}

/// The entries of a log file, and how many of its bytes they cover: a tail cut short by a crash
/// is left out, to be written over. `None` if this is no log file.
pub fn read_log(bytes: &[u8]) -> Option<(Vec<Entry>, usize)> {
    let mut r = Reader(bytes.strip_prefix(MAGIC)?);
    let mut out = Vec::new();
    let mut good = MAGIC.len();
    while !r.0.is_empty() {
        let at = r.0;
        let Some(n) = r.len() else { break };
        let Some((body, rest)) = r.0.split_at_checked(n) else {
            break;
        };
        let mut one = Reader(body);
        match one.entry() {
            Some(e) if one.0.is_empty() => out.push(e),
            _ => break,
        }
        r.0 = rest;
        good += at.len() - rest.len();
    }
    Some((out, good))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Op> {
        let mut d = crate::Document::parse(
            "# a note\nw = 4 mm # width\nsk: sketch xy\n  rect 0 0 w 2\n  dist l1 w  # d\nend\nextrude 1 sk\n",
        );
        let items: Vec<Item> = d.read_items("z: ball 2 at 0 0 9\n\nh = 3 deg");
        let mut linked = d.items[3].clone();
        if let Body::Feature { args, .. } = &mut linked.body {
            args[1].to = Some(d.items[2].id);
        }
        vec![
            Op::SetParam {
                item: d.items[1].id,
                expr: "12.5".into(),
            },
            Op::SetDimension {
                item: d.items[2].id,
                row: 1,
                value: "w/2".into(),
            },
            Op::SetArg {
                item: d.items[3].id,
                arg: 0,
                text: "2".into(),
            },
            Op::SetItem { item: linked },
            Op::Insert {
                before: Some(d.items[0].id),
                items,
            },
            Op::Insert {
                before: None,
                items: vec![],
            },
            Op::Delete { item: d.items[0].id },
            Op::Move {
                item: d.items[3].id,
                before: None,
            },
            Op::Text {
                src: "ball 1\n".into(),
            },
        ]
    }

    #[test]
    fn every_op_comes_back() {
        for op in sample() {
            let mut b = Vec::new();
            put_op(&mut b, &op);
            let mut r = Reader(&b);
            assert_eq!(r.op().as_ref(), Some(&op));
            assert!(r.0.is_empty());
            // cut anywhere it is refused or short, never a panic
            for n in 0..b.len() {
                let mut r = Reader(&b[..n]);
                assert_ne!(r.op().as_ref(), Some(&op));
            }
        }
    }

    #[test]
    fn setting_a_value_is_a_few_bytes() {
        let mut b = Vec::new();
        put_op(
            &mut b,
            &Op::SetArg {
                item: ItemId(17),
                arg: 2,
                text: "12.5".into(),
            },
        );
        assert_eq!(b.len(), 8);
    }

    #[test]
    fn a_log_survives_a_torn_tail() {
        let e = Entry {
            kind: Kind::Undo,
            who: 3,
            ops: sample(),
            undo: sample()[..2].to_vec(),
            hash: hash(b"x"),
            faces: 6,
            volume: 1.5,
        };
        let mut b = log_header();
        put_entry(&mut b, &e);
        let whole = b.len();
        put_entry(&mut b, &e);
        assert_eq!(read_log(&b), Some((vec![e.clone(), e.clone()], b.len())));
        let (got, good) = read_log(&b[..b.len() - 1]).expect("a log");
        assert_eq!((got, good), (vec![e], whole));
        assert!(read_log(b"not a log").is_none());
    }
}
