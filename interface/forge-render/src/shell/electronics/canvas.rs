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
            if self.grid {
                (v * 2.).round() as f64 / 2.
            } else {
                (v as f64 * 1000.).round() / 1000.
            }
        };
        [snap(d.x), snap(-d.y)]
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
        for s in &self.page.strokes {
            let (a, b) = s.geom.bounds();
            include(a.x, a.y);
            include(b.x, b.y);
        }
        for t in &self.page.texts {
            include(t.at.x, t.at.y);
            include(t.at.x + forge_draw::page::text_width(&t.text, t.h), t.at.y + t.h);
        }
        for (_, poly) in &self.page.fills {
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
                | Item::Note(at, _)
                | Item::Port(at)
                | Item::Junction(at) => self.screen(*at).distance(p),
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
    pub(super) fn canvas(&mut self, ui: &mut Ui, r: Rect, t: &Theme) {
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
            let mut spacing = self.zoom * 2.5;
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
        let ink = t.ink;
        let pen = Stroke::new(1.4, ink);
        for s in &self.page.strokes {
            let pts = s
                .geom
                .points((0.35 / self.zoom) as f64)
                .into_iter()
                .map(|p| self.screen([p.x, p.y]))
                .collect();
            painter.add(Shape::line(pts, pen));
        }
        for (_, poly) in &self.page.fills {
            painter.add(Shape::convex_polygon(
                poly.iter().map(|p| self.screen([p.x, p.y])).collect(),
                t.success,
                Stroke::NONE,
            ));
        }
        for (k, (a, b, via)) in &self.circuit.wires {
            if k.rsplit_once('/').is_some_and(|(s, _)| s == self.subject) {
                let pts = std::iter::once(&self.circuit.locations[a])
                    .chain(via)
                    .chain(std::iter::once(&self.circuit.locations[b]))
                    .map(|p| self.screen(*p))
                    .collect();
                painter.add(Shape::line(
                    pts,
                    Stroke::new(
                        1.8,
                        if k.rsplit('/').next() == Some(&self.selected) {
                            t.focus
                        } else {
                            t.success
                        },
                    ),
                ));
            }
        }
        for text in &self.page.texts {
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
            let galley =
                painter.layout_no_wrap(text.text.clone(), theme::sans((h / 0.72).clamp(11., 96.)), ink);
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
        let anchors = self.anchors();
        let hp = resp.hover_pos();
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
        if let Some(row) = self
            .doc
            .rows
            .iter()
            .find(|r| r.id == self.selected && r.sheet == self.sheet())
            && let Item::Place { at, .. } | Item::Gate { at, .. } = row.item
        {
            painter.circle_stroke(self.screen(at), 9., Stroke::new(1.6, t.focus));
        }
        if let Some(p) = hp {
            let q = self.world(p);
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
                painter.add(Shape::line(pts, Stroke::new(1.5, t.focus)));
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
        let tree = syntax::parse(&self.src).map_err(|e| e.text())?;
        let existing = tree.iter().find_map(|s| {
            let StmtKind::Assign { name, value } = &s.kind else {
                return None;
            };
            let ExprKind::Call(function, args) = &value.kind else {
                return None;
            };
            let is_power = matches!(&function.kind, ExprKind::Name(n) if n == "library")
                && args.iter().any(|arg| {
                    arg.name.as_ref().is_some_and(|(k, _)| k == "file")
                        && matches!(&arg.value.kind, ExprKind::Str(path) if path == "forge:power")
                });
            is_power.then(|| name.clone())
        });
        let mut ops = Vec::new();
        let alias = existing.unwrap_or_else(|| {
            let alias = self.fresh("power");
            ops.push(Op::AddFeature {
                before: None,
                text: format!(r#"{alias} = library(file: "forge:power")"#),
            });
            alias
        });
        let symbol = sch::power_symbols()
            .get(&self.form.text)
            .ok_or("Choose a power net.")?;
        let name = self.fresh("PWR");
        let sheet = self.sheet();
        let anchor = near
            .map(|key| self.circuit.locations[&format!("{}/{}", self.subject, key)])
            .unwrap_or(p);
        ops.push(Op::AddFeature {
            before: None,
            text: format!(
                "{name} = place(sheet: {sheet}, symbol: {alias}.{symbol}, at: {})",
                coord(anchor)
            ),
        });
        if let Some(endpoint) = near {
            ops.push(Op::AddFeature {
                before: None,
                text: format!(
                    "{} = wire(sheet: {sheet}, from: {endpoint}, to: {name}.net)",
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
                    self.apply(vec![arg(self.geometric_selection(), "at", at)])?;
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
            Note => (
                "note",
                format!(
                    "note(sheet: {sheet}, at: {at}, value: {})",
                    quote(&self.form.text)
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
                let unit = quote(&self.unit);
                let shape = match self.tool {
                    Text => format!("text(at: {at}, value: {}, unit: {unit})", quote(&self.form.text)),
                    Line => format!("line(from: {a}, to: {at}, unit: {unit})"),
                    Rect => format!("rect(from: {a}, to: {at}, unit: {unit})"),
                    Circle => format!(
                        "circle(at: {a}, radius: {}mm, unit: {unit})",
                        forge_math::Vec2::new(self.points[0][0] - p[0], self.points[0][1] - p[1]).length()
                    ),
                    _ => format!(
                        "arc(from: {a}, mid: {}, to: {at}, unit: {unit})",
                        coord(self.points[1])
                    ),
                };
                self.points.clear();
                self.change_member("graphics", None, None, Some(shape))?;
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
