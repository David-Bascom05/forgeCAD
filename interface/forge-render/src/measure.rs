//! The kernel's own exact measure of what's selected, off the window's thread: the first ask of a
//! part builds its body. Entities are the mesh's numbers, which are the body's.

use crate::render::Entity;
use crate::scene::Scene;
use forge_agent::measure as kernel;
use forge_agent::select::Ent;
use forge_mesh::{Quality, TriMesh};
use forge_topo::{Body, EdgeId, FaceId, VertexId};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};

fn ent(e: Entity) -> Ent {
    match e {
        Entity::Face(i) => Ent::Face(FaceId(i)),
        Entity::Edge(i) => Ent::Edge(EdgeId(i)),
        Entity::Vertex(i) => Ent::Vert(VertexId(i)),
    }
}

/// What the kernel says of one entity or two of the same part, in words.
pub fn say(body: &Body, mesh: &TriMesh, of: &[Entity]) -> String {
    let lines = match of {
        [a] => kernel::one(body, mesh, ent(*a)),
        [a, b] => kernel::between(body, mesh, ent(*a), ent(*b)),
        _ => return String::new(),
    };
    let words: Vec<String> = lines.iter().map(|(k, v)| format!("{k} {v:.4}")).collect();
    words.join(" ")
}

struct Ask {
    id: u64,
    part: String,
    of: Vec<Entity>,
}

#[derive(Default)]
pub struct Measurer {
    names: Vec<String>,
    to: Option<Sender<Ask>>,
    from: Option<Receiver<(u64, String)>>,
    last: u64,
    answered: u64,
}

impl Measurer {
    pub fn new(names: Vec<String>) -> Measurer {
        Measurer {
            names,
            ..Measurer::default()
        }
    }

    fn start(&mut self) {
        let (to, asks) = channel::<Ask>();
        let (answers, from) = channel();
        std::thread::spawn(move || {
            let mut bodies: HashMap<String, Result<(Body, TriMesh), String>> = HashMap::new();
            while let Ok(mut a) = asks.recv() {
                // only the latest matters
                while let Ok(newer) = asks.try_recv() {
                    a = newer;
                }
                let got = bodies.entry(a.part.clone()).or_insert_with(|| {
                    let body = crate::load::body(&a.part)?;
                    let mesh = forge_mesh::tessellate::tessellate_tolerant(&body, &Quality::default())
                        .map_err(|e| forge_lang::error::words(&e))?;
                    Ok((body, mesh))
                });
                let text = match got {
                    Ok((b, m)) => say(b, m, &a.of),
                    Err(e) => e.clone(),
                };
                if answers.send((a.id, text)).is_err() {
                    return;
                }
            }
        });
        self.to = Some(to);
        self.from = Some(from);
    }

    /// Asks about the selection. Two on different placements have no one body to ask.
    pub fn ask(&mut self, selected: Vec<(u32, Entity)>, scene: &Scene) {
        self.last += 1;
        self.answered = self.last;
        let same = selected.windows(2).all(|w| w[0].0 == w[1].0);
        let (Some(first), true, 1..=2) = (selected.first(), same, selected.len()) else {
            return;
        };
        let part = scene.placements[first.0 as usize].part as usize;
        let Some(name) = self.names.get(part).cloned() else {
            return;
        };
        if self.to.is_none() {
            self.start();
        }
        let of = selected.iter().map(|s| s.1).collect();
        self.answered = self.last - 1;
        if let Some(to) = &self.to {
            let _ = to.send(Ask {
                id: self.last,
                part: name,
                of,
            });
        }
    }

    /// Whether the last ask is yet to come back.
    pub fn waiting(&self) -> bool {
        self.to.is_some() && self.answered != self.last
    }

    /// New words for the selection, if any have come; nothing selected clears them.
    pub fn answer(&mut self) -> Option<String> {
        let from = self.from.as_ref()?;
        let mut got = None;
        while let Ok((id, text)) = from.try_recv() {
            if id == self.last {
                got = Some(text);
                self.answered = id;
            }
        }
        got
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kernel_measures_the_picked_entities_exactly() {
        let body = crate::load::body("crank").unwrap();
        let _ = body;
        let m = forge_agent::bench::model("extrude 20 rect -50 -30 100 60");
        let body = m.body().unwrap().clone();
        let mesh = forge_mesh::tessellate(&body, &Quality::default()).unwrap();
        // the top face
        let top = body
            .face_ids()
            .find(|&f| {
                use forge_geom::SurfaceGeom;
                body.face(f)
                    .surface
                    .distance(forge_math::Point3::new(0.0, 0.0, 20.0))
                    < 1e-9
            })
            .unwrap();
        let bottom = body
            .face_ids()
            .find(|&f| {
                use forge_geom::SurfaceGeom;
                body.face(f)
                    .surface
                    .distance(forge_math::Point3::new(0.0, 0.0, 0.0))
                    < 1e-9
            })
            .unwrap();
        let area = say(&body, &mesh, &[Entity::Face(top.0)]);
        assert!(area.contains("area 6000.0000"), "{area}");
        let apart = say(&body, &mesh, &[Entity::Face(top.0), Entity::Face(bottom.0)]);
        assert!(apart.contains("20.0000"), "{apart}");
        // an edge along x is 100 long
        let e = body
            .edge_ids()
            .find(|&e| {
                let edge = body.edge(e);
                let (a, b) = (body.vertex(edge.start).point, body.vertex(edge.end).point);
                (a.x - b.x).abs() > 99.0
            })
            .unwrap();
        let length = say(&body, &mesh, &[Entity::Edge(e.0)]);
        assert!(length.contains("100.0000"), "{length}");
        // a vertex answers something, or nothing, without trouble; three entities ask nothing
        assert_eq!(
            say(&body, &mesh, &[Entity::Face(0), Entity::Face(1), Entity::Face(2)]),
            ""
        );
    }

    #[test]
    fn an_answer_comes_back_for_the_latest_ask_only() {
        let parts = vec![crate::load::open("crank", false).unwrap().0];
        let scene = Scene::new(
            &crate::render::part_info(&parts),
            vec![crate::scene::Placement::at(0, [0.0; 3])],
        );
        let mut m = Measurer::new(vec!["crank".to_string()]);
        m.ask(vec![(0, Entity::Face(1))], &scene);
        m.ask(vec![(0, Entity::Face(2))], &scene);
        assert!(m.waiting());
        let text = loop {
            if let Some(t) = m.answer() {
                break t;
            }
            std::thread::yield_now();
        };
        assert!(text.contains("area"), "{text}");
        assert!(!m.waiting());
        // nothing to ask: nothing awaited
        m.ask(Vec::new(), &scene);
        assert!(!m.waiting());
    }
}
