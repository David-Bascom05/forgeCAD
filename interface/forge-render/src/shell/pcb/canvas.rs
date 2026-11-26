use super::*;
use crate::shell::{Theme, theme};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use forge_lang::pcb::{Film, PairRole, Shape};
use forge_math::Point2;

type Triangles = Vec<[Point; 3]>;
fn triangles(paths: &[&[Point]]) -> Result<Triangles, String> {
    let mut points = Vec::new();
    let mut contours = vec![];
    for path in paths {
        let first = points.len() as u32;
        points.extend(path.iter().map(|&[x, y]| Point2::new(x as f64, y as f64)));
        contours.push((first..points.len() as u32).collect())
    }
    let mesh = forge_mesh::cdt::triangulate(&points, &contours, &[])
        .map_err(|e| format!("Board display triangulation: {e:?}"))?;
    Ok(mesh
        .triangles
        .iter()
        .map(|t| {
            t.map(|i| {
                let p = mesh.points[i as usize];
                [p.x.round() as i64, p.y.round() as i64]
            })
        })
        .collect())
}
enum Geometry {
    Stroke([Point; 2], i64),
    Fill(Triangles),
}
struct Object {
    id: String,
    geometry: Geometry,
    pad: bool,
    pour: bool,
}
pub(super) struct SceneLayer {
    pub layer: PlotLayer,
    pub default_visible: bool,
    objects: Vec<Object>,
}
impl SceneLayer {
    pub(super) fn count(&self) -> usize {
        self.objects.len()
    }
}
pub(super) struct Scene {
    pub layers: Vec<SceneLayer>,
    body: Triangles,
    pub(super) bounds: [Point; 2],
}
impl Scene {
    pub(super) fn new(b: &Board) -> Result<Self, String> {
        let mut paths = vec![b.outline.as_slice()];
        paths.extend(b.layout.cutouts.values().map(Vec::as_slice));
        let body = triangles(&paths)?;
        let bounds = Shape::Region(b.outline.clone())
            .bounds2()
            .map(|p| p.map(|v| v / 2));
        let mut layers = vec![];
        for layer in b.plot_layers() {
            let mut objects = vec![];
            for m in b.plot(&layer) {
                let geometry = match m.shape {
                    Shape::Stroke { axis, width } => Geometry::Stroke(axis, width),
                    Shape::Region(ps) if ps.len() == 3 => Geometry::Fill(vec![[ps[0], ps[1], ps[2]]]),
                    Shape::Region(ps) => Geometry::Fill(triangles(&[&ps])?),
                };
                objects.push(Object {
                    id: m.id.into(),
                    geometry,
                    pour: b.layout.fills.contains_key(m.id),
                    pad: b.layout.pads.contains_key(m.id) || b.vias.contains_key(m.id),
                });
            }
            let default_visible = match &layer {
                PlotLayer::Copper(id) => {
                    b.layers.first().is_some_and(|l| l.id == *id)
                        || b.layers.last().is_some_and(|l| l.id == *id)
                }
                PlotLayer::Film(_, Film::Silk)
                | PlotLayer::Profile
                | PlotLayer::PlatedDrill
                | PlotLayer::NonPlatedDrill
                | PlotLayer::Courtyard(_)
                | PlotLayer::Keepout
                | PlotLayer::Airwires => true,
                _ => false,
            };
            layers.push(SceneLayer {
                layer,
                objects,
                default_visible,
            });
        }
        Ok(Self { layers, body, bounds })
    }
}
pub(super) fn color(layer: &PlotLayer, board: &Board, t: &Theme) -> Color32 {
    let rgb = Color32::from_rgb;
    match layer {
        PlotLayer::Copper(id) if board.layers.first().is_some_and(|l| l.id == *id) => rgb(224, 107, 72),
        PlotLayer::Copper(id) if board.layers.last().is_some_and(|l| l.id == *id) => rgb(84, 157, 222),
        PlotLayer::Copper(_) => rgb(174, 121, 202),
        PlotLayer::Film(_, Film::Silk) => rgb(229, 229, 200),
        PlotLayer::Film(_, Film::Mask) => rgb(80, 189, 129),
        PlotLayer::Film(_, Film::Paste) => rgb(175, 187, 201),
        PlotLayer::Courtyard(_) => rgb(97, 171, 168),
        PlotLayer::Profile => rgb(214, 194, 119),
        PlotLayer::Airwires => rgb(237, 206, 120),
        PlotLayer::Keepout => rgb(220, 141, 180),
        PlotLayer::Paired(_, PairRole::Adhesive) => rgb(214, 138, 58),
        PlotLayer::PlatedDrill | PlotLayer::NonPlatedDrill => rgb(105, 116, 123),
        _ => {
            if t.dark {
                rgb(151, 160, 193)
            } else {
                rgb(105, 114, 159)
            }
        }
    }
}
fn world(p: Point) -> Vec2 {
    vec2(mm(p[0]) as f32, mm(p[1]) as f32)
}
fn fill(painter: &egui::Painter, tris: &Triangles, screen: &impl Fn(Point) -> Pos2, color: Color32) {
    let mut mesh = egui::Mesh::default();
    for tri in tris {
        let start = mesh.vertices.len() as u32;
        for &p in tri {
            mesh.colored_vertex(screen(p), color)
        }
        mesh.add_triangle(start, start + 1, start + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
}
impl Editor {
    pub(super) fn canvas(&mut self, ui: &mut Ui, r: Rect, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let response = ui.interact(r, ui.scope_id().with("pcb-canvas"), Sense::click_and_drag());
        let painter = ui.painter_at(r);
        let bg = if t.dark {
            Color32::from_rgb(19, 25, 31)
        } else {
            Color32::from_rgb(221, 228, 231)
        };
        painter.rect_filled(r, 0., bg);
        if r.width() < 10. || r.height() < 10. {
            return;
        }
        if self.fit {
            let [a, b] = self.rev.scene.bounds.map(world);
            let size = b - a;
            self.zoom = (r.width() / (size.x + 4.)).min(r.height() / (size.y + 4.)) * 0.86;
            self.center = (a + b) * 0.5;
            self.fit = false;
        }
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            let d = response.drag_delta();
            self.center -= vec2(d.x, -d.y) / self.zoom;
        }
        let anchor = r.center();
        if response.hovered()
            && let Some(p) = response.hover_pos()
        {
            let d = ui.input(|i| i.smooth_scroll_delta.y);
            if d != 0. {
                let old = self.zoom;
                self.zoom = (old * (d * 0.003).exp()).clamp(0.05, 15000.);
                self.center += vec2(p.x - anchor.x, anchor.y - p.y) * (1. / old - 1. / self.zoom);
            }
        }
        let center = self.center;
        let zoom = self.zoom;
        let screen = |p: Point| {
            let d = (world(p) - center) * zoom;
            anchor + vec2(d.x, -d.y)
        };
        let snap = |p: Pos2| {
            let d = (p - anchor) / zoom;
            let v = center + vec2(d.x, -d.y);
            [v.x, v.y].map(|n| {
                let nm = f64::from(n) * 1e6;
                if self.snap {
                    ((nm / (self.pitch * 1e6)).round() * self.pitch * 1e6).round() as i64
                } else {
                    nm.round() as i64
                }
            })
        };
        let under = response.hover_pos().map(snap);
        self.cursor = under;
        let substrate = if t.dark {
            Color32::from_rgb(37, 63, 59)
        } else {
            Color32::from_rgb(44, 77, 70)
        };
        fill(&painter, &self.rev.scene.body, &screen, substrate);
        if self.grid {
            let mut step = self.pitch as f32 * zoom;
            while step < 14. {
                step *= 2.
            }
            let origin = screen([0, 0]);
            let x0 = r.left() + (origin.x - r.left()).rem_euclid(step);
            let mut y = r.top() + (origin.y - r.top()).rem_euclid(step);
            let c = if t.dark {
                Color32::from_white_alpha(40)
            } else {
                Color32::from_black_alpha(40)
            };
            while y < r.bottom() {
                let mut x = x0;
                while x < r.right() {
                    painter.circle_filled(pos2(x, y), 0.75, c);
                    x += step;
                }
                y += step;
            }
        }
        let rank = |l: &PlotLayer| match l {
            PlotLayer::PlatedDrill | PlotLayer::NonPlatedDrill => 7,
            _ if *l == self.active => 5,
            PlotLayer::Copper(_) => 1,
            PlotLayer::Film(_, Film::Silk) => 4,
            _ => 3,
        };
        let mut order: Vec<_> = self
            .rev
            .scene
            .layers
            .iter()
            .filter(|l| !self.hidden.contains(&l.layer))
            .collect();
        order.sort_by_key(|l| rank(&l.layer));
        let mut hit = None;
        for l in order {
            let ink = color(&l.layer, &self.rev.board, t);
            let drill = matches!(l.layer, PlotLayer::PlatedDrill | PlotLayer::NonPlatedDrill);
            for o in &l.objects {
                let placement = self
                    .rev
                    .board
                    .layout
                    .pads
                    .get(&o.id)
                    .map(|p| p.placement.as_str());
                let selected = o.id == self.selected || placement == Some(self.selected.as_str());
                let c = if drill {
                    bg
                } else if selected {
                    t.on_ink
                } else if o.pour {
                    ink.gamma_multiply(0.30)
                } else if o.pad && matches!(l.layer, PlotLayer::Copper(_)) {
                    Color32::from_rgb(207, 171, 98)
                } else {
                    ink
                };
                let pointer = response.hover_pos();
                let hovering = match &o.geometry {
                    Geometry::Stroke(axis, w) => {
                        let [a, b] = axis.map(screen);
                        let width = (mm(*w) as f32 * zoom).max(1.);
                        if a == b {
                            painter.circle_filled(a, width / 2., c);
                        } else {
                            painter.line_segment([a, b], Stroke::new(width, c));
                            if width > 2. {
                                painter.circle_filled(a, width / 2., c);
                                painter.circle_filled(b, width / 2., c);
                            }
                        }
                        pointer.is_some_and(|p| {
                            let ab = b - a;
                            let u = ((p - a).dot(ab) / ab.length_sq().max(f32::EPSILON)).clamp(0., 1.);
                            p.distance(a + ab * u) <= width / 2. + 3.
                        })
                    }
                    Geometry::Fill(tris) => {
                        fill(&painter, tris, &screen, c);
                        pointer.is_some_and(|p| {
                            tris.iter().any(|tri| {
                                let [a, b, c] = tri.map(screen);
                                let side =
                                    |a: Pos2, b: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                                let signs = [side(a, b), side(b, c), side(c, a)];
                                signs.iter().all(|&v| v >= 0.) || signs.iter().all(|&v| v <= 0.)
                            })
                        })
                    }
                };
                if hovering && (!o.pour || hit.is_none()) {
                    hit = Some(placement.unwrap_or(&o.id).to_owned());
                }
            }
        }
        // Body labels and hit targets belong to the placement layer. They stay readable when zooming out.
        for (id, p) in &self.rev.board.layout.placements {
            if self.hidden.contains(&PlotLayer::Courtyard(p.pose.side)) {
                continue;
            }
            let at = screen(p.pose.at);
            let rect = painter.text(
                at + vec2(0., -3.),
                egui::Align2::CENTER_BOTTOM,
                &p.reference,
                theme::semi(13.),
                if &self.selected == id {
                    t.focus
                } else {
                    Color32::from_rgb(220, 233, 225)
                },
            );
            let target = rect.union(Rect::from_center_size(at, vec2(12., 12.)));
            if r.contains(at) {
                places.push((format!("pcb:placement:{id}"), target));
            }
            if response.hover_pos().is_some_and(|p| target.contains(p)) {
                hit = Some(id.clone());
            }
        }
        if self.mode == Command::Route {
            let points: Vec<_> = self.draft.iter().copied().chain(under).map(screen).collect();
            if points.len() > 1 {
                painter.add(egui::Shape::line(
                    points,
                    Stroke::new((self.width as f32 * zoom).max(1.), t.focus),
                ));
            }
        }
        if response.clicked()
            && let Some(at) = under
        {
            match self.mode {
                Command::Place => self.place_at(at),
                Command::Move => {
                    let id = self.selected.clone();
                    if self.edit(vec![field(&id, "at", coord(at))]) {
                        self.mode = Command::Select
                    }
                }
                Command::Route => {
                    if self.draft.last() != Some(&at) {
                        self.draft.push(at)
                    }
                }
                _ => {
                    self.selected = hit.unwrap_or_default();
                    self.panel = Inspector::Properties;
                }
            }
        }
        if response.double_clicked() && self.mode == Command::Route {
            self.commit_route();
        }
    }
}
