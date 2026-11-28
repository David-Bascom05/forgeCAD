use forge_lang::pcb::{self, Film, Rule, Shape, Side};
use std::path::Path;
const EXAMPLE: &str = include_str!("../../../examples/electronics/fabrication-controller.fg");
const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/electronics");
fn board(src: &str) -> pcb::Board {
    pcb::compile(src, Some(Path::new(DIR)), None).unwrap()
}
fn rules(src: &str) -> Vec<Rule> {
    board(src).check().findings.iter().map(|f| f.rule).collect()
}
fn replace(a: &str, b: &str) -> String {
    assert!(EXAMPLE.contains(a));
    EXAMPLE.replacen(a, b, 1)
}

#[test]
fn surface_images_follow_real_placed_lands_and_via_tenting() {
    let b = board(EXAMPLE);
    assert!(b.check().findings.is_empty(), "{:?}", b.check().findings);
    assert!(b.unplaced().is_empty());
    assert_eq!(b.layout.pads.len(), 6);
    assert_eq!(b.layout.pads["mcu.14"].at, [24_000_000, 6_000_000]);
    let top = b.film(Side::Top, Film::Mask);
    let bottom = b.film(Side::Bottom, Film::Mask);
    assert_eq!((top.len(), bottom.len()), (5, 4));
    assert!(top.iter().any(|m| m.id == "supply_via") && !bottom.iter().any(|m| m.id == "supply_via"));
    let paste = b.film(Side::Top, Film::Paste);
    assert_eq!(paste.len(), 2);
    assert_eq!(
        paste.iter().find(|m| m.id == "cap.1").unwrap().shape.bounds2(),
        [[22_700_000, 11_100_000], [25_300_000, 12_900_000]]
    );
    assert!(
        b.film(Side::Top, Film::Silk)
            .iter()
            .any(|m| m.id == "cap:legend0")
    );
    let moved = board(&replace(
        "at: (12,8), side: \"top\"",
        "at: (13,8), side: \"bottom\"",
    ));
    assert!(
        moved
            .film(Side::Bottom, Film::Silk)
            .iter()
            .any(|m| m.id == "cap:legend0")
    );
    assert!(moved.film(Side::Top, Film::Paste).is_empty());
}

#[test]
fn drilled_and_assembled_outputs_share_one_datum_and_are_deterministic() {
    let b = board(EXAMPLE);
    let files = b.fabrication_files(None).unwrap();
    assert_eq!(files, b.fabrication_files(None).unwrap());
    assert_eq!(files.keys().filter(|k| k.ends_with(".gbr")).count(), 12);
    assert!(files["top-mask.gbr"].contains("TF.FilePolarity,Negative"));
    assert!(files["bottom-paste.gbr"].contains("TF.FileFunction,Paste,Bot"));
    assert!(files["nonplated.drl"].contains("G00X2.000000Y2.000000\nM15\nG01X4.000000Y2.000000"));
    assert!(files["plated.drl"].contains("X18.000000Y6.000000"));
    assert!(files["placements.csv"].contains("bottom,24.000000,8.000000,180"));
    assert!(files.contains_key("bom.csv"));
    assert!(files.contains_key("review.json"));
    assert!(b.fabrication_files(Some("missing")).is_err());
}

#[test]
fn clearances_keepouts_and_cutouts_generate_object_specific_findings() {
    assert!(
        rules(&replace(
            "points: [(10.5,5),(13.5,5)]",
            "points: [(10.5,6),(13.5,6)]"
        ))
        .contains(&Rule::SilkToMask)
    );
    assert!(
        rules(&replace(
            "[(24,14),(25,14),(25,16),(24,16)]",
            "[(11,5),(13,5),(13,7),(11,7)]"
        ))
        .contains(&Rule::CopperInKeepout)
    );
    assert!(
        rules(&replace(
            "[(25,13),(28,13),(28,16),(25,16)]",
            "[(13,5),(14,5),(14,7),(13,7)]"
        ))
        .contains(&Rule::CopperInCutout)
    );
    assert!(rules(&replace("(24,10)])", "(22,10)])")).contains(&Rule::UnroutedNet));
    assert!(rules(&replace("min_width: 0.25mm", "min_width: 0.31mm")).contains(&Rule::MinTrackWidth));
    let same = format!(
        "{EXAMPLE}\nextra = mask_opening(side: \"top\", points: [(12.8,5.7),(13,5.7),(13,6.3),(12.8,6.3)])"
    );
    assert!(rules(&same).contains(&Rule::MinMaskWeb));
    assert!(
        !rules(&same.replace(
            "extra = mask_opening(side: \"top\"",
            "extra = mask_opening(side: \"bottom\""
        ))
        .contains(&Rule::MinMaskWeb)
    );
}

#[test]
fn pad_map_mismatches_and_bad_package_geometry_are_located() {
    for (a, b, kind) in [
        (
            "footprint: packages.header",
            "footprint: packages.device",
            "PadMapMismatch",
        ),
        (
            "occurrence: \"main/u\"",
            "occurrence: \"main/c\"",
            "DuplicatePlacement",
        ),
        (
            "footprints/controller.fg",
            "../../../Cargo.toml",
            "FootprintLibraryUnavailable",
        ),
        ("to: (4,2), drill: 1.6mm", "to: (4,2), drill: 1deg", "WrongDim"),
        (
            "clearance: 0.25mm, points:",
            "clearance: 0.25mm, thermal_gap: 0.2mm, points:",
            "IncompleteThermal",
        ),
    ] {
        let err = pcb::compile(&replace(a, b), Some(Path::new(DIR)), None).unwrap_err();
        assert_eq!(err.kind, kind);
        assert!(err.span.hi > err.span.lo);
    }
}

#[test]
fn pour_refill_can_split_and_rejoin_the_physical_net() {
    let added =
        format!("{EXAMPLE}\ndivider = keepout(layer: plane, points: [(13,2),(14,2),(14,14),(13,14)])");
    let b = board(&added);
    assert_eq!(b.layout.fills.len(), 2);
    let b = board(EXAMPLE);
    assert_eq!(b.layout.fills.len(), 1);
    let with_thermal = board(&replace(
        "clearance: 0.25mm, points:",
        "clearance: 0.25mm, thermal_gap: 0.24mm, spoke_width: 0.3mm, points:",
    ));
    assert!(
        with_thermal.check().findings.is_empty(),
        "{:?}",
        with_thermal.check().findings
    );
    assert_ne!(
        b.fabrication_files(None).unwrap()["copper-L2-plane.gbr"],
        with_thermal.fabrication_files(None).unwrap()["copper-L2-plane.gbr"]
    );
}

#[test]
fn explicit_stencil_windows_and_plated_slots_move_with_the_package() {
    let src="pcb
edge1 = edge(points: [(0,0),(10,0),(10,10),(0,10)])
a = copper(weight_oz_ft2: 1, thickness: 0.035mm)
d = dielectric(material: \"FR-4\", thickness: 1mm)
b = copper(weight_oz_ft2: 1, thickness: 0.035mm)
stack = stackup(layers: [a,d,b])
g = net(name: \"GND\")
pkg = footprint(pads: [pth(number: \"1\", shape: \"oval\", at: (0,0), size: (3,1.4), drill: 0.7mm, slot_length: 2mm)], paste_windows: [[(-0.7,-0.3),(-0.1,-0.3),(-0.1,0.3),(-0.7,0.3)],[(0.1,-0.3),(0.7,-0.3),(0.7,0.3),(0.1,0.3)]])
j = place(footprint: pkg, bindings: [bind(pad: \"1\", net:g)], at: (5,5), angle:90deg, side:\"bottom\")
route = track(net:g, layer:b, width:0.2mm, points:[(5,5),arc(via:(6,6),to:(7,5))])";
    let b = board(src);
    assert!(b.check().findings.is_empty());
    assert_eq!(
        b.layout.pads["j.1"].drill_axis,
        [[5_000_000, 5_650_000], [5_000_000, 4_350_000]]
    );
    assert_eq!(b.film(Side::Bottom, Film::Paste).len(), 2);
    assert!(b.film(Side::Top, Film::Paste).is_empty());
    assert!(b.tracks["route"].points.contains(&[6_000_000, 6_000_000]));
    assert!(b.tracks["route"].points.len() > 3);
    assert!(matches!(&b.layout.pads["j.1"].copper, Shape::Stroke { .. }));
    assert!(b.fabrication_files(None).unwrap()["plated.drl"].contains("G01X5.000000Y4.350000"));
}

#[test]
fn a_late_schematic_net_alias_keeps_the_same_routed_copper() {
    let line = "power = net(terminal: \"main/u.vdd\")";
    let class = "supply_class = net_class(nets: [power], min_width: 0.25mm, min_spacing: 0.25mm)";
    let src = EXAMPLE
        .replace(line, "")
        .replace(class, "")
        .replace("supply_feed =", &format!("{line}\n{class}\nsupply_feed ="));
    let b = board(&src);
    assert_eq!(b.nets.len(), 2);
    assert_eq!(b.layout.pads["reg.3"].net, "power");
    assert!(b.check().findings.is_empty());
}
#[test]
fn ratsnest_and_shorts_distinguish_opens_from_contacts() {
    use forge_agent::json::Json;
    let b = board(&replace("[(18,6),(24,6)]", "[(18,6),(22,6)]"));
    let r = pcb::response(&b, &["ratsnest".into()]).unwrap();
    let Some(Json::Arr(wires)) = r.get("result") else {
        panic!("{r}")
    };
    assert_eq!(wires.len(), 1);
    assert_eq!(wires[0].get("net").unwrap().str(), Some("power"));
    assert!(b.check().findings.iter().any(|f| f.rule == Rule::UnroutedNet));
    assert!(rules(&replace("[(18,6),(24,6)]", "[(18,6),(24,10),(24,6)]")).contains(&Rule::ShortCircuit));
}
#[test]
fn variants_keep_the_board_artwork_and_filter_assembly() {
    let mut b = board(EXAMPLE);
    b.schematic
        .as_mut()
        .unwrap()
        .document
        .variants
        .insert("omit-cap".into(), [("main/c".to_owned(), false)].into());
    let base = b.fabrication_files(None).unwrap();
    let fitted = b.fabrication_files(Some("omit-cap")).unwrap();
    for (name, v) in &base {
        if name.ends_with(".gbr") || name.ends_with(".drl") {
            assert_eq!(v, &fitted[name]);
        }
    }
    assert!(base["bom.csv"].contains("packages.passives.chip"));
    assert!(!fitted["bom.csv"].contains("packages.passives.chip"));
    assert!(!fitted["placements.csv"].contains("main/c"));
}

#[test]
fn layer_identity_placement_and_fabrication_policy() {
    let source = r#"pcb
perimeter = edge(points:[(0,0),(20,0),(20,15),(0,15)])
top = copper(thickness:0.035mm,weight_oz_ft2:1)
core = dielectric(kind:"core",material:"FR4",thickness:1.53mm,dk:4.2,df:0.018,frequency_mhz:1000)
bottom = copper(thickness:0.035mm,weight_oz_ft2:1)
stack = stackup(layers:[top,core,bottom])
coat = surface_finish(kind:"mask",side:"top",material:"LPI",color:"purple",thickness:0.015mm)
prototype = process(vendor:"oshpark",service:"two-layer",min_width:0.1mm)
debug = user_layer(name:"Fixture notes")
notes = layer_line(layer:"user.debug",points:[(1,1),(2,2)],width:0.1mm)
probe = user_layer(name:"Probe map",plot:true)
target = layer_text(layer:"user.probe",value:"TP1",at:(2,2),height:1mm,width:0.1mm)
pkg = footprint(pads:[],artwork:[layer_line(layer:"adhesive",points:[(1,0),(2,0)],width:0.2mm)])
placed = place(footprint:pkg,at:(10,5),side:"bottom",bindings:[])
"#;
    let b = board(source);
    assert_eq!(b.layers[1].laminate.dk, Some(4.2));
    assert_eq!(b.layout.finishes[&(Side::Top, Film::Mask)].thickness_nm, 15_000);
    assert_eq!(b.process.as_ref().unwrap().min_width_nm, 152_400);
    let glue = pcb::PlotLayer::Paired(Side::Bottom, pcb::PairRole::Adhesive);
    assert_eq!(
        b.plot(&glue)[0].shape,
        Shape::Stroke {
            axis: [[9_000_000, 5_000_000], [8_000_000, 5_000_000]],
            width: 200_000
        }
    );
    let f = b.fabrication_files(None).unwrap();
    assert!(f["bottom-adhesive.gbr"].contains("Glue,Bot"));
    assert!(!f.contains_key("top-adhesive.gbr"));
    assert!(!f.contains_key("documentation/user.debug.gbr"));
    assert!(f["documentation/user.probe.gbr"].contains("OtherDrawing,user.probe"));
    assert!(b.plot_layers().contains(&pcb::PlotLayer::DrillGuide));
    for (old, new, kind) in [
        ("two-layer", "four-layer", "VendorStackMismatch"),
        ("oshpark", "no-such-vendor", "UnknownPcbVendor"),
        ("two-layer", "unknown", "UnknownPcbService"),
        ("user.debug", "drawign", "UnknownDrawingLayer"),
        ("dk:4.2", "dk:0.5", "InvalidLaminateProperty"),
    ] {
        assert_eq!(
            pcb::compile(&source.replace(old, new), None, None)
                .unwrap_err()
                .kind,
            kind
        );
    }
}

#[test]
fn all_layer_and_selective_keepouts_and_margins() {
    let src = format!(
        "{EXAMPLE}\ngap = keepout(layer:\"all\", restrict:[\"via\"], points:[(11,5),(13,5),(13,7),(11,7)])"
    );
    assert!(!rules(&src).contains(&Rule::CopperInKeepout));
    assert!(rules(&src.replace("restrict:[\"via\"]", "restrict:[\"smd\"]")).contains(&Rule::CopperInKeepout));
    let src =
        format!("{EXAMPLE}\nedge_limit = layer_line(layer:\"margin\", points:[(12,4),(12,8)],width:0.1mm)");
    let b = board(&src);
    assert!(
        b.check()
            .findings
            .iter()
            .any(|f| f.rule == Rule::MinEdgeClearance)
    );
}

#[test]
fn filter_bank_service_and_connectivity() {
    let b = board(include_str!("../../../examples/electronics/filter-bank.fg"));
    assert_eq!(
        (
            b.nets.len(),
            b.layout.placements.len(),
            b.layout.pads.len(),
            b.vias.len()
        ),
        (17, 18, 50, 8)
    );
    assert!(b.check().findings.is_empty(), "{:?}", b.check().findings);
    assert!(b.plot(&pcb::PlotLayer::Airwires).is_empty());
    let f = b.fabrication_files(None).unwrap();
    assert!(f.contains_key("documentation/top.fab.gbr"));
    assert!(!f.contains_key("documentation/user.test_map.gbr"));
}

#[test]
fn fenced_project_vendor_override_drives_real_drc() {
    let dir = forge_dirs::temp_dir().join(format!("forge-pcb-project-profile-{}", forge_dirs::process_id()));
    let vendors = dir.join("vendors");
    std::fs::create_dir_all(&vendors).unwrap();
    let custom = include_str!("../../../kernel/forge-vendor/data/pcbway.toml")
        .replace("min_width = 0.1", "min_width = 0.6");
    std::fs::write(vendors.join("local.toml"), custom).unwrap();
    let src = r#"pcb
edge1 = edge(points:[(0,0),(10,0),(10,10),(0,10)])
top = copper(thickness:0.035mm,weight_oz_ft2:1)
core = dielectric(thickness:1.53mm,material:"FR4")
bottom = copper(thickness:0.035mm,weight_oz_ft2:1)
stack = stackup(layers:[top,core,bottom])
fab = process(vendor:"pcbway",service:"two-layer-1oz")
rail = net(name:"VDD")
trace1 = track(net:rail,layer:top,width:0.2mm,points:[(2,2),(5,5)])
"#;
    let b = pcb::compile(src, Some(&dir), Some(&dir)).unwrap();
    assert_eq!(b.process.as_ref().unwrap().min_width_nm, 600_000);
    assert!(b.check().findings.iter().any(|f| f.rule == Rule::MinTrackWidth));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn service_applies_slot_and_inner_bore_rules_at_their_boundaries() {
    let src = replace(
        "prototype = process(",
        "prototype = process(vendor: \"oshpark\", service: \"four-layer\", ",
    );
    let b = board(&src);
    assert_eq!(
        b.process.as_ref().unwrap().inner_drill_clearance_nm,
        Some(254_000)
    );
    assert!(b.check().findings.is_empty(), "{:?}", b.check().findings);
    assert!(rules(&src.replace("drill: 1.6mm", "drill: 0.507999mm")).contains(&Rule::MinSlotWidth));
    assert!(!rules(&src.replace("drill: 1.6mm", "drill: 0.508mm")).contains(&Rule::MinSlotWidth));
    let inner_trace = format!(
        "{src}\nstray = track(net:ground,layer:inner,width:0.2mm,points:[(17,6.503999),(19,6.503999)])"
    );
    assert!(rules(&inner_trace).contains(&Rule::MinInnerDrillClearance));
    assert!(!rules(&inner_trace.replace("6.503999", "6.504000")).contains(&Rule::MinInnerDrillClearance));
}
