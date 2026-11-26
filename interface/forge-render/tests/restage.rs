//! An assembly's view made again when which part is placed where changes (a part's param edited,
//! research/big-render): what the person chose for the view stays, occlusion turned off with `O`
//! and a background picture.

use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::Dim;
use forge_render::gui::Headless;
use std::path::PathBuf;

const ASM: &str = "# a plate beside a part: editing the plate restages the view
shared L = 120
shared T = 8

sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (L, L / 2))
}
plate = extrude(plan, T)
k = part(\"knob.fg\")

ground(plate)
ground(k)
";

const KNOB: &str = "shared r = 10
shared h = 12
sketch s(on: xy) {
  c = circle((0, 0), r)
}
knob = extrude(s.c, h)
";

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-restage-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("asm.fg"), ASM).unwrap();
    std::fs::write(d.join("knob.fg"), KNOB).unwrap();
    d
}

/// The assembly open, built and on screen; `None` with no GPU.
fn open(d: &std::path::Path) -> Option<Headless> {
    let mut h = Headless::new((960.0, 600.0), 1.0).ok()?;
    h.gui.open(d.join("asm.fg").to_str().unwrap());
    h.settle();
    h.settle_worker();
    for _ in 0..10 {
        h.step();
    }
    assert!(h.gui.view.is_some(), "the assembly is on screen");
    Some(h)
}

/// What mesh each placement draws: a restage changes it (the plate's new mesh).
fn placed(h: &Headless) -> Vec<u32> {
    h.gui
        .view
        .as_ref()
        .unwrap()
        .scene
        .placements
        .iter()
        .map(|p| p.part)
        .collect()
}

/// `L` set, the build back and the view made again.
fn edit(h: &mut Headless, value: f64) {
    let before = placed(h);
    let op = Op::SetParam {
        param: ParamKey {
            owner: Vec::new(),
            name: "L".into(),
        },
        value: Literal {
            value,
            dim: Dim::Length,
        },
    };
    h.gui.fg_apply(op).unwrap();
    for _ in 0..2000 {
        h.step();
        h.settle_worker();
        if placed(h) != before {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_ne!(placed(h), before, "the edit restaged the view");
    for _ in 0..5 {
        h.step();
    }
}

#[test]
fn occlusion_turned_off_stays_off_across_an_edit() {
    let d = scratch("ao");
    let Some(mut h) = open(&d) else { return };
    // what `O` does (KeyCmd::Ao)
    h.gui.view.as_mut().unwrap().renderer.ao = false;
    h.step();
    edit(&mut h, 140.0);
    assert!(!h.gui.view.as_ref().unwrap().renderer.ao, "occlusion stays off");
    edit(&mut h, 160.0);
    assert!(!h.gui.view.as_ref().unwrap().renderer.ao, "and again");
}

#[test]
fn a_background_picture_stays_across_an_edit() {
    let d = scratch("image");
    // a 4 x 2 picture
    let png = d.join("back.png");
    {
        let f = std::fs::File::create(&png).unwrap();
        let mut e = png::Encoder::new(std::io::BufWriter::new(f), 4, 2);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header()
            .unwrap()
            .write_image_data(&[200u8; 4 * 2 * 4])
            .unwrap();
    }
    let Some(mut h) = open(&d) else { return };
    let mut s = h.gui.shell.settings.clone();
    s.backdrop = forge_render::look::Backdrop::Image;
    s.image = png.to_str().unwrap().into();
    h.gui.shell.use_settings(s);
    for _ in 0..5 {
        h.step();
    }
    let size = |h: &Headless| h.gui.view.as_ref().unwrap().renderer.background_image_size();
    assert_eq!(size(&h), (4, 2), "the picture is shown");
    edit(&mut h, 140.0);
    assert_eq!(size(&h), (4, 2), "the picture stays");
}
