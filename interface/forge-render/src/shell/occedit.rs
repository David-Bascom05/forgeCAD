//! Edit Occurrence: one placed call of a component of this file (`p1 = part(peg(d: small))`),
//! its params as fields, Inventor's iPart member picked by values. OK writes the call back as
//! one SetArg on the occurrence's `part(..)`; the definition is untouched (Edit Feature on its
//! features edits every call).

use forge_lang::error::Op;
use forge_lang::syntax::{self, Arg, Expr, ExprKind, StmtKind};

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    /// The call's text for it, or empty for the default.
    pub text: String,
    pub default: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OccEdit {
    pub occ: String,
    pub component: String,
    pub fields: Vec<Field>,
}

fn text(src: &str, e: &Expr) -> String {
    src.get(e.span.lo as usize..e.span.hi as usize)
        .unwrap_or("")
        .to_string()
}

/// A component definition's params and defaults, as written.
fn params(src: &str, prog: &[syntax::Stmt], name: &str) -> Option<Vec<(String, String)>> {
    prog.iter().find_map(|s| match &s.kind {
        StmtKind::Block {
            kind: syntax::BlockKind::Component,
            name: n,
            args,
            ..
        } if n == name => Some(
            args.iter()
                .filter_map(|a| Some((a.name.as_ref()?.0.clone(), text(src, &a.value))))
                .collect(),
        ),
        _ => None,
    })
}

impl OccEdit {
    /// The dialog for top-level occurrence `occ`, when it places a component of this file.
    pub fn open(src: &str, occ: &str) -> Option<OccEdit> {
        let prog = syntax::parse(src).ok()?;
        let call = prog.iter().find_map(|s| match &s.kind {
            StmtKind::Assign { name, value } if name == occ => match &value.kind {
                ExprKind::Call(f, args) if matches!(&f.kind, ExprKind::Name(n) if n == "part") => {
                    args.iter().find(|a| a.name.is_none()).map(|a| a.value.clone())
                }
                _ => None,
            },
            _ => None,
        })?;
        let (component, given): (String, Vec<Arg>) = match &call.kind {
            ExprKind::Name(n) => (n.clone(), vec![]),
            ExprKind::Call(f, args) => match &f.kind {
                ExprKind::Name(n) => (n.clone(), args.clone()),
                _ => return None,
            },
            _ => return None,
        };
        let defs = params(src, &prog, &component)?;
        let fields = defs
            .into_iter()
            .map(|(name, default)| Field {
                text: given
                    .iter()
                    .find(|a| a.name.as_ref().is_some_and(|x| x.0 == name))
                    .map_or_else(String::new, |a| text(src, &a.value)),
                name,
                default,
            })
            .collect();
        Some(OccEdit {
            occ: occ.into(),
            component,
            fields,
        })
    }

    /// The call as the fields say: the params given (an empty field is the default).
    pub fn call(&self) -> String {
        let given: Vec<String> = self
            .fields
            .iter()
            .filter(|f| !f.text.trim().is_empty())
            .map(|f| format!("{}: {}", f.name, f.text.trim()))
            .collect();
        if given.is_empty() {
            self.component.clone()
        } else {
            format!("{}({})", self.component, given.join(", "))
        }
    }

    pub fn op(&self) -> Op {
        Op::SetArg {
            id: self.occ.clone(),
            arg: "#0".into(),
            text: self.call(),
        }
    }
}

/// How many occurrences place a component (`#peg(..)` files), for "Editing peg (3 occurrences)".
pub fn occurrences(occ: &[super::fgasm::OccRow], component: &str) -> usize {
    let head = format!("#{component}(");
    occ.iter().filter(|o| o.file.starts_with(&head)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "shared small = 6
shared plen = 30
component peg(d: 6, h: 30) {
  sketch s(on: xy) {
    c = circle((0, 0), d / 2)
  }
  shank = extrude(s, h)
}
p1 = part(peg(d: small, h: plen), at: (0, 0, 8))
p2 = part(peg)
";

    #[test]
    fn an_occurrence_reads_its_args_and_writes_its_call() {
        let mut e = OccEdit::open(SRC, "p1").unwrap();
        assert_eq!(e.component, "peg");
        assert_eq!(e.fields[0].text, "small");
        assert_eq!(e.fields[1].default, "30");
        e.fields[1].text.clear();
        e.fields[0].text = "9".into();
        let out = forge_lang::edit::apply(SRC, &e.op()).unwrap();
        assert!(out.contains("p1 = part(peg(d: 9), at: (0, 0, 8))"), "{out}");
        let r = forge_lang::run(&out, Default::default(), Default::default()).0;
        assert!(
            r.ok(),
            "{:?}",
            r.errors.iter().map(|x| x.text()).collect::<Vec<_>>()
        );
        assert!(r.components.iter().any(|c| c.id == "peg(d=9,h=30)"));
    }

    #[test]
    fn the_defaults_occurrence_and_one_that_is_no_component() {
        let mut e = OccEdit::open(SRC, "p2").unwrap();
        assert!(e.fields.iter().all(|f| f.text.is_empty()));
        assert_eq!(e.call(), "peg");
        e.fields[1].text = "plen".into();
        assert_eq!(e.call(), "peg(h: plen)");
        assert!(OccEdit::open(SRC, "small").is_none());
        assert!(OccEdit::open("k = part(\"knob.fg\")\n", "k").is_none());
    }
}
