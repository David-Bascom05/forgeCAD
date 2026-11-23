//! A part built as `forgecad build F --json` builds it (the same options), its measures exact
//! (the volume's and area's bits): the same function natively and in the browser, so the two
//! kernels can be compared part by part (web/tests/parity.spec.mjs against `forge-parity`).

use forge_agent::json::Json;

fn num(x: f64) -> Json {
    Json::Num(x)
}

/// The measures as the page reports them, exact.
pub fn measure_fields(m: &forge_lang::Measure) -> Vec<(&'static str, Json)> {
    vec![
        ("volume", num(m.volume)),
        ("volume_bits", format!("{:016x}", m.volume.to_bits()).into()),
        ("area_bits", format!("{:016x}", m.area.to_bits()).into()),
        ("faces", num(m.faces as f64)),
        ("edges", num(m.edges as f64)),
        ("bodies", num(m.bodies as f64)),
        ("problems", num(m.problems as f64)),
    ]
}

/// The file at `path` built, its report as one JSON line.
pub fn build_report(path: &str) -> String {
    let src = match forge_dirs::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return Json::obj(vec![("error", format!("{path}: {e}").into())]).to_string(),
    };
    let opts = forge_lang::Options {
        build: true,
        dir: std::path::Path::new(path).parent().map(|d| d.to_path_buf()),
        fit_sizes: true,
        ..forge_lang::Options::default()
    };
    let (run, _) = forge_lang::run(&src, opts, Default::default());
    let mut f = vec![
        // the threads the kernel's parallel paths had (more than one only on a thread that may wait)
        ("threads", num(forge_math::par::threads() as f64)),
        ("ok", run.ok().into()),
        (
            "errors",
            Json::Arr(run.errors.iter().map(|e| e.text().into()).collect()),
        ),
    ];
    if let Some(m) = &run.measure {
        f.extend(measure_fields(m));
    }
    Json::obj(f).to_string()
}

/// The bench parts as files (`NAME.fg`) with the font their text is cut in.
pub fn bench_files() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = forge_lang::bench::NAMES
        .iter()
        .filter_map(|n| Some((format!("{n}.fg"), forge_lang::bench::src(n)?.as_bytes().to_vec())))
        .collect();
    v.push((
        forge_lang::bench::FONT.to_string(),
        forge_lang::bench::font_bytes(),
    ));
    v
}
