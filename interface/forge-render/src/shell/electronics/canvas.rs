use super::*;
use crate::shell::theme;
use egui::{Color32, Sense, Shape, Stroke};
impl Editor {
    pub(super) fn prompt(&self) -> &str {
        use Tool::*;
        match self.tool {
            Select | Properties => "Select a symbol, pin or connection",
            Place | Gate | Subsheet => "Choose the part, then click its position",
            Wire if self.first.is_some() => "Click a bend or a destination pin · Esc cancels",
            Wire => "Click the first pin, port or junction",
            Move if self.selected.is_empty() && self.pin.is_empty() => "Select an object to move",
            Move => "Click the new position",
            Intent => "Click the consumer pin, then the provider pin",
            Flow => "Click the source, then the destination",
            Label | NoConnect => "Click a pin, port or junction",
            Bus => "Click member pins, then create the bus",
            Line | Rect | Circle => "Click the first point, then the second point",
            Arc => "Click the start, a point on the arc, then the end",
            Pin | Text | Note | Junction | Port => "Click to place",
            _ => self.tool.button().0,
        }
    }
    fn screen(&self, p: Point) -> Pos2 {
        self.origin + vec2(p[0] as f32 * self.zoom, -p[1] as f32 * self.zoom)
    }
    fn world(&self, p: Pos2) -> Point {
        let d = (p - self.origin) / self.zoom;
        let snap = |v: f32| {
            if self.snap {
                (f64::from(v) / self.pitch).round() * self.pitch
            } else {
                (v as f64 * 1000.).round() / 1000.
            }
        };
        [snap(d.x), snap(-d.y)]
    }
    fn snap_drawing(&self, cursor: Pos2, pins: &[(String, Point)]) -> Option<Point> {
        let mut best = None;
        let mut distance = 8.;
        let mut consider = |pt: Point| {
            let d = self.screen(pt).distance(cursor);
            if d < distance {
                best = Some(pt);
                distance = d;
            }
        };
        for (_, pt) in pins {
            consider(*pt);
        }
        for row in self.doc.rows.iter().filter(|r| r.sheet == self.sheet()) {
            if let Item::Drawing { shape, .. } = &row.item {
                for p in annotation::control_points(shape) {
                    consider(p);
                }
                match shape {
                    sch::Shape::Rect(a, b) => {
                        consider([a[0], b[1]]);
                        consider([b[0], a[1]]);
                    }
                    sch::Shape::Circle(a, r) => {
                        for (x, y) in [(1., 0.), (-1., 0.), (0., 1.), (0., -1.)] {
                            consider([a[0] + r * x, a[1] + r * y]);
                        }
                    }
                    _ => {}
                }
            }
        }
        best
    }
    fn fit_page(&mut self, r: Rect) {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        let mut include = |x: f64, y: f64| {
            lo[0] = lo[0].min(x);
            lo[1] = lo[1].min(y);
            hi[0] = hi[0].max(x);
            hi[1] = hi[1].max(y);
        };
        let page = self.focus_pin.as_ref().map_or(&self.page, |f| &f.page);
        for s in &page.strokes {
            let (a, b) = s.geom.bounds();
            include(a.x, a.y);
            include(b.x, b.y);
        }
        for t in &page.texts {
            include(t.at.x, t.at.y);
            include(t.at.x + forge_draw::page::text_width(&t.text, t.h), t.at.y + t.h);
        }
        for (_, poly) in &page.fills {
            for p in poly {
                include(p.x, p.y);
            }
        }
        if !lo[0].is_finite() {
            lo = [-10., -8.];
            hi = [10., 8.];
        }
        self.zoom = (r.width() / ((hi[0] - lo[0]) as f32 + 5.))
            .min(r.height() / ((hi[1] - lo[1]) as f32 + 5.))
            * 0.88;
        self.zoom = self.zoom.clamp(0.1, 65.);
        self.origin = r.center()
            - vec2(
                ((hi[0] + lo[0]) * 0.5) as f32 * self.zoom,
                -((hi[1] + lo[1]) * 0.5) as f32 * self.zoom,
            );
        self.fit = false;
    }
    fn anchors(&self) -> Vec<(String, Point)> {
        if self.library() {
            return self
                .doc
                .symbols
                .get(&self.subject)
                .into_iter()
                .flat_map(|s| &s.pins)
                .filter(|(_, p)| p.unit == self.unit)
                .map(|(k, p)| (k.clone(), p.at))
                .collect();
        }
        let prefix = format!("{}/", self.subject);
        self.circuit
            .locations
            .range(prefix.clone()..)
            .take_while(|(k, _)| k.starts_with(&prefix))
            .filter_map(|(k, p)| {
                let id = &k[prefix.len()..];
                (!id.contains('/')).then(|| (id.into(), *p))
            })
            .collect()
    }
    fn pick_graphic(&self, point: Point) -> Option<usize> {
        let pointer = self.screen(point);
        self.doc
            .symbols
            .get(&self.subject)?
            .graphics
            .iter()
            .enumerate()
            .filter(|(_, g)| g.unit.as_ref().is_none_or(|u| u == &self.unit))
            .filter_map(|(i, g)| {
                let page = sch::graphic_page(&g.shape);
                let mut distance = f32::INFINITY;
                for stroke in page.strokes {
                    let points = stroke.geom.points(0.35 / f64::from(self.zoom));
                    for segment in points.windows(2) {
                        let a = self.screen([segment[0].x, segment[0].y]);
                        let b = self.screen([segment[1].x, segment[1].y]);
                        distance = distance.min(segment_distance(a, b, pointer));
                    }
                }
                for text in page.texts {
                    let w = forge_draw::page::text_width(&text.text, text.h);
                    distance = distance.min(
                        Rect::from_two_pos(
                            self.screen([text.at.x, text.at.y]),
                            self.screen([text.at.x + w, text.at.y + text.h]),
                        )
                        .distance_to_pos(pointer),
                    );
                }
                (distance <= 10.).then_some((i, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
    fn picked(&self, p: Pos2) -> Option<String> {
        let mut best = 12.;
        let mut result = None;
        for row in self.doc.rows.iter().filter(|r| r.sheet == self.sheet()) {
            let d = match &row.item {
                Item::Place { at, .. }
                | Item::Gate { at, .. }
                | Item::SheetInstance { at, .. }
                | Item::Label { at, .. }
                | Item::Port(at)
                | Item::Junction(at) => self.screen(*at).distance(p),
                Item::Drawing { shape, width } => {
                    let page = sch::graphic_page(shape);
                    page.strokes
                        .iter()
                        .flat_map(|s| {
                            s.geom
                                .points(0.35 / f64::from(self.zoom))
                                .windows(2)
                                .map(|w| {
                                    segment_distance(
                                        self.screen([w[0].x, w[0].y]),
                                        self.screen([w[1].x, w[1].y]),
                                        p,
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .fold(f32::INFINITY, f32::min)
                        - *width as f32 * self.zoom / 2.
                }
                Item::Note { at, text, size, .. } => Rect::from_two_pos(
                    self.screen(*at),
                    self.screen([at[0] + forge_draw::page::text_width(text, *size), at[1] + size]),
                )
                .distance_to_pos(p),
                Item::Wire { .. } => {
                    let (a, b, via) = self.circuit.wires.get(&format!("{}/{}", self.subject, row.id))?;
                    let mut prev = self.screen(self.circuit.locations[a]);
                    let mut min = f32::MAX;
                    for q in via.iter().chain(std::iter::once(&self.circuit.locations[b])) {
                        let next = self.screen(*q);
                        min = min.min(segment_distance(prev, next, p));
                        prev = next;
                    }
                    min
                }
                _ => continue,
            };
            if d < best {
                best = d;
                result = Some(row.id.clone());
            }
        }
        result
    }
    pub(super) fn canvas(&mut self, ui: &mut Ui, r: Rect, t: &Theme, places: &mut Vec<(String, Rect)>) {
        let painter = ui.painter_at(r);
        painter.rect_filled(
            r,
            0.,
            if t.dark {
                t.bg
            } else {
                Color32::from_rgb(249, 250, 249)
            },
        );
        if self.fit {
            self.fit_page(r);
        }
        let resp = ui.interact(
            r,
            ui.scope_id().with("electronics-canvas"),
            Sense::click_and_drag(),
        );
        if let Some(hp) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                let z = (self.zoom * (scroll * 0.002).exp()).clamp(0.1, 150.);
                self.origin = hp + (self.origin - hp) * (z / self.zoom);
                self.zoom = z;
            }
        }
        if resp.dragged_by(egui::PointerButton::Middle) {
            self.origin += ui.input(|i| i.pointer.delta());
        }
        if self.grid {
            let mut spacing = self.zoom * self.pitch as f32;
            while spacing < 13. {
                spacing *= 2.;
            }
            let mut x = r.left() + (self.origin.x - r.left()).rem_euclid(spacing);
            while x < r.right() {
                let mut y = r.top() + (self.origin.y - r.top()).rem_euclid(spacing);
                while y < r.bottom() {
                    painter.circle_filled(pos2(x, y), 0.7, t.line);
                    y += spacing;
                }
                x += spacing;
            }
        }
        let phase = if self.markers.active() && !self.directions.is_empty() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(32));
            ui.input(|i| i.time * 36.0)
        } else {
            0.
        };
        for (page, foreground) in
            std::iter::once((&self.page, false)).chain(self.focus_pin.iter().map(|f| (&f.page, true)))
        {
            let dim = !foreground && self.focus_pin.is_some();
            let color = |value: Color32| {
                if dim {
                    t.bg.lerp_to_gamma(value, 0.29)
                } else {
                    value
                }
            };
            let ink = color(t.ink);
            let pen = Stroke::new(1.4, ink);
            for s in &page.strokes {
                let pts = s
                    .geom
                    .points((0.35 / self.zoom) as f64)
                    .into_iter()
                    .map(|p| self.screen([p.x, p.y]))
                    .collect();
                let stroke = if matches!(s.pen, forge_draw::Pen::Annotation(_)) {
                    Stroke::new((s.pen.width() as f32 * self.zoom).max(0.7), ink)
                } else {
                    pen
                };
                painter.add(Shape::line(pts, stroke));
            }
            for (_, poly) in &page.fills {
                painter.add(Shape::convex_polygon(
                    poly.iter().map(|p| self.screen([p.x, p.y])).collect(),
                    color(t.success),
                    Stroke::NONE,
                ));
            }
            for (k, (a, b, via)) in &self.circuit.wires {
                if foreground && !self.focus_pin.as_ref().unwrap().inspection.wires.contains(k) {
                    continue;
                }
                if k.rsplit_once('/').is_some_and(|(s, _)| s == self.subject) {
                    let pts: Vec<_> = std::iter::once(&self.circuit.locations[a])
                        .chain(via)
                        .chain(std::iter::once(&self.circuit.locations[b]))
                        .map(|p| self.screen(*p))
                        .collect();
                    if self.markers.active()
                        && let Some(dir) = self.directions.get(k)
                    {
                        let tone = color(if *dir == sch::Direction::Conflict {
                            t.error
                        } else {
                            t.success
                        });
                        painter.add(Shape::line(pts.clone(), Stroke::new(1.6, tone)));
                        paint_signal(&painter, &pts, *dir, phase, tone, self.markers);
                    } else {
                        let tone = if k.rsplit('/').next() == Some(&self.selected) {
                            t.focus
                        } else if self.markers.active() {
                            t.ink2
                        } else {
                            t.success
                        };
                        painter.add(Shape::line(pts, Stroke::new(1.8, color(tone))));
                    }
                }
            }
            for text in &page.texts {
                // Pin names remain available on hover and in Properties when the full sheet is fitted.
                let size = text.h as f32 * self.zoom * 1.12;
                if !self.library() && text.h < 1. && size < 11. {
                    continue;
                }
                let h = text.h as f32 * self.zoom;
                let w = forge_draw::page::text_width(&text.text, text.h) as f32 * self.zoom;
                let baseline = self.screen([text.at.x, text.at.y]);
                if !r.intersects(Rect::from_two_pos(baseline, baseline + vec2(w, -h))) {
                    continue;
                }
                // Use print-space bounds at every zoom. egui's line box includes leading;
                // the actual glyph mesh gives us its visible baseline and cap height.
                let galley = painter.layout_no_wrap(
                    text.text.clone(),
                    if text.pen == forge_draw::Pen::BoldText {
                        theme::semi((h / 0.72).clamp(11., 96.))
                    } else {
                        theme::sans((h / 0.72).clamp(11., 96.))
                    },
                    ink,
                );
                let bounds = galley.mesh_bounds;
                if !bounds.is_positive() {
                    continue;
                }
                let scale = (h / bounds.height()).min(w / bounds.width());
                let mut glyphs = Shape::galley(Pos2::ZERO, galley, ink);
                glyphs.transform(egui::emath::TSTransform {
                    scaling: scale,
                    translation: baseline.to_vec2() - bounds.left_bottom().to_vec2() * scale,
                });
                painter.add(glyphs);
            }
        }
        let anchors = self.anchors();
        if !self.library() {
            for (name, pt) in &anchors {
                let point = self.screen(*pt);
                if r.contains(point) {
                    places.push((
                        format!("electronics:anchor:{name}"),
                        Rect::from_center_size(point, vec2(8., 8.)),
                    ));
                }
            }
            for row in self.doc.rows.iter().filter(|row| row.sheet == self.sheet()) {
                if let Item::Place { at, .. } | Item::Gate { at, .. } = row.item {
                    let point = self.screen(at);
                    if r.contains(point) {
                        places.push((
                            format!("electronics:component:{}", row.id),
                            Rect::from_center_size(point, vec2(8., 8.)),
                        ));
                    }
                }
            }
        }
        if !self.selected.is_empty()
            && let Some(row) = self.doc.rows.iter().find(|r| r.id == self.selected)
            && let Item::Place { at, .. } | Item::Gate { at, .. } = row.item
        {
            places.push((
                format!("electronics:selected:{}", self.selected),
                Rect::from_center_size(self.screen(at), vec2(18., 18.)),
            ));
        }
        let hp = resp.interact_pointer_pos().or_else(|| resp.hover_pos());
        let near = hp.and_then(|p| {
            anchors
                .iter()
                .filter_map(|(k, q)| {
                    let dist = self.screen(*q).distance(p);
                    (dist < 11.).then_some((dist, k.clone()))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|a| a.1)
        });
        if resp.secondary_clicked()
            && !self.library()
            && let Some(point) = hp
        {
            // At a stacked anchor prefer a physical pin to its coincident power marker.
            let closest = anchors
                .iter()
                .filter_map(|(k, q)| {
                    let key = format!("{}/{}", self.subject, k);
                    let terminal = self.circuit.terminals.get(&key)?;
                    let dist = self.screen(*q).distance(point);
                    (dist < 11.).then(|| (dist, self.doc.symbols[&terminal.symbol].power_net.is_some(), key))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            if let Some((_, _, key)) = closest {
                self.focus_on(&key);
            }
        }
        if let Some(anchor) = &near {
            resp.clone().on_hover_text(anchor);
        }
        for (k, q) in &anchors {
            let selected =
                !self.pin.is_empty() && (k == &self.pin || k == &format!("{}.{}", self.selected, self.pin));
            if !self.library() {
                let key = format!("{}/{}", self.subject, k);
                if self.circuit.is_attached(&key) || !self.circuit.terminals.contains_key(&key) {
                    continue;
                }
            }
            if self
                .focus_pin
                .as_ref()
                .is_some_and(|f| !f.inspection.pins.contains(&format!("{}/{}", self.subject, k)))
            {
                continue;
            }
            painter.circle_stroke(
                self.screen(*q),
                if selected || near.as_ref() == Some(k) {
                    4.5
                } else {
                    2.4
                },
                Stroke::new(
                    1.,
                    if selected || near.as_ref() == Some(k) {
                        t.focus
                    } else {
                        t.ink2
                    },
                ),
            );
        }
        if let Some(focus) = &self.focus_pin {
            if let Some(at) = self.circuit.locations.get(&focus.id) {
                painter.rect_stroke(
                    Rect::from_center_size(self.screen(*at), vec2(9., 9.)),
                    1.,
                    Stroke::new(1.5, t.focus),
                    egui::StrokeKind::Middle,
                );
            }
        } else if let Some(row) = self
            .doc
            .rows
            .iter()
            .find(|r| r.id == self.geometric_selection() && r.sheet == self.sheet())
            && let Item::Place { at, .. } | Item::Gate { at, .. } = row.item
        {
            painter.circle_stroke(self.screen(at), 9., Stroke::new(1.6, t.focus));
        }
        if let Some(p) = hp {
            let mut q = self.world(p);
            if matches!(
                self.tool,
                Tool::Power | Tool::Wire | Tool::Label | Tool::NoConnect
            ) && let Some(pt) = near
                .as_ref()
                .and_then(|name| anchors.iter().find(|(key, _)| key == name))
            {
                q = pt.1;
            }
            if self.snap
                && matches!(
                    self.tool,
                    Tool::Line | Tool::Rect | Tool::Circle | Tool::Arc | Tool::Note | Tool::Move
                )
            {
                q = self.snap_drawing(p, &anchors).unwrap_or(q);
            }
            if matches!(
                self.tool,
                Tool::Place
                    | Tool::Power
                    | Tool::Wire
                    | Tool::Line
                    | Tool::Rect
                    | Tool::Circle
                    | Tool::Arc
                    | Tool::Note
                    | Tool::Junction
                    | Tool::Move
            ) {
                let cross = self.screen(
                    near.as_deref()
                        .filter(|_| matches!(self.tool, Tool::Wire | Tool::Power))
                        .and_then(|k| anchors.iter().find(|(id, _)| id == k))
                        .map_or(q, |(_, p)| *p),
                );
                painter.line_segment(
                    [cross - vec2(5., 0.), cross + vec2(5., 0.)],
                    Stroke::new(1., t.focus),
                );
                painter.line_segment(
                    [cross - vec2(0., 5.), cross + vec2(0., 5.)],
                    Stroke::new(1., t.focus),
                );
            }
            let start = self
                .first
                .as_ref()
                .and_then(|k| anchors.iter().find(|(id, _)| id == k).map(|(_, p)| *p));
            if let Some(a) = self.points.last().copied().or(start) {
                let mut pts = start
                    .into_iter()
                    .chain(self.points.iter().copied())
                    .chain(std::iter::once(q))
                    .map(|p| self.screen(p))
                    .collect::<Vec<_>>();
                if pts.len() < 2 {
                    pts.insert(0, self.screen(a));
                }
                let preview = Stroke::new(1.5, t.focus);
                let start = self.screen(a);
                let end = self.screen(q);
                match self.tool {
                    Tool::Rect => {
                        painter.rect_stroke(
                            Rect::from_two_pos(start, end),
                            0.,
                            preview,
                            egui::StrokeKind::Middle,
                        );
                    }
                    Tool::Circle => {
                        painter.circle_stroke(start, start.distance(end), preview);
                    }
                    Tool::Arc if self.points.len() == 2 => {
                        let shape = sch::Shape::Arc(self.points[0], self.points[1], q);
                        for curve in sch::graphic_page(&shape).strokes {
                            painter.add(Shape::line(
                                curve
                                    .geom
                                    .points(0.35 / f64::from(self.zoom))
                                    .iter()
                                    .map(|p| self.screen([p.x, p.y]))
                                    .collect(),
                                preview,
                            ));
                        }
                    }
                    _ => {
                        painter.add(Shape::line(pts, preview));
                    }
                }
            }
            if resp.clicked() {
                let picked = self.picked(p);
                let result = self.click(q, near.as_deref(), picked.as_deref());
                if let Err(e) = result {
                    self.message = e;
                }
            }
        }
        if !ui.ctx().egui_wants_keyboard_input() {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.start(Tool::Select);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                self.start(Tool::Delete);
            }
        }
    }
    /// Tie a branch to a named junction. Splitting at a bend partitions the existing route.
    fn branch(&mut self, wire: &str, position: Point) -> Result<()> {
        let scope = self.sheet();
        let key = format!("{}/{}", self.subject, wire);
        let (source, destination, via) = &self.circuit.wires[&key];
        let route: Vec<_> = std::iter::once(self.circuit.locations[source])
            .chain(via.iter().copied())
            .chain(std::iter::once(self.circuit.locations[destination]))
            .collect();
        let (index, point, _) = route
            .windows(2)
            .enumerate()
            .map(|(i, pair)| {
                let [a, b] = [pair[0], pair[1]];
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let u = (((position[0] - a[0]) * dx + (position[1] - a[1]) * dy)
                    / (dx * dx + dy * dy).max(f64::EPSILON))
                .clamp(0., 1.);
                let q = [a[0] + u * dx, a[1] + u * dy];
                let offset = forge_math::Vec2::new(q[0] - position[0], q[1] - position[1]);
                (i, q, offset.dot(offset))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .ok_or("Wire has no segments.")?;
        let row = self
            .doc
            .rows
            .iter()
            .find(|r| r.id == wire)
            .ok_or("Unknown wire.")?;
        let Item::Wire { to, .. } = &row.item else {
            return Err("Select a wire.".into());
        };
        let tree = syntax::parse(&self.src).map_err(|e| e.text())?;
        let expressions = match source_arg(&tree, wire, "via").map(|e| &e.kind) {
            Some(ExprKind::List(points)) => points
                .iter()
                .map(|e| &self.src[e.span.lo as usize..e.span.hi as usize])
                .collect::<Vec<_>>(),
            _ => vec![],
        };
        let vias = |range: std::ops::Range<usize>| {
            range
                .filter(|i| via[*i] != point)
                .map(|i| {
                    expressions
                        .get(i)
                        .map_or_else(|| coord(via[i]), |s| s.to_string())
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let junction = self.fresh("j");
        let continuation = self.fresh("wire");
        let mut ops = vec![
            Op::AddFeature {
                before: Some(wire.into()),
                text: format!("{junction} = junction(sheet: {scope}, at: {})", coord(point)),
            },
            arg(wire, "to", junction.clone()),
            arg(wire, "via", format!("[{}]", vias(0..index))),
            Op::AddFeature {
                before: None,
                text: format!(
                    "{continuation} = wire(sheet: {scope}, from: {junction}, to: {to}, via: [{}])",
                    vias(index..via.len())
                ),
            },
        ];
        if let Some(from) = &self.first {
            let bends = self
                .points
                .iter()
                .copied()
                .map(coord)
                .collect::<Vec<_>>()
                .join(", ");
            let branch = self.fresh("branch");
            ops.push(Op::AddFeature {
                before: None,
                text: format!(
                    "{branch} = wire(sheet: {scope}, from: {from}, to: {junction}, via: [{bends}])"
                ),
            });
        }
        self.apply(ops)?;
        self.points.clear();
        self.first = None;
        self.message = format!("Added junction {junction}");
        Ok(())
    }
    fn place_power(&mut self, p: Point, near: Option<&str>) -> Result<()> {
        let mut ops = Vec::new();
        let (symbol, terminal, unit, offset) = if let Some(id) = &self.form.power_symbol {
            let sym = self.doc.symbols.get(id).ok_or("Choose a power symbol.")?;
            let (key, pin) = sym
                .pins
                .first_key_value()
                .ok_or("Power symbol has no terminal.")?;
            (id.clone(), key.to_owned(), pin.unit.clone(), pin.at)
        } else {
            let symbol = sch::power_symbols()
                .get(&self.form.text)
                .ok_or("Choose a power net.")?;
            let alias = self.power_aliases.first().cloned().unwrap_or_else(|| {
                let alias = self.fresh("power");
                ops.push(Op::AddFeature {
                    before: None,
                    text: format!(r#"{alias} = library(file: "forge:power")"#),
                });
                alias
            });
            (format!("{alias}.{symbol}"), "net".into(), "A".into(), [0., 0.])
        };
        let name = self.fresh("PWR");
        let sheet = self.sheet();
        let anchor = near
            .map(|key| self.circuit.locations[&format!("{}/{}", self.subject, key)])
            .unwrap_or(p);
        ops.push(Op::AddFeature {
            before: None,
            text: format!(
                "{name} = place(sheet: {sheet}, symbol: {symbol}, unit: {}, at: {})",
                quote(&unit),
                coord([anchor[0] - offset[0], anchor[1] - offset[1]])
            ),
        });
        if let Some(endpoint) = near {
            ops.push(Op::AddFeature {
                before: None,
                text: format!(
                    "{} = wire(sheet: {sheet}, from: {endpoint}, to: {name}.{terminal})",
                    self.fresh("w")
                ),
            });
        }
        self.apply(ops)?;
        self.selected = name;
        self.start(Tool::Select);
        Ok(())
    }
    pub(super) fn click(&mut self, p: Point, near: Option<&str>, picked: Option<&str>) -> Result<()> {
        use Tool::*;
        let at = coord(p);
        let sheet = self.sheet().to_owned();
        let need = || near.ok_or("Choose a pin, port or junction.");
        if near.is_none()
            && (self.tool == Junction || self.tool == Wire && self.first.is_some())
            && let Some(wire) = picked
            && self
                .circuit
                .wires
                .contains_key(&format!("{}/{}", self.subject, wire))
        {
            return self.branch(wire, p);
        }
        if near.is_some() && self.tool == Junction {
            return Err("This point already has a connection.".into());
        }
        let (prefix, body) = match self.tool {
            Select | Properties => {
                if self.library() {
                    self.pin = near.unwrap_or("").into();
                    self.graphic = if near.is_none() {
                        self.pick_graphic(p)
                    } else {
                        None
                    };
                    self.inspect();
                } else if let Some(k) = near {
                    let (id, pin) = k.split_once('.').unwrap_or((k, ""));
                    self.select(id, pin);
                } else {
                    self.select(picked.unwrap_or(""), "");
                }
                return Ok(());
            }
            Move => {
                if self.library() {
                    if let Some(i) = self.graphic {
                        if let Some(a) = self.form.geometry.first().copied() {
                            for q in &mut self.form.geometry {
                                q[0] += p[0] - a[0];
                                q[1] += p[1] - a[1];
                            }
                            self.save_graphic(i)?;
                        }
                        self.start(Select);
                        return Ok(());
                    }
                    if self.pin.is_empty() {
                        if near.is_none() {
                            self.graphic = self.pick_graphic(p);
                            self.inspect();
                            return Ok(());
                        }
                        self.pin = need()?.into();
                        self.inspect();
                        return Ok(());
                    }
                    self.save_pin(p)?;
                } else {
                    if self.selected.is_empty() {
                        if let Some(endpoint) = near {
                            let (id, pin) = endpoint.split_once('.').unwrap_or((endpoint, ""));
                            self.selected = id.into();
                            self.pin = pin.into();
                        } else {
                            self.selected = picked.ok_or("Select an object first.")?.into();
                            self.pin.clear();
                        }
                        return Ok(());
                    }
                    if let Some(Item::Drawing { shape, .. }) = self
                        .doc
                        .rows
                        .iter()
                        .find(|r| r.id == self.selected)
                        .map(|r| &r.item)
                    {
                        let mut points = annotation::control_points(shape);
                        let origin = points[0];
                        for pt in &mut points {
                            pt[0] += p[0] - origin[0];
                            pt[1] += p[1] - origin[1];
                        }
                        let radius = if let sch::Shape::Circle(_, r) = shape {
                            *r
                        } else {
                            0.
                        };
                        self.apply(annotation::geometry_args(&self.selected, shape, &points, radius))?;
                    } else {
                        self.apply(vec![arg(self.geometric_selection(), "at", at)])?;
                    }
                }
                self.start(Select);
                return Ok(());
            }
            Power => return self.place_power(p, near),
            Place => {
                let name = self.form.name.clone();
                let symbol = &self.symbol;
                if !self.doc.symbols.contains_key(symbol) {
                    return Err("Link a symbol library first.".into());
                }
                self.add(
                    &name,
                    format!(
                        "place(sheet: {sheet}, symbol: {symbol}, unit: {}, at: {at})",
                        quote(&self.form.unit)
                    ),
                )?;
                self.selected = name;
                self.start(Select);
                return Ok(());
            }
            Gate => (
                "gate",
                format!(
                    "gate(sheet: {sheet}, of: {}, unit: {}, at: {at})",
                    self.selected,
                    quote(&self.form.unit)
                ),
            ),
            Subsheet => (
                "channel",
                format!("subsheet(sheet: {sheet}, of: {}, at: {at})", self.form.group),
            ),
            Junction => ("j", format!("junction(sheet: {sheet}, at: {at})")),
            Port => {
                let id = self.form.name.clone();
                self.add(&id, format!("port(sheet: {sheet}, at: {at})"))?;
                self.start(Select);
                return Ok(());
            }
            Label => {
                let target = need()?;
                let anchor = coord(self.circuit.locations[&format!("{}/{target}", self.subject)]);
                (
                    "net",
                    format!(
                        "label(sheet: {sheet}, on: {target}, value: {}, global: {}, at: {anchor})",
                        quote(&self.form.text),
                        self.form.global
                    ),
                )
            }
            NoConnect => ("nc", format!("no_connect(sheet: {sheet}, on: {})", need()?)),
            Note if self.form.text.trim().is_empty() => {
                return Err("Enter the text before placing it.".into());
            }
            Note => (
                "note",
                format!(
                    "note(sheet: {sheet}, at: {at}, value: {}, size:{}mm, bold:{})",
                    quote(&self.form.text),
                    self.form.text_size,
                    self.form.bold
                ),
            ),
            Bus => {
                let id = need()?;
                if !self.form.wants.iter().any(|s| s == id) {
                    self.form.wants.push(id.into());
                }
                return Ok(());
            }
            Wire | Flow | Intent => {
                if self.first.is_none() {
                    self.first = Some(need()?.into());
                    return Ok(());
                }
                if near.is_none() && self.tool == Wire {
                    self.points.push(p);
                    return Ok(());
                }
                let (a, b) = (self.first.as_deref().unwrap(), need()?);
                match self.tool {
                    Wire => (
                        "w",
                        format!(
                            "wire(sheet: {sheet}, from: {a}, to: {b}, via: [{}])",
                            self.points
                                .iter()
                                .copied()
                                .map(coord)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ),
                    Flow => ("flow", format!("flow(sheet: {sheet}, from: {a}, to: {b})")),
                    _ => {
                        let mut extra = String::new();
                        match (self.form.return_from.as_str(), self.form.return_to.as_str()) {
                            ("", "") => {}
                            ("", _) | (_, "") => return Err("Choose both return pins.".into()),
                            (from, to) => extra = format!(", return_from: {from}, return_to: {to}"),
                        }
                        if !self.form.preferred_distance.trim().is_empty() {
                            extra.push_str(&format!(", prefer: {}", self.form.preferred_distance));
                        }
                        (
                            "intent",
                            format!(
                                "fulfill(sheet: {sheet}, want: {a}, give: {b}, role: {}{extra})",
                                self.form.role
                            ),
                        )
                    }
                }
            }
            Pin => {
                self.save_pin(p)?;
                self.pin = self.form.name.clone();
                self.start(Select);
                return Ok(());
            }
            Line | Rect | Circle | Arc | Text => {
                self.points.push(p);
                let required = match self.tool {
                    Text => 1,
                    Arc => 3,
                    _ => 2,
                };
                if self.points.len() < required {
                    return Ok(());
                }
                let a = coord(self.points[0]);
                let scope = if self.library() {
                    format!("unit: {}", quote(&self.unit))
                } else {
                    format!("sheet: {sheet}, width: {}mm", self.form.line_width)
                };
                let shape = match self.tool {
                    Text => format!("text(at: {at}, value: {}, {scope})", quote(&self.form.text)),
                    Line => format!("line(from: {a}, to: {at}, {scope})"),
                    Rect => format!("rect(from: {a}, to: {at}, {scope})"),
                    Circle => format!(
                        "circle(at: {a}, radius: {}mm, {scope})",
                        forge_math::Vec2::new(self.points[0][0] - p[0], self.points[0][1] - p[1]).length()
                    ),
                    _ => format!(
                        "arc(from: {a}, mid: {}, to: {at}, {scope})",
                        coord(self.points[1])
                    ),
                };
                self.points.clear();
                if self.library() {
                    self.change_member("graphics", None, None, Some(shape))?;
                } else {
                    let id = self.fresh(match self.tool {
                        Line => "line",
                        Rect => "box",
                        Circle => "circle",
                        _ => "arc",
                    });
                    self.add(&id, shape)?;
                    self.selected = id;
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        let id = self.fresh(prefix);
        self.add(&id, body)?;
        self.first = None;
        self.points.clear();
        self.message = format!("Added {id}");
        Ok(())
    }
}

fn segment_distance(a: Pos2, b: Pos2, p: Pos2) -> f32 {
    let direction = b - a;
    let fraction = ((p - a).dot(direction) / direction.length_sq().max(f32::EPSILON)).clamp(0., 1.);
    (a + direction * fraction).distance(p)
}

/// Advance at constant screen speed over the entire polyline, including bends.
fn paint_signal(
    painter: &egui::Painter,
    points: &[Pos2],
    dir: sch::Direction,
    time: f64,
    color: Color32,
    markers: SignalMarkers,
) {
    let clip = painter.clip_rect();
    let length: f32 = points.windows(2).map(|p| p[0].distance(p[1])).sum();
    if length < 5. {
        return;
    }
    let spacing = length / (length / 48.).ceil();
    for reverse in [false, true] {
        if dir != sch::Direction::Conflict && reverse != (dir == sch::Direction::Reverse) {
            continue;
        }
        let phase = (if reverse { -time } else { time }).rem_euclid(spacing as f64) as f32;
        let mut travel = 0.;
        for pair in points.windows(2) {
            let delta = pair[1] - pair[0];
            let len = delta.length();
            if len < 0.01 {
                continue;
            }
            let unit = delta / len;
            let heading = if reverse { -unit } else { unit };
            let normal = egui::vec2(-heading.y, heading.x);
            // Bound iteration to the viewport even for a very long zoomed-in wire.
            let center = (clip.center() - pair[0]).dot(unit);
            let radius = (clip.width() * unit.x.abs() + clip.height() * unit.y.abs()) * 0.5 + 8.;
            let first = (center - radius).max(0.);
            let last = (center + radius).min(len);
            for (dot, enabled) in [(false, markers.arrows), (true, markers.dots)] {
                if !enabled {
                    continue;
                }
                let stagger = if dot && markers.arrows { spacing * 0.5 } else { 0. };
                let mut offset = first + (phase + stagger - travel - first).rem_euclid(spacing);
                while offset < last {
                    let point = pair[0]
                        + unit * offset
                        + normal * if dir == sch::Direction::Conflict { 3. } else { 0. };
                    if clip.expand(6.).contains(point) {
                        if dot {
                            painter.circle_filled(point, 2.6, color);
                        } else {
                            let back = point - heading * 4.5;
                            painter.add(Shape::line(
                                vec![back + normal * 2.8, point, back - normal * 2.8],
                                Stroke::new(1.6, color),
                            ));
                        }
                    }
                    offset += spacing;
                }
            }
            travel += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metric_and_mil_snap_agree_with_the_origin_and_off_grid_anchors() {
        let mut e = Editor::open(
            None,
            "schematic\nmain=sheet()\na=port(sheet:main,at:(1.1,2.2))\n".into(),
        )
        .unwrap();
        e.zoom = 20.;
        e.origin = pos2(250., 150.);
        for pitch in [0.25, 0.5, 1.27, 2.54, 0.37] {
            e.pitch = pitch;
            for grid in [true, false] {
                e.grid = grid;
                let pt = e.screen([2.49 * pitch, -1.51 * pitch]);
                assert_eq!(e.world(pt), [2. * pitch, -2. * pitch]);
            }
        }
        let at = [1.1, 2.2];
        assert_eq!(
            e.snap_drawing(e.screen(at) + vec2(2., 1.), &e.anchors()),
            Some(at)
        );
        e.snap = false;
        assert_eq!(e.world(e.screen(at)), at);
    }
    #[test]
    fn draw_edit_move_undo_and_toggle_markers_independently() {
        let mut e = Editor::open(None, "schematic\nmain=sheet()\n".into()).unwrap();
        for (tool, id) in [
            (Tool::Line, "line1"),
            (Tool::Rect, "box1"),
            (Tool::Circle, "circle1"),
            (Tool::Arc, "arc1"),
        ] {
            e.start(tool);
            e.form.line_width = 0.45;
            e.click([1., 1.], None, None).unwrap();
            e.click([3., 4.], None, None).unwrap();
            if tool == Tool::Arc {
                e.click([6., 1.], None, None).unwrap();
            }
            e.select(id, "");
            e.form.line_width = 0.7;
            e.save_properties().unwrap();
            let before = e.src.clone();
            e.start(Tool::Move);
            e.click([8., 8.], None, None).unwrap();
            assert_ne!(e.src, before);
            e.step(false);
            assert_eq!(e.src, before);
        }
        e.start(Tool::Note);
        e.form.text = "Filter".into();
        e.form.text_size = 3.;
        e.form.bold = true;
        e.click([2., 9.], None, None).unwrap();
        e.select("note1", "");
        assert!(e.form.bold);
        e.form.text_size = 2.;
        e.save_properties().unwrap();
        let back = Editor::open(None, e.src.clone()).unwrap();
        assert!(
            back.page
                .texts
                .iter()
                .any(|t| t.text == "Filter" && t.h == 2. && t.pen == forge_draw::Pen::BoldText)
        );
        for (tool, state) in [
            (Tool::Dots, (false, true)),
            (Tool::Directions, (true, true)),
            (Tool::Dots, (true, false)),
            (Tool::Directions, (false, false)),
        ] {
            e.start(tool);
            assert_eq!((e.markers.arrows, e.markers.dots), state);
        }
        assert!(e.circuit.nets.is_empty());
    }
}
