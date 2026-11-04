//! The sheet metal environment's host side: ribbon commands, a dialog's ops previewed on the part
//! and kept as one step of undo, the flat pattern shown, the DXF written with its layers.

use super::Gui;
use crate::shell::sheet::{Defaults, HwRow, SheetAct, SheetCmd, new_part_text};
use crate::shell::{FLAT_TAB, SHEET_TAB};

impl Gui {
    /// A Sheet Metal or Flat Pattern tab command.
    pub fn sheet_cmd(&mut self, c: SheetCmd) {
        let Some(fg) = &self.shell.fg else {
            self.shell.status = "Sheet metal: open or make a Forge part first".into();
            return;
        };
        if self.shell.sheet.catalogue.is_empty() {
            self.shell.sheet.catalogue = catalogue(&fg.params);
        }
        let got = self.shell.sheet.command(c, fg);
        match got {
            Ok(()) => {
                if c == SheetCmd::Convert {
                    self.shell.tab = SHEET_TAB;
                    self.shell.status = "Sheet Metal: Face makes the first wall from a sketch".into();
                }
                if c == SheetCmd::ConvertBack {
                    self.shell.tab = 0;
                }
                if c == SheetCmd::FlatPattern {
                    self.shell.tab = FLAT_TAB;
                }
                if c == SheetCmd::Folded {
                    self.shell.tab = SHEET_TAB;
                }
                if let Some(d) = &self.shell.sheet.dlg
                    && d.sent.is_none()
                {
                    if let Some(v) = &mut self.view {
                        v.nav.selected.clear();
                        v.clicked.clear();
                    }
                    if self.shell.tab != SHEET_TAB && self.shell.sheet.env {
                        self.shell.tab = SHEET_TAB;
                    }
                }
            }
            Err(why) => self.shell.status = why,
        }
    }

    /// Each frame: an edit asked from the browser, the dialog's ops previewed, and what the
    /// chrome asked done.
    pub(super) fn sheet_frame(&mut self) {
        let Some(fg) = &self.shell.fg else { return };
        // a sheet metal part opens in its environment
        let marked = fg.source.starts_with(crate::shell::sheet::PART_MARK);
        if !self.shell.sheet.env
            && !self.shell.sheet.left
            && (marked || !fg.sheets.is_empty())
            && !fg.assembly
        {
            self.shell.sheet.env = true;
            self.shell.sheet.defaults = Defaults::read(&fg.params);
            if self.shell.tab == 0 {
                self.shell.tab = SHEET_TAB;
            }
        }
        if let Some(id) = self.shell.sheet.edit_req.take() {
            let fg = self.shell.fg.as_ref().expect("checked");
            if let Err(why) = self.shell.sheet.edit(&id, fg) {
                self.shell.status = why;
            } else {
                self.shell.tab = SHEET_TAB;
            }
        }
        let fg = self.shell.fg.as_ref().expect("checked");
        let params = fg.params.clone();
        let settled = self.fg.as_ref().is_some_and(|s| !s.previewing && s.done == s.seq);
        let errors = fg
            .issues
            .iter()
            .filter(|i| i.severity == crate::shell::fg::Severity::Error)
            .count();
        self.shell.sheet.pump(&params, errors, settled);
        // OK: once the last preview is the dialog's, it's kept
        let done = self.shell.sheet.dlg.as_ref().is_some_and(|d| d.done);
        let acts = std::mem::take(&mut self.shell.sheet.out);
        for a in acts {
            match a {
                SheetAct::Preview(ops) => {
                    let Some(s) = &mut self.fg else { continue };
                    // the file stays as it is; what the ops would make is built in its place
                    let got = if ops.is_empty() {
                        s.preview(None);
                        Ok(())
                    } else {
                        let got = forge_lang::edit::apply_all_checked(&s.src, &ops).map(|_| ());
                        s.preview(Some(&ops));
                        got
                    };
                    if let Some(d) = &mut self.shell.sheet.dlg {
                        d.error = got.err().map(|e| format!("{}: {}", e.kind(), e.text()));
                    }
                    self.fg_text();
                }
                SheetAct::Commit => self.sheet_commit(),
                SheetAct::Cancel => {
                    // the dialog's already gone (or another took its place)
                    if let Some(s) = &mut self.fg
                        && s.previewing
                    {
                        s.preview(None);
                    }
                }
                SheetAct::Apply(ops) => {
                    let Some(s) = &mut self.fg else { continue };
                    self.shell.status = match s.apply_all(ops) {
                        Ok(()) => "Sheet metal defaults set".into(),
                        Err(e) => format!("refused: {}: {}", e.kind(), e.text()),
                    };
                    self.fg_text();
                }
                SheetAct::Flat(on) => {
                    if let Some(p) = &mut self.shell.fg {
                        p.flat = on;
                    }
                    self.fg_show();
                    // the flat as Inventor shows it: square on, all of it in sight
                    let frame = self.shell.sheet.flats.first().and_then(|f| f.frame);
                    match (on, frame, &mut self.view) {
                        (true, Some(f), Some(v)) => {
                            let z: forge_math::Vec3 = f.z().into();
                            let mut to = crate::nav::sheet_view(
                                v.nav.camera,
                                [z.x as f32, z.y as f32, z.z as f32],
                                &v.nav.orient,
                            );
                            let (c, radius) = v.scene.bounds();
                            to.target = c;
                            to.fit_to(radius * 1.1, v.size.0, v.size.1);
                            v.nav.go_to(to);
                        }
                        _ => self.apply_view(crate::shell::Action::FitAll),
                    }
                    if let Some(v) = &mut self.view {
                        v.mark_dirty();
                    }
                }
                SheetAct::WriteDxf(c) => {
                    let _ = self.sheet_write_dxf(&c);
                }
            }
        }
        if done {
            self.sheet_commit();
        }
    }

    /// OK: the dialog's ops, previewed or not yet, kept as one step; refused, the dialog stays.
    fn sheet_commit(&mut self) {
        let Some(mut d) = self.shell.sheet.dlg.take() else {
            return;
        };
        d.done = false;
        let params = match &d.base {
            Some(b) => b.clone(),
            None => self
                .shell
                .fg
                .as_ref()
                .map(|p| p.params.clone())
                .unwrap_or_default(),
        };
        let ops = match d.ops(&self.shell.sheet.defaults, &params) {
            Ok(ops) => ops,
            Err(why) => {
                d.error = Some(why);
                self.shell.sheet.dlg = Some(d);
                return;
            }
        };
        let Some(s) = &mut self.fg else { return };
        if let Err(e) = s.apply_all(ops) {
            d.error = Some(format!("{}: {}", e.kind(), e.text()));
            self.shell.sheet.dlg = Some(d);
            return;
        }
        let title = crate::shell::sheet::title(d.cmd).0;
        self.shell.status = match &d.edit {
            Some(id) => format!("{title}: {id} changed"),
            None => format!("{title}: {} added", d.name),
        };
        self.fg_text();
        // Apply (+): another of the same, its name the next free one
        if d.again
            && let Some(fg) = &self.shell.fg
        {
            let mut taken = crate::shell::sheet::taken(fg);
            taken.push(d.name.clone());
            let base = crate::shell::sheet::title(d.cmd).1;
            let mut k = 1;
            while taken.contains(&format!("{base}{k}"))
                || self
                    .fg
                    .as_ref()
                    .is_some_and(|s| s.src.contains(&format!("{base}{k} =")))
            {
                k += 1;
            }
            if let Some(mut n) = crate::shell::sheet::SheetDlg::new(d.cmd, &[]) {
                n.name = format!("{base}{k}");
                self.shell.sheet.dlg = Some(n);
            }
        }
    }

    /// The flat pattern as DXF, its lines on the layers chosen. Returns where it went.
    pub fn sheet_write_dxf(&mut self, c: &crate::shell::sheet::DxfChoice) -> Result<String, String> {
        let got = (|| -> Result<String, String> {
            let s = self
                .fg_sheets
                .iter()
                .find(|s| s.id == c.sheet)
                .ok_or(format!("no sheet {}", c.sheet))?;
            let p = s.pattern.as_ref().ok_or_else(|| {
                format!(
                    "{} has no flat pattern: {}",
                    c.sheet,
                    s.dxf.clone().err().unwrap_or_default()
                )
            })?;
            let [outer, inner, bend_up, bend_down, tangent, punch, form, notes] = c.all();
            let layers = forge_sheet::DxfLayers {
                outer,
                inner,
                bend_up,
                bend_down,
                tangent,
                punch,
                form,
                notes,
            };
            let text = forge_sheet::dxf_layered(p, Some(&s.note), &layers);
            // a bare name goes beside the part
            let path = std::path::Path::new(&c.path);
            let out = if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.fg
                    .as_ref()
                    .and_then(|s| s.path.as_deref())
                    .and_then(|p| std::path::Path::new(p).parent().map(|d| d.join(path)))
                    .unwrap_or_else(|| path.to_path_buf())
            };
            std::fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display()))?;
            Ok(out.display().to_string())
        })();
        self.shell.status = match &got {
            Ok(out) => format!("Wrote {out}"),
            Err(why) => why.clone(),
        };
        got
    }

    /// A new sheet metal part at `path`: its defaults written, opened in the environment.
    pub fn new_sheet_part(&mut self, path: &str) -> Result<(), String> {
        let path = if path.ends_with(".fg") {
            path.to_string()
        } else {
            format!("{path}.fg")
        };
        std::fs::write(&path, new_part_text(&Defaults::default())).map_err(|e| format!("{path}: {e}"))?;
        self.open(&path);
        self.shell.sheet.env = true;
        self.shell.tab = SHEET_TAB;
        Ok(())
    }
}

/// Insert Hardware's catalogue: the rows a shop writes in the part (`hardware(kind: .., size: ..,
/// hole: .., edge: .., code: .., source: ..)`), else forge-sheet's placeholder rows.
pub fn catalogue(params: &[crate::shell::fg::ParamRow]) -> Vec<HwRow> {
    let mut rows = vec![];
    for p in params {
        for part in p.text.split("hardware(").skip(1) {
            let field = |k: &str| {
                let at = part.find(&format!("{k}:"))? + k.len() + 1;
                let rest = part[at..].trim_start();
                let end = if let Some(r) = rest.strip_prefix('"') {
                    return r.find('"').map(|e| r[..e].to_string());
                } else {
                    rest.find([',', ')']).unwrap_or(rest.len())
                };
                Some(rest[..end].trim().to_string())
            };
            let (Some(kind), Some(size)) = (field("kind"), field("size")) else {
                continue;
            };
            let n = |k: &str| field(k).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            rows.push(HwRow {
                kind,
                size,
                hole: n("hole"),
                edge: n("edge"),
                code: field("code").unwrap_or_default(),
                source: field("source"),
            });
        }
    }
    if rows.is_empty() {
        rows = forge_sheet::placeholder_catalogue()
            .into_iter()
            .map(|h| HwRow {
                kind: h.kind.name().into(),
                size: h.size,
                hole: h.hole,
                edge: h.edge,
                code: h.code,
                source: None,
            })
            .collect();
    }
    rows
}
