//! Builds a space's scene. This first cut covers model-space LINE, CIRCLE, ARC, LWPOLYLINE
//! (straight segments) and TEXT (as `Fonts` shapes it), in their OCS taken as the WCS, colours
//! from the entity or its layer.

use crate::file::Drawing;
use crate::io::Href;
use crate::object::Object;
use crate::scene::{Fonts, Item, Options, Prim, Scene, SceneLayer, Space};
use crate::types::Data;
use forge_math::dmath;

/// AutoCAD colour index 1..9 (the rest grey until the full palette is in).
pub fn aci_rgb(i: i16) -> [u8; 3] {
    match i {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        7 => [255, 255, 255],
        8 => [128, 128, 128],
        9 => [192, 192, 192],
        _ => [160, 160, 160],
    }
}

fn layers(d: &Drawing) -> Vec<SceneLayer> {
    let mut out = vec![];
    for (h, o) in &d.objects {
        if let Data::Layer(l) = &o.data {
            let idx = l.color.index;
            out.push(SceneLayer {
                handle: *h,
                name: l.entry.name.text(),
                rgb: aci_rgb(idx.abs()),
                on: idx >= 0 && l.flags & 2 == 0,
                frozen: l.flags & 1 != 0,
                locked: l.flags & 8 != 0,
                plot: l.flags & 16 != 0,
            });
        }
    }
    out
}

fn arc(c: [f64; 3], r: f64, a0: f64, a1: f64, chord: f64) -> Vec<[f64; 2]> {
    let mut sweep = a1 - a0;
    while sweep <= 0.0 {
        sweep += dmath::TAU;
    }
    // segments so the sagitta stays under the chord tolerance
    let step = if r > chord {
        2.0 * dmath::acos(1.0 - chord / r)
    } else {
        0.5
    };
    let n = ((sweep / step.max(1e-3)).ceil() as usize).clamp(4, 4096);
    (0..=n)
        .map(|k| {
            let (s, co) = dmath::sin_cos(a0 + sweep * k as f64 / n as f64);
            [c[0] + r * co, c[1] + r * s]
        })
        .collect()
}

pub fn build(d: &Drawing, space: Space, fonts: &dyn Fonts, opts: &Options) -> Scene {
    let mut scene = Scene {
        layers: layers(d),
        ..Scene::default()
    };
    let ms = d.header.handle("BLOCK_RECORD_MODEL_SPACE").unwrap_or(0);
    let ps = d.header.handle("BLOCK_RECORD_PAPER_SPACE").unwrap_or(0);
    let _ = (space, ps);
    let chord = if opts.chord > 0.0 { opts.chord } else { 0.01 };
    let layer_of = |h: Href, s: &Scene| s.layers.iter().position(|l| l.handle == h.handle).unwrap_or(0);
    for (h, o) in &d.objects {
        let Some(e) = &o.ent else { continue };
        // model space only: entmode 2, or owned by *MODEL_SPACE
        let in_ms = e.entmode == 2 || (e.entmode == 0 && o.owner.handle == ms);
        if !in_ms {
            continue;
        }
        let layer = layer_of(e.layer, &scene);
        let rgb = match e.color.index {
            256 | 0 => scene.layers.get(layer).map_or([255; 3], |l| l.rgb),
            i => aci_rgb(i),
        };
        let mut push = |prim: Prim| {
            scene.items.push(Item {
                entity: *h,
                layer,
                rgb,
                weight: 0.0,
                prim,
            })
        };
        prims(o, chord, fonts, &mut push);
    }
    scene.grow_extents();
    scene
}

fn prims(o: &Object, chord: f64, fonts: &dyn Fonts, push: &mut dyn FnMut(Prim)) {
    match &o.data {
        Data::Line(l) => push(Prim::Path {
            pts: vec![[l.start[0], l.start[1]], [l.end[0], l.end[1]]],
            closed: false,
        }),
        Data::Circle(c) => push(Prim::Path {
            pts: arc(c.center, c.radius, 0.0, dmath::TAU, chord),
            closed: true,
        }),
        Data::Arc(a) => push(Prim::Path {
            pts: arc(a.circle.center, a.circle.radius, a.start, a.end, chord),
            closed: false,
        }),
        Data::LwPolyline(p) => push(Prim::Path {
            pts: p.points.clone(),
            closed: p.flag & 512 != 0,
        }),
        Data::Text(t) => {
            let s = fonts.shape("txt", "", &t.text.text());
            let (sn, cs) = dmath::sin_cos(t.rotation);
            let place = |q: &[f64; 2]| {
                let (x, y) = (q[0] * t.height * t.width, q[1] * t.height);
                [t.insert[0] + x * cs - y * sn, t.insert[1] + x * sn + y * cs]
            };
            for st in &s.strokes {
                push(Prim::Path {
                    pts: st.iter().map(place).collect(),
                    closed: false,
                });
            }
            if !s.fills.is_empty() {
                push(Prim::Fill {
                    rings: s.fills.iter().map(|r| r.iter().map(place).collect()).collect(),
                });
            }
        }
        _ => {}
    }
}
