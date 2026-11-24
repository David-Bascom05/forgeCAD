//! The language's proof ledger stays complete: research/forge-lang/lean/COVERAGE.md has a row for
//! every built-in, document header, keyword and edit op the Rust accepts (a Lean theorem, or a
//! GAP that says what is not proven), and the Lean it names exists. A language extension that
//! comes without its row fails here, so every extension brings its proof, or its GAP, with it.
//!
//! What is read, all from the sources (no other crate is linked):
//! * built-in families: forge-lang's name lists (`FAMILIES`, one line each), the `match` arms of
//!   forge-cam-lang's and forge-turn-lang's dispatch, and every other `const` list of strings in
//!   those crates, which COVERAGE.md must name as not a list of names a document resolves;
//! * headers: every `n == "WORD"` that tests a document's first statement (`is_*`, `declared`);
//! * keywords: `syntax::KEYWORDS`;
//! * edit ops: the arms of `Op::from_json`, `Refactor::from_json`, and the CLI's Undo/Redo.
//!
//! And the hypothesis of `Builtins.extend_conservative` for each family resolved in the part's one
//! namespace (`Model::Shadow`): no part, assembly or drawing in the repository binds any of its
//! names (forge-lang's `weld::builtins_bound_nowhere` is this for the weldment family).

use forge_lang::syntax::{self, Stmt, StmtKind};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// How a family's names resolve.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Model {
    /// The part's one namespace: a binding first, then a built-in; binding a built-in's name is
    /// `ShadowsBuiltin` (`builtins::NAMES`). Lean: `Builtins.extend_conservative`.
    Shadow,
    /// A machining document: calls only among built-ins, reads only among bindings. Lean:
    /// `Families.run_ext_accepted`.
    Closed,
    /// Resolved by its own code path, not modelled in Lean (its row says GAP).
    Own,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn owned(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

/// The quoted strings of a text.
fn quoted(t: &str) -> Vec<String> {
    t.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

/// The strings of the list a `const NAME` holds in a source file (`&[..]` or `[&str; N]`).
fn const_list(rel: &str, name: &str) -> Vec<String> {
    let src = read(rel);
    let at = src
        .find(&format!("const {name}:"))
        .unwrap_or_else(|| panic!("{rel}: no const {name}"));
    let body = &src[at..];
    let open = body.find("= ").expect("=") + 2;
    let body = &body[open..];
    let start = body.find('[').expect("[");
    let end = body.find("];").expect("];");
    quoted(&body[start..end])
}

/// The text of a function: from `marker` to the next item at its indentation or shallower.
fn function<'a>(src: &'a str, marker: &str) -> &'a str {
    let at = src.find(marker).unwrap_or_else(|| panic!("no `{marker}`"));
    let line_start = src[..at].rfind('\n').map_or(0, |i| i + 1);
    let indent = at - line_start;
    let rest = &src[at..];
    let mut off = rest.find('\n').unwrap_or(rest.len());
    while off < rest.len() {
        let line = &rest[off + 1..];
        let lead = line.len() - line.trim_start().len();
        let t = line.trim_start();
        if lead <= indent
            && (t.starts_with('}') && lead < indent || t.starts_with("fn ") || t.starts_with("pub fn "))
        {
            return &rest[..off];
        }
        off += 1 + line.find('\n').unwrap_or(line.len());
    }
    rest
}

/// The string patterns of a `match`'s top-level arms in a function: the arms at the indentation
/// of the first `"x" =>` line (rustfmt keeps nested matches deeper), continuation lines `| "y"`
/// included.
fn arms(rel: &str, marker: &str) -> Vec<String> {
    let src = read(rel);
    let f = function(&src, marker);
    let mut indent = None;
    let mut out = vec![];
    let mut pending = false;
    for line in f.lines() {
        let lead = line.len() - line.trim_start().len();
        let t = line.trim_start();
        let starts = t.starts_with('"') && (t.contains("\" =>") || t.contains("\" |"));
        if indent.is_none() && starts {
            indent = Some(lead);
        }
        let Some(i) = indent else { continue };
        if (lead == i && starts) || (pending && lead >= i && t.starts_with("| \"")) {
            let pat = t.split("=>").next().unwrap_or("");
            out.extend(quoted(pat));
            pending = !t.contains("=>");
        } else {
            pending = false;
        }
    }
    out
}

/// The names a nested call may take in a machining evaluator: `fname == "x"` tests and the arms
/// of `match fname.as_str()`.
fn expr_builtins(rel: &str) -> Vec<String> {
    let src = read(rel);
    let mut out = vec![];
    for line in src.lines() {
        // every `fname == "x"` of the line (`fname == "cylinder" || fname == "tube"`)
        for (i, _) in line.match_indices("fname == \"") {
            out.extend(quoted(&line[i..]).into_iter().take(1));
        }
    }
    if let Some(at) = src.find("match fname.as_str()") {
        let line_start = src[..at].rfind('\n').map_or(0, |i| i + 1);
        let indent = at - line_start + 4;
        for line in src[at..].lines().skip(1) {
            let lead = line.len() - line.trim_start().len();
            let t = line.trim_start();
            if lead < indent {
                break;
            }
            if lead == indent && t.starts_with('"') {
                out.extend(quoted(t.split("=>").next().unwrap_or("")));
            }
        }
    }
    out
}

/// The built-in families: one line each. A new family is a new line here and its row in
/// COVERAGE.md.
fn families() -> Vec<(&'static str, Vec<String>, Model)> {
    use forge_lang::*;
    const CAM: &str = "interface/forge-cam-lang/src/lib.rs";
    const TURN: &str = "interface/forge-turn-lang/src/lib.rs";
    const BEND: &str = "interface/forge-bend-lang/src/lib.rs";
    vec![
        ("core", owned(builtins::BUILTINS), Model::Shadow),
        ("shapes", owned(shapes::NAMES), Model::Shadow),
        (
            "constants",
            {
                let f = "interface/forge-lang/src/builtins.rs";
                [const_list(f, "CONSTANTS"), const_list(f, "MORE")].concat()
            },
            Model::Shadow,
        ),
        ("sheet", owned(sheet::BUILTINS), Model::Shadow),
        ("sheet-constants", owned(sheet::CONSTANTS), Model::Shadow),
        ("thermo", owned(thermo::BUILTINS), Model::Shadow),
        ("weld", owned(weld::BUILTINS), Model::Shadow),
        ("wired", owned(wired::NAMES), Model::Shadow),
        ("assembly", owned(asm::NAMES), Model::Own),
        ("assembly-part", owned(asm::PART_NAMES), Model::Own),
        ("assembly-kinds", owned(asm::KINDS), Model::Own),
        ("harness", owned(harness::BUILTINS), Model::Own),
        ("drawing", owned(drawing::doc::FUNCTIONS), Model::Closed),
        (
            "cam",
            arms(CAM, "fn call(&mut self, name: &str, f: &str"),
            Model::Closed,
        ),
        ("cam-expr", expr_builtins(CAM), Model::Closed),
        ("turn", arms(TURN, "fn call(&mut self"), Model::Closed),
        ("turn-expr", expr_builtins(TURN), Model::Closed),
        (
            "bend",
            arms(BEND, "fn call(&mut self, name: &str, f: &str"),
            Model::Closed,
        ),
    ]
}

/// Where the families' lists are (the `const`s), so every other `const` list must be excused.
const FAMILY_CONSTS: &[&str] = &[
    "interface/forge-lang/src/builtins.rs::BUILTINS",
    "interface/forge-lang/src/builtins.rs::CONSTANTS",
    "interface/forge-lang/src/builtins.rs::MORE",
    "interface/forge-lang/src/shapes.rs::NAMES",
    "interface/forge-lang/src/sheet.rs::BUILTINS",
    "interface/forge-lang/src/sheet.rs::CONSTANTS",
    "interface/forge-lang/src/thermo.rs::BUILTINS",
    "interface/forge-lang/src/weld.rs::BUILTINS",
    "interface/forge-lang/src/wired.rs::NAMES",
    "interface/forge-lang/src/asm.rs::NAMES",
    "interface/forge-lang/src/asm.rs::PART_NAMES",
    "interface/forge-lang/src/asm.rs::KINDS",
    "interface/forge-lang/src/harness.rs::BUILTINS",
    "interface/forge-lang/src/drawing/doc.rs::FUNCTIONS",
    "interface/forge-lang/src/syntax.rs::KEYWORDS",
    "interface/forge-bend-lang/src/lib.rs::NAMES",
];

/// The crates whose sources define what a document may say.
const LANG_DIRS: &[&str] = &[
    "interface/forge-lang/src",
    "interface/forge-cam-lang/src",
    "interface/forge-turn-lang/src",
    "interface/forge-bend-lang/src",
];

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// A path under `root()` (every path here is one joined onto it), relative to it.
fn rel_of(p: &Path) -> String {
    p.strip_prefix(root())
        .expect("under the root")
        .display()
        .to_string()
}

/// Every `const X: &[&str]` or `const X: [&str; N]` in the language crates, as `path::X`.
fn const_lists() -> Vec<String> {
    let mut files = vec![];
    for d in LANG_DIRS {
        rs_files(&root().join(d), &mut files);
    }
    files.sort();
    let mut out = vec![];
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        for line in src.lines() {
            let t = line.trim_start();
            let t = t.strip_prefix("pub ").unwrap_or(t);
            let Some(rest) = t.strip_prefix("const ") else {
                continue;
            };
            let Some((name, ty)) = rest.split_once(':') else {
                continue;
            };
            let ty = ty.trim_start();
            if ty.starts_with("&[&str]")
                || ty.starts_with("&[&'static str]")
                || ty.starts_with("[&str;")
                || ty.starts_with("[&'static str;")
            {
                out.push(format!("{}::{}", rel_of(&f), name.trim()));
            }
        }
    }
    out
}

/// The words a document's first statement is tested against, with the file that tests it:
/// `n == "WORD"` on a line matching `ExprKind::Name(n)`, the program's `.first()` read on it or
/// on one of the three lines before (rustfmt breaks a `matches!`), and an assembly's
/// `l == "WORD"` first line.
fn headers() -> BTreeSet<(String, String)> {
    let mut files = vec![];
    rs_files(&root().join("interface"), &mut files);
    // the code that reads documents, not its tests
    files.retain(|f| f.components().any(|c| c.as_os_str() == "src"));
    let mut out = BTreeSet::new();
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap_or_default();
        let lines: Vec<&str> = src.lines().collect();
        for (k, line) in lines.iter().enumerate() {
            let first = lines[k.saturating_sub(3)..=k]
                .iter()
                .any(|l| l.contains(".first()"));
            if first
                && line.contains("ExprKind::Name(n)")
                && let Some(i) = line.find("n == \"")
            {
                out.extend(quoted(&line[i..]).into_iter().take(1).map(|w| (w, rel_of(&f))));
            }
            if line.contains(".is_some_and(|l| l == \"") {
                let i = line.find("l == \"").unwrap();
                out.extend(quoted(&line[i..]).into_iter().take(1).map(|w| (w, rel_of(&f))));
            }
        }
    }
    out
}

/// The files that recognise a document by its header, each a kind whose built-ins are families
/// above (or that only reads the header). A new kind of document (a new file here) must register
/// its families and its header's row.
const HEADER_FILES: &[(&str, &str)] = &[
    ("interface/forge-cam-lang/src/lib.rs", "families cam, cam-expr"),
    ("interface/forge-turn-lang/src/lib.rs", "families turn, turn-expr"),
    ("interface/forge-bend-lang/src/lib.rs", "family bend"),
    ("interface/forge-lang/src/drawing/doc.rs", "family drawing"),
    (
        "interface/forge-lang/src/asm.rs",
        "families assembly, assembly-part, assembly-kinds",
    ),
    (
        "interface/forge-lang/src/edit_ops.rs",
        "reads the header only: Rename's `calls_closed`",
    ),
    (
        "interface/forge-render/src/shell/fg.rs",
        "reads the header only: `is_assembly` (families assembly*)",
    ),
];

/// The CamelCase arms of a `from_json`.
fn json_ops(rel: &str, marker: &str) -> Vec<String> {
    let src = read(rel);
    let f = function(&src, marker);
    f.lines()
        .map(str::trim_start)
        .filter(|t| t.starts_with('"') && t.contains("\" =>"))
        .filter_map(|t| quoted(t).into_iter().next())
        .filter(|w| w.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
        .collect()
}

fn edit_ops() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    out.extend(json_ops(
        "interface/forge-lang/src/error.rs",
        "pub fn from_json(j: &Json) -> Option<Op>",
    ));
    out.extend(json_ops(
        "interface/forge-lang/src/refactor.rs",
        "pub fn from_json(j: &Json) -> Option<Refactor>",
    ));
    // the drawing document's own edits (a drawing file and a drawing block alike)
    out.extend(
        json_ops(
            "interface/forge-lang/src/drawing/ops.rs",
            "pub fn from_json(j: &Json) -> Option<DrawOp>",
        )
        .into_iter()
        .map(|o| format!("drawing::{o}")),
    );
    let cli = read("interface/forge-cli/src/fg.rs");
    for line in cli.lines() {
        if line.contains("=> Some(forge_lang::undo::Step::") {
            out.extend(quoted(line).into_iter().take(1));
        }
    }
    out
}

/// COVERAGE.md's rows: kind, item, names, Rust site, Lean.
struct Row {
    kind: String,
    item: String,
    names: Vec<String>,
    lean: String,
}

fn coverage() -> Vec<Row> {
    let md = read("research/forge-lang/lean/COVERAGE.md");
    let mut rows = vec![];
    for line in md.lines() {
        let t = line.trim();
        if !t.starts_with('|') || t.starts_with("|--") || t.starts_with("| Kind") || t.starts_with("| ---") {
            continue;
        }
        let cells: Vec<&str> = t.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 5 {
            continue;
        }
        let tick = |s: &str| {
            s.split('`')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        rows.push(Row {
            kind: cells[0].to_string(),
            item: cells[1].trim_matches('`').to_string(),
            names: tick(cells[2]),
            lean: cells[4].to_string(),
        });
    }
    rows
}

fn row<'a>(rows: &'a [Row], kind: &str, item: &str) -> Option<&'a Row> {
    rows.iter().find(|r| r.kind == kind && r.item == item)
}

fn diff(what: &str, rust: &BTreeSet<String>, doc: &BTreeSet<String>, errs: &mut Vec<String>) {
    let missing: Vec<_> = rust.difference(doc).collect();
    let stale: Vec<_> = doc.difference(rust).collect();
    if !missing.is_empty() {
        errs.push(format!("{what}: in the Rust, no row in COVERAGE.md: {missing:?}"));
    }
    if !stale.is_empty() {
        errs.push(format!("{what}: in COVERAGE.md, not in the Rust: {stale:?}"));
    }
}

fn fail(errs: Vec<String>) {
    assert!(
        errs.is_empty(),
        "research/forge-lang/lean/COVERAGE.md is behind the language (add each row: its Lean theorem, or GAP and what is unproven):\n{}",
        errs.join("\n")
    );
}

#[test]
fn every_builtin_has_its_row() {
    let rows = coverage();
    let mut errs = vec![];
    for (fam, names, model) in families() {
        assert!(
            !names.is_empty(),
            "family {fam}: no names read (did its source move?)"
        );
        let rust: BTreeSet<String> = names.into_iter().collect();
        match row(&rows, "builtins", fam) {
            None => errs.push(format!(
                "family {fam} ({model:?}): no `builtins` row; its names: {rust:?}"
            )),
            Some(r) => {
                let doc: BTreeSet<String> = r.names.iter().cloned().collect();
                diff(&format!("family {fam}"), &rust, &doc, &mut errs);
            }
        }
    }
    fail(errs);
}

#[test]
fn every_name_list_is_a_family_or_excused() {
    let rows = coverage();
    let excused: BTreeSet<String> = rows
        .iter()
        .filter(|r| r.kind == "not-names")
        .map(|r| r.item.clone())
        .collect();
    let mut errs = vec![];
    for c in const_lists() {
        if !FAMILY_CONSTS.contains(&c.as_str()) && !excused.contains(&c) {
            errs.push(format!("`{c}`: a new list of strings; a built-in family (a line in `families()` and a `builtins` row) or a `not-names` row"));
        }
    }
    let all: BTreeSet<String> = const_lists().into_iter().collect();
    for e in &excused {
        if !all.contains(e) {
            errs.push(format!("`{e}`: a `not-names` row for a list that is gone"));
        }
    }
    fail(errs);
}

#[test]
fn every_header_keyword_and_op_has_its_row() {
    let rows = coverage();
    let mut errs = vec![];
    let doc = |k: &str| -> BTreeSet<String> {
        rows.iter()
            .filter(|r| r.kind == k)
            .map(|r| r.item.clone())
            .collect()
    };
    let words: BTreeSet<String> = headers().into_iter().map(|h| h.0).collect();
    diff("headers", &words, &doc("header"), &mut errs);
    let kw: BTreeSet<String> = syntax::KEYWORDS.iter().map(|s| s.to_string()).collect();
    match row(&rows, "keywords", "syntax::KEYWORDS") {
        None => errs.push("no `keywords` row".into()),
        Some(r) => diff("keywords", &kw, &r.names.iter().cloned().collect(), &mut errs),
    }
    diff("edit ops", &edit_ops(), &doc("op"), &mut errs);
    fail(errs);
}

/// Every file that recognises a document by its header is a known kind: a new one (a `robot`
/// document, say) must put its built-ins in `families()`, its file in `HEADER_FILES`, and its rows
/// in COVERAGE.md.
#[test]
fn every_document_kind_is_registered() {
    let known: BTreeSet<&str> = HEADER_FILES.iter().map(|h| h.0).collect();
    let mut errs = vec![];
    for (w, f) in headers() {
        if !known.contains(f.as_str()) {
            errs.push(format!(
                "`{f}` recognises `{w}` documents: a new kind; its built-ins as families (`families()`), the file in `HEADER_FILES`, its rows"
            ));
        }
    }
    let found: BTreeSet<String> = headers().into_iter().map(|h| h.1).collect();
    for k in &known {
        if !found.contains(*k) {
            errs.push(format!("`{k}` in HEADER_FILES recognises no header now"));
        }
    }
    fail(errs);
}

/// A crate's per-built-in argument lists, `pub const ARGS: &[(&str, &[&str])] = &[("op", &["a",
/// ..]), ..]` (keywords, `#0`.. for positionals), if it has one.
fn arg_lists(rel: &str) -> Option<Vec<(String, Vec<String>)>> {
    let src = read(rel);
    let at = src.find("pub const ARGS: &[(&str, &[&str])]")?;
    let body = &src[at..];
    let body = &body[body.find("= &[").expect("= &[") + 4..];
    let body = &body[..body.find("\n];").expect("];")];
    let mut out = vec![];
    let mut rest = body;
    // an entry: `(`, maybe a line break (rustfmt splits long ones), the name, its list
    while let Some(i) = rest.find('(') {
        rest = &rest[i + 1..];
        if !rest.trim_start().starts_with('"') {
            continue;
        }
        let op = quoted(rest).into_iter().next().unwrap_or_default();
        let open = rest.find("&[").expect("&[");
        let close = rest[open..].find(']').expect("]") + open;
        out.push((op, quoted(&rest[open..close])));
        rest = &rest[close..];
    }
    Some(out)
}

/// Each document kind whose built-ins take named arguments: its families and where its argument
/// lists would be.
const ARG_KINDS: &[(&str, &str, &[&str])] = &[
    ("cam", "interface/forge-cam-lang/src/lib.rs", &["cam", "cam-expr"]),
    (
        "turn",
        "interface/forge-turn-lang/src/lib.rs",
        &["turn", "turn-expr"],
    ),
    ("bend", "interface/forge-bend-lang/src/lib.rs", &["bend"]),
];

/// The arguments each built-in takes: with an `ARGS` list, one `args` row per built-in, its
/// names the list's, and the list's built-ins exactly the families'; without one, an `args` row
/// for the kind saying GAP (an argument no list holds is read by nothing: silently ignored).
#[test]
fn every_argument_list_has_its_rows() {
    let rows = coverage();
    let fams = families();
    let mut errs = vec![];
    for (kind, file, fam_names) in ARG_KINDS {
        let names: BTreeSet<String> = fams
            .iter()
            .filter(|f| fam_names.contains(&f.0))
            .flat_map(|f| f.1.iter().cloned())
            .collect();
        match arg_lists(file) {
            None => match row(&rows, "args", kind) {
                Some(r) if r.lean.contains("GAP") => {}
                _ => errs.push(format!(
                    "`{kind}`: no argument list in {file}: an `args` row for `{kind}` saying GAP"
                )),
            },
            Some(lists) => {
                if row(&rows, "args", kind).is_some() {
                    errs.push(format!(
                        "`{kind}`: {file} has ARGS now: replace the `args` `{kind}` row by one per built-in"
                    ));
                }
                let ops: BTreeSet<String> = lists.iter().map(|l| l.0.clone()).collect();
                // a built-in listed twice: the lookup takes the first, the second is dead
                if ops.len() != lists.len() {
                    let mut seen = BTreeSet::new();
                    let twice: Vec<&String> = lists.iter().map(|l| &l.0).filter(|o| !seen.insert(*o)).collect();
                    errs.push(format!("`{kind}` ARGS: listed more than once {twice:?} (only the first entry is read)"));
                }
                // a built-in without a list yet: its `args` row says GAP (any keyword is taken,
                // unchecked), and the row goes when the list comes
                let mut missing = vec![];
                for op in names.difference(&ops) {
                    match row(&rows, "args", &format!("{kind}::{op}")) {
                        Some(r) if r.lean.contains("GAP") && r.names.is_empty() => {}
                        _ => missing.push(op),
                    }
                }
                for op in ops.intersection(&names) {
                    if row(&rows, "args", &format!("{kind}::{op}")).is_some_and(|r| r.lean.contains("GAP no list")) {
                        errs.push(format!("`{kind}::{op}` has a list now: its row's names are the list's, not GAP"));
                    }
                }
                let extra: Vec<_> = ops.difference(&names).collect();
                if !missing.is_empty() || !extra.is_empty() {
                    errs.push(format!(
                        "`{kind}` ARGS: built-ins without a list {missing:?}; lists of no built-in {extra:?}"
                    ));
                }
                for (op, args) in &lists {
                    let item = format!("{kind}::{op}");
                    let want: BTreeSet<String> = args.iter().cloned().collect();
                    match row(&rows, "args", &item) {
                        None => errs.push(format!("`{item}`: no `args` row; its arguments: {want:?}")),
                        Some(r) => diff(&item, &want, &r.names.iter().cloned().collect(), &mut errs),
                    }
                }
            }
        }
    }
    fail(errs);
}

/// Every Lean name a row cites, `Module.theorem` in backticks, is a theorem (or def) of that
/// module; a row that cites none says GAP.
#[test]
fn every_cited_theorem_exists() {
    let lean = root().join("research/forge-lang/lean/ForgeLang");
    let mut errs = vec![];
    for r in coverage() {
        let cites: Vec<String> = r.lean.split('`').skip(1).step_by(2).map(str::to_string).collect();
        let cites: Vec<&String> = cites
            .iter()
            .filter(|c| c.contains('.') && !c.contains(' '))
            .collect();
        if cites.is_empty() && !r.lean.contains("GAP") && r.kind != "not-names" {
            errs.push(format!("{} {}: neither a theorem nor GAP", r.kind, r.item));
        }
        for c in cites {
            let (module, thm) = c.rsplit_once('.').unwrap();
            let file = lean.join(format!("{}.lean", module.replace('.', "/")));
            let src = std::fs::read_to_string(&file).unwrap_or_default();
            let found = ["theorem ", "def ", "private theorem "].iter().any(|k| {
                src.contains(&format!("{k}{thm} "))
                    || src.contains(&format!("{k}{thm}\n"))
                    || src.contains(&format!("{k}{thm} :"))
            });
            if !found {
                errs.push(format!(
                    "{} {}: cites `{c}`, not in {}",
                    r.kind,
                    r.item,
                    file.display()
                ));
            }
        }
    }
    fail(errs);
}

/// The Lean's lists of the closed families are the Rust's (`Families.camStmt` ..).
#[test]
fn lean_family_lists_are_the_rusts() {
    let src = read("research/forge-lang/lean/ForgeLang/Families.lean");
    let weld = read("research/forge-lang/lean/ForgeLang/Builtins.lean");
    let lean = |src: &str, def: &str| -> BTreeSet<String> {
        let at = src
            .find(&format!("def {def} : List Name :="))
            .unwrap_or_else(|| panic!("no def {def}"));
        let body = &src[at..];
        let end = body.find(']').unwrap();
        quoted(&body[..end]).into_iter().collect()
    };
    let fams = families();
    let rust = |f: &str| -> BTreeSet<String> {
        fams.iter()
            .find(|x| x.0 == f)
            .unwrap()
            .1
            .iter()
            .cloned()
            .collect()
    };
    assert_eq!(lean(&src, "camStmt"), rust("cam"), "Families.camStmt");
    assert_eq!(lean(&src, "camExpr"), rust("cam-expr"), "Families.camExpr");
    assert_eq!(lean(&src, "turnStmt"), rust("turn"), "Families.turnStmt");
    assert_eq!(lean(&src, "turnExpr"), rust("turn-expr"), "Families.turnExpr");
    assert_eq!(lean(&weld, "weld"), rust("weld"), "Builtins.weld");
    assert_eq!(lean(&src, "drawingFns"), rust("drawing"), "Families.drawingFns");
    let hd: BTreeSet<String> = lean(&src, "headers");
    let words: BTreeSet<String> = headers().into_iter().map(|h| h.0).collect();
    assert!(hd.is_subset(&words), "Families.headers: {hd:?} vs {words:?}");
}

/// No family's name is a keyword: each lexes as an identifier (`Families.families_not_keywords`
/// for the machining families, `Builtins.weld_not_keywords` for the weldment's).
#[test]
fn no_builtin_is_a_keyword() {
    for (fam, names, _) in families() {
        for n in names {
            assert!(
                !syntax::KEYWORDS.contains(&n.as_str()),
                "family {fam}: `{n}` is a keyword"
            );
        }
    }
}

/// The names a statement list binds where `ShadowsBuiltin` applies (`Eval::bind`, and `shared` and
/// `param` statements), at any depth.
fn binders(stmts: &[Stmt], out: &mut Vec<(String, u32)>) {
    for s in stmts {
        let at = s.span.lo;
        match &s.kind {
            StmtKind::Shared { name, .. } | StmtKind::Param { name, .. } | StmtKind::Assign { name, .. } => {
                out.push((name.clone(), at))
            }
            // a function's name and its params are bindings the refusal does not apply to (a
            // call reads a param, then a function, before a built-in: they keep their meaning
            // when a built-in of their name comes, with no hypothesis)
            StmtKind::Fn { body, .. } => binders(body, out),
            StmtKind::For { vars, body, .. } => {
                out.extend(vars.iter().map(|v| (v.clone(), at)));
                binders(body, out);
            }
            StmtKind::If { then, other, .. } => {
                binders(then, out);
                if let Some(o) = other {
                    binders(o, out);
                }
            }
            StmtKind::Block { name, body, .. } => {
                out.push((name.clone(), at));
                binders(body, out);
            }
            StmtKind::Return(_) | StmtKind::Expect { .. } | StmtKind::Expr(_) => {}
        }
    }
}

fn fg_files() -> Vec<PathBuf> {
    let mut stack = vec![root()];
    let mut out = vec![];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("a directory").flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.is_dir() {
                if !name.starts_with('.') && name != "target" {
                    stack.push(p);
                }
            } else if name.ends_with(".fg") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// `Builtins.extend_conservative`'s hypothesis for each family of the part's one namespace: no
/// document resolved in it (a part or an assembly: not a machining document, whose names are
/// closed, nor a drawing) binds any of the family's names. A file that does not parse binds
/// nothing.
#[test]
fn shadow_families_bound_nowhere() {
    let fams: Vec<_> = families().into_iter().filter(|f| f.2 == Model::Shadow).collect();
    let mut files = 0;
    let mut refused = 0;
    let mut errs = vec![];
    for p in fg_files() {
        let src = std::fs::read_to_string(&p).unwrap_or_default();
        let Ok(prog) = syntax::parse(&src) else { continue };
        let header = match prog.first().map(|s| &s.kind) {
            Some(StmtKind::Expr(e)) => match &e.kind {
                syntax::ExprKind::Name(n) => Some(n.clone()),
                _ => None,
            },
            _ => None,
        };
        if matches!(header.as_deref(), Some("machining" | "drawing")) {
            continue;
        }
        files += 1;
        let mut bs = vec![];
        binders(&prog, &mut bs);
        for (n, at) in bs {
            for (fam, names, _) in &fams {
                if names.contains(&n) {
                    // a document refused for it already (with every family it has now) is not one
                    // an extension refuses
                    let opts = forge_lang::Options {
                        build: false,
                        dir: p.parent().map(Path::to_path_buf),
                        ..Default::default()
                    };
                    let (r, _) = forge_lang::run(&src, opts, Default::default());
                    if r.errors.iter().any(
                        |e| matches!(e, forge_lang::error::Error::ShadowsBuiltin { name, .. } if *name == n),
                    ) {
                        refused += 1;
                        continue;
                    }
                    let line = src[..at as usize].lines().count().max(1);
                    errs.push(format!("{}:{line}: binds `{n}` of family {fam}", p.display()));
                }
            }
        }
    }
    eprintln!("{files} documents read; {refused} bindings of a built-in's name refused already");
    assert!(files > 50, "only {files} .fg files read");
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}
