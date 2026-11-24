//! What the window shows at one moment, for a before/after: the shell's placed names, egui's
//! visible layers, every text drawn (where, clipped by what, how big, what colour), the status
//! line, the tab and environment, the focused widget and the pixels.

use egui::epaint::{Shape, TextShape};
use egui::{Color32, Id, LayerId, Order, Rect};
use forge_render::gui::Headless;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Text {
    pub s: String,
    pub rect: Rect,
    pub clip: Rect,
    pub size: f32,
    pub color: Color32,
    pub elided: bool,
    /// Its place in paint order (later is on top).
    pub idx: usize,
}

#[derive(Clone)]
pub struct Snap {
    pub places: BTreeMap<String, Rect>,
    pub layers: Vec<(LayerId, Rect)>,
    pub texts: Vec<Text>,
    /// Filled rectangles in paint order: a popup's background hides what was painted before it.
    pub fills: Vec<(usize, Rect)>,
    pub status: String,
    /// The status bar's line as it's drawn (the words a person reads there).
    pub status_drawn: String,
    /// The document's own names (its features', its components'): a name the person gave is
    /// no raw key.
    pub names: Vec<String>,
    pub tab: usize,
    pub env: Option<forge_render::shell::env::EnvId>,
    pub focus: Option<Id>,
    pub seq: Option<u64>,
    pub docs: usize,
    pub dirty: bool,
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
}

impl Snap {
    pub fn take(h: &mut Headless) -> Snap {
        let places: BTreeMap<String, Rect> = h.gui.shell.places.iter().cloned().collect();
        let layers = h.gui.ctx.memory(|m| {
            let mut v: Vec<(LayerId, Rect)> = m
                .areas()
                .visible_layer_ids()
                .into_iter()
                .filter_map(|l| m.area_rect(l.id).map(|r| (l, r)))
                .collect();
            v.sort_by_key(|(l, _)| (l.order, l.id.value()));
            v
        });
        let focus = h.gui.ctx.memory(|m| m.focused());
        let mut texts = Vec::new();
        let mut fills = Vec::new();
        let mut n = 0;
        if let Some(shapes) = &h.gui.audit_shapes {
            for c in shapes {
                collect(&c.shape, c.clip_rect, &mut texts, &mut fills, &mut n);
            }
        }
        let (w, hh, rgb) = ppm(&h.ppm());
        let mut snap = Snap {
            places,
            layers,
            texts,
            fills,
            status: h.gui.shell.status.clone(),
            status_drawn: String::new(),
            names: h
                .gui
                .shell
                .fg
                .as_ref()
                .map(|f| f.feats.iter().map(|x| x.id.clone()).collect())
                .unwrap_or_default(),
            tab: h.gui.shell.tab,
            env: h.gui.shell.env,
            focus,
            seq: h.gui.fg.as_ref().map(|s| s.seq),
            docs: h.gui.documents().len(),
            dirty: h.gui.shell.fg.as_ref().is_some_and(|f| f.dirty),
            w,
            h: hh,
            rgb,
        };
        // the bar's row: the text drawn in the window's last 26 points, left of its middle
        let bar = Rect::from_min_max(
            egui::pos2(0.0, snap.h as f32 - 26.0),
            egui::pos2(snap.w as f32 * 0.6, snap.h as f32),
        );
        let mut words: Vec<&Text> = snap
            .texts
            .iter()
            .filter(|t| bar.contains(t.rect.center()))
            .collect();
        words.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
        snap.status_drawn = words.iter().map(|t| t.s.as_str()).collect::<Vec<_>>().join(" ");
        snap
    }

    /// Layers shown now that weren't before, tooltips and the debug/background ones aside.
    pub fn new_layers(&self, before: &Snap) -> Vec<(LayerId, Rect)> {
        self.layers
            .iter()
            .filter(|(l, r)| {
                l.order != Order::Tooltip
                    && l.order != Order::Debug
                    && l.order != Order::Background
                    && r.area() > 16.0
                    && !before.layers.iter().any(|(b, br)| b == l && approx(*br, *r))
            })
            .copied()
            .collect()
    }

    pub fn new_places(&self, before: &Snap) -> Vec<(String, Rect)> {
        self.places
            .iter()
            .filter(|(k, _)| !before.places.contains_key(*k))
            .map(|(k, r)| (k.clone(), *r))
            .collect()
    }

    pub fn px(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.w + x) * 3;
        if i + 2 < self.rgb.len() {
            [self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]]
        } else {
            [0, 0, 0]
        }
    }

    /// The box around the pixels that changed, in `region`, and how many did.
    pub fn changed(&self, before: &Snap, region: Rect) -> (usize, Option<Rect>) {
        if self.w != before.w || self.h != before.h {
            return (self.w * self.h, Some(region));
        }
        let x0 = region.left().max(0.0) as usize;
        let y0 = region.top().max(0.0) as usize;
        let x1 = (region.right() as usize).min(self.w);
        let y1 = (region.bottom() as usize).min(self.h);
        let mut n = 0;
        let mut bb: Option<Rect> = None;
        for y in y0..y1 {
            for x in x0..x1 {
                let a = self.px(x, y);
                let b = before.px(x, y);
                let d = a
                    .iter()
                    .zip(b)
                    .map(|(p, q)| (*p as i32 - q as i32).abs())
                    .max()
                    .unwrap_or(0);
                if d > 24 {
                    n += 1;
                    let p = egui::pos2(x as f32, y as f32);
                    bb = Some(bb.map_or(Rect::from_min_max(p, p), |r| r.union(Rect::from_min_max(p, p))));
                }
            }
        }
        (n, bb)
    }

    pub fn crop_png(&self, r: Rect, max_w: usize) -> Option<Vec<u8>> {
        let x0 = r.left().max(0.0) as usize;
        let y0 = r.top().max(0.0) as usize;
        let x1 = (r.right().ceil() as usize).min(self.w);
        let y1 = (r.bottom().ceil() as usize).min(self.h);
        if x1 <= x0 + 2 || y1 <= y0 + 2 {
            return None;
        }
        // whole pixels down: every `step`th pixel, so a thumbnail stays a few kB
        let step = (x1 - x0).div_ceil(max_w).max(1);
        let (cw, ch) = ((x1 - x0).div_ceil(step), (y1 - y0).div_ceil(step));
        let mut out = Vec::with_capacity(cw * ch * 3);
        for y in (y0..y1).step_by(step) {
            for x in (x0..x1).step_by(step) {
                out.extend_from_slice(&self.px(x, y));
            }
        }
        forge_render::agent::apptools::png(cw as u32, ch as u32, &out).ok()
    }

    pub fn png(&self) -> Option<Vec<u8>> {
        forge_render::agent::apptools::png(self.w as u32, self.h as u32, &self.rgb).ok()
    }

    /// Everything a test needs to tell whether the app is back where it started.
    pub fn signature(&self) -> String {
        let layers: Vec<String> = self
            .layers
            .iter()
            .filter(|(l, _)| l.order != Order::Tooltip && l.order != Order::Debug)
            .map(|(l, _)| format!("{:?}{}", l.order, l.id.value()))
            .collect();
        let places: Vec<&str> = self
            .places
            .keys()
            .map(|k| k.as_str())
            .filter(|k| !k.starts_with("tip") && !k.starts_with("view:"))
            .collect();
        format!(
            "{:?}|{}|{:?}|{}|{}|{}|{}",
            self.env,
            self.tab,
            self.seq,
            self.docs,
            self.dirty,
            layers.join(","),
            places.join(",")
        )
    }
}

fn approx(a: Rect, b: Rect) -> bool {
    (a.min - b.min).length() < 2.0 && (a.max - b.max).length() < 2.0
}

fn collect(s: &Shape, clip: Rect, out: &mut Vec<Text>, fills: &mut Vec<(usize, Rect)>, n: &mut usize) {
    *n += 1;
    match s {
        Shape::Vec(v) => {
            for x in v {
                collect(x, clip, out, fills, n);
            }
        }
        Shape::Text(t) => out.push(text(t, clip, *n)),
        Shape::Rect(r) if r.fill.a() > 200 => fills.push((*n, r.rect.intersect(clip))),
        _ => {}
    }
}

fn text(t: &TextShape, clip: Rect, idx: usize) -> Text {
    let g = &t.galley;
    let rect = g.rect.translate(t.pos.to_vec2());
    let sec = g.job.sections.first();
    let mut color = sec.map_or(t.fallback_color, |s| s.format.color);
    if color == Color32::PLACEHOLDER {
        color = t.fallback_color;
    }
    if let Some(c) = t.override_text_color {
        color = c;
    }
    if t.opacity_factor < 1.0 {
        color = color.gamma_multiply(t.opacity_factor);
    }
    let size = g
        .job
        .sections
        .iter()
        .map(|s| s.format.font_id.size)
        .fold(f32::INFINITY, f32::min);
    Text {
        s: g.job.text.clone(),
        rect,
        clip,
        size: if size.is_finite() { size } else { 0.0 },
        color,
        elided: g.elided,
        idx,
    }
}

/// P6 bytes: width, height, the pixels.
fn ppm(b: &[u8]) -> (usize, usize, Vec<u8>) {
    let mut parts = b.splitn(4, |c| *c == b'\n');
    let _ = parts.next();
    let dims = String::from_utf8_lossy(parts.next().unwrap_or_default()).to_string();
    let mut d = dims.split(' ').filter_map(|x| x.trim().parse::<usize>().ok());
    let (w, h) = (d.next().unwrap_or(0), d.next().unwrap_or(0));
    let _ = parts.next();
    (w, h, parts.next().unwrap_or_default().to_vec())
}
