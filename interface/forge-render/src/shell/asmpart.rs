//! A part's own tree inside the assembly's browser, and Inventor's in-place edit of one of its
//! features: the rest of the assembly dims, the feature's dialog opens over the view, and OK
//! writes the edit back to the part's own file (the assembly then builds again).

use super::Shell;
use super::fg::{DimRow, FeatRow, FgAct, From, parse_value, short};
use super::fgasm::{AsmPanel, PartFeats, dialog_head};
use super::theme::{medium, mono, sans};
use crate::shell::placed::Placed;
use egui::{Rect, pos2};
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::Dim;
use std::path::{Path, PathBuf};

/// Where a part file the solve names is on disk: beside the assembly, or where it says; None
/// for a standard part, which its table makes.
pub fn part_path(file: &str, dir: Option<&Path>) -> Option<PathBuf> {
    if file.starts_with(forge_lang::stdparts::KEY) {
        return None;
    }
    let p = Path::new(file);
    Some(if p.is_absolute() {
        p.to_path_buf()
    } else {
        dir.map_or_else(|| p.to_path_buf(), |d| d.join(p))
    })
}

/// A part file's text: its own, or a standard part's generated one.
pub fn part_text(file: &str, dir: Option<&Path>) -> Result<String, String> {
    match part_path(file, dir) {
        None => forge_lang::stdparts::text_of(file).ok_or(format!("{file}: no such standard part")),
        // through forge-lang's checked reader: a Forge file, or nothing of it said
        Some(p) => forge_lang::asm::read_capped(&p).ok_or_else(|| match std::fs::symlink_metadata(&p) {
            Err(_) => format!("{file}: no such file"),
            Ok(_) => format!("{file}: not a Forge file this reads"),
        }),
    }
}

/// A part file's features, read without building it (as `fg check` does): what its browser
/// lists.
pub fn read_feats(file: &str, dir: Option<&Path>) -> PartFeats {
    let text = part_text(file, dir)?;
    let opts = forge_lang::Options {
        build: false,
        dir: part_path(file, dir).and_then(|p| p.parent().map(Path::to_path_buf)),
        ..Default::default()
    };
    let run = forge_lang::run(&text, opts, Default::default()).0;
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(crate::fg::rows(&run, &text).0)
    }
    // the browser lists no part's features: their rows are made with the desktop's build
    #[cfg(target_arch = "wasm32")]
    {
        let _ = run;
        Err("a part's features are listed in the desktop app".to_string())
    }
}

/// An in-file component's rows from its file's features (`feats`): a call's (by its definition,
/// the last of the calls it is under: `#box(w=40)/box.lid(w=40)` is `box.lid`, its `box.lid.plate`
/// shown as `plate`), the document's own (`DOC`), the root's (every feature no component's and no
/// body's of `bodies`), or a body's (the features `chains` say made it, else its own).
pub fn infile_rows(
    feats: &[FeatRow],
    file: &str,
    bodies: &[&str],
    chains: &[(String, Vec<String>)],
) -> Vec<FeatRow> {
    let own = file.trim_start_matches('#');
    let own = own.rsplit('/').next().unwrap_or(own);
    let placing = |k: &str| {
        matches!(k, "component" | "instance" | "subassembly" | "fit" | "fit_rule")
            || forge_lang::asm::NAMES.contains(&k)
    };
    let comps: Vec<&str> = feats
        .iter()
        .filter(|f| f.kind == "component")
        .map(|f| f.id.as_str())
        .collect();
    let in_comp = |id: &str| id.split_once('.').is_some_and(|(c, _)| comps.contains(&c));
    match own.split_once('(') {
        Some((def, _)) => feats
            .iter()
            .filter_map(|f| {
                let rest = f.id.strip_prefix(def)?.strip_prefix('.')?;
                Some(FeatRow {
                    id: rest.to_string(),
                    ..f.clone()
                })
            })
            .collect(),
        None if file == super::fgasm::DOC => feats
            .iter()
            .filter(|f| !placing(&f.kind) && !in_comp(&f.id))
            .cloned()
            .collect(),
        None if file == forge_lang::asm::ROOT => feats
            .iter()
            .filter(|f| !placing(&f.kind) && !in_comp(&f.id) && !bodies.contains(&f.id.as_str()))
            .cloned()
            .collect(),
        // a body: every feature that made it, its sketches too
        None => {
            let chain = chains.iter().find(|c| c.0 == own).map(|c| &c.1);
            feats
                .iter()
                .filter(|f| chain.map_or(f.id == own, |c| c.contains(&f.id)))
                .cloned()
                .collect()
        }
    }
}

/// How one of the dialog's values is written back.
#[derive(Clone, Debug, PartialEq)]
pub enum Write {
    /// The param that drives it: set to the value.
    Param(ParamKey),
    /// A number written in the call as this argument: the argument set.
    Arg(String),
    /// Not one number the dialog can set, and why.
    Fixed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub what: String,
    pub dim: Dim,
    pub was: f64,
    pub text: String,
    pub write: Write,
}

/// A part feature being edited in its assembly.
#[derive(Clone, Debug)]
pub struct InPlace {
    /// The occurrence edited in, and its part file as the solve names it.
    pub occ: String,
    pub file: String,
    /// The file written back to; None for a standard part.
    pub path: Option<PathBuf>,
    pub source: String,
    pub feature: String,
    pub kind: String,
    pub fields: Vec<Field>,
    /// The feature's own tool, filled from its statement, where one writes it (Edit Feature's
    /// form): its dialog is the one shown; else the dimensions above.
    pub tool: Option<super::fgtool::FgTool>,
    /// The part's features, for the tool's dialog.
    pub feats: Vec<FeatRow>,
    /// Why the last OK was refused.
    pub error: Option<String>,
}

fn unit(d: Dim) -> &'static str {
    match d {
        Dim::Length => "mm",
        Dim::Angle => "°",
        _ => "",
    }
}

/// A number as written, with its unit if any: `90deg`, `12`.
fn plain(text: &str) -> bool {
    let t = text.trim();
    let t = ["deg", "mm"]
        .iter()
        .fold(t, |t, u| t.strip_suffix(u).unwrap_or(t));
    t.trim().parse::<f64>().is_ok()
}

/// A value as the call writes it: an angle in degrees.
fn literal_text(v: f64, d: Dim) -> String {
    if d == Dim::Angle {
        format!("{}deg", short(v))
    } else {
        short(v)
    }
}

impl InPlace {
    /// Feature `id` of occurrence `occ`'s part, its dimensions in the dialog: those a param drives
    /// set that param, a number written as one of the call's arguments sets the argument.
    pub fn open(
        occ: &str,
        file: &str,
        dir: Option<&Path>,
        feats: &[FeatRow],
        id: &str,
    ) -> Result<InPlace, String> {
        let f = feats
            .iter()
            .find(|f| f.id == id)
            .ok_or(format!("{id} is a group of features: open it and pick one"))?;
        if f.kind == "sketch" {
            return Err(format!(
                "{id} is a sketch: Edit Sketch works in the part's own window"
            ));
        }
        let path = part_path(file, dir);
        let source = part_text(file, dir)?;
        let args = super::sheet::call_args(&source, id)
            .map(|c| c.1)
            .unwrap_or_default();
        let fields = f
            .dims
            .iter()
            .map(|d: &DimRow| {
                let write = match &d.from {
                    _ if path.is_none() => Write::Fixed("a standard part's table makes it".into()),
                    From::Driving(k) => Write::Param(k.clone()),
                    // a number in the call (or its default, which SetArg writes in)
                    From::Const if args.iter().all(|a| a.0 != d.what || plain(&a.1)) => {
                        Write::Arg(d.what.clone())
                    }
                    From::Const => Write::Fixed("written as an expression".into()),
                    From::Driven(ps) => Write::Fixed(format!(
                        "driven by {}",
                        ps.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
                    )),
                };
                Field {
                    what: d.what.clone(),
                    dim: d.dim,
                    was: d.value,
                    text: short(d.value),
                    write,
                }
            })
            .collect();
        // the tool that writes the feature's kind, as the part's own Edit Feature opens it
        let tool = path.as_ref().and_then(|p| {
            let mut t = super::fgtool::FgTool::edit(id, &source).ok()?;
            super::fgdialog::offer_lists(&mut t, p.parent());
            Some(t)
        });
        Ok(InPlace {
            occ: occ.into(),
            file: file.into(),
            path,
            source,
            feature: id.into(),
            kind: f.kind.clone(),
            fields,
            tool,
            feats: feats.to_vec(),
            error: None,
        })
    }

    /// The edits the dialog's values make, as ops on the part's file.
    pub fn ops(&self) -> Result<Vec<Op>, String> {
        if let Some(t) = &self.tool {
            let text = t.text().map_err(|e| format!("{}: {}", e.kind(), e.text()))?;
            return Ok(vec![t.op(text)]);
        }
        let mut ops = vec![];
        for f in &self.fields {
            let Some(Literal { value, dim }) = parse_value(&f.text, f.dim) else {
                return Err(format!("{}: {:?} is not a {}", f.what, f.text, f.dim.name()));
            };
            if (value - f.was).abs() <= 1e-12 * value.abs().max(1.0) {
                continue;
            }
            match &f.write {
                Write::Param(k) => ops.push(Op::SetParam {
                    param: k.clone(),
                    value: Literal { value, dim },
                }),
                Write::Arg(a) => ops.push(Op::SetArg {
                    id: super::fg::statement_of(&self.feature).to_string(),
                    arg: a.clone(),
                    text: literal_text(value, dim),
                }),
                Write::Fixed(_) => {}
            }
        }
        Ok(ops)
    }

    /// The part's file as the edit leaves it, checked; written to disk.
    pub fn write(&self) -> Result<bool, String> {
        let ops = self.ops()?;
        if ops.is_empty() {
            return Ok(false);
        }
        let path = self.path.as_ref().ok_or("a standard part's table makes it")?;
        let text = forge_lang::edit::apply_all_checked(&self.source, &ops).map_err(|e| e.text())?;
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(true)
    }
}

impl AsmPanel {
    /// The features of a part occurrence's file, read the first time one of its rows opens; the
    /// browser made again with them.
    pub fn read_part(&mut self, key: &str) {
        let Some(o) = self.occ.iter().find(|o| o.path == key) else {
            return;
        };
        if self.part_feats.contains_key(&o.file) {
            return;
        }
        let file = o.file.clone();
        let feats = if file.starts_with('#') {
            Ok(self.infile_feats(&file))
        } else if let Some((owner, key)) = file.split_once("|#") {
            // a component of a subassembly file: its rows from that file's own features
            read_feats(owner, self.dir.as_deref()).map(|feats| {
                let key = format!("#{key}");
                infile_rows(&feats, &key, &[], &[])
            })
        } else {
            read_feats(&file, self.dir.as_deref())
        };
        self.part_feats.insert(file, feats);
        self.remake_tree();
    }

    /// This file's features as an in-file component's tree: an inline component's (by its
    /// definition, `peg.shank` shown as `shank`), a body's own feature, or the root's: every
    /// feature of the file that is no component's and places nothing.
    pub fn infile_feats(&self, file: &str) -> Vec<FeatRow> {
        let bodies: Vec<&str> = self
            .occ
            .iter()
            .filter(|o| o.file.starts_with('#') && !o.file.contains('(') && o.file != forge_lang::asm::ROOT)
            .map(|o| o.path.as_str())
            .collect();
        infile_rows(&self.doc_feats, file, &bodies, &self.body_chains)
    }

    /// This file's features, the part trees of its in-file components read from them again.
    pub fn set_doc_feats(&mut self, feats: Vec<FeatRow>) {
        if feats == self.doc_feats {
            return;
        }
        self.doc_feats = feats;
        let mut stale: Vec<String> = self
            .part_feats
            .keys()
            .filter(|f| f.starts_with('#'))
            .cloned()
            .collect();
        if !stale.iter().any(|f| f == super::fgasm::DOC) {
            stale.push(super::fgasm::DOC.into());
        }
        for f in &stale {
            let rows = Ok(self.infile_feats(f));
            self.part_feats.insert(f.clone(), rows);
        }
        if !stale.is_empty() {
            self.remake_tree();
        }
    }

    /// A feature row's in-place edit: its dialog up, the rest of the assembly dimmed. A
    /// feature of a component of this file is this file's: the host edits it (`host_edit`).
    pub fn edit_in_place(&mut self, key: &str) -> Result<(), String> {
        let (occ, id) = super::fgasm::feature_of(key).ok_or("not a part's feature")?;
        if occ == super::fgasm::DOC {
            self.host_edit = Some(id.to_string());
            self.selected = Some(key.to_string());
            return Ok(());
        }
        let file = self
            .occ
            .iter()
            .find(|o| o.path == occ)
            .map(|o| o.file.clone())
            .ok_or(format!("{occ}: no such occurrence"))?;
        if let Some(own) = file.strip_prefix('#') {
            self.host_edit = Some(match own.split_once('(') {
                Some((def, _)) => format!("{def}.{id}"),
                None => id.to_string(),
            });
            self.selected = Some(key.to_string());
            return Ok(());
        }
        let feats = match self.part_feats.get(&file) {
            Some(Ok(f)) => f.clone(),
            Some(Err(e)) => return Err(e.clone()),
            None => return Err(format!("{file}: not read yet")),
        };
        self.in_place = Some(InPlace::open(occ, &file, self.dir.as_deref(), &feats, id)?);
        self.selected = Some(key.to_string());
        self.reveal = Some(key.to_string());
        Ok(())
    }

    /// Whether occurrence `k` is dimmed: another than the one edited in place.
    pub fn dimmed(&self, k: usize) -> bool {
        self.in_place
            .as_ref()
            .is_some_and(|ip| self.occ[k].path != ip.occ)
    }
}

impl Shell {
    /// The in-place edit's dialog: the feature's values, OK writing them to the part's file.
    pub(super) fn in_place_window(
        ctx: &egui::Context,
        t: &super::theme::Theme,
        v: Rect,
        a: &mut AsmPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let ip = a.in_place.as_mut()?;
        if ip.tool.is_some() {
            return Self::in_place_tool(ctx, t, v, a, out, places);
        }
        let title = format!("Edit {}", super::fgdialog::title(&ip.kind));
        let (mut close, mut ok) = (false, false);
        let r = Shell::window(&title, "inplace", t, pos2(v.left() + 16.0, v.top() + 56.0), 330.0).show(
            ctx,
            |ui| {
                close = dialog_head(ui, t, super::panel::icon(&ip.kind), &title, "asm:inplace", places);
                ui.label(
                    egui::RichText::new(format!("{} · in {}", ip.feature, ip.occ))
                        .font(medium(12.0))
                        .color(t.ink),
                );
                ui.label(
                    egui::RichText::new(
                        Path::new(&ip.file)
                            .file_name()
                            .map_or(ip.file.clone(), |n| n.to_string_lossy().to_string()),
                    )
                    .font(mono(11.0))
                    .color(t.ink2),
                );
                ui.add_space(6.0);
                if ip.fields.is_empty() {
                    ui.label(
                        egui::RichText::new("No dimensions to set on this feature.")
                            .font(sans(12.0))
                            .color(t.ink2),
                    );
                }
                egui::Grid::new("inplace-fields")
                    .num_columns(3)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        for (n, f) in ip.fields.iter_mut().enumerate() {
                            ui.label(egui::RichText::new(&f.what).font(sans(12.0)).color(t.ink));
                            match &f.write {
                                Write::Fixed(why) => {
                                    ui.label(
                                        egui::RichText::new(format!("{} {}", short(f.was), unit(f.dim)))
                                            .font(sans(12.0))
                                            .color(t.ink2),
                                    )
                                    .on_hover_text(why.as_str());
                                    ui.label(
                                        egui::RichText::new(why.as_str()).font(sans(10.0)).color(t.ink2),
                                    );
                                }
                                w => {
                                    let r = ui.add(
                                        egui::TextEdit::singleline(&mut f.text)
                                            .id(egui::Id::unique(("inplace", n)))
                                            .font(sans(12.0))
                                            .desired_width(90.0),
                                    );
                                    places.push((format!("asm:inplace:field:{}", f.what), r.rect));
                                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        ok = true;
                                    }
                                    let src = match w {
                                        Write::Param(k) => k.to_string(),
                                        _ => String::new(),
                                    };
                                    ui.label(
                                        egui::RichText::new(format!("{} {src}", unit(f.dim)))
                                            .font(sans(10.0))
                                            .color(t.ink2),
                                    );
                                }
                            }
                            ui.end_row();
                        }
                    });
                if let Some(e) = &ip.error {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if Shell::fg_button(ui, "OK", "asm:inplace:ok".into(), places) {
                        ok = true;
                    }
                    if Shell::fg_button(ui, "Cancel", "asm:inplace:cancel".into(), places) {
                        close = true;
                    }
                });
            },
        );
        Self::in_place_done(a, ok, close, out);
        r.map(|r| r.response.rect)
    }

    /// The in-place edit in the feature's own tool dialog, as the part's Edit Feature has it:
    /// its values and choices, what it was made from kept (picks are the part's own view's).
    fn in_place_tool(
        ctx: &egui::Context,
        t: &super::theme::Theme,
        v: Rect,
        a: &mut AsmPanel,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> Option<Rect> {
        let ip = a.in_place.as_mut()?;
        let mut tool = ip.tool.take()?;
        let part = super::fg::FgPanel {
            file: ip.file.clone(),
            source: ip.source.clone(),
            feats: ip.feats.clone(),
            ..Default::default()
        };
        let mut asked = super::fgdialog::Asked::Nothing;
        let r = Shell::window(
            "Edit Feature",
            "inplace",
            t,
            pos2(v.left() + 16.0, v.top() + 56.0),
            300.0,
        )
        .show_placed(ctx, places, |ui, places| {
            ui.label(
                egui::RichText::new(format!("in {} · {}", ip.occ, file_name(&ip.file)))
                    .font(mono(11.0))
                    .color(t.ink2),
            );
            ui.spacing_mut().item_spacing.y = 6.0;
            asked = Shell::fg_dialog_body(ui, t, &part, &mut tool, places);
            if let Some(e) = &ip.error {
                ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.error));
            }
        });
        use super::fgdialog::Asked;
        let ok = matches!(asked, Asked::Ok | Asked::Again);
        let close = asked == Asked::Cancel;
        ip.tool = Some(tool);
        Self::in_place_done(a, ok, close, out);
        r.map(|r| r.response.rect)
    }

    /// OK writes the part's file (the assembly builds again); OK or Cancel ends the edit.
    fn in_place_done(a: &mut AsmPanel, ok: bool, mut close: bool, out: &mut Vec<FgAct>) {
        let Some(ip) = a.in_place.as_mut() else { return };
        if ok {
            match ip.write() {
                Ok(wrote) => {
                    close = true;
                    if wrote {
                        // the part's file changed: the assembly builds again, reading it
                        let file = ip.file.clone();
                        let feats = read_feats(&file, a.dir.as_deref());
                        a.part_feats.insert(file, feats);
                        a.remake_tree();
                        out.push(FgAct::AsmPreview(None));
                    }
                }
                Err(e) => ip.error = Some(e),
            }
        }
        if close {
            a.in_place = None;
            out.push(FgAct::AsmShow);
        }
    }
}

fn file_name(file: &str) -> String {
    Path::new(file)
        .file_name()
        .map_or(file.to_string(), |n| n.to_string_lossy().to_string())
}
