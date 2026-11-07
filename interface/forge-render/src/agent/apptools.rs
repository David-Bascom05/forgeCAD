//! What the agent can do in the app itself, not the file: the view, visibility, selection, the
//! thermoform analysis, a joint driven for a look, a picture of the view. Both backends call the
//! same tools: Claude Code through ForgeCAD's MCP server (`mcp.rs`), the OpenAI-compatible loop
//! directly. A call goes to the frame's thread and waits for its answer there.

use super::Waker;
use forge_agent::json::Json;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

/// A tool's answer: its words, a PNG when it's a picture, whether it failed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reply {
    pub text: String,
    pub png: Option<Vec<u8>>,
    pub error: bool,
}

impl Reply {
    pub fn ok(text: impl Into<String>) -> Reply {
        Reply {
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn err(text: impl Into<String>) -> Reply {
        Reply {
            text: text.into(),
            error: true,
            ..Default::default()
        }
    }
}

pub struct AppCall {
    pub name: String,
    pub args: Json,
    pub reply: Sender<Reply>,
}

/// The way to the frame's thread, cloned into each backend.
#[derive(Clone)]
pub struct AppTools {
    tx: Sender<AppCall>,
    wake: Waker,
    /// The open document's folder: where the fg tools may look.
    dir: std::sync::Arc<std::sync::Mutex<Option<std::path::PathBuf>>>,
}

impl AppTools {
    pub fn new(wake: Waker) -> (AppTools, Receiver<AppCall>) {
        let (tx, rx) = channel();
        (
            AppTools {
                tx,
                wake,
                dir: Default::default(),
            },
            rx,
        )
    }

    pub fn set_dir(&self, dir: &std::path::Path) {
        *self.dir.lock().expect("no panics while held") = Some(dir.to_path_buf());
    }

    /// Runs a tool on the frame's thread and waits for its answer (a picture takes a frame).
    pub fn call(&self, name: &str, args: Json) -> Reply {
        if !NAMES.iter().any(|n| n.0 == name) {
            return Reply::err(format!("no app tool `{name}`"));
        }
        // the fg tools build on the caller's thread, never the frame's
        if name.starts_with("fg_") {
            let Some(dir) = self.dir.lock().expect("no panics while held").clone() else {
                return Reply::err("no document is open");
            };
            let (text, error) = super::tools::fg_call(name, &args, &dir);
            return Reply {
                text,
                png: None,
                error,
            };
        }
        let (reply, got) = channel();
        if self
            .tx
            .send(AppCall {
                name: name.to_string(),
                args,
                reply,
            })
            .is_err()
        {
            return Reply::err("the app has closed");
        }
        (self.wake)();
        got.recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| Reply::err("the app didn't answer in 30 s"))
    }
}

/// Each tool: its name, what it does, its parameters (JSON Schema properties) and the required ones.
pub const NAMES: &[(&str, &str, &str, &[&str])] = &[
    (
        "tree",
        "The browser of the open document: each row's id, kind and whether it's shown. An assembly lists its top-level parts, subassemblies and patterns; `filter` lists every row (at any depth) whose id or name has it; `relationships: true` adds joints and constraints.",
        r#"{"filter":{"type":"string"},"relationships":{"type":"boolean"}}"#,
        &[],
    ),
    (
        "harness",
        "The open assembly's wire harnesses as JSON: connectors and pins, nodes, segments (length, bundle and packed diameter, minimum bend), every wire (from, to, colour code, type, path, cut length and its terms, resistance, current, voltage drop), every wire type with all its fields (conductor, gauge or mm², strands, ohm/km, insulation, wall, OD, colour, temperature, voltage, spec and spec tag, maker, MPN, our part number, source) and the BOM. Edit them with fg_edit: wire_type(..), wire(.., colour:, current:) lines in the harness block.",
        "{}",
        &[],
    ),
    (
        "wire_library",
        "The wire library's rows beside the open document (its wires/*.csv, then ~/.config/forgecad/wires/*.csv): each row's name and fields. `search` keeps the rows any of whose fields has it. A row is used in .fg as wire_type(\"NAME\").",
        r#"{"search":{"type":"string"}}"#,
        &[],
    ),
    (
        "set_visibility",
        "Show or hide rows by id: occurrences (an assembly's), the part's solid (`solid`), sketches and work planes. One view step.",
        r#"{"ids":{"type":"array","items":{"type":"string"}},"visible":{"type":"boolean"}}"#,
        &["ids", "visible"],
    ),
    (
        "isolate",
        "Show only this occurrence (and what's under it). One view step.",
        r#"{"id":{"type":"string"}}"#,
        &["id"],
    ),
    (
        "show_all",
        "Show everything hidden or isolated. One view step.",
        "{}",
        &[],
    ),
    (
        "set_overlay",
        "Switch one of the view's overlays: axes, grid, planes (origin planes, sketch and work planes), anchors (relationship glyphs and anchors). One view step.",
        r#"{"name":{"type":"string","enum":["axes","grid","planes","anchors"]},"on":{"type":"boolean"}}"#,
        &["name", "on"],
    ),
    (
        "select",
        "Select a row by id (a feature, an occurrence, a drawing item), as a click in the browser does. One view step.",
        r#"{"id":{"type":"string"}}"#,
        &["id"],
    ),
    ("clear_selection", "Select nothing. One view step.", "{}", &[]),
    ("get_selection", "What's selected in the app, by id.", "{}", &[]),
    (
        "set_view",
        "Turn the camera to a named view and fit the model: iso, front, back, left, right, top, bottom, home, fit. One view step.",
        r#"{"view":{"type":"string","enum":["iso","front","back","left","right","top","bottom","home","fit"]}}"#,
        &["view"],
    ),
    (
        "visual_style",
        "The view's visual style, as the View tab's: realistic, shaded, shaded-edges (the default), shaded-hidden, wireframe, wireframe-hidden, wireframe-visible, monochrome, illustration. One view step.",
        r#"{"style":{"type":"string","enum":["realistic","shaded","shaded-edges","shaded-hidden","wireframe","wireframe-hidden","wireframe-visible","monochrome","illustration"]}}"#,
        &["style"],
    ),
    (
        "section",
        "The view's section plane on or off. One view step.",
        r#"{"on":{"type":"boolean"}}"#,
        &["on"],
    ),
    (
        "thermo_view",
        "A thermoformed part's view, as its browser rows and Thermoform tab set it: which bodies show (part = the formed part, mould, blank), the wall as simulated or as modelled, the analysis colours (none, draft, thickness: the simulated wall's thickness, stretch: the larger principal stretch, webbing: where the sheet gathers and folds), and Check Forming open or closed. Give only what changes. One view step.",
        r#"{"analysis":{"type":"string","enum":["none","draft","thickness","stretch","webbing"]},"part":{"type":"boolean"},"mould":{"type":"boolean"},"blank":{"type":"boolean"},"simulated":{"type":"boolean"},"check":{"type":"boolean"}}"#,
        &[],
    ),
    (
        "thermo_report",
        "A thermoformed part's numbers as Check Forming shows them (thinnest and thickest wall, the material's limit, the forming facts) and what of it shows now.",
        "{}",
        &[],
    ),
    (
        "measure",
        "Measure, as Inspect > Measure does: one pick (its length, area, radius..) or two (distance, angle, delta X/Y/Z..). A pick is a face or edge by its Forge anchor (extrude1.end, edges(between: (A, B))), `solid`, an origin plane (xy, xz, yz), or in an assembly an occurrence's browser key. Says the values and what it draws in the model.",
        r#"{"a":{"type":"string"},"b":{"type":"string"}}"#,
        &["a"],
    ),
    (
        "drive_joint",
        "Move a joint for a look, the file kept as it is: its driving value (degrees for a turn, mm for a slide). For 'open the door', 'move X', 'show it open'. One view step.",
        r#"{"joint":{"type":"string"},"value":{"type":"number"},"what":{"type":"string","description":"which of the joint's values, when it drives more than one"}}"#,
        &["joint", "value"],
    ),
    (
        "return_to_design",
        "Put driven joints back where the file has them.",
        "{}",
        &[],
    ),
    (
        "keep_position",
        "Write the driven joints' values into the file, as one undo step: only when the person asks to keep the position.",
        "{}",
        &[],
    ),
    (
        "open_document",
        "Open another .fg file in the folder in the app (absolute path).",
        r#"{"path":{"type":"string"}}"#,
        &["path"],
    ),
    (
        "screenshot",
        "A picture of the view as the person sees it now.",
        "{}",
        &[],
    ),
    (
        "undo_view",
        "Take back the last view step an app tool made.",
        "{}",
        &[],
    ),
    (
        "fg_run",
        "Build a .fg file in the open document's folder and report its errors, warnings and measures (as `fg run`); a drawing: its views, annotations and problems. `file` is its name in the folder.",
        r#"{"file":{"type":"string"}}"#,
        &["file"],
    ),
    (
        "fg_check",
        "Parse and check a .fg file in the folder without building it: rules, types, dimensions.",
        r#"{"file":{"type":"string"}}"#,
        &["file"],
    ),
    (
        "fg_edit",
        "Apply a typed Forge edit op (as the check's fixes give them) to a .fg file in the folder, then build and check it; refused if the result wouldn't check.",
        r#"{"file":{"type":"string"},"op":{"type":"object"}}"#,
        &["file", "op"],
    ),
    (
        "fg_handles",
        "Every parameter of a .fg file in the folder: its value and dimension.",
        r#"{"file":{"type":"string"}}"#,
        &["file"],
    ),
    (
        "fg_query",
        "Numbers from a .fg part or assembly in the folder, built: `holes` (each hole's centre, axis, diameter), `section z=67` (the outline at a plane, as lines and arcs), `aligned` (holes of different parts on one axis; `aligned A B` for two), `gap A B` (between two faces), `interference` (where parts clash, the faces involved), `frame SKETCH` (where a sketch on a sheet wall lands, folded and on the flat), `faces` (every face's name). Assembly coordinates, `in OCC` for a part's; names in an assembly are OCC/NAME.",
        r#"{"file":{"type":"string"},"query":{"type":"string"}}"#,
        &["file", "query"],
    ),
    ("fg_reference", "The Forge language reference.", "{}", &[]),
];

/// The tools as JSON Schema objects: `(name, description, inputSchema)`.
pub fn schemas() -> Vec<(String, String, Json)> {
    NAMES
        .iter()
        .map(|(name, what, props, required)| {
            let schema = Json::obj(vec![
                ("type", "object".into()),
                (
                    "properties",
                    Json::parse(props).expect("a tool's properties are JSON"),
                ),
                (
                    "required",
                    Json::Arr(required.iter().map(|r| Json::from(*r)).collect()),
                ),
            ]);
            (name.to_string(), what.to_string(), schema)
        })
        .collect()
}

/// The tools as chat completions' `tools` take them, prefixed `app_`.
pub fn openai_specs() -> Vec<Json> {
    schemas()
        .into_iter()
        // its own file tools do the fg work
        .filter(|(name, _, _)| !name.starts_with("fg_"))
        .map(|(name, what, schema)| {
            Json::obj(vec![
                ("type", "function".into()),
                (
                    "function",
                    Json::obj(vec![
                        ("name", format!("app_{name}").into()),
                        ("description", what.into()),
                        ("parameters", schema),
                    ]),
                ),
            ])
        })
        .collect()
}

/// Base64, for a picture in a JSON reply.
pub fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        o.push(A[(n >> 18) as usize & 63] as char);
        o.push(A[(n >> 12) as usize & 63] as char);
        o.push(if c.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        o.push(if c.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    o
}

/// RGB rows as a PNG.
pub fn png(w: u32, h: u32, rgb: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut e = png::Encoder::new(&mut out, w, h);
    e.set_color(png::ColorType::Rgb);
    e.set_depth(png::BitDepth::Eight);
    let mut w = e.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgb).map_err(|e| e.to_string())?;
    drop(w);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_as_the_rfc_has_it() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(schemas().len(), NAMES.len());
    }
}
