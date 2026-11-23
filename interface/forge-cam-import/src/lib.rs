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

/// Converts a Fusion tool library. `source` names the file, `taken` the date, for the records.
pub fn fusion(src: &str, source: &str, taken: &str) -> Result<Imported, String> {
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
        let id = format!(
            "{}-{}",
            kind,
            if desc.is_empty() {
                format!("{k}")
            } else {
                desc.replace(' ', "-").to_lowercase()
            }
        );
        let (Some(dc), Some(lcf), Some(oal)) = (need("DC"), need("LCF"), need("OAL")) else {
            warn.push(format!(
                "tool {k} (`{desc}`): DC, LCF or OAL missing: not imported"
            ));
            continue;
        };
        let dcon = need("SFDM").unwrap_or(dc);
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
                Some(re) => r.push_str(&format!("re = {re}\n")),
                None => {
                    warn.push(format!("tool {k}: a bull nose with no RE: not imported"));
                    continue;
                }
            }
        }
        match kind {
            "drill" | "spot" | "countersink" => {
                let sig = num(g, "SIG").unwrap_or(if kind == "drill" { 118.0 } else { 90.0 });
                if num(g, "SIG").is_none() {
                    warn.push(format!("tool {k}: no SIG; {sig}° assumed: check it"));
                }
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
            }
            "tap" | "thread_mill" => match need("TP") {
                Some(tp) => r.push_str(&format!("tp = {tp}\n")),
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
            "source = {}\ntaken = {}\nnote = \"imported; verify with the vendor\"\n\n",
            q(&format!("Fusion library {source}")),
            q(taken)
        ));
        out.push_str(&r);
        tools += 1;
        // the assembly: T from post-process, stick-out from LB (length below holder)
        let pp = t.get("post-process");
        if let Some(n) = num(pp, "number") {
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
}
