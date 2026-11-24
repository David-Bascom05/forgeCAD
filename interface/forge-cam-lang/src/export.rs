//! NC export: a machining document's programs as files, the same for the app and the CLI
//! (`forgecad cam F post [PROGRAM] [-o FILE|DIR] [--allow-uncertified]`).
//!
//! A program's export settings are keyword arguments of its own statement, so they are saved
//! with the document and versioned with it:
//!
//! ```text
//! prog = program(op1, post: "fanuc", number: 1001,
//!                file: "flange-op1", out: "nc", comment: "flange, first side", tools: "list")
//! ```
//!
//! - `file:` the file name without extension (default: the program's name);
//! - `ext:` the extension (default: the dialect's, `ext_of`);
//! - `out:` the folder, relative to the document (default: beside it);
//! - `comment:` a comment line in the header;
//! - `tools:` `"list"` puts a tool list in the header (T, tool, holder, stick-out, ops).
//!
//! The text is the posted program as it was certified, with comment lines added after its first
//! line (`%`) or at its top: comments only, so what the control runs is what was certified. A
//! program is exported only when every op it posts is certified in full; one released "except
//! G5" (a setup not placed: positions unchecked) is refused unless `allow_uncertified`, which
//! stamps the header with what was not checked. A program with a refused op has no text at all
//! and is never exported.
use forge_lang::syntax::{self, ExprKind, StmtKind};

/// One program as the run made it (milling's or turning's).
#[derive(Clone, Debug)]
pub struct ProgIn {
    pub name: String,
    pub post: String,
    /// The posted text: None when an op it posts was refused.
    pub text: Option<String>,
    /// Why it has no text.
    pub error: Option<String>,
    /// "full", or what was not checked ("except G5 ...").
    pub certified: String,
}

/// A file to write: its path relative to the document's folder, and its text.
#[derive(Clone, Debug, PartialEq)]
pub struct NcFile {
    pub path: String,
    pub text: String,
}

/// Why a program is not exported.
#[derive(Clone, Debug, PartialEq)]
pub enum ExportError {
    /// No program of that name (the document's programs).
    NoProgram { name: String, known: Vec<String> },
    /// The document has no program(..) statement.
    NoPrograms,
    /// An op the program posts is refused: no text was made.
    Refused { program: String, why: String },
    /// Released, but not certified in full; `allow_uncertified` exports it with a stamp.
    NotCertified { program: String, certified: String },
}

impl ExportError {
    pub fn text(&self) -> String {
        match self {
            ExportError::NoProgram { name, known } => {
                format!("no program `{name}` (the document's: {})", known.join(", "))
            }
            ExportError::NoPrograms => "the document has no program(..): nothing to post".into(),
            ExportError::Refused { program, why } => format!("program `{program}` is not posted: {why}"),
            ExportError::NotCertified { program, certified } => {
                let fix = if certified.contains("no `at:`") {
                    " (fix: add `at: (x, y, z)` to the setup, the WCS origin in machine coordinates, the work offset G54 holds)"
                } else {
                    ""
                };
                format!(
                    "program `{program}` is certified {certified}{fix}; export it anyway with --allow-uncertified (its header says so)"
                )
            }
        }
    }
}

/// The file extension a dialect's programs get.
pub fn ext_of(post: &str) -> &'static str {
    match post {
        "linuxcnc" | "linuxcnc-lathe" => "ngc",
        "grbl" => "gcode",
        "siemens" => "mpf",
        "heidenhain" => "h",
        "mach3" | "mach4" => "tap",
        _ => "nc",
    }
}

/// A program statement's export settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub file: Option<String>,
    pub ext: Option<String>,
    pub out: Option<String>,
    pub comment: Option<String>,
    pub tool_list: bool,
}

fn unquote(t: &str) -> String {
    let t = t.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// A call's arguments: keyword (None: positional) and the text as written.
type Args = Vec<(Option<String>, String)>;

/// The statements `NAME = func(..)` of `src`: name, function, keyword args as written.
fn calls(src: &str) -> Vec<(String, String, Args)> {
    let Ok(prog) = syntax::parse(src) else { return vec![] };
    let mut out = vec![];
    for s in &prog {
        if let StmtKind::Assign { name, value } = &s.kind
            && let ExprKind::Call(f, args) = &value.kind
            && let ExprKind::Name(func) = &f.kind
        {
            let args = args
                .iter()
                .map(|a| {
                    (
                        a.name.as_ref().map(|(n, _)| n.clone()),
                        src[a.value.span.lo as usize..a.value.span.hi as usize].to_string(),
                    )
                })
                .collect();
            out.push((name.clone(), func.clone(), args));
        }
    }
    out
}

/// Program `name`'s export settings, from its statement in `src`.
pub fn settings(src: &str, name: &str) -> Settings {
    let mut s = Settings::default();
    for (n, _, args) in calls(src) {
        if n != name {
            continue;
        }
        for (k, v) in args {
            match k.as_deref() {
                Some("file") => s.file = Some(unquote(&v)),
                Some("ext") => s.ext = Some(unquote(&v).trim_start_matches('.').to_string()),
                Some("out") => s.out = Some(unquote(&v)),
                Some("comment") => s.comment = Some(unquote(&v)),
                Some("tools") => s.tool_list = unquote(&v) == "list",
                _ => {}
            }
        }
    }
    s
}

/// The tool list of `src`: each tool statement's T, id, holder and stick-out, and the ops using it.
pub fn tool_list(src: &str) -> Vec<String> {
    let cs = calls(src);
    let arg = |args: &[(Option<String>, String)], k: &str| {
        args.iter().find(|a| a.0.as_deref() == Some(k)).map(|a| unquote(&a.1))
    };
    cs.iter()
        .filter(|(_, f, _)| f == "tool" || f == "lathe_tool")
        .map(|(name, _, args)| {
            let id = args.iter().find(|a| a.0.is_none()).map(|a| unquote(&a.1)).unwrap_or_default();
            let ops: Vec<&str> = cs
                .iter()
                .filter(|(_, _, a)| arg(a, "tool").as_deref() == Some(name.as_str()))
                .map(|(n, _, _)| n.as_str())
                .collect();
            let mut line = format!("T{} {id}", arg(args, "t").unwrap_or_else(|| "?".into()));
            if let Some(h) = arg(args, "holder") {
                line.push_str(&format!(" in {h}"));
            }
            if let Some(so) = arg(args, "stickout") {
                line.push_str(&format!(", stick-out {so}"));
            }
            if !ops.is_empty() {
                line.push_str(&format!(": {}", ops.join(", ")));
            }
            line
        })
        .collect()
}

/// A comment line in G-code: parentheses taken out of the words.
fn comment(words: &str) -> String {
    format!("({})", words.replace(['(', ')'], ""))
}

/// `text` with `lines` of comment after its first line when that is `%`, else at its top.
fn with_header(text: &str, lines: &[String]) -> String {
    if lines.is_empty() {
        return text.to_string();
    }
    let head: String = lines.iter().map(|l| comment(l) + "\n").collect();
    match text.split_once('\n') {
        Some((first, rest)) if first.trim() == "%" => format!("{first}\n{head}{rest}"),
        _ => format!("{head}{text}"),
    }
}

/// The files of program `which` (or of every program), named and placed by their statements'
/// settings. Refused (typed) as the module doc says.
pub fn files(
    src: &str,
    progs: &[ProgIn],
    which: Option<&str>,
    allow_uncertified: bool,
) -> Result<Vec<NcFile>, ExportError> {
    if progs.is_empty() {
        return Err(ExportError::NoPrograms);
    }
    let chosen: Vec<&ProgIn> = match which {
        Some(n) => {
            let p = progs.iter().find(|p| p.name == n).ok_or_else(|| ExportError::NoProgram {
                name: n.into(),
                known: progs.iter().map(|p| p.name.clone()).collect(),
            })?;
            vec![p]
        }
        None => progs.iter().collect(),
    };
    let tools = tool_list(src);
    let mut out = vec![];
    for p in chosen {
        let Some(text) = &p.text else {
            return Err(ExportError::Refused {
                program: p.name.clone(),
                why: p.error.clone().unwrap_or_else(|| "an op it posts is refused".into()),
            });
        };
        let full = p.certified == "full";
        if !full && !allow_uncertified {
            return Err(ExportError::NotCertified {
                program: p.name.clone(),
                certified: p.certified.clone(),
            });
        }
        let s = settings(src, &p.name);
        let mut head = vec![];
        if let Some(c) = &s.comment {
            head.push(c.clone());
        }
        if !full {
            head.push(format!("NOT CERTIFIED IN FULL: {}", p.certified));
        }
        if s.tool_list {
            head.push("TOOLS".into());
            head.extend(tools.iter().cloned());
        }
        let file = format!(
            "{}.{}",
            s.file.clone().unwrap_or_else(|| p.name.clone()),
            s.ext.clone().unwrap_or_else(|| ext_of(&p.post).to_string())
        );
        let path = match &s.out {
            Some(d) if !d.is_empty() => format!("{}/{file}", d.trim_end_matches('/')),
            _ => file,
        };
        out.push(NcFile {
            path,
            text: with_header(text, &head),
        });
    }
    Ok(out)
}

impl crate::Cam {
    /// This run's programs, for `files`.
    pub fn export_programs(&self) -> Vec<ProgIn> {
        self.programs
            .iter()
            .map(|p| ProgIn {
                name: p.name.clone(),
                post: p.post.clone(),
                text: p.text.clone(),
                error: p.error.clone(),
                certified: if p.error.is_some() {
                    "no".into()
                } else if self.placed(p) {
                    "full".into()
                } else {
                    "except G5 (unchecked: no `at:`)".into()
                },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "machining
f  = part(\"p.fg\")
t1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")
op1 = setup(f)
skim = facing(op1, tool: t1)
prog = program(op1, post: \"linuxcnc\", number: 5, file: \"side-1\", out: \"nc\", comment: \"flange (first side)\", tools: \"list\")
p2 = program(op1, post: \"grbl\", number: 6)
";

    fn progs(certified: &str) -> Vec<ProgIn> {
        vec![
            ProgIn {
                name: "prog".into(),
                post: "linuxcnc".into(),
                text: Some("%\nO0005\nG0 X0\n%\n".into()),
                error: None,
                certified: certified.into(),
            },
            ProgIn {
                name: "p2".into(),
                post: "grbl".into(),
                text: None,
                error: Some("op skim refused".into()),
                certified: "no".into(),
            },
        ]
    }

    #[test]
    fn a_program_is_named_placed_and_headed_by_its_statement() {
        let f = files(SRC, &progs("full"), Some("prog"), false).expect("exported");
        assert_eq!(f[0].path, "nc/side-1.ngc");
        assert_eq!(
            f[0].text,
            "%\n(flange first side)\n(TOOLS)\n(T1 flat-10 in bt40-er32, stick-out 32: skim)\nO0005\nG0 X0\n%\n"
        );
    }

    #[test]
    fn refused_and_uncertified_programs_are_not_exported_unless_asked() {
        let e = files(SRC, &progs("full"), Some("p2"), true).unwrap_err();
        assert!(matches!(e, ExportError::Refused { .. }), "{e:?}");
        let e = files(SRC, &progs("except G5 (unchecked: no `at:`)"), Some("prog"), false).unwrap_err();
        assert!(matches!(e, ExportError::NotCertified { .. }), "{e:?}");
        let f = files(SRC, &progs("except G5 (unchecked: no `at:`)"), Some("prog"), true).expect("allowed");
        assert!(f[0].text.contains("(NOT CERTIFIED IN FULL: except G5 unchecked: no `at:`)"), "{}", f[0].text);
        assert!(matches!(files(SRC, &progs("full"), Some("nope"), false), Err(ExportError::NoProgram { .. })));
        assert_eq!(ext_of("grbl"), "gcode");
    }
}
