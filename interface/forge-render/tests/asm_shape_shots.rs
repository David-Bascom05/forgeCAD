//! Pictures for UX-LEAD of an assembly's checks after its shape, only with SHAPE_FIRST_SHOTS=DIR:
//! hinge_pin.fg, copied with rack18u/hw to a temporary folder with the pin's thread diameter
//! driven by size_m6 (`with: {d: size_m6}`: in the example it drives only the fit rule), grown to
//! 6.3 and then 6.35: each a new pin, so each check is a slow exact screw intersection on main's
//! kernel. The Issues window after the checks, during the next edit's checks (the clash kept,
//! "checking", with the spinner), and after them. Edits go through the session's typed ops, as the app's do.
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::Dim;
use forge_render::gui::{Gui, Headless};

fn shot(h: &Headless, dir: &str, name: &str) {
    std::fs::write(format!("{dir}/{name}.png"), h.png().unwrap()).unwrap();
}

fn stale(g: &Gui) -> bool {
    g.shell.fg.as_ref().is_some_and(|f| f.issues.iter().any(|i| i.stale))
}

fn checked_clash(g: &Gui) -> bool {
    g.fg.as_ref().is_some_and(|s| s.done == s.seq)
        && g.shell.fg.as_ref().is_some_and(|f| f.issues.iter().any(|i| i.kind == "Interference" && !i.stale))
}

fn thread(h: &mut Headless, d: f64) {
    let op = Op::SetParam {
        param: ParamKey { owner: vec![], name: "size_m6".into() },
        value: Literal { value: d, dim: Dim::Length },
    };
    h.gui.fg.as_mut().unwrap().apply(op).expect("the edit applies");
}

fn copy(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

#[test]
fn shots_of_shape_first() {
    let Ok(dir) = std::env::var("SHAPE_FIRST_SHOTS") else {
        return;
    };
    let Ok(mut h) = Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    h.step();
    let hw = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rack18u/hw");
    let tmp = std::env::temp_dir().join(format!("fg-shape-shots-{}", std::process::id()));
    copy(&hw, &tmp.join("hw"));
    let f = tmp.join("hw/tests/hinge_pin.fg");
    let text = std::fs::read_to_string(&f).unwrap();
    let driven = text.replace("pin = part(\"../iso7379_8x12_m6.fg\")", "pin = part(\"../iso7379_8x12_m6.fg\", with: {d: size_m6})");
    assert_ne!(driven, text);
    std::fs::write(&f, driven).unwrap();
    h.gui.open(f.to_str().unwrap());
    assert!(h.settle_worker());
    h.gui.fg.as_mut().unwrap().check_overlap();
    thread(&mut h, 6.3);
    assert!(h.until(checked_clash), "{}", h.gui.shell.status);
    if let Some(p) = h.gui.shell.fg.as_mut() {
        p.issues_list = true;
    }
    for _ in 0..20 {
        h.step();
    }
    shot(&h, &dir, "01-checked");
    // the next edit: its shape at once, the clash kept while its checks run
    thread(&mut h, 6.35);
    assert!(h.until(stale), "{}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
    shot(&h, &dir, "02-checking");
    assert!(h.until(|g| !stale(g) && checked_clash(g)));
    for _ in 0..5 {
        h.step();
    }
    shot(&h, &dir, "03-checked-again");
}
