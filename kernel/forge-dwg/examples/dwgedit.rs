//! Adds a layer, a line and a circle to model space, erases a line, writes. `dwgedit IN OUT`
use forge_dwg::edit::History;
use forge_dwg::types::{Circle, Data, Line};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut d = forge_dwg::file::read(&std::fs::read(&a[1]).unwrap()).unwrap();
    let mut hist = History::default();
    hist.begin();
    let layer = d.add_layer(&mut hist, "FORGE_EDIT", 1).unwrap();
    let ms = d.model_space();
    let line = Line { start: [-50.0, -50.0, 0.0], end: [123.456, 78.9, 0.0], ..Line::default() };
    let l = d.add_entity(&mut hist, ms, layer, Data::Line(line)).unwrap();
    let c = Circle { center: [-25.0, 40.0, 0.0], radius: 12.5, ..Circle::default() };
    let ci = d.add_entity(&mut hist, ms, layer, Data::Circle(c)).unwrap();
    // erase the first model-space line there was
    let victim = d
        .objects
        .iter()
        .find(|(h, o)| **h != l && matches!(o.data, Data::Line(_)) && d.block_of(**h) == Some(ms))
        .map(|(h, _)| *h);
    if let Some(v) = victim {
        d.erase(&mut hist, v).unwrap();
    }
    hist.end();
    std::fs::write(&a[2], forge_dwg::file::write(&d).unwrap()).unwrap();
    eprintln!("added layer {layer:#x}, line {l:#x}, circle {ci:#x}; erased {victim:x?}");
    // and undo brings back the original
    let before = forge_dwg::file::read(&std::fs::read(&a[1]).unwrap()).unwrap();
    hist.undo(&mut d);
    let same = d.objects.keys().eq(before.objects.keys()) && d.header == before.header;
    eprintln!("undo restores the drawing: {same}");
}
