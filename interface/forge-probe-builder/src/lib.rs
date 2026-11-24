//! forge-probe's devices (kernel/forge-probe: spindle work probes, contact and laser tool
//! setters) as forge-library kinds, for the shared library builder
//! (research/cam/library-builder.md). Both kinds live in one folder, `cam/probes`, as `[[probe]]`
//! records; each kind's parse reads only its own (`kind = "work-probe"` or `"tool-setter"`).
//! A record is forge-probe's typed `Device`, written by its canonical writer.
use forge_library::Kind;
use forge_library::schema::{Field, Ty, Unit};
use forge_probe::{Device, DeviceKind};

pub use forge_probe::Device as Rec;

fn device_kind(word: &str) -> Option<DeviceKind> {
    match word {
        "work_probe" => Some(DeviceKind::WorkProbe),
        "tool_setter" => Some(DeviceKind::ToolSetter),
        _ => None,
    }
}

/// The records of a kind (`work_probe` or `tool_setter`) in a file's text; any record forge-probe
/// refuses is an error, where and why.
pub fn parse_kind(word: &str, text: &str) -> Result<Vec<Device>, String> {
    let k = device_kind(word).ok_or_else(|| format!("no probe kind `{word}`"))?;
    Ok(forge_probe::parse(text)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|d| d.kind == k)
        .collect())
}

/// A file of records, canonical; what is written must read back as what was given.
pub fn write_kind(word: &str, header: &str, rs: &[Device]) -> Result<String, String> {
    let k = device_kind(word).ok_or_else(|| format!("no probe kind `{word}`"))?;
    if let Some(d) = rs.iter().find(|d| d.kind != k) {
        return Err(format!("`{}` is a {}, not a {word}", d.id, d.kind.word()));
    }
    let out = forge_probe::write::file(header, rs);
    if parse_kind(word, &out)? != rs {
        return Err("the records do not write canonically".into());
    }
    Ok(out)
}

/// The built-in records of a kind, one file of them.
pub fn builtin_of(word: &str) -> Vec<(String, String)> {
    let Some(k) = device_kind(word) else {
        return Vec::new();
    };
    let rs: Vec<Device> = forge_probe::builtin()
        .into_iter()
        .filter(|d| d.kind == k)
        .collect();
    if rs.is_empty() {
        return Vec::new();
    }
    vec![(format!("{word}s.toml"), forge_probe::write::file("", &rs))]
}

macro_rules! kinds {
    ($($ty:ident, $word:literal, $title:literal;)*) => {
        $(
            #[doc = concat!("The `", $word, "` kind.")]
            pub struct $ty;
            impl Kind for $ty {
                type Record = Device;
                const WORD: &'static str = $word;
                const FOLDER: &'static str = "probes";
                fn builtin() -> Vec<(String, String)> {
                    builtin_of($word)
                }
                fn parse(text: &str) -> Result<Vec<Device>, String> {
                    parse_kind($word, text)
                }
                fn write(header: &str, rs: &[Device]) -> Result<String, String> {
                    write_kind($word, header, rs)
                }
                fn id(r: &Device) -> &str {
                    &r.id
                }
            }
        )*
        /// Every kind: (word, title).
        pub const KINDS: &[(&str, &str)] = &[$(($word, $title)),*];
    };
}

kinds! {
    WorkProbes, "work_probe", "Spindle work probe";
    ToolSetters, "tool_setter", "Tool setter";
}

/// The form's fields of a kind. Paths are forge-probe's keys and cite paths; `body` rows are
/// (r, from, to) along the device's +z from its origin (a work probe: the stylus mounting face,
/// toward the spindle; a setter: its mounting face on the table, up), as setup.tool.body's rows
/// run along the tool axis.
pub fn fields(word: &str) -> Vec<Field> {
    let Some(k) = device_kind(word) else {
        return Vec::new();
    };
    let rec =
        |p: &'static str, l: &'static str, ty: Ty, u: Unit| Field::new(p, l, ty, u).group("Record").uncited();
    let mut v = vec![
        rec("id", "Id", Ty::Text, Unit::None),
        rec("vendor", "Vendor", Ty::Text, Unit::None).optional("not stated"),
        rec("product", "Product", Ty::Text, Unit::None).optional("not stated"),
        rec("note", "Note", Ty::Text, Unit::None).optional("none"),
        rec("real", "A real product", Ty::Bool, Unit::None).optional("false: every value is the user's"),
    ];
    let g = "Sensing";
    if k == DeviceKind::ToolSetter {
        v.push(
            Field::new(
                "sensing",
                "Sensing",
                Ty::Choice(&["contact", "laser"]),
                Unit::None,
            )
            .group(g)
            .uncited(),
        );
    }
    v.push(
        Field::new(
            "transmission",
            "Transmission",
            Ty::Choice(&["optical", "radio", "wired"]),
            Unit::None,
        )
        .group(g)
        .help("how the trigger reaches the control"),
    );
    v.push(
        Field::new("repeat", "Repeatability (2σ)", Ty::Num, Unit::Length)
            .group(g)
            .help("unidirectional trigger repeatability, 2σ"),
    );
    v.push(
        Field::new("max_feed", "Max probing feed", Ty::Num, Unit::Feed)
            .group(g)
            .help("the highest measuring feed the maker states; a data sheet's test velocity is not one"),
    );
    let geo = "Geometry";
    match k {
        DeviceKind::WorkProbe => {
            v.push(
                Field::new("overtravel", "Stylus overtravel", Ty::Vec(2), Unit::None)
                    .group(g)
                    .help("± degrees in XY, mm in +Z"),
            );
            v.push(
                Field::new("stylus", "Stylus", Ty::Vec(2), Unit::Length)
                    .group(geo)
                    .help("ball diameter and length from the mounting face to the ball centre, when the maker ships one")
                    .optional("the setup gives the stylus"),
            );
        }
        DeviceKind::ToolSetter => {
            v.push(
                Field::new("trigger", "Trigger point", Ty::Vec(3), Unit::Length)
                    .group(geo)
                    .help("the point a tool is brought to: a disc's top centre, a beam's crossing"),
            );
            v.push(
                Field::new("disc_d", "Disc diameter", Ty::Num, Unit::Length)
                    .group(geo)
                    .when("sensing = contact"),
            );
            v.push(
                Field::new("min_tool_d", "Smallest tool", Ty::Num, Unit::Length)
                    .group(geo)
                    .when("sensing = laser"),
            );
        }
    }
    v.push(
        Field::new(
            "body",
            "Body",
            Ty::Points(&[("r", Unit::Length), ("from", Unit::Length), ("to", Unit::Length)]),
            Unit::Length,
        )
        .group(geo)
        .help(
            "coaxial rings (radius, from, to) along the device's +z from its origin: its collision envelope",
        ),
    );
    v.push(
        Field::new("mass", "Mass", Ty::Num, Unit::Mass)
            .group(geo)
            .optional("not stated"),
    );
    v
}

/// Templates to start a new record from: every value empty and cited unknown would be refused
/// by nothing, so a template is a user record (`real = false`) with forgecad's placeholder
/// numbers to be replaced: a Ø40 probe body and a 50 mm stylus; a Ø60 contact setter.
pub fn templates() -> Vec<Device> {
    let src = r#"
[[probe]]
id = "template/work_probe"
kind = "work-probe"
note = "a template: replace every value with the maker's, cited"
sensing = "contact"
transmission = "optical"
repeat = 0.001
max_feed = 500
overtravel = [10, 5]
stylus = [6, 50]
body = [[20, 0, 60]]

[[probe]]
id = "template/tool_setter"
kind = "tool-setter"
note = "a template: replace every value with the maker's, cited"
sensing = "contact"
transmission = "wired"
repeat = 0.001
max_feed = 500
trigger = [0, 0, 80]
disc_d = 12
body = [[30, 0, 60], [6, 60, 80]]
"#;
    forge_probe::parse(src).expect("the templates read")
}

/// A check's finding.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub path: String,
    pub what: String,
    pub fix: String,
    /// A value not published: the checks needing it report not checked (not a failure).
    pub not_checked: bool,
}

/// The record checked by forge-probe's reader (refusals), then each value cited unknown or
/// estimated: the checks that need it report not checked.
pub fn check(r: &Device) -> Vec<Finding> {
    let text = forge_probe::write::file("", std::slice::from_ref(r));
    if let Err(e) = forge_probe::parse(&text) {
        return vec![Finding {
            path: String::new(),
            what: e.to_string(),
            fix: "correct the value or its cite".into(),
            not_checked: false,
        }];
    }
    r.not_checked()
        .into_iter()
        .map(|p| {
            let est = r.status(&p) == Some(forge_probe::CiteStatus::Estimate);
            Finding {
                what: if est {
                    format!("{p}: estimated; the checks needing it use the estimate and say so")
                } else {
                    format!("{p}: not published; the checks needing it report not checked")
                },
                fix: format!("enter the maker's published {p}, cited"),
                path: p,
                not_checked: true,
            }
        })
        .collect()
}

/// A turned outline to revolve: (z, r) points along the device's axis, mm.
pub fn preview(r: &Device) -> Result<Vec<[f64; 2]>, Finding> {
    forge_probe::outline(r).ok_or_else(|| Finding {
        path: "body".into(),
        what: if r.bodies.is_empty() {
            "no envelope (cited unknown): nothing to draw".into()
        } else {
            "the envelope is not coaxial rings: no turned preview".into()
        },
        fix: "give the body as rings (r, from, to)".into(),
        not_checked: r.bodies.is_empty(),
    })
}
