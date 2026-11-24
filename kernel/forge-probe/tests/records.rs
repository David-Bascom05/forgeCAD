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
feed = [100, 3000]
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
        ("feed = [100, 3000]", "feed = [3000, 100]", "feed"),
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
