use super::*;
use sch::Shape;
pub(super) fn control_points(shape: &Shape) -> Vec<Point> {
    match shape {
        Shape::Line(a, b) | Shape::Rect(a, b) => vec![*a, *b],
        Shape::Circle(a, _) | Shape::Text(a, _) => vec![*a],
        Shape::Arc(a, m, b) => vec![*a, *m, *b],
        Shape::Polyline(ps) => ps.clone(),
    }
}
pub(super) fn geometry_args(id: &str, shape: &Shape, points: &[Point], radius: f64) -> Vec<Op> {
    let keys: &[&str] = match shape {
        Shape::Line(..) | Shape::Rect(..) => &["from", "to"],
        Shape::Arc(..) => &["from", "mid", "to"],
        Shape::Circle(..) => &["at"],
        _ => &[],
    };
    let mut ops: Vec<_> = keys
        .iter()
        .zip(points)
        .zip(control_points(shape))
        .filter(|((_, p), old)| **p != *old)
        .map(|((k, p), _)| arg(id, k, coord(*p)))
        .collect();
    if matches!(shape, Shape::Circle(_,old) if *old!=radius) {
        ops.push(arg(id, "radius", format!("{radius}mm")));
    }
    ops
}
pub(super) fn point_fields(ui: &mut Ui, points: &mut [Point]) {
    for (index, p) in points.iter_mut().enumerate() {
        ui.label(format!("Point {}", index + 1));
        ui.horizontal(|ui| {
            for (axis, v) in ["X ", "Y "].into_iter().zip(p) {
                ui.add(egui::DragValue::new(v).prefix(axis).suffix(" mm").speed(0.1));
            }
        });
    }
}
fn length_field(ui: &mut Ui, name: &str, value: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(name);
        ui.add_space(5.);
        ui.add(
            egui::DragValue::new(value)
                .speed(0.05)
                .range(0.001..=f64::MAX)
                .suffix(" mm"),
        );
    });
}
pub(super) fn controls(ui: &mut Ui, form: &mut Inputs, shape: Option<&Shape>) {
    length_field(ui, "Stroke width", &mut form.line_width);
    if let Some(shape) = shape {
        point_fields(ui, &mut form.geometry);
        if matches!(shape, Shape::Circle(..)) {
            length_field(ui, "Radius", &mut form.radius);
        }
    }
}
pub(super) fn text_controls(ui: &mut Ui, form: &mut Inputs) {
    length_field(ui, "Text height", &mut form.text_size);
    ui.checkbox(&mut form.bold, "Bold");
}
impl Editor {
    pub(super) fn grid_controls(&mut self, ui: &mut Ui, places: &mut Vec<(String, Rect)>) {
        ui.horizontal(|ui| {
            let r = ui.menu_button(format!("Grid · {} mm", self.pitch), |ui| {
                for (name, mm) in [
                    ("0.1 mm", 0.1),
                    ("0.25 mm", 0.25),
                    ("0.5 mm", 0.5),
                    ("1 mm", 1.),
                    ("2.5 mm", 2.5),
                    ("10 mil", 0.254),
                    ("25 mil", 0.635),
                    ("50 mil", 1.27),
                    ("100 mil", 2.54),
                ] {
                    let choice = ui.selectable_value(&mut self.pitch, mm, name);
                    places.push((format!("electronics:grid:{name}"), choice.rect));
                    if choice.clicked() {
                        ui.close();
                    }
                }
                ui.separator();
                length_field(ui, "Custom pitch", &mut self.pitch);
                ui.checkbox(&mut self.grid, "Show grid");
            });
            places.push(("electronics:grid".into(), r.response.rect));
            places.push((format!("electronics:pitch:{}", self.pitch), r.response.rect));
            let r = ui.checkbox(&mut self.snap, "Snap");
            places.push(("electronics:snap".into(), r.rect));
        });
    }
}
