//! Run-scoped ERC evidence. Optional data remains optional on every library symbol.
//! Rules read through require after establishing that a comparison applies.
use forge_agent::json::Json;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotRun {
    pub rule: String,
    pub object: String,
    pub fields: Vec<String>,
}
impl NotRun {
    pub fn json(&self) -> Json {
        Json::obj(vec![
            ("rule", self.rule.clone().into()),
            ("object", self.object.clone().into()),
            (
                "missing",
                Json::Arr(self.fields.iter().cloned().map(Into::into).collect()),
            ),
        ])
    }
}
type Ledger = BTreeMap<(String, String), BTreeSet<String>>;
thread_local! {static RUN:RefCell<Vec<Ledger>>=const{RefCell::new(Vec::new())};}
pub fn missing(rule: &str, object: &str, field: &str) {
    RUN.with_borrow_mut(|stack| {
        if let Some(active) = stack.last_mut() {
            active
                .entry((rule.into(), object.into()))
                .or_default()
                .insert(field.into());
        }
    });
}
/// Retains the original value and records an absent prerequisite for this occurrence.
pub fn require<T>(rule: &str, object: &str, field: &str, value: Option<T>) -> Option<T> {
    if value.is_none() {
        missing(rule, object, field);
    }
    value
}
pub(crate) fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<NotRun>) {
    struct Frame;
    impl Drop for Frame {
        fn drop(&mut self) {
            RUN.with_borrow_mut(|s| {
                s.pop();
            });
        }
    }
    RUN.with_borrow_mut(|s| s.push(Ledger::new()));
    let _frame = Frame;
    let result = f();
    let ledger = RUN.with_borrow_mut(|s| std::mem::take(s.last_mut().unwrap()));
    (
        result,
        ledger
            .into_iter()
            .map(|((rule, object), fields)| NotRun {
                rule,
                object,
                fields: fields.into_iter().collect(),
            })
            .collect(),
    )
}
