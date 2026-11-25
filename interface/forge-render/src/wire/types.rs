//! The build's data as `Wire` values: what a build reports (rows, issues, measures, an
//! assembly's solve, a sheet's flat pattern, a thermoform's row) and the meshes it draws. The
//! `record!`/`choice!` lines name every field and variant of each type (mostly made by reading
//! the types themselves); the few that can't be written that way are by hand below them.

use super::{R, W, Wire};

impl Wire for forge_math::Frame {
    const MIN: usize = 96;
    fn put(&self, o: &mut W) {
        self.origin.put(o);
        self.x().get().put(o);
        self.y().get().put(o);
        self.z().get().put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        let origin = Wire::take(i)?;
        let (x, y, z) = (Wire::take(i)?, Wire::take(i)?, Wire::take(i)?);
        // these exact axes (a frame's axes are orthonormal: they were when written)
        forge_math::Frame::from_axes(origin, x, y, z)
    }
}

impl Wire for forge_math::UnitVec3 {
    const MIN: usize = 24;
    fn put(&self, o: &mut W) {
        self.get().put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        Some(forge_math::UnitVec3::new_unchecked(Wire::take(i)?))
    }
}

impl<T: Wire> Wire for std::ops::Range<T> {
    fn put(&self, o: &mut W) {
        self.start.put(o);
        self.end.put(o);
    }
    fn take(i: &mut R) -> Option<Self> {
        Some(Wire::take(i)?..Wire::take(i)?)
    }
}

record!(crate::shell::fg::FeatRow {
    id,
    kind,
    line,
    status,
    dims,
    reads,
    frame
});
record!(crate::shell::fg::DimRow {
    what,
    value,
    dim,
    from
});
choice!(crate::shell::fg::Status {
    0 => Ok,
    1 => Failed,
    2 => NotBuilt,
});
choice!(crate::shell::fg::From {
    0 => Const,
    1 => Driving(a0),
    2 => Driven(a0),
});
record!(crate::shell::fg::ParamRow {
    key,
    value,
    text,
    links,
    used_by,
    structural,
    line
});
record!(crate::shell::fg::Issue {
    severity,
    kind,
    text,
    line,
    col,
    fixes,
    feature,
    asm
});
choice!(crate::shell::fg::Severity {
    0 => Error,
    1 => Lint,
    2 => Warning,
    3 => Violation,
});
record!(forge_lang::error::ParamKey { owner, name });
record!(forge_lang::error::Literal { value, dim });
choice!(forge_lang::error::Op {
    0 => SetParam { param, value },
    1 => AddLocalParam { feature, name, value },
    2 => MakeShared { param },
    3 => SplitInstance { id, param },
    4 => AddFeature { before, text },
    5 => DeleteFeature { id },
    6 => ReplaceFeature { id, text },
    7 => MoveFeature { id, before },
    8 => LiftLiteral { span, name, scope },
    9 => SetArg { id, arg, text },
    10 => Rename { from, to },
    11 => SetProperty { key, text },
    12 => Suppress { id },
    13 => Unsuppress { id },
    14 => Respell { span, to },
});
choice!(forge_lang::error::Scope {
    0 => Shared,
    1 => Local(a0),
});
record!(forge_lang::syntax::Seg { name, index });
choice!(forge_lang::syntax::Key {
    0 => Ix(a0),
    1 => Label(a0),
});
record!(forge_lang::syntax::Span { lo, hi });
choice!(forge_lang::values::Dim {
    0 => Length,
    1 => Angle,
    2 => Count,
    3 => Ratio,
    4 => Area,
    5 => Volume,
    6 => Any,
});
record!(forge_lang::Measure {
    volume,
    area,
    centroid,
    faces,
    edges,
    bodies,
    problems,
    invalid,
    bbox,
    mass
});
record!(forge_lang::Timings {
    parse_ms,
    check_ms,
    eval_ms,
    kernel_ms,
    measure_ms,
    total_ms
});
record!(forge_lang::validity::Invalid {
    body,
    kind,
    reason,
    at
});
record!(forge_topo::mass::MassProps {
    volume,
    area,
    centroid,
    inertia,
    principal,
    axes,
    err,
    faces
});
record!(forge_math::Point3 { x, y, z });
record!(forge_math::Vec3 { x, y, z });
record!(forge_math::Point2 { x, y });
record_tuple!(forge_topo::Name { 0 });
record!(crate::livegpu::Delta {
    gone,
    added,
    center,
    radius,
    chord,
    faces,
    meshed
});
record!(crate::livegpu::FaceGpu {
    vertices,
    indices,
    segments,
    lo,
    hi
});
record!(crate::mesh::PartMesh { center, radius, lods });
record!(crate::mesh::Lod {
    chord,
    vertices,
    indices,
    segments,
    fins,
    faces,
    edges
});
record!(crate::mesh::FaceRange {
    face,
    first_tri,
    tris,
    first_vertex,
    vertices
});
record!(crate::mesh::EdgeRange {
    edge,
    first,
    count,
    start,
    end,
    start_at,
    end_at
});
record!(crate::fg::SheetInfo {
    id,
    bends,
    dxf,
    facts,
    lists,
    pattern,
    note,
    view
});
record!(crate::shell::sheet::FlatView {
    sheet,
    frame,
    outline,
    bends,
    tools,
    lo,
    hi
});
record!(crate::shell::sheet::FlatBend {
    a,
    b,
    angle,
    radius,
    allowance,
    feature
});
record!(forge_sheet::Pattern {
    contours,
    bends,
    hits,
    forms
});
choice!(forge_sheet::Curve2 {
    0 => Line { a, b },
    1 => Arc { c, r, start, sweep, from, to },
    2 => Spline { degree, knots, points, weights },
});
record!(forge_sheet::BendLine {
    joint,
    a,
    b,
    angle,
    radius,
    allowance
});
record!(forge_sheet::punch::Hit { tool, at, angle });
record!(forge_sheet::form::FormMark { tool, at, angle, up });
record!(crate::fg::AnchorView {
    faces,
    edges,
    face_names,
    edge_names,
    sizes,
    planes,
    sketches,
    sheet_edges,
    by_face,
    by_edge,
    role_faces
});
record!(forge_lang::anchors::SketchRefs {
    id,
    lines,
    closed,
    points,
    curves,
    shapes
});
record!(crate::fg::Occ {
    path,
    file,
    placement,
    tooling
});
choice!(crate::fg::HarnessPiece {
    0 => Wire(a0),
    1 => Sleeve(a0),
    2 => Ties,
    3 => Clips,
    4 => Stripe(a0, a1),
});
record!(forge_lang::asm::JointOut {
    id,
    kind,
    values,
    free,
    limits
});
record!(forge_lang::asm::ExpectOut {
    line,
    metric,
    measured,
    limit,
    ok
});
record!(forge_lang::asm::AsmTimings {
    load_ms,
    anchors_ms,
    solve_ms,
    overlap_ms,
    expects_ms,
    incremental,
    at_runs,
    built
});
record!(forge_lang::asm::InstOut {
    path,
    file,
    sub,
    ground,
    suppressed
});
record!(forge_lang::asm::LinkOut {
    owner,
    id,
    family,
    kind,
    a,
    b,
    suppressed
});
choice!(forge_lang::asm::LinkFamily {
    0 => Joint,
    1 => Constraint,
    2 => Relation,
    3 => Fit,
    4 => FitRule,
});
choice!(forge_lang::asm::Issue {
    0 => WrongDocument { func, assembly },
    1 => PartFile { file, problem },
    2 => BrokenAnchor { item, side, anchor, reason },
    3 => WrongReference { item, side, anchor, want, found },
    4 => Redundant { item, rows, implied_by },
    5 => Conflict { items, residuals, culprit },
    6 => NoGround,
    7 => NotConverged { items, residual },
    8 => LimitViolated { joint, freedom, value, range, angle, fix },
    9 => BadFreedom { joint, freedom },
    10 => BadRelation { relation },
    11 => DuplicateItem { id },
    12 => SelfLink { item },
    13 => CyclicAssembly { file },
    14 => PartChanged { file },
    15 => Interference { a, b, volume, confirmed, boolean_failed },
    16 => IntendedContact { a, b, by },
    17 => ShallowEngagement { a, b, rule, engaged, min },
    18 => UnknownKind { kind, declared },
    19 => ExpectFailed { metric, measured, cmp, limit },
    20 => UnknownMetric { metric },
    21 => ExpectUnevaluable { metric, why },
    22 => Inside { path, issue },
    23 => AtPastLimit { limit },
    24 => Component(a0),
    25 => Harness(a0),
});
record!(crate::shell::weld_ui::CutList { lines, plates });
record!(crate::shell::weld_ui::CutLine {
    qty,
    section,
    kind,
    length,
    overall,
    arc,
    ends,
    members
});
record!(crate::shell::weld_ui::PlateLine {
    id,
    kind,
    t,
    area,
    on
});
record!(crate::shell::thermo::ThermoRow {
    id,
    facts,
    min,
    max,
    limit,
    model,
    material,
    female,
    from,
    pull,
    t,
    shrink,
    draft_min,
    face_drafts,
    section,
    section_part,
    base,
    stages,
    points,
    modelled,
    simulated,
    density,
    passes,
    unformed,
    blank,
    shape,
    stretch,
    web,
    shrink_band,
    shrink_band_from,
    shrink_source
});
record!(crate::shell::thermo::Masses {
    volume,
    area,
    mass,
    cog,
    inertia
});
record!(forge_lang::props::Props {
    fields,
    custom,
    material,
    bodies,
    span
});
record!(forge_lang::props::Physical {
    volume,
    area,
    mass,
    centroid,
    inertia,
    origin_inertia,
    principal,
    solids,
    missing,
    rel_err
});
record!(forge_lang::props::SolidPhys {
    id,
    material,
    density,
    volume,
    area,
    mass,
    centroid
});
record_tuple!(forge_sheet::error::JointId { 0 });
choice!(forge_lang::asm::FileProblem {
    0 => Missing,
    1 => NotForge,
    2 => Outside,
    3 => Errors { count, first },
    4 => WrongKind { assembly },
    5 => Cyclic,
    6 => NoBody { body },
    7 => NoParam { param },
    8 => NoComponent { name },
    9 => NoHistory,
    10 => NoVersion { version, newest },
});
choice!(forge_asm::model::Side {
    0 => A,
    1 => B,
});
choice!(forge_asm::diag::AnchorReason {
    0 => NoInstance { at },
    1 => Suppressed { at },
    2 => BadPath { at },
    3 => NoDocument { doc },
    4 => NoAnchor,
    5 => Part(a0),
});
record!(forge_lang::asm::ItemName { id, part });
choice!(forge_lang::syntax::Cmp {
    0 => Eq,
    1 => Lt,
    2 => Le,
    3 => Gt,
    4 => Ge,
});
choice!(forge_lang::asm::Unevaluable {
    0 => MissingArg { index },
    1 => NotAPlace { found },
    2 => NoReference { reference },
    3 => NotAJointValue { found },
    4 => NoJointValue { joint },
    5 => NoGeometry { reference },
    6 => BooleanFailed { a, b, reason },
    7 => NoLimit,
    8 => AtFailed,
    9 => AtUnknownParam { param },
    10 => AtNotANumber { param },
    11 => NotParallel { angle },
});
choice!(forge_lang::asm::CompProblem {
    0 => NotTopLevel,
    1 => Shared,
    2 => Nested { func },
    3 => NoComponent,
    4 => BodyGone { id },
    5 => NotYet { what },
    6 => ReadsOutside { name, reader },
    7 => StillReadBy { name, readers },
    8 => ReadOtherwise { component, reader },
    9 => FileExists { file },
    10 => TargetIsLink { file },
    11 => BadFileName { file },
    12 => UntitledDocument,
    13 => Outside { file },
    14 => NotInlinable { file, why },
    15 => Cycle { a, b },
});
choice!(forge_lang::harness::HarnessProblem {
    0 => NotTopLevel,
    1 => BendTooTight { harness, segment, corner, at, needed, got },
    2 => BreakoutTooTight { harness, wire, connector, needed, got },
    3 => PinTaken { harness, connector, pin, wires },
    4 => CableTooShort { harness, cable, needed, length },
    5 => WireClash { harness, wire, with, at },
    6 => GroupSplit { harness, group, wire },
    7 => KinkAtNode { harness, node, wire, angle, toward },
    8 => NoPath { harness, wire, from, to },
    9 => PathBroken { harness, wire, at },
    10 => PinUnknown { harness, connector, pin, known },
    11 => NoExitNode { harness, connector },
    12 => BrokenRef { harness, item, reference },
    13 => UnknownWireType { name, known },
    14 => WireRowUnsourced { file, line },
    15 => WireRowBad { file, line, why },
    16 => NoInsulationOd { wire_type, conductor },
    17 => SegmentEmpty { harness, segment },
    18 => Occurrences { file, missing, unknown },
    19 => PlacedOnly { occurrences },
    20 => InFile { file, line, col, kind, text },
});
record_tuple!(forge_topo::body::FaceId { 0 });
choice!(forge_lang::values::Ty {
    0 => Num,
    1 => Bool,
    2 => Str,
    3 => List,
    4 => Map,
    5 => Tuple,
    6 => Vec2,
    7 => Function,
    8 => Word,
    9 => Dir,
    10 => Line,
    11 => Plane,
    12 => Axis,
    13 => Point,
    14 => Sketch,
    15 => Entity,
    16 => Disc,
    17 => Feature,
    18 => Faces,
    19 => Edges,
    20 => Verts,
    21 => Extent,
    22 => Layout,
    23 => Motion,
    24 => Unresolved,
    25 => Sheet,
    26 => SheetEdge,
    27 => Unfold,
    28 => BendRule,
    29 => Scaled,
    30 => Stock,
    31 => Shop,
    32 => Relief,
    33 => CornerKind,
    34 => Hardware,
    35 => ReliefRule,
    36 => ThermoMaterial,
    37 => ThermoShop,
    38 => Instance,
    39 => Reference,
    40 => Joint,
    41 => JointValue,
    42 => Check,
    43 => Anchor,
    44 => AnchorKind,
    45 => Pose,
    46 => Harness,
});
record_tuple!(forge_asm::model::DocId { 0 });
choice!(forge_asm::model::AnchorFault {
    0 => EntityGone { entity },
    1 => Degenerate { entity },
});

impl Wire for std::sync::Arc<str> {
    fn put(&self, o: &mut W) {
        o.size(self.len());
        o.raw(self.as_bytes());
    }
    fn take(i: &mut R) -> Option<Self> {
        let n = i.count(1)?;
        Some(std::str::from_utf8(i.raw(n)?).ok()?.into())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::super::tests::round;
    use crate::fg::{Msg, Session};
    use std::sync::Arc;

    /// Every message of a session on `file` (an example), the overlap check on when `checks`,
    /// until its build is back checked.
    fn session(file: &str, checks: bool) -> Vec<Msg> {
        let path = format!("{}/../../examples/{file}", env!("CARGO_MANIFEST_DIR"));
        let mut s = Session::open(&path, None, Arc::new(|| {})).expect("the example");
        let want = if checks { s.check_overlap() } else { s.seq };
        let mut got = vec![];
        let t = std::time::Instant::now();
        // and the whole mesh after it
        while s.done < want || s.whole < s.changed {
            assert!(t.elapsed().as_secs() < 600, "{file} built in time");
            got.extend(s.poll());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        got
    }

    /// What a session's messages held: builds, and of them sheets, thermoforms, cut lists,
    /// assemblies, meshes' bytes.
    #[derive(Default, Debug)]
    struct Seen {
        built: usize,
        sheets: usize,
        thermos: usize,
        cuts: usize,
        asm: usize,
        issues: usize,
        mesh_bytes: usize,
    }

    /// Each plain part of each message read back as it was written.
    fn every(file: &str, checks: bool) -> Seen {
        let mut n = Seen::default();
        for m in session(file, checks) {
            match m {
                Msg::Built(b) => {
                    assert_eq!(round(&b.feats), b.feats);
                    assert_eq!(round(&b.params), b.params);
                    round(&b.issues);
                    round(&b.measure);
                    round(&b.timings);
                    assert_eq!(
                        round(&b.mesh).map(|m| m.to_bytes()),
                        b.mesh.as_ref().map(|m| m.to_bytes())
                    );
                    round(&b.delta);
                    round(&b.flat);
                    round(&b.sheets);
                    for (row, _) in &b.thermos {
                        assert_eq!(&round(row), row);
                    }
                    round(&b.anchors);
                    round(&b.cutlist);
                    if let Some(d) = &b.inspect {
                        round(&d.props);
                        round(&d.physical);
                        round(&d.placed);
                        round(&d.names);
                    }
                    if let Some(a) = &b.asm {
                        assert_eq!(round(&a.occ), a.occ);
                        round(&a.placed);
                        round(&a.meshes);
                        round(&a.restage);
                        round(&a.joints);
                        round(&(a.mobility, a.bom.clone(), a.overlap, a.parts, a.meshed));
                        round(&a.expects);
                        round(&a.timings);
                        round(&a.boxes);
                        round(&a.grounded);
                        round(&a.insts);
                        round(&a.links);
                        round(&a.link_at);
                        round(&a.poses);
                        round(&a.thermo_files);
                        round(&a.body_chains);
                        round(&a.harness_placed);
                    }
                    n.built += 1;
                    n.sheets += b.sheets.len();
                    n.thermos += b.thermos.len();
                    n.cuts += b.cutlist.as_ref().map_or(0, |c| c.lines.len());
                    n.asm += b.asm.is_some() as usize;
                    n.issues += b.issues.len();
                    n.mesh_bytes += b.mesh.as_ref().map_or(0, |m| m.to_bytes().len());
                }
                Msg::Warm(d) => {
                    round(&d);
                }
                Msg::Final { mesh, flat, .. } => {
                    n.mesh_bytes += mesh.to_bytes().len();
                    round(&mesh);
                    round(&flat);
                }
                Msg::Posed { poses, .. } => {
                    round(&poses);
                }
            }
        }
        n
    }

    #[test]
    fn parts() {
        let n = every("demo/part_tools.fg", false);
        assert!(n.built > 0 && n.mesh_bytes > 0, "{n:?}");
        let n = every("demo/sheet_tools.fg", false);
        assert!(n.sheets > 0, "{n:?}");
        let n = every("thermoform/tray.fg", false);
        assert!(n.thermos > 0, "{n:?}");
        let n = every("weldments/table_frame.fg", false);
        assert!(n.cuts > 0, "{n:?}");
    }

    #[test]
    fn assemblies() {
        for f in ["rack18u/hw/tests/hinge_pin.fg", "harness/robot.fg"] {
            let n = every(f, true);
            assert!(n.asm > 0, "{f}: {n:?}");
        }
    }
}
