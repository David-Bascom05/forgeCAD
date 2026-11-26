//! One hosted commit. A dialog's typed operations travel and land together.
use crate::{Refusal, inverse::inverse};
use forge_agent::json::Json;
use forge_lang::edit::{At, apply_at};
use forge_lang::error::{Error, Op};

#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    One(Op),
    Batch(Vec<Op>),
}

impl From<Op> for Edit {
    fn from(op: Op) -> Self {
        Self::One(op)
    }
}

impl Edit {
    pub fn ops(&self) -> &[Op] {
        match self {
            Self::One(op) => std::slice::from_ref(op),
            Self::Batch(ops) => ops,
        }
    }

    pub fn json(&self) -> Json {
        match self {
            Self::One(op) => op.json(),
            Self::Batch(ops) => Json::Arr(ops.iter().map(Op::json).collect()),
        }
    }

    pub fn from_json(j: &Json) -> Option<Self> {
        match j {
            Json::Arr(ops) if !ops.is_empty() => {
                Some(Self::Batch(ops.iter().map(Op::from_json).collect::<Option<_>>()?))
            }
            _ => Op::from_json(j).map(Self::One),
        }
    }

    pub fn apply(&self, src: &str, at: &At) -> Result<String, Error> {
        match self {
            Self::One(op) => apply_at(src, op, at),
            Self::Batch(ops) => ops
                .iter()
                .try_fold(src.to_owned(), |text, op| apply_at(&text, op, at)),
        }
    }

    pub(crate) fn checked(&self, src: &str) -> Result<String, Error> {
        match self {
            Self::One(op) => forge_lang::edit::apply_checked(src, op).map(|(text, _)| text),
            Self::Batch(ops) => forge_lang::edit::apply_all_checked(src, ops),
        }
    }

    pub(crate) fn check_limits(&self) -> Result<(), Refusal> {
        if self.ops().is_empty() {
            return Err(Refusal::Invalid {
                why: "an edit must contain an operation".into(),
            });
        }
        self.ops().iter().try_for_each(crate::check_limits)
    }

    pub(crate) fn inverse(&self, before: &str, after: &str) -> Option<Self> {
        match self {
            Self::One(op) => inverse(before, op, after).map(Self::One),
            Self::Batch(ops) => {
                let mut text = before.to_owned();
                let mut undo = Vec::with_capacity(ops.len());
                for op in ops {
                    let next = forge_lang::edit::apply(&text, op).ok()?;
                    undo.push(inverse(&text, op, &next)?);
                    text = next;
                }
                undo.reverse();
                (text == after).then_some(Self::Batch(undo))
            }
        }
    }
}
