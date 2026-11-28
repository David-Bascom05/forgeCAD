//! Quantities, arguments and diagnostics shared by native electronics documents.
use crate::syntax::{self, Arg, BinOp, Expr, ExprKind as E, Span, StmtKind as S, Unit};
use crate::values::{self, Dim, Num};
use forge_agent::json::Json;
use std::collections::BTreeMap as Map;
pub(crate) struct Call<'a> {
    pub(crate) name: &'a str,
    pub(crate) args: Map<&'a str, &'a Expr>,
    pub(crate) span: Span,
}
impl<'a> Call<'a> {
    pub(crate) fn with_signature(expr: &'a Expr, vocabulary: &[(&str, &[&str])]) -> Result<Self> {
        if let E::Call(head, args) = &expr.kind
            && let E::Name(name) = &head.kind
        {
            let signature = vocabulary
                .iter()
                .find(|(word, _)| *word == name)
                .ok_or_else(|| fault(head, "UnknownBuiltin", name))?
                .1;
            let mut fields = Map::new();
            for Arg { name: key, value } in args {
                let Some((key, span)) = key else {
                    return Err(fault(value, "ExpectedNamedArgument", "use a named argument"));
                };
                if !signature.contains(&key.as_str()) {
                    return Err(Fault::new(
                        "UnknownArgument",
                        *span,
                        format!("{name} has no argument {key}"),
                    ));
                }
                if fields.insert(key.as_str(), value).is_some() {
                    return Err(Fault::new("DuplicateArgument", *span, key));
                }
            }
            Ok(Self {
                name,
                args: fields,
                span: expr.span,
            })
        } else {
            Err(fault(expr, "ExpectedCall", "expected a named declaration call"))
        }
    }
    pub(crate) fn take(&mut self, key: &str) -> Option<&'a Expr> {
        self.args.remove(key)
    }
    pub(crate) fn req(&mut self, key: &str) -> Result<&'a Expr> {
        self.take(key).ok_or_else(|| {
            Fault::new(
                "MissingArgument",
                self.span,
                format!("{} requires {key}", self.name),
            )
        })
    }
    pub(crate) fn end(self) -> Result<()> {
        if let Some((k, e)) = self.args.first_key_value() {
            Err(fault(
                e,
                "UnknownArgument",
                format!("{} has no argument {k}", self.name),
            ))
        } else {
            Ok(())
        }
    }
}
pub(crate) fn fault(e: &Expr, k: &'static str, msg: impl Into<String>) -> Fault {
    Fault::new(k, e.span, msg)
}
pub(crate) fn string(e: &Expr) -> Result<String> {
    match &e.kind {
        E::Str(s) => Ok(s.clone()),
        _ => Err(fault(e, "ExpectedString", "expected quoted text")),
    }
}
pub(crate) fn boolean(e: &Expr) -> Result<bool> {
    match e.kind {
        E::Bool(x) => Ok(x),
        _ => Err(fault(e, "ExpectedBool", "expected true or false")),
    }
}
pub(crate) fn reference(e: &Expr) -> Result<String> {
    match &e.kind {
        E::Name(s) => Ok(s.clone()),
        E::Attr(base, k) => Ok(format!("{}.{k}", reference(base)?)),
        _ => Err(fault(
            e,
            "ExpectedReference",
            "expected a declared name or pin reference",
        )),
    }
}
pub(crate) fn list(e: &Expr) -> Result<&[Expr]> {
    if let E::List(xs) = &e.kind {
        Ok(xs)
    } else {
        Err(fault(e, "ExpectedList", "expected a list"))
    }
}
pub(crate) type Params = Map<String, Num>;
pub(crate) fn number(e: &Expr, params: &Params) -> Result<Num> {
    let n = match &e.kind {
        E::Num(x, u) => match u {
            Unit::In => Num::konst(x * 25.4, Dim::Length),
            Unit::Mm => Num::konst(*x, Dim::Length),
            Unit::Deg => Num::konst(*x, Dim::Angle),
            Unit::None => Num::konst(*x, Dim::Any),
        },
        E::Neg(v) => values::neg(&number(v, params)?),
        E::Name(k) => params
            .get(k)
            .cloned()
            .ok_or_else(|| fault(e, "UnknownParameter", k))?,
        E::Bin(op, l, r) => {
            let (l, r) = (number(l, params)?, number(r, params)?);
            match op {
                BinOp::Add | BinOp::Sub => values::add(&l, &r, *op == BinOp::Sub, e.span),
                BinOp::Mul => values::mul(&l, &r, e.span),
                BinOp::Div => values::div(&l, &r, e.span),
                _ => return Err(fault(e, "ExpectedQuantity", "expected arithmetic")),
            }
            .map_err(|err| fault(e, err.kind(), err.text()))?
        }
        _ => return Err(fault(e, "ExpectedQuantity", "expected a numeric expression")),
    };
    if !n.v.is_finite() || n.v.abs() > 1e12 {
        return Err(fault(
            e,
            "QuantityOutOfRange",
            "quantity exceeds the supported range",
        ));
    }
    Ok(n)
}
pub(crate) fn measure(e: &Expr, p: &Params, d: Dim) -> Result<f64> {
    let n = number(e, p)?;
    if n.dim != Dim::Any && n.dim != d {
        return Err(fault(
            e,
            "WrongDim",
            format!("expected {}, got {}", d.name(), n.dim.name()),
        ));
    }
    Ok(n.v)
}
pub(crate) fn positive(e: &Expr, p: &Params) -> Result<f64> {
    let v = measure(e, p, Dim::Length)?;
    if v <= 0.0 {
        return Err(fault(e, "ExpectedPositiveLength", "expected a positive length"));
    }
    Ok(v)
}
pub(crate) fn point(e: &Expr, p: &Params) -> Result<[f64; 2]> {
    if let E::Tuple(xs) = &e.kind
        && let [x, y] = xs.as_slice()
    {
        return Ok([measure(x, p, Dim::Length)?, measure(y, p, Dim::Length)?]);
    }
    Err(fault(e, "ExpectedPoint", "expected (x, y) in length units"))
}

pub(crate) fn header(src: &str) -> Option<&str> {
    src.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .find(|l| !l.is_empty())
}

pub fn parameter_dimension(src: &str, key: &crate::error::ParamKey) -> Option<Dim> {
    let mut params = Map::new();
    for stmt in syntax::parse(src).ok()? {
        if let S::Shared { name, value } = stmt.kind {
            let mut n = number(&value, &params).ok()?;
            if n.dim == Dim::Any {
                n.dim = Dim::Length;
            }
            if crate::error::ParamKey::shared(&name) == *key {
                return Some(n.dim);
            }
            params.insert(name, n);
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
    pub kind: &'static str,
    pub message: String,
    pub span: Span,
    pub file: Option<String>,
}
impl Fault {
    pub(crate) fn new(kind: &'static str, span: Span, msg: impl Into<String>) -> Self {
        Self {
            kind,
            message: msg.into(),
            span,
            file: None,
        }
    }
}
pub type Result<T> = std::result::Result<T, Fault>;

impl Fault {
    pub fn json_for(&self, schema: &str) -> Json {
        Json::obj(vec![
            ("schema", schema.into()),
            ("ok", false.into()),
            (
                "error",
                Json::obj(vec![
                    ("kind", self.kind.into()),
                    ("message", self.message.clone().into()),
                    ("span", crate::error::span_json(self.span)),
                    ("file", self.file.clone().map_or(Json::Null, Into::into)),
                ]),
            ),
        ])
    }
}

/// Compose library defaults independently of import order. The consuming document
/// may resolve a disagreement with its own explicit setting.
#[derive(Default)]
pub(crate) struct RuleDefaults(Map<String, Map<String, String>>);
impl RuleDefaults {
    pub(crate) fn import(&mut self, alias: &str, rules: Map<String, String>) {
        for (rule, level) in rules {
            self.0.entry(rule).or_default().insert(alias.into(), level);
        }
    }
    pub(crate) fn resolve(self, local: &mut Map<String, String>, span: Span) -> Result<()> {
        for (rule, sources) in self.0 {
            if local.contains_key(&rule) {
                continue;
            }
            let mut values = sources.values();
            let first = values.next().expect("a policy source");
            if values.any(|v| v != first) {
                return Err(Fault::new(
                    "ConflictingRuleDefaults",
                    span,
                    format!(
                        "{rule} has conflicting library severities {sources:?}. Set its severity in this document."
                    ),
                ));
            }
            local.insert(rule, first.clone());
        }
        Ok(())
    }
}
pub(crate) fn rule_severity(e: &Expr) -> Result<String> {
    let v = string(e)?;
    if matches!(v.as_str(), "error" | "warning" | "info") {
        Ok(v)
    } else {
        Err(fault(e, "InvalidSeverity", v))
    }
}
