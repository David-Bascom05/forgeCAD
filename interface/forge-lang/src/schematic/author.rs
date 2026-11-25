//! A pin-directed placement is a proposed, inspectable batch for the existing edit/undo path.
use super::*;
use crate::error::Op;
use forge_agent::json::Json;

fn error(k: &'static str, m: &str) -> Fault {
    Fault::new(k, Span::default(), m)
}
fn check_name(name: &str) -> Result<()> {
    if !name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || crate::syntax::KEYWORDS.contains(&name)
    {
        return Err(error("InvalidName", "choose an .fg identifier"));
    }
    Ok(())
}
fn resolve_role<'a>(doc: &'a Document, name: &str) -> Result<(&'a String, &'a RoleId)> {
    doc.role_names
        .get_key_value(name)
        .or_else(|| doc.role_names.iter().find(|(_, id)| id.0 == name))
        .ok_or_else(|| error("UnknownRole", name))
}
fn editable_sheet<'a>(c: &'a Circuit, scope: &str) -> Result<&'a String> {
    let sheet = &c.occurrences[scope];
    if c.occurrences.values().filter(|s| *s == sheet).count() > 1 {
        return Err(error(
            "SharedSheetDefinition",
            "this edit would change several occurrences; edit the sheet definition explicitly",
        ));
    }
    Ok(sheet)
}

pub(super) fn fulfillment(doc: &Document, c: &Circuit, args: &[&str]) -> Result<Json> {
    let [want, give, role, name, returns @ ..] = args else {
        return Err(error(
            "InvalidOperation",
            "plan-fulfillment WANT GIVE ROLE NEW_ID [RETURN_FROM RETURN_TO]",
        ));
    };
    check_name(name)?;
    let (alias, role) = resolve_role(doc, role)?;
    let pin = |id: &str| c.terminals.get(id).ok_or_else(|| error("UnknownPin", id));
    if !pin(want)?.wants.contains(role) || !pin(give)?.gives.contains(role) || want == give {
        return Err(error(
            "RoleMismatch",
            "select distinct pins that want and give this role",
        ));
    }
    let (scope, local) = want.rsplit_once('/').expect("qualified pin");
    let sheet = editable_sheet(c, scope)?;
    let on_sheet = |s: &str| -> Result<String> {
        pin(s)?;
        match s.rsplit_once('/') {
            Some((owner, key)) if owner == scope => Ok(key.into()),
            _ => Err(error("WrongSheet", "select pins on the same sheet occurrence")),
        }
    };
    let provider = on_sheet(give)?;
    if c.net_of[*want] != c.net_of[*give] {
        return Err(error(
            "DisconnectedFulfillment",
            "the pins have no electrical connection",
        ));
    }
    let ret = match returns {
        [] => String::new(),
        [a, b] => {
            let (ra, rb) = (on_sheet(a)?, on_sheet(b)?);
            if c.net_of[*a] != c.net_of[*b] || c.net_of[*a] == c.net_of[*want] || a == b {
                return Err(error(
                    "ReturnMismatch",
                    "choose distinct return pins on a common net separate from the supply",
                ));
            }
            format!(", return_from: {ra}, return_to: {rb}")
        }
        _ => return Err(error("InvalidReturn", "supply both return pins or neither")),
    };
    if c.links
        .iter()
        .any(|l| l.want == *want && l.give == *give && l.role == *role)
    {
        return Err(error(
            "AlreadyFulfilled",
            "this pair already has a confirmed relationship for that role",
        ));
    }
    let op = Op::AddFeature {
        before: None,
        text: format!(
            "{name} = fulfill(sheet: {sheet}, want: {local}, give: {provider}, role: {alias}{ret})"
        ),
    };
    Ok(Json::obj(vec![
        ("ops", Json::Arr(vec![op.json()])),
        ("sheet", sheet.clone().into()),
    ]))
}

pub(super) fn bypass(doc: &Document, c: &Circuit, args: &[&str]) -> Result<Json> {
    let [want, ret, sym, provider, other, role, name, x, y] = args else {
        return Err(error(
            "InvalidOperation",
            "plan-bypass WANT_PIN RETURN_PIN SYMBOL GIVE_PIN RETURN_PIN_NAME ROLE NEW_ID X_MM Y_MM",
        ));
    };
    let (alias, role) = resolve_role(doc, role)?;
    if doc.roles[role].kind != RoleKind::Bypass {
        return Err(error("RoleMismatch", "select a declared bypass role"));
    }
    let p = c.terminals.get(*want).ok_or_else(|| error("UnknownPin", want))?;
    let (scope, local) = want.rsplit_once('/').ok_or_else(|| error("UnknownPin", want))?;
    let (ret_scope, ret_local) = ret.rsplit_once('/').ok_or_else(|| error("UnknownPin", ret))?;
    if scope != ret_scope
        || !c.terminals.contains_key(*ret)
        || *want == *ret
        || c.net_of[*want] == c.net_of[*ret]
    {
        return Err(error(
            "InvalidReturn",
            "select a separate return pin on the same sheet and a distinct net",
        ));
    }
    let sheet = editable_sheet(c, scope)?;
    let def = doc.symbols.get(*sym).ok_or_else(|| error("UnknownSymbol", sym))?;
    if !p.wants.contains(role) || !def.pins.get(*provider).is_some_and(|p| p.gives.contains(role)) {
        return Err(error(
            "RoleMismatch",
            "the selected pins must want and give the same nominal bypass role",
        ));
    }
    if provider == other || !def.pins.contains_key(*other) {
        return Err(error(
            "InvalidReturn",
            "choose a distinct return pin on the new symbol",
        ));
    }
    check_name(name)?;
    let coordinate = |s: &str| {
        s.parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && v.abs() <= 1e12)
            .ok_or_else(|| error("InvalidCoordinate", s))
    };
    let (x, y) = (coordinate(x)?, coordinate(y)?);
    let role = alias;
    let rows = [
        format!("{name} = place(sheet: {sheet}, symbol: {sym}, at: ({x}mm, {y}mm))"),
        format!("{name}_feed = wire(sheet: {sheet}, from: {local}, to: {name}.{provider})"),
        format!("{name}_return = wire(sheet: {sheet}, from: {name}.{other}, to: {ret_local})"),
        format!(
            "{name}_intent = fulfill(sheet: {sheet}, want: {local}, give: {name}.{provider}, role: {role}, return_from: {name}.{other}, return_to: {ret_local})"
        ),
    ];
    let ops = rows
        .into_iter()
        .map(|text| Op::AddFeature { before: None, text }.json())
        .collect();
    Ok(Json::obj(vec![
        ("ops", Json::Arr(ops)),
        ("sheet", sheet.clone().into()),
        ("instance", (*name).into()),
    ]))
}
