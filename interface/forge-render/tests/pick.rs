//! Picks against shapes whose every face, edge and vertex is known, through the GPU.

use forge_geom::SurfaceGeom;
use forge_math::Point3;
use forge_render::camera::Camera;
use forge_render::mesh::PartMesh;
use forge_render::render::{self, Entity, Offscreen, Overlay, PickHit, Renderer, part_info};
use forge_render::scene::{Placement, Rows, Scene};
use forge_topo::{Body, EdgeId, FaceId, VertexId};

const SIZE: (u32, u32) = (1000, 800);

struct Rig {
    body: Body,
    parts: Vec<PartMesh>,
    scene: Scene,
    r: Renderer,
    target: Offscreen,
    camera: Camera,
}

fn rig(src: &str, placed: Vec<Placement>, camera: Camera) -> Option<Rig> {
    let m = forge_agent::bench::model(src);
    assert!(m.error.is_none(), "{:?}", m.error);
    let body = m.body().unwrap().clone();
    let parts = vec![forge_render::tess::build_part(&body).unwrap()];
    let scene = Scene::new(&part_info(&parts), placed);
    let instance = render::native_instance();
    let Ok((device, queue, adapter)) = render::block_on(render::open_device(&instance, None)) else {
        eprintln!("skipped: no GPU");
        return None;
    };
    let r = Renderer::new(
        device,
        queue,
        adapter.get_info(),
        render::OFFSCREEN,
        SIZE,
        &parts,
        &scene,
    );
    let target = Offscreen::new(&r.device, SIZE);
    Some(Rig {
        body,
        parts,
        scene,
        r,
        target,
        camera,
    })
}

impl Rig {
    fn pick(&mut self, px: (f32, f32)) -> Option<PickHit> {
        self.r.pick_start(&mut self.scene, &self.camera, px);
        self.r.wait();
        let (at, hit, _) = self.r.pick_poll(&self.scene).expect("it was waited for");
        assert_eq!(at, px);
        hit
    }

    fn world(&self, placement: usize, q: [f64; 3]) -> [f32; 3] {
        let rows: Rows = self.scene.placements[placement].rows;
        let q = q.map(|v| v as f32);
        rows.map(|r| r[0] * q[0] + r[1] * q[1] + r[2] * q[2] + r[3])
    }

    fn pixel(&self, w: [f32; 3]) -> (f32, f32) {
        self.camera.pixel_of(w, SIZE.0, SIZE.1).expect("ahead of the eye")
    }

    /// The local point's pick.
    fn pick_local(&mut self, placement: usize, q: [f64; 3]) -> Option<PickHit> {
        let px = self.pixel(self.world(placement, q));
        self.pick(px)
    }
}

fn point(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

/// Which body face the point lies on, on a box.
fn face_at(body: &Body, q: [f64; 3]) -> Vec<u32> {
    body.face_ids()
        .filter(|&f| {
            let s = &body.face(f).surface;
            s.distance(Point3::new(q[0], q[1], q[2])) < 1e-6
        })
        .map(|f: FaceId| f.0)
        .collect()
}

const BOX: &str = "extrude 20 rect -50 -30 100 60";

/// A view from above with the +x, +y and +z faces of the box showing.
fn from_above(radius: f32) -> Camera {
    let mut c = Camera::fit([0.0, 0.0, 10.0], radius);
    c.yaw = 0.7;
    c.pitch = 0.6;
    c
}

fn visible_of_box(q: [f64; 3]) -> bool {
    // on a +x, +y or +z side
    q[0] > 49.9 || q[1] > 29.9 || q[2] > 19.9
}

#[test]
fn every_face_edge_and_vertex_of_a_box_answers_to_its_own_pixel() {
    let Some(mut g) = rig(BOX, vec![Placement::at(0, [0.0; 3])], from_above(90.0)) else {
        return;
    };
    let body = g.body.clone();

    // the three faces towards the eye, at their middles
    for (q, _) in [
        ([0.0, 0.0, 20.0], "top"),
        ([50.0, 0.0, 10.0], "+x"),
        ([0.0, 30.0, 10.0], "+y"),
    ] {
        let hit = g.pick_local(0, q).expect("a face");
        let Entity::Face(f) = hit.entity else {
            panic!("{q:?}: {:?}", hit.entity)
        };
        assert_eq!(vec![f], face_at(&body, q), "{q:?}");
        // and the point it says it hit is that point
        let w = g.world(0, q);
        let d = (0..3).map(|k| (hit.point[k] - w[k]).powi(2)).sum::<f32>().sqrt();
        assert!(d < 0.5, "{q:?} {:?} off by {d}", hit.point);
    }

    // every edge: at its middle, its own if it can be seen, a face if it can't
    let mut seen_edges = 0;
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let (a, b) = (body.vertex(edge.start).point, body.vertex(edge.end).point);
        let mid = [(a.x + b.x) / 2.0, (a.y + b.y) / 2.0, (a.z + b.z) / 2.0];
        let on: Vec<u32> = edge.coedges.iter().map(|&c| body.coedge_face(c).0).collect();
        // seen if a face beside it is towards the eye: the ones we looked at above
        let shown = on.iter().any(|&f| {
            let c = body.face(FaceId(f));
            let n = c
                .surface
                .normal(c.surface.project(Point3::new(mid[0], mid[1], mid[2])))
                .unwrap()
                .get();
            let n = if c.same_sense { n } else { -n };
            n.x > 0.5 || n.y > 0.5 || n.z > 0.5
        });
        let hit = g.pick_local(0, mid);
        if shown {
            assert_eq!(
                hit.map(|h| h.entity),
                Some(Entity::Edge(e.0)),
                "edge {} at {mid:?}",
                e.0
            );
            seen_edges += 1;
        } else {
            // it is behind the box: whatever answers, it isn't that edge
            if let Some(h) = hit {
                assert_ne!(h.entity, Entity::Edge(e.0), "hidden edge {} at {mid:?}", e.0);
            }
        }
    }
    assert_eq!(seen_edges, 9);

    // every vertex the eye can see
    let mut seen_vertices = 0;
    for v in body.vertex_ids() {
        let q = point(body.vertex(v).point);
        let hit = g.pick_local(0, q);
        if visible_of_box(q) {
            assert_eq!(
                hit.map(|h| h.entity),
                Some(Entity::Vertex(v.0)),
                "vertex {} at {q:?}",
                v.0
            );
            seen_vertices += 1;
        }
    }
    assert_eq!(seen_vertices, 7);
}

#[test]
fn a_turned_placement_is_picked_the_same_and_only_what_is_in_front_answers() {
    // the same part twice: one at home, one turned a quarter about z, moved along x, behind the other
    let turned = Placement {
        part: 0,
        rows: [
            [0.0, -1.0, 0.0, 300.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
        tint: [0.6; 4],
    };
    let cam = from_above(300.0);
    let Some(mut g) = rig(BOX, vec![Placement::at(0, [0.0; 3]), turned], cam) else {
        return;
    };
    let body = g.body.clone();
    // the middle of the turned one's top
    let top = face_at(&body, [0.0, 0.0, 20.0]);
    let hit = g.pick_local(1, [0.0, 0.0, 20.0]).expect("a face");
    assert_eq!(hit.placement, 1);
    assert_eq!(hit.entity, Entity::Face(top[0]));
    // the part's +x face now looks along +y of the world, towards the eye: same B-rep face
    let plus_x = face_at(&body, [50.0, 0.0, 10.0]);
    let hit = g.pick_local(1, [50.0, 0.0, 10.0]).expect("a face");
    assert_eq!((hit.placement, hit.entity), (1, Entity::Face(plus_x[0])));

    // nothing here: off to a corner of the window
    assert_eq!(g.pick((3.0, 3.0)), None);

    // a part hiding another: looking along x, the home one is in front of the turned one
    g.camera.yaw = 0.0;
    g.camera.pitch = 0.0;
    g.camera.target = [150.0, 0.0, 10.0];
    let hit = g
        .pick(((SIZE.0 / 2) as f32, (SIZE.1 / 2) as f32))
        .expect("a part");
    assert_eq!(hit.placement, 1, "the eye at +x sees the one at x = 300 first");
}

#[test]
fn round_faces_and_their_circle_edges_are_told_apart() {
    let mut cam = Camera::fit([0.0, 0.0, 5.0], 40.0);
    cam.yaw = 0.3;
    cam.pitch = 0.7;
    let Some(mut g) = rig("extrude 10 circle 0 0 20", vec![Placement::at(0, [0.0; 3])], cam) else {
        return;
    };
    let body = g.body.clone();
    // top face, mid side (towards the eye, along yaw 0.3)
    let (s, c) = (0.3f32.sin() as f64, 0.3f32.cos() as f64);
    let side = [20.0 * c, 20.0 * s, 5.0];
    let hit = g.pick_local(0, [0.0, 0.0, 10.0]).expect("the top");
    assert_eq!(hit.entity, Entity::Face(face_at(&body, [0.0, 0.0, 10.0])[0]));
    let hit = g.pick_local(0, side).expect("the side");
    assert_eq!(hit.entity, Entity::Face(face_at(&body, side)[0]));
    // the rim towards the eye, far from the circle edge's one vertex
    let rim = [20.0 * c, 20.0 * s, 10.0];
    let hit = g.pick_local(0, rim).expect("the rim");
    let Entity::Edge(e) = hit.entity else {
        // the circle's vertex may be anywhere round it, so check it is far first
        let starts: Vec<[f64; 3]> = body
            .vertex_ids()
            .map(|v: VertexId| point(body.vertex(v).point))
            .collect();
        panic!("{:?} (vertices {starts:?})", hit.entity);
    };
    let edge = body.edge(EdgeId(e));
    let curve = edge
        .coedges
        .iter()
        .map(|&c| body.coedge_face(c).0)
        .collect::<Vec<_>>();
    assert!(curve.contains(&face_at(&body, side)[0]), "not the side's edge");
    assert!(
        curve.contains(&face_at(&body, [0.0, 0.0, 10.0])[0]),
        "not the top's edge"
    );
}

#[test]
fn what_is_picked_can_be_marked_and_shows() {
    let Some(mut g) = rig(BOX, vec![Placement::at(0, [0.0; 3])], from_above(90.0)) else {
        return;
    };
    let hit = g.pick_local(0, [0.0, 0.0, 20.0]).expect("the top");
    let px = g.pixel(g.world(0, [0.0, 0.0, 20.0]));
    let shot = |g: &mut Rig, marks: &[render::Mark]| {
        let overlay = Overlay {
            marks,
            ..Overlay::default()
        };
        let camera = g.camera;
        g.r.frame(&g.target.view, &mut g.scene, &camera, overlay);
        g.r.wait();
        let rgb = g.target.read_rgb(&g.r);
        let at = px.1 as usize * SIZE.0 as usize * 3 + px.0 as usize * 3;
        [rgb[at], rgb[at + 1], rgb[at + 2]]
    };
    let plain = shot(&mut g, &[]);
    let hovered = shot(
        &mut g,
        &[render::Mark {
            placement: 0,
            entity: hit.entity,
            selected: false,
            second: false,
        }],
    );
    let selected = shot(
        &mut g,
        &[render::Mark {
            placement: 0,
            entity: hit.entity,
            selected: true,
            second: false,
        }],
    );
    assert!(
        hovered[2] as i32 > plain[2] as i32 && hovered[0] < plain[0],
        "{plain:?} {hovered:?}"
    );
    assert!(selected[0] as i32 > selected[2] as i32, "{selected:?}");
    let _ = &g.parts;
}

#[test]
fn a_cut_lets_picks_through_and_shows_the_inside_as_the_cut_face() {
    let mut cam = from_above(90.0);
    cam.yaw = 0.0;
    let Some(mut g) = rig(BOX, vec![Placement::at(0, [0.0; 3])], cam) else {
        return;
    };
    let body = g.body.clone();
    let px = g.pixel(g.world(0, [50.0, 0.0, 10.0]));
    let side = face_at(&body, [50.0, 0.0, 10.0])[0];
    assert_eq!(g.pick(px).map(|h| h.entity), Some(Entity::Face(side)));

    // cut away x > 20: that side's gone, the ray goes on, past the inside, to nothing or the back
    g.r.section = Some([1.0, 0.0, 0.0, -20.0]);
    if let Some(h) = g.pick(px) {
        assert_ne!(h.entity, Entity::Face(side));
    }
    let shot = |g: &mut Rig| {
        let camera = g.camera;
        g.r.frame(&g.target.view, &mut g.scene, &camera, Overlay::default());
        g.r.wait();
        g.target.read_rgb(&g.r)
    };
    let cut = shot(&mut g);
    // the middle of the cut, on the plane x = 20: drawn, in the part's colour or a hatch line
    let mid = g.pixel(g.world(0, [20.0, 0.0, 10.0]));
    let at = mid.1 as usize * SIZE.0 as usize * 3 + mid.0 as usize * 3;
    let corner = &cut[3 * SIZE.0 as usize * (SIZE.1 as usize / 2)..][..3];
    assert_ne!(
        &cut[at..at + 3],
        corner,
        "the cut face shows against the background"
    );
    // nothing of the part past the plane: the old side's pixel is background now
    let at = px.1 as usize * SIZE.0 as usize * 3 + px.0 as usize * 3;
    let was_side = &cut[at..at + 3];
    let row = 3 * SIZE.0 as usize * px.1 as usize;
    assert_eq!(was_side, &cut[row..row + 3], "the cut-away side is gone");
}

#[test]
fn a_pick_asked_before_the_meshes_changed_gives_no_answer() {
    let Some(mut g) = rig(BOX, vec![Placement::at(0, [0.0; 3])], from_above(90.0)) else {
        return;
    };
    let px = (SIZE.0 as f32 / 2.0, SIZE.1 as f32 / 2.0);
    let g0 = g.r.geometry();
    // asked of the box, then the box drawn anew (a build landed) before the answer's read
    g.r.pick_start(&mut g.scene, &g.camera, px);
    let parts = g.parts.clone();
    g.r.set_parts(&parts);
    assert_ne!(g.r.geometry(), g0);
    g.r.wait();
    assert!(g.r.pick_poll(&g.scene).is_none(), "an answer of the meshes before");
    assert!(!g.r.pick_busy(), "and it's done: the next is asked of these");
    // asked again: answered
    assert!(g.pick(px).is_some());
}
