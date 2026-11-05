//! The host's side of the Thermoform tab: the materials Convert offers, fixes re-simulated off the
//! window's thread, and the mould's and part's exports written beside the part's file.
use super::Gui;
use crate::shell::thermo_ui::{Export, MaterialRow};
use std::sync::Mutex;

/// Re-simulated fixes, as they come back: the fix's op as JSON text, and the thinnest wall.
static RESIM: Mutex<Vec<(String, Option<f64>)>> = Mutex::new(Vec::new());

impl Gui {
    pub(super) fn thermo_host(&mut self) {
        let Some(p) = &mut self.shell.fg else { return };
        let dir = self
            .fg
            .as_ref()
            .and_then(|s| s.path.as_ref())
            .and_then(|f| std::path::Path::new(f).parent().map(|d| d.to_path_buf()));
        if p.thermo_materials.is_empty() {
            let libs = forge_material::Libraries::cached(dir.as_deref());
            p.thermo_materials = forge_thermo::shop_of(libs.all().map(|x| x.1))
                .materials
                .iter()
                .map(|m| MaterialRow {
                    name: m.name.clone(),
                    law: m.fits.first().map_or(String::new(), |f| {
                        if f.stand_in {
                            "stand-in law".into()
                        } else {
                            f.source.clone()
                        }
                    }),
                    window: m.forming,
                    normal: m.forming_temperature(),
                    fitted: m.fits.first().filter(|f| !f.stand_in).map(|f| f.temperature),
                })
                .collect();
        }
        // fixes asked for, each run through the whole part once, off this thread
        let want = std::mem::take(&mut p.thermo.want_resim);
        if let (Some(s), false) = (&self.fg, want.is_empty()) {
            let src = s.src.clone();
            let id = p.thermos.first().map(|r| r.id.clone()).unwrap_or_default();
            let mut list = RESIM.lock().unwrap_or_else(|e| e.into_inner());
            let new: Vec<_> = want
                .into_iter()
                .filter(|op| {
                    let k = op.json().to_string();
                    !list.iter().any(|x| x.0 == k)
                })
                .collect();
            if !new.is_empty() {
                list.extend(new.iter().map(|op| (op.json().to_string(), None)));
                let waker = self.waker.clone();
                std::thread::spawn(move || {
                    for op in new {
                        let min = forge_lang::edit::apply(&src, &op).ok().and_then(|text| {
                            let opts = forge_lang::Options {
                                dir: dir.clone(),
                                ..forge_lang::Options::default()
                            };
                            let run = forge_lang::run(&text, opts, Default::default()).0;
                            let o = run.thermos.iter().find(|o| o.id == id)?;
                            o.walls.as_ref().map(|w| w.min.0 * o.t)
                        });
                        let k = op.json().to_string();
                        let mut list = RESIM.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(x) = list.iter_mut().find(|x| x.0 == k) {
                            x.1 = Some(min.unwrap_or(f64::NAN));
                        }
                        drop(list);
                        waker();
                    }
                });
            }
        }
        p.thermo.resim = RESIM.lock().unwrap_or_else(|e| e.into_inner()).clone();
        // the eyes or the state changed: the part's mesh again, from what's kept
        let key = Some(p.thermo.view_key());
        if p.thermo.drawn != key {
            p.thermo.drawn = key;
            self.fg_show();
        }
        let Some(p) = &mut self.shell.fg else { return };
        // the body chosen or under the pointer in the browser changed: its marks drawn again
        let marked = Some((p.thermo.picked, p.thermo.hover));
        if p.thermo.marked != marked {
            p.thermo.marked = marked;
            if let Some(v) = &mut self.view {
                v.dirty = true;
            }
        }
        let Some(p) = &mut self.shell.fg else { return };
        if let Some((id, e)) = p.thermo.want_export.take() {
            let said = self.thermo_export(&id, e);
            if let Some(p) = &mut self.shell.fg {
                p.thermo.said = Some(said.clone());
            }
            self.shell.status = said;
        }
    }

    /// The part's mesh as the thermoform's eyes and state ask (None: as it is).
    pub(super) fn thermo_compose(&self, full: &crate::mesh::PartMesh) -> Option<crate::mesh::PartMesh> {
        let u = &self.shell.fg.as_ref()?.thermo;
        let g = self.fg_walls.first()?;
        let sim = u.simulated && g.sim.is_some();
        let hide_part = !u.show.part || sim;
        if !hide_part && u.show.mould && !u.show.blank {
            return None;
        }
        let mut hide = vec![];
        if hide_part {
            hide.extend(g.part_span.as_ref());
        }
        if !u.show.mould {
            hide.extend(g.mould_span.as_ref());
        }
        let mut extra = vec![];
        if sim && u.show.part {
            extra.extend(g.sim.as_ref());
        }
        if u.show.blank {
            extra.push(&g.blank);
        }
        Some(crate::thermo_view::compose(full, &hide, &extra))
    }

    /// Writes an export beside the part's file; says what it wrote, or why not.
    fn thermo_export(&self, id: &str, e: Export) -> String {
        let Some(g) = self.fg_walls.first() else {
            return "nothing formed to export".into();
        };
        let Some(path) = self.fg.as_ref().and_then(|s| s.path.clone()) else {
            return "save the part first: exports go beside it".into();
        };
        let stem = path.trim_end_matches(".fg").to_string();
        let (file, bytes): (String, Result<Vec<u8>, String>) = match e {
            Export::MouldStep | Export::PartStep => {
                let (body, what) = if e == Export::MouldStep {
                    (&g.mould, "mould")
                } else {
                    (&g.part, "part")
                };
                let opts = forge_step::Options {
                    name: format!("{id}-{what}"),
                    ..forge_step::Options::default()
                };
                (
                    format!("{stem}-{id}-{what}.step"),
                    forge_step::export(body, &opts)
                        .map(|x| x.text.into_bytes())
                        .map_err(|e| e.to_string()),
                )
            }
            Export::MouldStl => (
                format!("{stem}-{id}-mould.stl"),
                forge_mesh::tessellate(&g.mould, &forge_mesh::Quality::default())
                    .map(|m| forge_mesh::stl::to_binary_stl(&m, &format!("{id} mould")))
                    .map_err(|e| forge_lang::error::words(&e)),
            ),
            Export::PartStl => (
                format!("{stem}-{id}-part-simulated.stl"),
                g.sim_mesh
                    .as_ref()
                    .map(|m| forge_mesh::stl::to_binary_stl(m, &format!("{id} simulated")))
                    .ok_or_else(|| "no thickness model (walls: false)".to_string()),
            ),
            Export::TrimDxf => (format!("{stem}-{id}-trim.dxf"), Ok(dxf(&g.trim).into_bytes())),
            Export::ProcessPdf => (
                format!("{stem}-{id}-process.pdf"),
                Ok(crate::thermo_view::process_pdf(&g.sheet)),
            ),
            Export::ThicknessCsv => {
                let mut s = String::from("x_mm,y_mm,z_mm,wall_mm\n");
                for (p, t) in &g.points {
                    s += &format!("{:.3},{:.3},{:.3},{:.4}\n", p[0], p[1], p[2], t);
                }
                (format!("{stem}-{id}-thickness.csv"), Ok(s.into_bytes()))
            }
        };
        match bytes.and_then(|b| std::fs::write(&file, b).map_err(|e| e.to_string())) {
            Ok(()) => format!("Wrote {file}"),
            Err(why) => format!("Not written: {why}"),
        }
    }
}

/// The trim line as an R12 DXF: its segments as LINEs on layer TRIM.
fn dxf(pts: &[[f64; 3]]) -> String {
    let mut s = String::from("0\nSECTION\n2\nENTITIES\n");
    for k in 0..pts.len() {
        let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
        s += &format!(
            "0\nLINE\n8\nTRIM\n10\n{:.6}\n20\n{:.6}\n30\n{:.6}\n11\n{:.6}\n21\n{:.6}\n31\n{:.6}\n",
            a[0], a[1], a[2], b[0], b[1], b[2]
        );
    }
    s + "0\nENDSEC\n0\nEOF\n"
}
