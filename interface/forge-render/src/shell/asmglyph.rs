//! The assembly's joints and constraints in the view, as Inventor draws them: a small glyph of
//! each one's kind where it ties its parts, the same size on screen however far, red when it's in
//! trouble. Hovering a glyph (or its browser row) lights the two parts and the two anchors it
//! ties; a click picks its row, a double click opens its Joint or Constrain dialog. As Inventor's,
//! they show when called for: the chosen component's, the dragged one's, the relationship whose
//! row is chosen or hovered; the View tab's Relationships shows them all. The eyes on the
//! Relationships rows filter either way.

use super::Shell;
use super::fgasm::{AsmPanel, Flag, kind_icon, under};
use super::icons;
use egui::{Color32, Rect, Stroke, pos2, vec2};
use forge_lang::asm::{LinkFamily, LinkOut};
use forge_math::{Frame, Point3};
use std::collections::{BTreeSet, HashSet};

/// The two parts the relationship under the pointer ties.
pub const TIED_TINT: [f32; 4] = [0.25, 0.62, 0.95, 0.0];
/// The rest of the assembly while one part's feature is edited in place.
pub const DIM_TINT: [f32; 4] = [0.36, 0.38, 0.42, 0.0];

/// The glyphs turned off: all, by kind (`Joint:revolute`), or one by one (`owner/id`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Glyphs {
    /// Show All (the View tab's Relationships): every glyph, not only those called for.
    pub all: bool,
    pub hidden: bool,
    pub kinds_off: BTreeSet<String>,
    pub off: HashSet<String>,
    /// The relationship whose browser row is under the pointer.
    pub row: Option<usize>,
}

pub fn kind_key(family: LinkFamily, kind: &str) -> String {
    format!("{family:?}:{kind}")
}

pub fn link_key(l: &LinkOut) -> String {
    format!("{}/{}", l.owner, l.id)
}

/// The relationships drawn in the view: joints and constraints.
pub fn drawn(l: &LinkOut) -> bool {
    matches!(l.family, LinkFamily::Joint | LinkFamily::Constraint)
}

impl Glyphs {
    pub fn shown(&self, l: &LinkOut) -> bool {
        !self.hidden
            && drawn(l)
            && !l.suppressed
            && !self.kinds_off.contains(&kind_key(l.family, &l.kind))
            && !self.off.contains(&link_key(l))
    }

    pub fn flip_kind(&mut self, family: LinkFamily, kind: &str) {
        let k = kind_key(family, kind);
        if !self.kinds_off.remove(&k) {
            self.kinds_off.insert(k);
        }
    }

    pub fn flip_link(&mut self, l: &LinkOut) {
        let k = link_key(l);
        if !self.off.remove(&k) {
            self.off.insert(k);
        }
    }
}

/// Where a relationship's glyph sits: its first end, else its second.
pub fn glyph_at(at: &[Option<Frame>; 2]) -> Option<Point3> {
    at[0].or(at[1]).map(|f| f.origin)
}

impl AsmPanel {
    /// The occurrences relationship `l` ties: under its two ends.
    pub fn tied(&self, l: usize) -> Vec<usize> {
        let Some(lk) = self.links.get(l) else {
            return vec![];
        };
        (0..self.occ.len())
            .filter(|&k| {
                let p = &self.occ[k].path;
                (!lk.a.0.is_empty() && under(p, &lk.a.0)) || (!lk.b.0.is_empty() && under(p, &lk.b.0))
            })
            .collect()
    }

    /// The browser row a relationship is listed by under Relationships.
    pub fn link_row_key(&self, l: usize) -> String {
        format!("l:{l}")
    }

    /// Whether relationship `l`'s glyph is called for: Show All, its row chosen or under the
    /// pointer, or it ties the chosen component or the one being dragged.
    pub fn glyph_wanted(&self, l: usize) -> bool {
        if self.glyphs.all || self.glyphs.row == Some(l) || self.hover_link == Some(l) {
            return true;
        }
        let Some(lk) = self.links.get(l) else {
            return false;
        };
        let ties = |key: &str| {
            !key.is_empty()
                && ((!lk.a.0.is_empty() && under(&lk.a.0, key))
                    || (!lk.b.0.is_empty() && under(&lk.b.0, key)))
        };
        if let Some(g) = &self.posing.grab
            && ties(super::asmdrag::top(&g.occ))
        {
            return true;
        }
        let Some(sel) = self.selected.as_deref() else {
            return false;
        };
        // a relationship's own row (under Relationships, or under a component that it holds)
        if let Some((_, k)) = sel.rsplit_once("l:") {
            return k.parse() == Ok(l);
        }
        let key = sel
            .strip_prefix("s:")
            .or_else(|| sel.strip_prefix("p:"))
            .unwrap_or(sel);
        // a feature's row of a part in the assembly: that part's
        let key = key.split_once(">f:").map_or(key, |x| x.0);
        !key.contains(':') && ties(key)
    }

    /// The relationships whose glyph is drawn, with where.
    pub fn glyph_list(&self) -> Vec<(usize, Point3)> {
        // editing a part in place: the assembly's relationships stand back
        if self.in_place.is_some() {
            return vec![];
        }
        self.links
            .iter()
            .enumerate()
            .filter(|(k, l)| self.glyphs.shown(l) && self.glyph_wanted(*k))
            .filter_map(|(k, _)| Some((k, glyph_at(self.link_at.get(k)?)?)))
            .collect()
    }
}

/// What the view's glyphs asked for this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlyphHit {
    pub hover: Option<usize>,
    pub click: Option<usize>,
    pub double: Option<usize>,
}

impl Shell {
    /// The relationships' glyphs over an assembly's view, and the anchors the one under the
    /// pointer (or whose browser row is) ties. Returns what was hovered, clicked, double clicked.
    pub(super) fn asm_link_glyphs(&mut self, ctx: &egui::Context, a: &AsmPanel) -> GlyphHit {
        let mut hit = GlyphHit::default();
        let Some((camera, size)) = self.camera else {
            return hit;
        };
        let v = self.view_rect;
        let ppp = ctx.pixels_per_point();
        let t = self.theme;
        let screen = |p: Point3| {
            camera
                .pixel_of([p.x as f32, p.y as f32, p.z as f32], size.0, size.1)
                .map(|q| pos2(v.left() + q.0 / ppp, v.top() + q.1 / ppp))
                .filter(|q| v.contains(*q))
        };
        let p = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::unique("asm-link-glyphs"),
            ))
            .with_clip_rect(v);
        let cursor = ctx.pointer_hover_pos().filter(|c| v.contains(*c));
        let mut list = a.glyph_list();
        let flag_of = |k: usize| {
            let l = &a.links[k];
            a.flags.links.get(&(l.owner.clone(), l.id.clone())).copied()
        };
        // those in trouble first, so a crowd shows its worst
        list.sort_by_key(|&(k, _)| std::cmp::Reverse(flag_of(k)));
        // one chip per patch of screen: glyphs too close to tell apart are one, with their count
        // (they come apart as the view closes in)
        let mut placed: Vec<(usize, Rect, usize)> = Vec::new();
        let mut cells: std::collections::HashMap<(i32, i32), usize> = std::collections::HashMap::new();
        for (k, at) in &list {
            let Some(q) = screen(*at) else { continue };
            let cell = ((q.x / CELL).floor() as i32, (q.y / CELL).floor() as i32);
            match cells.get(&cell) {
                Some(&i) => placed[i].2 += 1,
                None => {
                    cells.insert(cell, placed.len());
                    placed.push((*k, Rect::from_center_size(q, vec2(CHIP, CHIP)), 1));
                }
            }
        }
        let over = cursor.and_then(|c| placed.iter().rev().find(|x| x.1.contains(c)).map(|x| x.0));
        hit.hover = over;
        let lit = over.or(a.hover_link);
        let blue = Color32::from_rgb(0x40, 0x9E, 0xF2);
        for (k, r, count) in &placed {
            let l = &a.links[*k];
            let flag = flag_of(*k);
            let on = lit == Some(*k) || a.selected.as_deref() == Some(a.link_row_key(*k).as_str());
            let (fill, ink, edge) = match flag {
                Some(Flag::Error) => (t.error, Color32::WHITE, t.error),
                Some(Flag::Warning) => (
                    Color32::from_rgb(0xE0, 0xA0, 0x30),
                    Color32::BLACK,
                    Color32::BLACK,
                ),
                None if on => (blue, Color32::WHITE, Color32::WHITE),
                None => (t.surface.gamma_multiply(0.92), t.ink, t.line),
            };
            p.rect_filled(*r, 4.0, fill);
            p.rect_stroke(*r, 4.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
            icons::draw(
                &p,
                r.min + vec2(2.0, 2.0),
                CHIP - 4.0,
                kind_icon(l.family, &l.kind),
                1.25,
                ink,
            );
            if *count > 1 {
                let c = r.right_top() + vec2(-1.0, 1.0);
                let g = ctx.fonts_mut(|fo| {
                    fo.layout_no_wrap(count.to_string(), super::theme::sans(9.0), Color32::WHITE)
                });
                let bg = Rect::from_center_size(c, vec2(g.size().x + 6.0, 12.0));
                p.rect_filled(bg, 6.0, Color32::from_rgb(0x50, 0x58, 0x66));
                p.galley(bg.center() - g.size() * 0.5, g, Color32::WHITE);
            }
            self.places.push((format!("asm:glyph:{}", l.id), *r));
            if Some(*k) == over {
                self.view_blocks.push(*r);
            }
        }
        // what the lit one ties: its two anchors, a line between them, its name
        if let Some(k) = lit
            && let Some(at) = a.link_at.get(k)
        {
            let orange = Color32::from_rgb(0xF0, 0x8C, 0x28);
            let mut ends = vec![];
            for f in at.iter().flatten() {
                let Some(o) = screen(f.origin) else { continue };
                let r = camera.axes().1;
                let per_mm = screen(f.origin + forge_math::Vec3::new(r[0] as f64, r[1] as f64, r[2] as f64))
                    .map_or(1.0, |q| q.distance(o).max(1e-3));
                let reach = (22.0 / per_mm) as f64;
                // the anchor's plane, a square in its x and y, and its normal
                let corner = |sx: f64, sy: f64| {
                    screen(f.origin + f.x().get() * (sx * reach) + f.y().get() * (sy * reach))
                };
                let sq: Vec<egui::Pos2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .filter_map(|&(x, y)| corner(x, y))
                    .collect();
                if sq.len() == 4 {
                    p.add(egui::Shape::convex_polygon(
                        sq,
                        orange.gamma_multiply(0.25),
                        Stroke::new(1.5, orange),
                    ));
                }
                if let Some(e) = screen(f.origin + f.z().get() * (reach * 1.6)) {
                    p.arrow(o, e - o, Stroke::new(2.0, orange));
                }
                p.circle_filled(o, 3.5, orange);
                ends.push(o);
            }
            if let [x, y] = ends[..] {
                p.line_segment([x, y], Stroke::new(1.0, orange));
            }
            if let Some((_, r, _)) = placed.iter().find(|x| x.0 == k) {
                let l = &a.links[k];
                let label = format!("{} · {} · {} to {}", l.id, l.kind.replace('_', " "), l.a.0, l.b.0);
                let g =
                    ctx.fonts_mut(|fo| fo.layout_no_wrap(label, super::theme::sans(11.0), Color32::WHITE));
                let bg = Rect::from_min_size(r.right_top() + vec2(6.0, -2.0), g.size() + vec2(8.0, 4.0));
                p.rect_filled(bg, 3.0, Color32::from_black_alpha(190));
                p.galley(bg.min + vec2(4.0, 2.0), g, Color32::WHITE);
            }
        }
        if let Some(k) = over {
            let (click, double) = ctx.input(|i| {
                (
                    i.pointer.primary_clicked(),
                    i.pointer.button_double_clicked(egui::PointerButton::Primary),
                )
            });
            if double {
                hit.double = Some(k);
            } else if click {
                hit.click = Some(k);
            }
        }
        hit
    }
}

/// A glyph's size on screen, in points, and the patch of screen one chip stands for.
pub const CHIP: f32 = 18.0;
const CELL: f32 = 24.0;
