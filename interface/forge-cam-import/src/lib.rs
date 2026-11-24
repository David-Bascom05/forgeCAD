//! Fusion tool library JSON (`{ "data": [tool, ..], "version": N }`, research/cam/libraries-tools.md
//! §3.3) to our `*.tools.toml` (§5.2), plus `[[assembly]]` records from each tool's post-process
//! number and length below holder.
//!
//! Interoperability only: the format is read to import the user's own library. The output is
//! our TOML, checked by reading it back with forge-tooling. What is not mapped is a warning,
//! never a guess:
//! - unknown tool types are skipped, named;
//! - Fusion's `TA` on chamfer and engraving tools is read as the half angle (the file convention
//!   the research found, against the schema's text): the included angle written is 2·TA, and a
//!   warning says so;
//! - Fusion `LB` is the length below the holder, which is our stick-out (not ISO LB).
use forge_agent::json::Json;

#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    pub toml: String,
    pub tools: usize,
    pub warnings: Vec<String>,
}

fn kind_of(t: &str) -> Option<&'static str> {
    Some(match t {
        "flat end mill" | "face mill" | "slot mill" => "flat",
        "ball end mill" => "ball",
        "bull nose end mill" | "torus mill" => "bull",
        "chamfer mill" => "chamfer",
        "spot drill" | "center drill" => "spot",
        "drill" => "drill",
        "tap right hand" | "tap left hand" => "tap",
        "thread mill" => "thread_mill",
        "reamer" => "reamer",
        "boring bar" => "bore",
        "counter bore" => "counterbore",
        "counter sink" => "countersink",
        "engrave mill" | "engraving" => "engrave",
        _ => return None,
    })
}

fn num(g: Option<&Json>, k: &str) -> Option<f64> {
    g.and_then(|g| g.get(k)).and_then(|v| v.num())
}

fn q(s: &str) -> String {
    format!("{s:?}")
}

/// How [`fusion_with`] writes the records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opts {
    /// Cite every value to the file (`user` cites: the user's own file and its version), ids from
    /// the maker and product id, and refuse what would need an assumption (a drill with no SIG).
    pub cite: bool,
    /// The tools' start values as `[[tool.preset]]`, grouped by ISO 513 from the preset's name.
    pub presets: bool,
    /// `[[assembly]]` records from each tool's post-process number (not for number 0).
    pub assemblies: bool,
}

/// Converts a Fusion tool library. `source` names the file, `taken` the date, for the records.
pub fn fusion(src: &str, source: &str, taken: &str) -> Result<Imported, String> {
    fusion_with(
        src,
        source,
        taken,
        Opts {
            cite: false,
            presets: false,
            assemblies: true,
        },
    )
}

/// The ISO 513 group a material's name names (P M K N S H, O for plastics and composites), as a
/// preset's group; `None` when it names none.
pub fn iso_group(name: &str) -> Option<&'static str> {
    let m = name.to_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| m.contains(w));
    if has(&["copper alloy", "brass", "bronze"]) {
        return Some("N");
    }
    if has(&[
        "titan",
        "inconel",
        "nickel",
        "monel",
        "hastelloy",
        "waspaloy",
        "superalloy",
        "cobalt alloy",
    ]) {
        return Some("S");
    }
    if has(&["stainless", "duplex", "austenit"]) {
        return Some("M");
    }
    if has(&["cast iron", "gray iron", "grey iron", "ductile", "nodular"]) {
        return Some("K");
    }
    if has(&["harden", "hrc 4", "hrc 5", "hrc 6"]) && has(&["steel"]) {
        return Some("H");
    }
    if has(&["alumin", "copper", "magnes", "zinc", "non-ferrous", "nonferrous"]) {
        return Some("N");
    }
    if has(&[
        "plastic",
        "acrylic",
        "acetal",
        "delrin",
        "nylon",
        "polycarbonate",
        "peek",
        "ptfe",
        "uhmw",
        "cfrp",
        "carbon fiber",
        "composite",
        "graphite",
    ]) {
        return Some("O");
    }
    if has(&["steel", "carbon", "iron"]) {
        return Some("P");
    }
    None
}

/// Converts a Fusion tool library as [`Opts`] says.
pub fn fusion_with(src: &str, source: &str, taken: &str, opts: Opts) -> Result<Imported, String> {
    let j = Json::parse(src)?;
    let data = match j.get("data") {
        Some(Json::Arr(a)) => a,
        _ => return Err("not a Fusion tool library: no `data` array".into()),
    };
    let version = num(Some(&j), "version").unwrap_or(0.0);
    let mut out = String::new();
    let mut warn = vec![];
    let mut tools = 0;
    out.push_str(&format!("# Imported from a Fusion tool library ({source}, version {version}) on {taken}.\n# Restated as our records; verify with the vendor.\n\n"));
    let mut asm = String::new();
    let mut ids = std::collections::BTreeSet::new();
    let src_label = format!("{source} (Fusion tool library version {version})");
    for (k, t) in data.iter().enumerate() {
        let ty = t.get("type").and_then(|x| x.str()).unwrap_or("");
        let Some(kind) = kind_of(ty) else {
            if ty != "holder" && ty != "probe" {
                warn.push(format!("tool {k}: type `{ty}` is not imported"));
            }
            continue;
        };
        let g = t.get("geometry");
        let inch = t
            .get("unit")
            .and_then(|x| x.str())
            .is_some_and(|u| u.starts_with("inch"));
        let s = if inch { 25.4 } else { 1.0 };
        let need = |key: &str| num(g, key).map(|v| v * s);
        let desc = t.get("description").and_then(|x| x.str()).unwrap_or("");
        let vendor = t.get("vendor").and_then(|x| x.str()).unwrap_or("");
        let pid = t.get("product-id").and_then(|x| x.str()).unwrap_or("");
        let mut id = if opts.cite && !pid.is_empty() {
            slug(&format!("{vendor}-{pid}"))
        } else {
            format!(
                "{}-{}",
                kind,
                if desc.is_empty() {
                    format!("{k}")
                } else {
                    desc.replace(' ', "-").to_lowercase()
                }
            )
        };
        if opts.cite {
            let mut n = 2;
            let base = id.clone();
            while !ids.insert(id.clone()) {
                id = format!("{base}-{n}");
                n += 1;
            }
        }
        let unit = if inch { "in" } else { "mm" };
        let mut cites: Vec<(String, String)> = vec![];
        let geo = |key: &str| num(g, key).map(|v| format!("geometry.{key} {v} {unit}"));
        let (Some(dc), Some(lcf), Some(oal)) = (need("DC"), need("LCF"), need("OAL")) else {
            warn.push(format!(
                "tool {k} (`{desc}`): DC, LCF or OAL missing: not imported"
            ));
            continue;
        };
        let dcon = need("SFDM").unwrap_or(dc);
        for (p, key) in [("dc", "DC"), ("lcf", "LCF"), ("oal", "OAL"), ("dcon", "SFDM")] {
            if let Some(v) = geo(key) {
                cites.push((p.into(), v));
            }
        }
        if need("SFDM").is_none() {
            cites.push(("dcon".into(), "no SFDM: the shank taken as DC".into()));
        }
        if let Some(v) = geo("NOF") {
            cites.push(("nof".into(), v));
        }
        let mut r = format!(
            "[[tool]]\nid = {}\ntype = {}\ndc = {dc}\nlcf = {lcf}\ndcon = {dcon}\noal = {oal}\n",
            q(&id),
            q(kind)
        );
        if let Some(n) = num(g, "NOF") {
            r.push_str(&format!("nof = {n}\n"));
        }
        if kind == "bull" {
            match need("RE") {
                Some(re) => {
                    r.push_str(&format!("re = {re}\n"));
                    if let Some(v) = geo("RE") {
                        cites.push(("re".into(), v));
                    }
                }
                None => {
                    warn.push(format!("tool {k}: a bull nose with no RE: not imported"));
                    continue;
                }
            }
        }
        match kind {
            "drill" | "spot" | "countersink" => {
                if opts.cite && num(g, "SIG").is_none() {
                    warn.push(format!("tool {k} (`{desc}`): no SIG: not imported"));
                    continue;
                }
                let sig = num(g, "SIG").unwrap_or(if kind == "drill" { 118.0 } else { 90.0 });
                if num(g, "SIG").is_none() {
                    warn.push(format!("tool {k}: no SIG; {sig}° assumed: check it"));
                }
                cites.push((
                    if kind == "drill" { "sig" } else { "angle" }.into(),
                    format!("geometry.SIG {sig}"),
                ));
                r.push_str(&format!(
                    "{} = {sig}\n",
                    if kind == "drill" { "sig" } else { "angle" }
                ));
            }
            "chamfer" | "engrave" => {
                let ta = num(g, "TA").ok_or(format!("tool {k}: a {kind} with no TA"))?;
                r.push_str(&format!(
                    "angle = {}\ntip_d = {}\n",
                    2.0 * ta,
                    need("tip-diameter").unwrap_or(0.0)
                ));
                warn.push(format!(
                    "tool {k}: TA {ta} read as the half angle (included {})",
                    2.0 * ta
                ));
                cites.push((
                    "angle".into(),
                    format!("geometry.TA {ta}, read as the half angle"),
                ));
                cites.push((
                    "tip_d".into(),
                    format!("geometry.tip-diameter {}", num(g, "tip-diameter").unwrap_or(0.0)),
                ));
            }
            "tap" | "thread_mill" => match need("TP") {
                Some(tp) => {
                    r.push_str(&format!("tp = {tp}\n"));
                    cites.push(("tp".into(), format!("geometry.TP {} {unit}", tp / s)));
                }
                None => {
                    warn.push(format!("tool {k}: a {kind} with no TP: not imported"));
                    continue;
                }
            },
            _ => {}
        }
        if ty == "tap left hand" {
            r.push_str("hand = \"left\"\n");
        }
        for (key, ours) in [("vendor", "vendor"), ("product-id", "product_id")] {
            if let Some(v) = t.get(key).and_then(|x| x.str()) {
                r.push_str(&format!("{ours} = {}\n", q(v)));
            }
        }
        r.push_str(&format!(
            "source = {}\ntaken = {}\nnote = \"imported; verify with the vendor\"\n",
            q(&format!("Fusion library {source}")),
            q(taken)
        ));
        let mut presets = vec![];
        if opts.presets {
            presets = start_values(t, inch, dc, &mut warn, k);
            for p in &presets {
                r.push_str(&p.0);
            }
        }
        if opts.cite {
            for (path, value) in cites.iter().chain(presets.iter().map(|p| &p.1)) {
                r.push_str(&format!(
                    "[[tool.cite]]\npath = {}\nstatus = \"user\"\nsource = {}\ntaken = {}\nvalue = {}\n",
                    q(path),
                    q(&src_label),
                    q(taken),
                    q(value)
                ));
            }
        }
        r.push('\n');
        // a record our reader refuses (e.g. OAL no longer than the flutes) is left out, named
        if let Err(e) = forge_tooling::Library::default().add("import", &r) {
            warn.push(format!("tool {k} (`{desc}`): not imported: {e}"));
            continue;
        }
        out.push_str(&r);
        tools += 1;
        // the assembly: T from post-process, stick-out from LB (length below holder)
        let pp = t.get("post-process");
        if let Some(n) = num(pp, "number").filter(|n| opts.assemblies && (!opts.cite || *n > 0.0)) {
            let lb = num(g, "LB").map(|v| v * s).unwrap_or(lcf + 5.0);
            if num(g, "LB").is_none() {
                warn.push(format!("tool {k}: no LB; stick-out set to the flutes + 5 mm"));
            }
            asm.push_str(&format!(
                "[[assembly]]\nt = {n}\ntool = {}\nstickout = {lb}\n\n",
                q(&id)
            ));
        }
    }
    out.push_str(&asm);
    // read it back: our own reader must take what we wrote
    let mut lib = forge_tooling::Library::default();
    lib.add("imported", &out)
        .map_err(|e| format!("the converted library does not read back: {e}"))?;
    Ok(Imported {
        toml: out,
        tools,
        warnings: warn,
    })
}

/// An id from free text: lower-case letters, digits and dashes.
fn slug(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '.' {
            o.push(c.to_ascii_lowercase());
        } else if !o.ends_with('-') {
            o.push('-');
        }
    }
    o.trim_matches('-').to_string()
}

/// A tool's Fusion start values as `[[tool.preset]]` text and its cite (path, value): those with
/// a cutting speed and a feed per tooth (or per revolution, a drill), grouped by ISO 513 from the
/// preset's name (a preset whose name names no group, or whose v_c is not pi D n, is left out,
/// with a warning). Inch
/// libraries' speeds are ft/min and feeds inches: converted exactly.
fn start_values(
    t: &Json,
    inch: bool,
    dc: f64,
    warn: &mut Vec<String>,
    k: usize,
) -> Vec<(String, (String, String))> {
    let mut out = vec![];
    let mut names = std::collections::BTreeSet::new();
    let Some(Json::Arr(ps)) = t.get("start-values").and_then(|s| s.get("presets")) else {
        return out;
    };
    let (vs, ls) = if inch { (0.3048, 25.4) } else { (1.0, 1.0) };
    for p in ps {
        let name = p.get("name").and_then(|x| x.str()).unwrap_or("");
        let vc = num(Some(p), "v_c").unwrap_or(0.0);
        let fz = num(Some(p), "f_z")
            .filter(|f| *f > 0.0)
            .or_else(|| num(Some(p), "f_n"))
            .unwrap_or(0.0);
        if vc <= 0.0 || fz <= 0.0 {
            continue;
        }
        // the speed must be the spindle speed's at this diameter (pi D n), within 5%: a preset
        // whose numbers disagree with each other is not imported
        if let Some(n) = num(Some(p), "n").filter(|n| *n > 0.0) {
            let v = std::f64::consts::PI * dc * n / 1000.0;
            if ((vc * vs) - v).abs() > 0.05 * v {
                warn.push(format!(
                    "tool {k}: preset `{name}`: v_c disagrees with n at this DC: left out"
                ));
                continue;
            }
        }
        let Some(g) = iso_group(name) else {
            warn.push(format!(
                "tool {k}: preset `{name}` names no ISO 513 group: left out"
            ));
            continue;
        };
        let pname = format!("{g} {}", name.trim());
        if !names.insert(pname.clone()) {
            continue;
        }
        let on = |flag: &str, key: &str| {
            (p.get(flag).and_then(|x| match x {
                Json::Bool(b) => Some(*b),
                _ => None,
            }) == Some(true))
            .then(|| num(Some(p), key))
            .flatten()
            .filter(|v| *v > 0.0)
        };
        let mut s = format!(
            "[[tool.preset]]\nname = {}\ngroup = {}\nvc = {}\nfz = {}\n",
            q(&pname),
            q(g),
            vc * vs,
            fz * ls
        );
        let (ap, ae) = (on("use-stepdown", "stepdown"), on("use-stepover", "stepover"));
        if let Some(v) = ap {
            s.push_str(&format!("ap = {}\n", v * ls));
        }
        if let Some(v) = ae {
            s.push_str(&format!("ae = {}\n", v * ls));
        }
        let u = if inch { ("ft/min", "in") } else { ("m/min", "mm") };
        out.push((
            s,
            (
                format!("preset.{pname}"),
                format!("start values `{name}`: v_c {vc} {}, f {fz} {}", u.0, u.1),
            ),
        ));
    }
    out
}

/// Every Fusion tool library (`*.json`) in a directory, imported with cites and presets (the
/// opt-in runtime load of a cache, e.g. tools/fetch-tool-libraries.sh's: nothing is written):
/// (file name, the import).
pub fn load_dir(dir: &std::path::Path, taken: &str) -> Result<Vec<(String, Imported)>, String> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut out = vec![];
    for f in files {
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let src = std::fs::read_to_string(&f).map_err(|e| format!("{name}: {e}"))?;
        let imp = fusion_with(
            &src,
            &name,
            taken,
            Opts {
                cite: true,
                presets: true,
                assemblies: false,
            },
        )
        .map_err(|e| format!("{name}: {e}"))?;
        out.push((name, imp));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // a hand-written library in Fusion's layout (our own numbers, not a vendor's file)
    const SAMPLE: &str = r#"{"version": 36, "data": [
      {"guid": "a", "type": "flat end mill", "unit": "millimeters", "description": "6mm 3fl",
       "geometry": {"DC": 6, "LCF": 18, "OAL": 57, "SFDM": 6, "NOF": 3, "LB": 24},
       "post-process": {"number": 2}, "start-values": {}},
      {"guid": "b", "type": "bull nose end mill", "unit": "inches", "description": "half inch",
       "geometry": {"DC": 0.5, "LCF": 1.0, "OAL": 3.0, "SFDM": 0.5, "RE": 0.03, "NOF": 4},
       "post-process": {"number": 3}, "start-values": {}},
      {"guid": "c", "type": "chamfer mill", "unit": "millimeters", "description": "vee",
       "geometry": {"DC": 10, "LCF": 8, "OAL": 63, "SFDM": 10, "TA": 45, "tip-diameter": 1},
       "post-process": {"number": 4}, "start-values": {}},
      {"guid": "d", "type": "dovetail mill", "unit": "millimeters", "geometry": {}, "post-process": {}, "start-values": {}},
      {"guid": "e", "type": "drill", "unit": "millimeters", "description": "6.8",
       "geometry": {"DC": 6.8, "LCF": 69, "OAL": 109, "SIG": 118}, "post-process": {"number": 5}, "start-values": {}}
    ]}"#;

    #[test]
    fn converts_and_reads_back() {
        let r = fusion(SAMPLE, "sample.json", "2026-11-23").unwrap();
        assert_eq!(r.tools, 4, "{:?}", r.warnings);
        let mut lib = forge_tooling::Library::default();
        lib.add("x", &r.toml).unwrap();
        let bull = lib
            .tools
            .iter()
            .find(|t| t.kind == forge_tooling::Kind::Bull)
            .unwrap();
        assert!((bull.dc - 12.7).abs() < 1e-12 && (bull.re - 0.762).abs() < 1e-12);
        let ch = lib
            .tools
            .iter()
            .find(|t| t.kind == forge_tooling::Kind::Chamfer)
            .unwrap();
        assert_eq!(ch.angle, 90.0);
        assert_eq!(lib.assemblies.iter().find(|a| a.t == 2).unwrap().stickout, 24.0);
        assert!(r.warnings.iter().any(|w| w.contains("dovetail")));
        assert!(r.warnings.iter().any(|w| w.contains("half angle")));
        // every imported tool makes an exact shape
        for t in &lib.tools {
            t.shape(t.lcf + 5.0, None).unwrap();
        }
    }

    #[test]
    fn cited_import_with_presets() {
        // our own numbers in Fusion's layout: a maker, a product id, a preset naming a material,
        // a preset naming none, and a drill with no SIG (refused when citing: no assumption)
        let src = r#"{"version": 36, "data": [
          {"type": "flat end mill", "unit": "inches", "vendor": "Acme", "product-id": "E-1",
           "geometry": {"DC": 0.5, "LCF": 1.0, "OAL": 3.0, "SFDM": 0.5, "NOF": 3},
           "post-process": {"number": 0},
           "start-values": {"presets": [
             {"name": "6061 Aluminum", "v_c": 1000, "f_z": 0.002, "n": 7639.4,
              "use-stepdown": true, "stepdown": 0.25, "use-stepover": false, "stepover": 0.1},
             {"name": "Default preset", "v_c": 500, "f_z": 0.001}]}},
          {"type": "drill", "unit": "millimeters", "vendor": "Acme", "product-id": "D-6",
           "geometry": {"DC": 6, "LCF": 40, "OAL": 80}, "post-process": {}, "start-values": {}}
        ]}"#;
        let opts = Opts {
            cite: true,
            presets: true,
            assemblies: false,
        };
        let r = fusion_with(src, "acme.json", "2026-11-24", opts).unwrap();
        assert_eq!(r.tools, 1, "{:?}", r.warnings);
        let mut lib = forge_tooling::Library::default();
        lib.add("x", &r.toml).unwrap();
        let t = lib.tool("acme-e-1").unwrap();
        assert!((t.dc - 12.7).abs() < 1e-12 && (t.oal - 76.2).abs() < 1e-12);
        let p = &t.presets[0];
        assert_eq!((p.name.as_str(), p.group.as_str()), ("N 6061 Aluminum", "N"));
        assert!((p.vc - 304.8).abs() < 1e-9 && (p.fz - 0.0508).abs() < 1e-12);
        assert_eq!((p.ap, p.ae), (Some(6.35), None));
        assert_eq!(t.presets.len(), 1);
        let c = t.cites.iter().find(|c| c.path == "dc").unwrap();
        assert_eq!(c.status, forge_tooling::CiteStatus::User);
        assert!(c.source.contains("acme.json") && c.source.contains("version 36"));
        assert!(r.warnings.iter().any(|w| w.contains("no SIG: not imported")));
        assert!(r.warnings.iter().any(|w| w.contains("names no ISO 513 group")));
        assert!(lib.assemblies.is_empty());
    }
}
