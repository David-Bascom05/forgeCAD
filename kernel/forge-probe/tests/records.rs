//! The reader's refusals, the canonical writer, and the built-in records.
use forge_probe::{DeviceKind, Sensing, builtin, parse, write};

/// A made-up record (real = false: the values are the test's own, not a product's).
const TOY: &str = r#"
[[probe]]
id = "toy-probe"
kind = "work-probe"
sensing = "contact"
transmission = "optical"
repeat = 0.002
max_feed = 3000
overtravel = [10, 5]
stylus = [6, 50]
[[probe.cyl]]
base = [0, 0, 50]
axis = [0, 0, 1]
r = 20
h = 50
[[probe.cite]]
path = "repeat"
status = "user"
"#;

#[test]
fn a_record_reads_and_writes_back_byte_stable() {
    let ds = parse(TOY).unwrap();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].kind, DeviceKind::WorkProbe);
    assert_eq!(ds[0].sensing, Sensing::Contact);
    let w = write::file("header", &ds);
    let back = parse(&w).unwrap();
    assert_eq!(back, ds);
    assert_eq!(write::file("header", &back), w);
}

#[test]
fn an_absent_value_needs_an_unknown_cite() {
    let src = TOY.replace("repeat = 0.002\n", "");
    let e = parse(&src).unwrap_err().to_string();
    assert!(e.contains("`repeat` is absent"), "{e}");
    let ok = src.replace(
        "path = \"repeat\"\nstatus = \"user\"",
        "path = \"repeat\"\nstatus = \"unknown\"\nbasis = \"not published\"",
    );
    let d = &parse(&ok).unwrap()[0];
    assert_eq!(d.repeat, None);
    assert_eq!(d.not_checked(), vec!["repeat".to_string()]);
}

#[test]
fn refusals_name_the_field() {
    for (from, to, want) in [
        ("kind = \"work-probe\"", "kind = \"wand\"", "kind"),
        ("sensing = \"contact\"", "sensing = \"laser\"", "sensing"),
        ("max_feed = 3000", "feed = [3000, 100]", "feed"),
        ("stylus = [6, 50]", "stylus = [6, 2]", "stylus"),
        ("r = 20", "r = -1", "`r`"),
        ("repeat = 0.002", "repeat = 0.002\ndisc_d = 12", "disc_d"),
        ("repeat = 0.002", "repeat = 0.002\ncolour = \"red\"", "colour"),
        ("status = \"user\"", "status = \"rumour\"", "status"),
    ] {
        let e = parse(&TOY.replace(from, to)).unwrap_err().to_string();
        assert!(e.contains(want), "{to}: {e}");
    }
    let real = TOY.replace("sensing = \"contact\"", "sensing = \"contact\"\nreal = true");
    let e = parse(&real).unwrap_err().to_string();
    assert!(e.contains("uncited"), "{e}");
}

#[test]
fn the_builtin_records_read_and_write_back() {
    let ds = builtin();
    let w = write::file("", &ds);
    assert_eq!(parse(&w).unwrap(), ds);
    for d in &ds {
        assert!(d.real, "{}: built-in records are real products", d.id);
    }
}

#[test]
fn the_renishaw_sheets_values() {
    let ds = builtin();
    let f = |id: &str| forge_probe::find(&ds, id).unwrap().clone();
    let p = f("renishaw-omp40-2");
    assert_eq!(
        (p.repeat, p.overtravel, p.max_feed),
        (Some(0.001), Some([12.5, 6.0]), None)
    );
    assert!(p.not_checked().contains(&"max_feed".to_string()));
    let t = f("renishaw-ts27r");
    assert_eq!(
        (t.kind, t.disc_d, t.trigger),
        (DeviceKind::ToolSetter, Some(12.7), None)
    );
    assert!(t.bodies.is_empty());
    let e = forge_probe::find(&ds, "renishaw-omp40").unwrap_err().to_string();
    assert!(e.contains("renishaw-omp40-2"), "{e}");
}

#[test]
fn body_rings_read_write_and_outline() {
    let src = TOY.replace(
        "[[probe.cyl]]\nbase = [0, 0, 50]\naxis = [0, 0, 1]\nr = 20\nh = 50\n",
        "body = [[20, 50, 100], [8, 100, 120]]\n[[probe.box]]\nmin = [-1, -1, -1]\nmax = [1, 1, 0]\n",
    );
    let ds = parse(&src).unwrap();
    let w = write::file("", &ds);
    assert!(w.contains("body = [[20, 50, 100], [8, 100, 120]]"), "{w}");
    assert_eq!(parse(&w).unwrap(), ds);
    // a box is not turned: no outline
    assert_eq!(forge_probe::outline(&ds[0]), None);
    let toy = parse(TOY).unwrap();
    assert_eq!(
        forge_probe::outline(&toy[0]),
        Some(vec![[50.0, 0.0], [50.0, 20.0], [100.0, 20.0], [100.0, 0.0]])
    );
    // the built-in work probes have outlines
    let b = builtin();
    assert!(forge_probe::outline(forge_probe::find(&b, "renishaw-omp40-2").unwrap()).is_some());
}
