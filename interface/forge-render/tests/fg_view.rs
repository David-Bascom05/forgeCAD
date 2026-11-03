//! A Forge file's view, headless: the view cube in sight beside the panel and snapping to every
//! face, edge and corner it's clicked on, dragged to orbit, Home; the view's switches (Axes, Grid,
//! Planes, Anchors) off for assemblies and remembered per kind; the issues as one line of counts;
//! a part's tree eyes; and the 18U rack's browser, Inventor's: Origin and Relationships first,
//! the components in the file's order, subassemblies with their own, shut, filtered, an eye taking
//! its occurrences out of the draws, a click in the view revealing its row, all quickly.

use forge_render::camera::{Camera, dot};
use forge_render::gui::{Gui, Headless};
use forge_render::shell::fgasm::{AsmPanel, Eye};
use forge_render::viewcube::{self, Hit};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-view-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

const PART: &str = "shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 40))
}
plate = extrude(plan, t)
sketch side(on: xz) {
  outline = rect(corner: (0, 0), size: (20, 20))
}
fin = extrude(side, 4)
";

fn link(len: f64) -> String {
    format!(
        "sketch plan(on: xy) {{
  outline = rect(corner: (-4, -4), size: ({}, 8))
  p0 = pt(0, 0)
  p1 = pt({len}, 0)
}}
bar = extrude(plan, 4)
a = anchor(plane(at: plan.p0, normal: plan.plane.normal, x: plan.outline[0].dir))
b = anchor(plane(at: plan.p1, normal: plan.plane.normal, x: plan.outline[0].dir))
",
        len + 8.0
    )
}

/// Two bars and three screws (in hw/) on them: a small assembly with groups.
fn small_asm(d: &Path) -> PathBuf {
    std::fs::create_dir_all(d.join("frame")).unwrap();
    std::fs::create_dir_all(d.join("hw")).unwrap();
    std::fs::write(d.join("frame/bar.fg"), link(100.0)).unwrap();
    std::fs::write(d.join("hw/screw_m4.fg"), link(6.0)).unwrap();
    let f = d.join("asm.fg");
    std::fs::write(
        &f,
        "assembly

shared gap = 30
base = part(\"frame/bar.fg\", ground: true)
top = part(\"frame/bar.fg\", ground: true, at: pose(at: (0, 0, 4)))
for i in 0..3 {
  screw = part(\"hw/screw_m4.fg\", ground: true, at: pose(at: (gap * i, 0, 8)))
}
",
    )
    .unwrap();
    f
}

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.is_loading()
}

fn open_in(h: &mut Headless, file: &Path, secs: u64) {
    h.gui.open(file.to_str().unwrap());
    let t = std::time::Instant::now();
    let mut ok = false;
    while t.elapsed().as_secs() < secs {
        if h.until(settled) {
            ok = true;
            break;
        }
    }
    assert!(ok, "never ready: {}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
}

fn headless() -> Option<Headless> {
    match Headless::new((1300.0, 900.0), 1.0) {
        Ok(mut h) => {
            h.step();
            Some(h)
        }
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

fn asm(h: &Headless) -> &AsmPanel {
    h.gui.shell.fg.as_ref().unwrap().asm.as_ref().unwrap()
}

fn asm_mut(h: &mut Headless) -> &mut AsmPanel {
    h.gui.shell.fg.as_mut().unwrap().asm.as_mut().unwrap()
}

fn camera(h: &Headless) -> Camera {
    h.gui.view.as_ref().unwrap().nav.camera
}

/// Where on the window the cube shows `want`, if it does.
fn cube_pixel(h: &Headless, want: Hit) -> Option<(f32, f32)> {
    let v = h.gui.view.as_ref().unwrap();
    let place = v.nav.cube_place();
    let cam = v.nav.camera;
    let at = h.gui.shell.view_rect.min;
    let n = 80;
    // the middle of the pixels it covers: in it, for a face, an edge or a corner
    let mut best: Option<((f32, f32), f32)> = None;
    for i in 0..n {
        for j in 0..n {
            let px = (
                place.at.0 + place.side * i as f32 / n as f32,
                place.at.1 + place.side * j as f32 / n as f32,
            );
            if viewcube::hit(&cam, &place, px) == Some(want) {
                best = Some(match best {
                    None => (px, 1.0),
                    Some((s, k)) => ((s.0 + px.0, s.1 + px.1), k + 1.0),
                });
            }
        }
    }
    let ((x, y), k) = best?;
    let (x, y) = (x / k, y / k);
    let v = h.gui.view.as_ref().unwrap();
    if viewcube::hit(&cam, &v.nav.cube_place(), (x, y)) != Some(want) {
        return None;
    }
    Some((at.x + x, at.y + y))
}

fn settle_camera(h: &mut Headless) {
    for _ in 0..400 {
        h.step();
        if !h.gui.view.as_ref().unwrap().nav.moving() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    h.step();
}

/// The eye looks from the region's side at the model: forward is the region's direction, reversed.
fn looks_from(cam: &Camera, region: [i8; 3]) -> bool {
    let (f, _, _) = cam.axes();
    let r = region.map(|v| v as f32);
    let l = dot(r, r).sqrt();
    dot(f, r.map(|v| -v / l)) > 0.999
}

/// Every face, then an edge and a corner: the cube's picture shows it, a click there goes to it.
fn cube_snaps(h: &mut Headless) {
    // the cube is in sight: not under the panel
    let v = h.gui.view.as_ref().unwrap();
    let place = v.nav.cube_place();
    let right = h.gui.shell.view_rect.min.x + place.at.0 + place.side;
    assert!(right <= h.gui.shell.view_rect.max.x, "the cube is off the view");
    let faces: [[i8; 3]; 6] = [
        [0, -1, 0],
        [0, 1, 0],
        [1, 0, 0],
        [-1, 0, 0],
        [0, 0, 1],
        [0, 0, -1],
    ];
    let regions: Vec<[i8; 3]> = faces.iter().copied().chain([[1, -1, 0], [-1, 1, 1]]).collect();
    for region in regions {
        // first a corner from which it shows, then a click on it
        let from: [i8; 3] = region.map(|c| if c == 0 { 1 } else { c });
        let from = if from == region { [region[0], -1, 1] } else { from };
        {
            let v = h.gui.view.as_mut().unwrap();
            v.nav.camera = forge_render::nav::region_view(v.nav.camera, from);
            v.mark_dirty();
        }
        h.step();
        let at = cube_pixel(h, Hit::Region(region))
            .unwrap_or_else(|| panic!("{region:?} not on the cube from {from:?}"));
        let t = std::time::Instant::now();
        h.click_at(at.0, at.1);
        // animated: it's on its way, then there quickly
        settle_camera(h);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        assert!(
            looks_from(&camera(h), region),
            "{region:?}: {:?}",
            camera(h).axes()
        );
        assert!(ms < 2000.0, "{region:?} took {ms:.0} ms");
    }
    // a drag on the cube orbits, and doesn't snap
    let at = cube_pixel(h, Hit::Region([0, -1, 0])).or_else(|| cube_pixel(h, Hit::Region([1, -1, 0])));
    if let Some(at) = at {
        let before = camera(h);
        h.move_to(at.0, at.1);
        h.step();
        h.button(true);
        h.step();
        for k in 1..=10 {
            h.move_to(at.0 + 6.0 * k as f32, at.1 + 2.0 * k as f32);
            h.step();
        }
        h.button(false);
        settle_camera(h);
        let after = camera(h);
        assert!(
            (after.yaw - before.yaw).abs() > 0.05,
            "a drag didn't orbit: {} {}",
            before.yaw,
            after.yaw
        );
    }
    // Home: back to the home corner
    if let Some(at) = cube_pixel(h, Hit::Home) {
        h.click_at(at.0, at.1);
        settle_camera(h);
        assert!(
            looks_from(&camera(h), forge_render::nav::HOME_CORNER),
            "{:?}",
            camera(h).axes()
        );
    } else {
        panic!("no Home on the cube");
    }
    // the quarter turn arrows show square on to a face
    {
        let v = h.gui.view.as_mut().unwrap();
        v.nav.camera = forge_render::nav::region_view(v.nav.camera, [0, -1, 0]);
        v.mark_dirty();
    }
    h.step();
    let step = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |y| (x, y)))
        .filter(|s| *s != (0, 0))
        .find_map(|(x, y)| cube_pixel(h, Hit::Step(x, y)));
    let at = step.expect("no quarter turn arrow square on to the front");
    h.click_at(at.0, at.1);
    settle_camera(h);
    assert!(!looks_from(&camera(h), [0, -1, 0]), "a quarter turn didn't turn");
}

#[test]
fn the_cube_shows_beside_the_panel_and_snaps_in_a_part() {
    let Some(mut h) = headless() else { return };
    let d = scratch("cube-part");
    let f = d.join("part.fg");
    std::fs::write(&f, PART).unwrap();
    open_in(&mut h, &f, 60);
    cube_snaps(&mut h);
}

#[test]
fn the_cube_snaps_in_an_assembly() {
    let Some(mut h) = headless() else { return };
    let f = small_asm(&scratch("cube-asm"));
    open_in(&mut h, &f, 60);
    assert!(h.gui.fg_asm.is_some());
    cube_snaps(&mut h);
}

fn pixels_differ(a: &[u8], b: &[u8]) -> usize {
    a.chunks(3).zip(b.chunks(3)).filter(|(x, y)| x != y).count()
}

/// The model's part of the window: the view, less its toolbar and the cube's corner.
fn model_pixels(h: &Headless) -> Vec<u8> {
    let p = h.gui_pixels();
    let v = h.gui.shell.view_rect;
    let (x0, x1) = (v.min.x as usize, (v.max.x - 200.0) as usize);
    let (y0, y1) = ((v.min.y + 50.0) as usize, v.max.y as usize);
    let mut out = vec![];
    for y in y0..y1 {
        for x in x0..x1 {
            out.extend_from_slice(&p.at(x, y));
        }
    }
    out
}

#[test]
fn the_views_switches_are_off_for_assemblies_and_kept_per_kind() {
    let Some(mut h) = headless() else { return };
    let d = scratch("switches");
    let part = d.join("part.fg");
    std::fs::write(&part, PART).unwrap();
    let a = small_asm(&d);
    open_in(&mut h, &a, 60);
    // an assembly: nothing over the model
    let o = h.gui.shell.overlays();
    assert!(!o.axes && !o.grid && !o.planes && !o.anchors, "{o:?}");
    assert!(h.gui.shell.origin_drawn().is_empty());
    for name in ["axes", "grid", "planes", "anchors"] {
        assert!(h.gui.shell.place(&format!("view:{name}")).is_some(), "{name}");
    }
    // each switch draws what it says, at once
    let before = model_pixels(&h);
    assert!(h.click("view:axes"));
    h.step();
    h.step();
    assert!(h.gui.shell.overlays().axes);
    assert!(!h.gui.shell.origin_drawn().is_empty());
    let axes = model_pixels(&h);
    assert!(pixels_differ(&before, &axes) > 200, "the axes didn't show");
    assert!(h.click("view:grid"));
    h.step();
    h.step();
    let grid = model_pixels(&h);
    assert!(pixels_differ(&axes, &grid) > 200, "the grid didn't show");
    assert!(h.click("view:grid"));
    h.step();
    h.step();
    assert!(pixels_differ(&axes, &model_pixels(&h)) < 50, "the grid didn't go");
    // anchors: the chosen part's, found and drawn
    assert!(h.click("view:anchors"));
    asm_mut(&mut h).selected = Some("base".into());
    let t = std::time::Instant::now();
    h.step();
    assert!(h.gui.shell.overlays().anchors);
    while t.elapsed().as_secs() < 30 {
        h.step();
        if pixels_differ(&axes, &model_pixels(&h)) > 40 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(pixels_differ(&axes, &model_pixels(&h)) > 40, "no anchors drawn");
    // a part keeps its own: axes and grid on
    open_in(&mut h, &part, 60);
    let o = h.gui.shell.overlays();
    assert!(o.axes && o.grid && !o.anchors, "{o:?}");
    assert!(h.click("view:grid"));
    h.step();
    // back to the assembly: its choice kept for the session (axes and anchors on)
    open_in(&mut h, &a, 60);
    let o = h.gui.shell.overlays();
    assert!(o.axes && !o.grid && o.anchors, "{o:?}");
    // and the part's, too
    open_in(&mut h, &part, 60);
    assert!(!h.gui.shell.overlays().grid);
}

#[test]
fn issues_are_one_line_open_only_with_errors() {
    let Some(mut h) = headless() else { return };
    let d = scratch("issues");
    let f = d.join("part.fg");
    // a loose literal: a lint, no error
    std::fs::write(&f, "sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (60, 40))\n}\nplate = extrude(plan, 6)\n").unwrap();
    open_in(&mut h, &f, 60);
    let p = h.gui.shell.fg.as_ref().unwrap();
    let errors = p
        .issues
        .iter()
        .filter(|i| i.severity == forge_render::shell::fg::Severity::Error)
        .count();
    assert_eq!(errors, 0);
    assert!(h.gui.shell.place("fg:issues").is_some());
    assert!(
        h.gui.shell.place("fg:issue:0").is_none(),
        "a warning opened the list"
    );
    if !p.issues.is_empty() {
        assert!(h.click("fg:issues"));
        h.step();
        h.step();
        assert!(h.gui.shell.place("fg:issue:0").is_some());
    }
    // an error opens it by itself
    std::fs::write(&f, "plate = extrude(nothing_here, 6)\n").unwrap();
    open_in(&mut h, &f, 60);
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(
        h.gui.shell.place("fg:issue:0").is_some(),
        "an error left the list shut: {:?} {:?}",
        p.issues
            .iter()
            .map(|i| (i.severity, i.kind.clone()))
            .collect::<Vec<_>>(),
        p.issues_shown
    );
}

#[test]
fn a_parts_tree_has_eyes_for_its_sketches_and_its_solid() {
    let Some(mut h) = headless() else { return };
    let d = scratch("eyes");
    let f = d.join("part.fg");
    std::fs::write(&f, PART).unwrap();
    open_in(&mut h, &f, 60);
    assert!(h.gui.shell.place("fg:eye:plan").is_some());
    assert!(h.gui.shell.place("fg:eye:side").is_some());
    assert!(
        h.gui.shell.place("fg:eye:plate").is_none(),
        "a solid feature has no eye of its own"
    );
    // a sketch's eye shows its plane, though Planes is off
    let before = model_pixels(&h);
    assert!(h.click("fg:eye:plan"));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.fg.as_ref().unwrap().eyes.get("plan"), Some(&true));
    assert!(
        pixels_differ(&before, &model_pixels(&h)) > 200,
        "the sketch's plane didn't show"
    );
    // the solid's eye takes it out of the draws, and back
    assert!(h.click("fg:eye:solid"));
    h.step();
    h.step();
    assert!(h.gui.view.as_ref().unwrap().scene.is_gone(0));
    assert!(h.click("fg:eye:solid"));
    h.step();
    assert!(!h.gui.view.as_ref().unwrap().scene.is_gone(0));
}

#[test]
fn a_view_click_chooses_and_reveals_its_row() {
    let Some(mut h) = headless() else { return };
    let f = small_asm(&scratch("reveal"));
    open_in(&mut h, &f, 60);
    // shut: nothing open
    assert!(asm(&h).expanded.is_empty());
    let (occ, _) = h.gui.fg_asm.clone().unwrap();
    // the second screw, where it's drawn
    let k = occ.iter().position(|o| o.path == "screw[1]").unwrap();
    let pl = occ[k].placement.unwrap();
    let (c, _) = h.gui.view.as_ref().unwrap().scene.sphere(pl);
    let v = h.gui.view.as_ref().unwrap();
    let px = v.nav.camera.pixel_of(c, v.nav.size.0, v.nav.size.1).unwrap();
    let at = h.gui.shell.view_rect.min;
    h.click_at(at.x + px.0, at.y + px.1);
    for _ in 0..10 {
        h.step();
    }
    let a = asm(&h);
    assert_eq!(a.selected.as_deref(), Some("screw[1]"));
    assert!(a.expanded.contains("p:screw"), "{:?}", a.expanded);
    assert!(
        h.gui.shell.place("asm:row:screw[1]").is_some(),
        "its row isn't shown"
    );
    let v = h.gui.view.as_ref().unwrap();
    assert_eq!(v.scene.placements[pl].tint, forge_render::fg::PICKED_TINT);
}

fn rack() -> Option<PathBuf> {
    let r = Path::new(env!("HOME")).join("projects/forgecad-gate/rack18u/asm/rack.fg");
    if r.exists() {
        Some(r)
    } else {
        eprintln!("skipped: no {}", r.display());
        None
    }
}

#[test]
fn the_racks_browser_is_inventors_and_quick() {
    let Some(r) = rack() else { return };
    let Some(mut h) = headless() else { return };
    open_in(&mut h, &r, 600);
    let a = asm(&h);
    let t = &a.tree;
    // the assembly's own row on top, everything under it
    assert_eq!(t.roots.len(), 1);
    let top = t.nodes[t.roots[0]].kids.clone();
    let roots: Vec<&str> = top.iter().map(|&n| t.nodes[n].key.as_str()).collect();
    eprintln!("rack roots: {} rows, first {:?}", roots.len(), &roots[..6]);
    // Origin, Relationships, then the components as the file makes them
    assert_eq!(&roots[..4], ["o:", "r:", "base_pan", "top_pan"]);
    assert_eq!(a.occ.len(), 860);
    assert!(
        t.home.iter().all(|&n| n != usize::MAX),
        "an occurrence with no row"
    );
    // every relationship the file writes, by kind
    let rel = &t.nodes[top[1]];
    let mine = a.links.iter().filter(|l| l.owner.is_empty()).count();
    let listed: usize = rel.kids.iter().map(|&k| t.nodes[k].kids.len()).sum();
    assert_eq!(listed, mine);
    let kinds: Vec<&str> = rel.kids.iter().map(|&k| t.nodes[k].label.as_str()).collect();
    for k in [
        "Fastened joints",
        "Revolute joints",
        "Slider joints",
        "Gear relations",
        "Fits",
    ] {
        assert!(kinds.contains(&k), "{kinds:?}");
    }
    // a subassembly with its own origin and relationships; a loop's screws as one row
    for s in ["post_fl", "car_tr", "fans"] {
        let n = t.find(&format!("s:{s}")).unwrap_or_else(|| panic!("no {s}"));
        let kids: Vec<&str> = t.nodes[n]
            .kids
            .iter()
            .map(|&k| t.nodes[k].label.as_str())
            .collect();
        assert_eq!(&kids[..2], ["Origin", "Relationships"], "{s}: {kids:?}");
    }
    let scr = &t.nodes[t.find("p:scr_bl").unwrap()];
    assert_eq!((scr.kids.len(), scr.detail.as_str()), (3, "ISO 7380 M6×12"));
    // a part holds the relationships that hold it
    let base = &t.nodes[t.find("base_pan").unwrap()];
    assert!(base.ground && base.kids.len() > 10, "{}", base.kids.len());
    // all shut: the top rows only
    let n_roots = top.len();
    assert_eq!(asm_mut(&mut h).rows().len(), 1 + n_roots);
    // the filter: what matches, and the rows above it
    asm_mut(&mut h).filter = "7380".into();
    let rows = asm_mut(&mut h).rows();
    let t = &asm(&h).tree;
    assert!(rows.len() > 5);
    assert!(
        rows.iter().all(|&(n, open)| open
            || t.nodes[n].detail.contains("7380")
            || t.nodes[n].label.contains("7380")),
        "a row the filter doesn't find, shut"
    );
    asm_mut(&mut h).filter.clear();
    h.step();

    // hiding a subassembly: one click, out of the draws, at once
    let (occ, _) = h.gui.fg_asm.clone().unwrap();
    let post = asm(&h).tree.find("s:post_fl").unwrap();
    let post_occ: Vec<usize> = asm(&h).tree.nodes[post].occ.clone();
    assert!(post_occ.len() > 20, "{}", post_occ.len());
    let mut times = vec![];
    for _ in 0..5 {
        assert!(h.click("asm:eye:s:post_fl"));
        // the frame the click lands in: the toggle, the scene, the draw
        let t = std::time::Instant::now();
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1e3);
        let v = h.gui.view.as_ref().unwrap();
        let hidden = asm(&h).eye(post) == Eye::Hidden;
        for &k in &post_occ {
            let pl = occ[k].placement.unwrap();
            assert_eq!(v.scene.is_gone(pl), hidden, "{}", occ[k].path);
        }
    }
    assert_eq!(asm(&h).eye(post), Eye::Hidden);
    h.step();
    assert!(h.click("asm:treeshowall"));
    h.step();
    h.step();
    assert!(asm(&h).hidden.is_empty());
    // opening the Relationships and their biggest kind, and a frame with it open
    let mut open_ms = vec![];
    for _ in 0..3 {
        for _ in 0..2 {
            assert!(h.click("asm:open:r:"));
            let t = std::time::Instant::now();
            h.step();
            open_ms.push(t.elapsed().as_secs_f64() * 1e3);
        }
    }
    let rel = asm(&h).tree.nodes[asm(&h).tree.roots[0]].kids[1];
    let big = asm(&h).tree.nodes[rel]
        .kids
        .iter()
        .copied()
        .max_by_key(|&k| asm(&h).tree.nodes[k].kids.len())
        .unwrap();
    let key = asm(&h).tree.nodes[big].key.clone();
    let n_big = asm(&h).tree.nodes[big].kids.len();
    asm_mut(&mut h).expanded.insert("r:".into());
    asm_mut(&mut h).expanded.insert(key);
    let t = std::time::Instant::now();
    h.step();
    let frame_ms = t.elapsed().as_secs_f64() * 1e3;
    times.sort_by(f64::total_cmp);
    open_ms.sort_by(f64::total_cmp);
    eprintln!(
        "rack browser: frame with an eye on {} occurrences {:.1} ms median ({:.1} worst); with Relationships opened or shut {:.1} ms median; with {n_big} rows open {frame_ms:.1} ms",
        post_occ.len(),
        times[2],
        times[4],
        open_ms[3]
    );
    // the whole frame the click lands in, drawing included
    assert!(times[2] < 60.0, "eye {:.1} ms", times[2]);
    assert!(open_ms[3] < 60.0, "open {:.1} ms", open_ms[3]);
}
