//! The View tab's Visual Style button and its list, Inventor's: each style a small cube drawn the
//! way the style draws parts.

use super::ribbon::Tool;
use super::theme::sans;
use super::{Action, Shell};
use crate::render::VisualStyle;
use egui::{Align2, Color32, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2};

pub(super) const BUTTON: &str = "Visual Style";
const ROW_H: f32 = 30.0;
const LIST_W: f32 = 270.0;

fn open_id() -> Id {
    Id::unique("visual style list open")
}

/// A unit cube's corners seen from above the front right, and its edges: the three at the far
/// corner (0) are the hidden ones.
const CORNERS: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [-1.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
];
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (0, 2),
    (0, 4),
    (1, 3),
    (1, 5),
    (2, 3),
    (2, 6),
    (3, 7),
    (4, 5),
    (4, 6),
    (5, 7),
    (6, 7),
];
/// Top, right, left: lightest to darkest.
const FACES: [[usize; 4]; 3] = [[4, 5, 7, 6], [1, 3, 7, 5], [2, 3, 7, 6]];

fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// A style's picture in the square at `at`, `side` wide; `ink` for its edges.
pub(super) fn icon(p: &Painter, at: Pos2, side: f32, style: VisualStyle, ink: Color32) {
    let k = side * 0.24;
    let c = at + vec2(side * 0.5, side * 0.52);
    let to = |q: [f32; 3]| c + vec2((q[0] - q[1]) * 0.866, (q[0] + q[1]) * 0.5 - q[2]) * k;
    let fills: Option<[u32; 3]> = match style {
        VisualStyle::Realistic => Some([0xD3DCE6, 0x8D9DB0, 0x5A6879]),
        VisualStyle::Monochrome => Some([0xE2E2E2, 0xA9A9A9, 0x727272]),
        VisualStyle::Illustration => Some([0xFBF7EE, 0xF1E6CF, 0xD9C7A3]),
        s if s.wireframe() => None,
        _ => Some([0xC9CED6, 0x98A1AD, 0x6A7380]),
    };
    if style == VisualStyle::Realistic {
        let floor = Rect::from_center_size(c + vec2(0.0, k * 1.55), vec2(k * 3.6, k * 1.1));
        p.add(Shape::ellipse_filled(
            floor.center(),
            floor.size() * 0.5,
            Color32::from_black_alpha(70),
        ));
    }
    if let Some(f) = fills {
        for (face, colour) in FACES.iter().zip(f) {
            let pts = face.iter().map(|&i| to(CORNERS[i])).collect();
            p.add(Shape::convex_polygon(pts, rgb(colour), Stroke::NONE));
        }
    }
    if style == VisualStyle::Realistic {
        // a highlight across the top
        p.line_segment(
            [to([-0.6, -1.0, 1.0]), to([1.0, 0.4, 1.0])],
            Stroke::new(1.5, Color32::from_white_alpha(170)),
        );
    }
    let (seen, behind) = match style {
        VisualStyle::Realistic | VisualStyle::Shaded => (false, false),
        VisualStyle::ShadedHidden | VisualStyle::WireframeHidden => (true, true),
        VisualStyle::Wireframe => (true, true),
        _ => (true, false),
    };
    let width = if style == VisualStyle::Illustration {
        1.8
    } else {
        1.1
    };
    let edge = if fills.is_some() && ink.r() > 128 && style != VisualStyle::Wireframe {
        // a light theme's ink on light faces reads; a dark theme's light ink on them doesn't
        rgb(0x15191E)
    } else {
        ink
    };
    for &(a, b) in &EDGES {
        let (pa, pb) = (to(CORNERS[a]), to(CORNERS[b]));
        if a == 0 {
            if !behind {
                continue;
            }
            if style == VisualStyle::Wireframe {
                p.line_segment([pa, pb], Stroke::new(width, edge));
            } else {
                p.extend(Shape::dashed_line(
                    &[pa, pb],
                    Stroke::new(1.0, edge.gamma_multiply(0.7)),
                    2.5,
                    2.0,
                ));
            }
        } else if seen {
            p.line_segment([pa, pb], Stroke::new(width, edge));
        }
    }
}

/// The View tab's ground and shadow switches, Inventor's Appearance panel's.
pub(super) fn ground_tools(s: &super::settings::Settings) -> Vec<Tool> {
    use crate::look::Toggle as G;
    [
        ("Ground Plane", "groundplane", G::GroundPlane),
        ("Reflections", "groundreflect", G::Reflections),
        ("Ground Shadows", "groundshadow", G::GroundShadows),
        ("Object Shadows", "objshadow", G::ObjectShadows),
        ("Ambient Shadows", "ambientshadow", G::AmbientShadows),
    ]
    .into_iter()
    .map(|(name, icon, g)| Tool {
        name,
        icon,
        action: Some(Action::Look(g)),
        on: s.toggle(g),
    })
    .collect()
}

impl Shell {
    /// The ribbon's Visual Style button: the style it's in, and a list of them below on a click.
    pub(super) fn style_button(&mut self, ui: &mut Ui, tool: &Tool, x: f32, y: f32, w: f32) {
        let was_open = ui.data(|d| d.get_temp::<bool>(open_id())).unwrap_or(false);
        let r = Rect::from_min_size(pos2(x, y), vec2(w - 2.0, 72.0));
        // pressed while its list is open: known only after this frame's click, so its fill goes
        // in a slot under the rest
        let fill = ui.painter().add(Shape::Noop);
        let (live_ink, resp) = self.tool_response(ui, tool, r);
        let mut open = was_open ^ resp.clicked();
        if open {
            open = self.style_list(ui, pos2(r.left(), r.bottom() + 2.0), &resp);
        }
        let t = self.theme;
        let ink = if open { t.on_ink } else { live_ink };
        if open {
            ui.painter().set(fill, Shape::rect_filled(r, 4.0, t.on));
        }
        let style = self.view.style;
        let p = ui.painter();
        icon(p, pos2(r.center().x - 15.0, y + 4.0), 30.0, style, ink);
        p.text(
            pos2(r.center().x, y + 43.0),
            Align2::CENTER_CENTER,
            "Visual",
            sans(12.0),
            ink,
        );
        p.text(
            pos2(r.center().x - 4.0, y + 58.0),
            Align2::CENTER_CENTER,
            "Style",
            sans(12.0),
            ink,
        );
        let tip = pos2(r.center().x + 18.0, y + 58.0);
        p.add(Shape::convex_polygon(
            vec![
                tip + vec2(-3.5, -2.0),
                tip + vec2(3.5, -2.0),
                tip + vec2(0.0, 2.0),
            ],
            ink,
            Stroke::NONE,
        ));
        ui.data_mut(|d| d.insert_temp(open_id(), open));
    }

    /// The list under the button; whether it stays open.
    fn style_list(&mut self, ui: &mut Ui, at: Pos2, button: &egui::Response) -> bool {
        let t = self.theme;
        let current = self.view.style;
        let mut chosen = None;
        let mut rows = Vec::new();
        let area = egui::Area::new(Id::unique("visual style list"))
            .order(egui::Order::Tooltip)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                let h = ROW_H * VisualStyle::ALL.len() as f32 + 8.0;
                let (frame, _) = ui.allocate_exact_size(vec2(LIST_W, h), Sense::hover());
                let p = ui.painter();
                p.rect_filled(
                    frame.translate(vec2(0.0, 3.0)),
                    6.0,
                    Color32::from_black_alpha(50),
                );
                p.rect_filled(frame, 6.0, t.raised.to_opaque());
                p.rect_stroke(frame, 6.0, Stroke::new(1.0, t.line), egui::StrokeKind::Inside);
                for (k, s) in VisualStyle::ALL.into_iter().enumerate() {
                    let row = Rect::from_min_size(
                        frame.min + vec2(4.0, 4.0 + k as f32 * ROW_H),
                        vec2(LIST_W - 8.0, ROW_H),
                    );
                    let resp = ui.interact(row, Id::unique(("visual style", s.key())), Sense::click());
                    let on = s == current;
                    let p = ui.painter();
                    if on {
                        p.rect_filled(row, 4.0, t.on);
                    } else if resp.hovered() {
                        p.rect_filled(row, 4.0, t.hover);
                    }
                    let ink = if on { t.on_ink } else { t.ink };
                    icon(p, row.min + vec2(4.0, 2.0), 26.0, s, ink);
                    p.text(
                        pos2(row.min.x + 40.0, row.center().y),
                        Align2::LEFT_CENTER,
                        s.name(),
                        sans(12.0),
                        ink,
                    );
                    if resp.clicked() {
                        chosen = Some(s);
                    }
                    rows.push((format!("style:{}", s.name()), row));
                }
            });
        self.places.extend(rows);
        if let Some(s) = chosen {
            self.out.push(Action::VisualStyle(s));
            return false;
        }
        let elsewhere = ui.input(|i| i.pointer.any_click()) && !area.response.hovered() && !button.hovered();
        !(elsewhere || ui.input(|i| i.key_pressed(egui::Key::Escape)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_has_its_own_word_and_comes_back_from_it() {
        let keys: std::collections::HashSet<&str> = VisualStyle::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(keys.len(), VisualStyle::ALL.len());
        for s in VisualStyle::ALL {
            assert_eq!(VisualStyle::from_key(s.key()), Some(s));
        }
        assert_eq!(VisualStyle::default(), VisualStyle::ShadedEdges);
    }
}
