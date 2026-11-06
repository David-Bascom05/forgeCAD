//! Builds a space's scene, as AutoCAD shows it in plan.
//!
//! Each entity becomes `Raw` pieces in the coordinates of the block it is in, its layer, colour,
//! lineweight and line type kept as the entity says them (by layer, by block, layer "0" in a
//! block). A block's pieces are made once (per chord tolerance step) and an insert refers to
//! them through its transform, putting its own properties where they say "by block" and its
//! layer where they are on "0" (the pieces are copied only into the scene's items). At the top (model space, a layout, or model space through a layout's
//! viewport) every piece is resolved: layer visibility, colour, lineweight, the line type's
//! dashes laid along it in world units, and the view's transform and clip.
//!
//! Not drawn: proxy entities' graphics, OLE2FRAME, MTEXT columns (the text flows as one
//! column), vertical text styles (set horizontally), complex line types' text and shapes (their
//! dashes are drawn), non-rectangular viewport clips (clipped to the viewport's rectangle).

use super::color::{Says, says};
use super::geom::{
    self, Mat, P2, P3, Rect, add, arc_pts, bulged, ccw_sweep, clip_path, clip_ring, cross, ellipse_pts,
    interp_pts, len, mul, nurbs_pts, sub, unit,
};
use super::hatch;
use super::ltype::{self, Pattern};
use super::text::{self, Font, Frame, Laid, Look};
use crate::file::Drawing;
use crate::io::Color;
use crate::object::{EntCommon, Object};
use crate::scene::{Fonts, Item, Options, Prim, Scene, SceneLayer, Space};
use crate::types::*;
use forge_math::dmath;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A quick hash for handle keys (looked up only, never iterated, so the order is no matter).
#[derive(Default)]
struct IdHash(u64);

impl Hasher for IdHash {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0.rotate_left(8) ^ *b as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }
    fn write_u32(&mut self, n: u32) {
        self.0 = (self.0.rotate_left(32) ^ n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    fn write_i32(&mut self, n: i32) {
        self.write_u32(n as u32);
    }
    fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 29)
    }
}

type Map<K, V> = HashMap<K, V, BuildHasherDefault<IdHash>>;
use std::rc::Rc;

/// The lineweight AutoCAD shows for "default" (LWDEFAULT's usual value), mm.
const DEFAULT_WEIGHT: f32 = 0.25;
/// The shortest line type period drawn, in chord tolerances: finer patterns show solid.
const MIN_PERIOD: f64 = 8.0;
/// How deep inserts may nest.
const MAX_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lay {
    /// Layer "0" inside a block: the insert's layer.
    Zero,
    At(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Col {
    ByBlock,
    /// The colour of a layer.
    Layer(Lay),
    Rgb([u8; 3]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Wt {
    ByBlock,
    Layer(Lay),
    Mm(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Lt {
    ByBlock,
    Layer(Lay),
    Solid,
    /// An LTYPE record with a pattern.
    Pattern(u32),
}

/// What an entity is drawn with, as it says it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Props {
    layer: Lay,
    col: Col,
    wt: Wt,
    lt: Lt,
    lts: f64,
}

impl Props {
    /// Inside something with properties `p` (an insert's): by block takes p's, layer "0" p's
    /// layer.
    fn within(&mut self, p: &Props) {
        let lay = |l: Lay| if l == Lay::Zero { p.layer } else { l };
        self.layer = lay(self.layer);
        self.col = match self.col {
            Col::ByBlock => p.col,
            Col::Layer(l) => Col::Layer(lay(l)),
            c => c,
        };
        self.wt = match self.wt {
            Wt::ByBlock => p.wt,
            Wt::Layer(l) => Wt::Layer(lay(l)),
            w => w,
        };
        self.lt = match self.lt {
            Lt::ByBlock => p.lt,
            Lt::Layer(l) => Lt::Layer(lay(l)),
            t => t,
        };
    }
    /// Only the by-block properties from p (an insert's attributes keep their own layer).
    fn by_block(&mut self, p: &Props) {
        if self.col == Col::ByBlock {
            self.col = p.col;
        }
        if self.wt == Wt::ByBlock {
            self.wt = p.wt;
        }
        if self.lt == Lt::ByBlock {
            self.lt = p.lt;
        }
    }
    fn solid(self) -> Props {
        Props {
            lt: Lt::Solid,
            ..self
        }
    }
    fn with_col(self, c: Col) -> Props {
        Props { col: c, ..self }
    }
}

#[derive(Clone, Debug)]
enum Shape {
    /// Points `at..at + n` of the geometry's `pts`; `restarts`: where the line type's pattern
    /// starts again.
    Path {
        at: u32,
        n: u32,
        closed: bool,
        restarts: Vec<usize>,
    },
    Fill(Vec<Vec<P3>>),
    Mask(Vec<Vec<P3>>),
    /// Separate segments, each pair of points one (`at..at + n` of `pts`).
    Segs {
        at: u32,
        n: u32,
    },
    /// A block's pieces through a transform; `lts` is how much it scales line type patterns.
    Insert {
        geo: Rc<Geo>,
        m: Box<Mat>,
        lts: f64,
    },
    /// From a point along a direction (and back, `both`).
    Ray(Box<(P3, P3)>, bool),
}

#[derive(Clone, Debug)]
struct Raw {
    p: Props,
    shape: Shape,
}

/// What a block (or one top-level entity) is drawn as, in drawing order: its own pieces, and
/// the blocks it inserts (`Shape::Insert`), each through its transform and with the properties
/// it hands down. Kept as a tree so an insert's pieces are never copied before the scene's items
/// are made.
#[derive(Debug, Default)]
struct Geo {
    raws: Vec<Raw>,
    /// The paths' points, one after another (one list rather than one each).
    pts: Vec<P3>,
}

impl Geo {
    fn clear(&mut self) {
        self.raws.clear();
        self.pts.clear();
    }
    /// Adds points; where they start and how many.
    fn add(&mut self, pts: impl IntoIterator<Item = P3>) -> (u32, u32) {
        let at = self.pts.len();
        self.pts.extend(pts);
        (at as u32, (self.pts.len() - at) as u32)
    }
}

struct LayerInfo {
    visible: bool,
    rgb: [u8; 3],
    wt: f32,
    lt: Lt,
}

struct StyleInfo {
    font: Font,
    width: f64,
    oblique: f64,
    fixed: f64,
}

/// A ray waiting for the extents to clip to.
struct PendingRay {
    entity: u32,
    layer: usize,
    rgb: [u8; 3],
    weight: f32,
    at: P2,
    dir: P2,
    both: bool,
    clip: Option<Rect>,
}

/// One view's resolution: its transform to the scene, the line type factor, its clip and the
/// layers frozen in it.
struct View {
    m: Mat,
    /// Line type patterns' scale in the scene's units (a viewport's zoom, unless PSLTSCALE
    /// keeps them paper-sized).
    ltk: f64,
    clip: Option<Rect>,
    frozen: Vec<usize>,
}

struct B<'a> {
    d: &'a Drawing,
    fonts: &'a dyn Fonts,
    /// Shape text and lay patterns (off in the first, rough pass for the extents).
    fine: bool,
    chord: f64,
    layers: Vec<SceneLayer>,
    linfo: Vec<LayerInfo>,
    layer_of: Map<u32, usize>,
    layer0: usize,
    /// LTYPE: what it is, its pattern (unscaled).
    ltypes: Map<u32, (Vec<f64>, f64)>,
    lt_kind: Map<u32, Lt>,
    styles: Map<u32, StyleInfo>,
    /// Entities by owner, in handle order (model and paper space by their block records).
    by_owner: std::cell::OnceCell<Map<u32, Vec<u32>>>,
    cache: Map<(u32, i32), Rc<Geo>>,
    expanding: Vec<u32>,
    ltscale: f64,
    psltscale: bool,
    pdmode: i64,
    pdsize: f64,
    /// The drawing's height, for point sizes relative to the view.
    view_h: f64,
    wipeout_frame: bool,
    ms: u32,
    ps: u32,
    items: Vec<Item>,
    /// The items' box so far: min x, min y, max x, max y.
    ext: [f64; 4],
    rays: Vec<PendingRay>,
}

fn header_f(d: &Drawing, n: &str, dflt: f64) -> f64 {
    match d.header.get(n) {
        Some(crate::header::Val::D(x)) => *x,
        Some(crate::header::Val::I(x)) => *x as f64,
        _ => dflt,
    }
}

fn weight_mm(code: i32) -> Option<f32> {
    (0..24)
        .contains(&code)
        .then(|| crate::WEIGHTS[code as usize] as f32 / 100.0)
}

/// Whether x is a number above zero (not NaN).
fn pos(x: f64) -> bool {
    x > 0.0
}

/// The power of two at or below x (for keying caches by chord tolerance).
fn bucket(x: f64) -> i32 {
    if !pos(x) || !x.is_finite() {
        return i32::MAX;
    }
    ((x.to_bits() >> 52) & 0x7FF) as i32 - 1023
}

fn pow2(e: i32) -> f64 {
    if e == i32::MAX {
        return f64::INFINITY;
    }
    f64::from_bits(((e.clamp(-1022, 1023) + 1023) as u64) << 52)
}

impl<'a> B<'a> {
    fn new(d: &'a Drawing, fonts: &'a dyn Fonts, fine: bool, chord: f64, all: bool) -> B<'a> {
        let ms = d.header.handle("BLOCK_RECORD_MODEL_SPACE").unwrap_or(0);
        let ps = d.header.handle("BLOCK_RECORD_PAPER_SPACE").unwrap_or(0);
        let mut b = B {
            d,
            fonts,
            fine,
            chord,
            layers: vec![],
            linfo: vec![],
            layer_of: Map::default(),
            layer0: 0,
            ltypes: Map::default(),
            lt_kind: Map::default(),
            styles: Map::default(),
            by_owner: std::cell::OnceCell::new(),
            cache: Map::default(),
            expanding: vec![],
            ltscale: header_f(d, "LTSCALE", 1.0),
            psltscale: !matches!(d.header.get("PSLTSCALE"), Some(crate::header::Val::B(false))),
            pdmode: header_f(d, "PDMODE", 0.0) as i64,
            pdsize: header_f(d, "PDSIZE", 0.0),
            view_h: 1.0,
            wipeout_frame: true,
            ms,
            ps,
            items: vec![],
            ext: [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
            rays: vec![],
        };
        if !pos(b.ltscale) {
            b.ltscale = 1.0;
        }
        // the tables, once: their records from the control objects (every object only when one
        // is missing)
        let mut recs: Vec<(u32, &Object)> = vec![];
        let mut whole = false;
        for name in ["LTYPE_CONTROL", "STYLE_CONTROL", "LAYER_CONTROL"] {
            let c = d
                .header
                .handle(name)
                .and_then(|h| d.objects.get(&h))
                .and_then(|o| match &o.data {
                    Data::LtypeControl(LtypeControl(c))
                    | Data::StyleControl(StyleControl(c))
                    | Data::LayerControl(LayerControl(c)) => Some(c),
                    _ => None,
                });
            match c {
                Some(c) => {
                    for x in c.entries.iter().chain(&c.extra) {
                        if let Some(o) = d.objects.get(&x.handle) {
                            recs.push((x.handle, o));
                        }
                    }
                }
                None => whole = true,
            }
        }
        if let Some(Object {
            data: Data::Dictionary(nod),
            ..
        }) = d
            .header
            .handle("DICT_NAMED_OBJECTS")
            .and_then(|h| d.objects.get(&h))
        {
            for (k, x) in &nod.items {
                if k.text() == "ACAD_WIPEOUT_VARS"
                    && let Some(o) = d.objects.get(&x.handle)
                {
                    recs.push((x.handle, o));
                }
            }
        }
        if whole {
            recs = d.objects.iter().map(|(h, o)| (*h, o)).collect();
        }
        recs.sort_by_key(|r| r.0);
        recs.dedup_by_key(|r| r.0);
        for &(h, o) in &recs {
            let h = &h;
            match &o.data {
                Data::Ltype(l) => {
                    let name = l.entry.name.text().to_uppercase();
                    let kind = match name.as_str() {
                        "BYLAYER" => Lt::Layer(Lay::Zero),
                        "BYBLOCK" => Lt::ByBlock,
                        _ if l.dashes.len() < 2 || l.dashes.iter().all(|x| x.length >= 0.0) => Lt::Solid,
                        _ => Lt::Pattern(*h),
                    };
                    let pat: Vec<f64> = l.dashes.iter().map(|x| x.length).collect();
                    let period = pat.iter().map(|x| x.abs()).sum();
                    b.lt_kind.insert(*h, kind);
                    b.ltypes.insert(*h, (pat, period));
                }
                Data::Style(s) => {
                    let mut font = s.font.text();
                    if font.is_empty() {
                        font = "txt".into();
                    }
                    b.styles.insert(
                        *h,
                        StyleInfo {
                            font: Font {
                                file: font,
                                big: s.bigfont.text(),
                            },
                            width: if s.width > 0.0 { s.width } else { 1.0 },
                            oblique: s.oblique,
                            fixed: s.fixed_height,
                        },
                    );
                }
                Data::WipeoutVariables(w) => b.wipeout_frame = w.display_frame != 0,
                _ => {}
            }
        }
        for &(h, o) in &recs {
            let h = &h;
            if let Data::Layer(l) = &o.data {
                let s = says(&l.color);
                let rgb = match s {
                    Says::Rgb(c) => c,
                    _ => [255; 3],
                };
                let name = l.entry.name.text();
                let on = l.color.index >= 0 && l.flags & 2 == 0;
                let frozen = l.flags & 1 != 0;
                let idx = b.layers.len();
                if name == "0" {
                    b.layer0 = idx;
                }
                let lt = match b.lt_kind.get(&l.ltype.handle) {
                    Some(Lt::Pattern(p)) => Lt::Pattern(*p),
                    _ => Lt::Solid,
                };
                b.linfo.push(LayerInfo {
                    visible: all || (on && !frozen),
                    rgb,
                    wt: weight_mm(((l.flags >> 5) & 31) as i32).unwrap_or(DEFAULT_WEIGHT),
                    lt,
                });
                b.layers.push(SceneLayer {
                    handle: *h,
                    name,
                    rgb,
                    on,
                    frozen,
                    locked: l.flags & 8 != 0,
                    plot: l.flags & 16 != 0,
                });
                b.layer_of.insert(*h, idx);
            }
        }
        if b.layers.is_empty() {
            b.layers.push(SceneLayer {
                handle: 0,
                name: "0".into(),
                rgb: [255; 3],
                on: true,
                frozen: false,
                locked: false,
                plot: true,
            });
            b.linfo.push(LayerInfo {
                visible: true,
                rgb: [255; 3],
                wt: DEFAULT_WEIGHT,
                lt: Lt::Solid,
            });
        }
        b
    }

    /// Entities by owner, made the first time a file needs it (R2000 has no owned lists).
    fn by_owner(&self) -> &Map<u32, Vec<u32>> {
        self.by_owner.get_or_init(|| {
            let mut m: Map<u32, Vec<u32>> = Map::default();
            for (h, o) in &self.d.objects {
                let Some(e) = &o.ent else { continue };
                let owner = match e.entmode {
                    1 => self.ps,
                    2 => self.ms,
                    _ => o.owner.handle,
                };
                m.entry(owner).or_default().push(*h);
            }
            m
        })
    }

    fn obj(&self, h: u32) -> Option<&'a Object> {
        self.d.objects.get(&h)
    }

    /// A block record's entities, in order.
    fn block_list(&self, br: u32) -> Vec<&'a Object> {
        let hs: Vec<u32> = match self.obj(br) {
            Some(Object {
                data: Data::BlockHeader(bh),
                ..
            }) if !bh.owned.is_empty() || self.d.ver() >= crate::version::Version::R2004 => {
                bh.owned.iter().map(|h| h.handle).collect()
            }
            _ => self.by_owner().get(&br).cloned().unwrap_or_default(),
        };
        let mut hs = hs;
        // draw order: a SORTENTSTABLE in the block's extension dictionary gives entities sort
        // handles to go by instead of their own
        let sort = self
            .obj(br)
            .and_then(|o| o.xdic)
            .and_then(|x| match self.obj(x.handle) {
                Some(Object {
                    data: Data::Dictionary(dict),
                    ..
                }) => dict
                    .items
                    .iter()
                    .find(|(k, _)| k.text() == "ACAD_SORTENTS")
                    .map(|(_, h)| h.handle),
                _ => None,
            })
            .and_then(|h| match self.obj(h) {
                Some(Object {
                    data: Data::SortEntsTable(t),
                    ..
                }) if !t.entries.is_empty() => Some(t),
                _ => None,
            });
        if let Some(t) = sort {
            let by: Map<u32, u32> = t.entries.iter().map(|(s, e)| (e.handle, s.handle)).collect();
            let mut keyed: Vec<(u32, u32)> = hs
                .iter()
                .map(|h| (by.get(h).copied().unwrap_or(*h), *h))
                .collect();
            if !keyed.windows(2).all(|w| w[0].0 <= w[1].0) {
                // (ties broken by the handle: the same order every time)
                keyed.sort_unstable();
                hs = keyed.into_iter().map(|k| k.1).collect();
            }
        }
        // (not filtered to entities here: that would touch every object twice)
        self.objs(&hs)
    }

    /// The objects of some handles, in their order (missing ones left out). A long list is
    /// matched against the objects in one walk rather than looked up one by one.
    fn objs(&self, hs: &[u32]) -> Vec<&'a Object> {
        if hs.len() < 256 || hs.len() * 8 < self.d.objects.len() {
            return hs.iter().filter_map(|h| self.obj(*h)).collect();
        }
        let mut order: Vec<(u32, usize)> = hs.iter().enumerate().map(|(i, h)| (*h, i)).collect();
        if !hs.windows(2).all(|w| w[0] < w[1]) {
            order.sort_unstable();
        }
        let mut found: Vec<Option<&'a Object>> = vec![None; hs.len()];
        let mut it = self.d.objects.range(order[0].0..).peekable();
        for (h, i) in order {
            // step through near handles, jump to far ones
            if it.peek().is_some_and(|(k, _)| **k + 64 < h) {
                it = self.d.objects.range(h..).peekable();
            }
            while it.peek().is_some_and(|(k, _)| **k < h) {
                it.next();
            }
            if let Some((k, o)) = it.peek()
                && **k == h
            {
                found[i] = Some(*o);
            }
        }
        found.into_iter().flatten().collect()
    }

    /// What a complex entity owns (attributes, vertices).
    fn owned(&self, h: u32, o: &Owned) -> Vec<&'a Object> {
        let hs: Vec<u32> = if !o.items.is_empty() || self.d.ver() >= crate::version::Version::R2004 {
            o.items.iter().map(|x| x.handle).collect()
        } else {
            self.by_owner().get(&h).cloned().unwrap_or_default()
        };
        self.objs(&hs)
    }

    fn lt_of(&self, h: u32, layer: Lay) -> Lt {
        match self.lt_kind.get(&h) {
            Some(Lt::Layer(_)) => Lt::Layer(layer),
            Some(k) => *k,
            None => Lt::Solid,
        }
    }

    fn col_of(&self, c: &Color, layer: Lay) -> Col {
        match says(c) {
            Says::ByLayer => Col::Layer(layer),
            Says::ByBlock => Col::ByBlock,
            Says::Rgb(x) => Col::Rgb(x),
        }
    }

    /// An entity's properties; `in_block` makes layer "0" the insert's.
    fn props(&self, e: &EntCommon, in_block: bool) -> Props {
        let li = self.layer_of.get(&e.layer.handle).copied().unwrap_or(self.layer0);
        let layer = if in_block && li == self.layer0 {
            Lay::Zero
        } else {
            Lay::At(li)
        };
        let wt = match e.lineweight {
            29 => Wt::Layer(layer),
            30 => Wt::ByBlock,
            k => Wt::Mm(weight_mm(k as i32).unwrap_or(DEFAULT_WEIGHT)),
        };
        let lt = match e.ltype_flags {
            0 => Lt::Layer(layer),
            1 => Lt::ByBlock,
            2 => Lt::Solid,
            _ => self.lt_of(e.ltype.handle, layer),
        };
        Props {
            layer,
            col: self.col_of(&e.color, layer),
            wt,
            lt,
            lts: if e.ltype_scale > 0.0 { e.ltype_scale } else { 1.0 },
        }
    }

    fn shown(&self, l: Lay) -> bool {
        match l {
            Lay::Zero => true,
            Lay::At(i) => self.linfo[i].visible,
        }
    }

    fn style(&self, h: u32) -> StyleInfo {
        match self.styles.get(&h) {
            Some(s) => StyleInfo {
                font: s.font.clone(),
                width: s.width,
                oblique: s.oblique,
                fixed: s.fixed,
            },
            None => StyleInfo {
                font: Font {
                    file: "txt".into(),
                    big: String::new(),
                },
                width: 1.0,
                oblique: 0.0,
                fixed: 0.0,
            },
        }
    }

    fn dimvars(&self, h: u32) -> Option<&'a DimVars> {
        match self.obj(h) {
            Some(Object {
                data: Data::DimStyle(s),
                ..
            }) => Some(&s.vars),
            _ => None,
        }
    }

    // ---- the pieces

    fn path(&self, out: &mut Geo, p: Props, pts: impl AsRef<[P3]>, closed: bool) {
        let pts = pts.as_ref();
        if pts.len() >= 2 {
            let (at, n) = out.add(pts.iter().copied());
            out.raws.push(Raw {
                p,
                shape: Shape::Path {
                    at,
                    n,
                    closed,
                    restarts: vec![],
                },
            });
        }
    }

    fn path_m(&self, out: &mut Geo, p: Props, m: &Mat, pts: impl AsRef<[P3]>, closed: bool) {
        let pts = pts.as_ref();
        if m.is_id() {
            return self.path(out, p, pts, closed);
        }
        if pts.len() >= 2 {
            let (at, n) = out.add(pts.iter().map(|q| m.apply(*q)));
            out.raws.push(Raw {
                p,
                shape: Shape::Path {
                    at,
                    n,
                    closed,
                    restarts: vec![],
                },
            });
        }
    }

    fn fill_m(&self, out: &mut Geo, p: Props, m: &Mat, rings: Vec<Vec<P3>>, mask: bool) {
        let rings: Vec<Vec<P3>> = rings
            .into_iter()
            .filter(|r| r.len() >= 3)
            .map(|r| {
                if m.is_id() {
                    r
                } else {
                    r.into_iter().map(|q| m.apply(q)).collect()
                }
            })
            .collect();
        if rings.is_empty() {
            return;
        }
        let p = p.solid();
        out.raws.push(Raw {
            p,
            shape: if mask {
                Shape::Mask(rings)
            } else {
                Shape::Fill(rings)
            },
        });
    }

    /// Laid-out text through `m` (its plane to the block's coordinates).
    fn laid(&self, out: &mut Geo, p: Props, m: &Mat, l: Laid) {
        let p = p.solid();
        let col = |c: Option<Says>| match c {
            None => p,
            Some(Says::ByBlock) => p.with_col(Col::ByBlock),
            Some(Says::ByLayer) => p.with_col(Col::Layer(p.layer)),
            Some(Says::Rgb(x)) => p.with_col(Col::Rgb(x)),
        };
        for (c, s) in l.strokes {
            self.path_m(
                out,
                col(c),
                m,
                s.into_iter().map(|q| [q[0], q[1], 0.0]).collect::<Vec<P3>>(),
                false,
            );
        }
        for (c, rs) in l.fills {
            let rings = rs
                .into_iter()
                .map(|r| r.into_iter().map(|q| [q[0], q[1], 0.0]).collect())
                .collect();
            self.fill_m(out, col(c), m, rings, false);
        }
    }

    /// A stand-in box for text in the rough pass.
    fn text_box(&self, out: &mut Geo, p: Props, m: &Mat, lo: P2, hi: P2) {
        let r = vec![
            [lo[0], lo[1], 0.0],
            [hi[0], lo[1], 0.0],
            [hi[0], hi[1], 0.0],
            [lo[0], hi[1], 0.0],
        ];
        self.path_m(out, p, m, r, true);
    }

    fn look(&self, st: &StyleInfo, h: f64, w: f64, obl: f64) -> Look {
        Look {
            font: st.font.clone(),
            height: h,
            width: w,
            oblique: obl,
            tracking: 1.0,
            color: None,
            under: false,
            over: false,
            strike: false,
        }
    }

    /// TEXT (and ATTRIB, ATTDEF with the text given).
    fn text(&self, out: &mut Geo, p: Props, t: &Text, s: &str) {
        let st = self.style(t.style.handle);
        let mut h = if t.height > 0.0 {
            t.height
        } else if st.fixed > 0.0 {
            st.fixed
        } else {
            2.5
        };
        let w = if t.width > 0.0 { t.width } else { st.width };
        let obl = t.oblique;
        let look = self.look(&st, h, w, obl);
        let (mut laid, mut adv) = if self.fine {
            text::line(self.fonts, s, &look)
        } else {
            let n = s.chars().count() as f64;
            (Laid::default(), n * 0.9 * h * w)
        };
        let mut rot = t.rotation;
        let mut refp = t.insert;
        let ap = t.align_pt.unwrap_or(t.insert);
        let (ha, va) = (t.halign, t.valign);
        if matches!(ha, 3 | 5) && t.align_pt.is_some() {
            let dx = ap[0] - t.insert[0];
            let dy = ap[1] - t.insert[1];
            let l = (dx * dx + dy * dy).sqrt();
            if l > 1e-12 && adv > 1e-12 {
                rot = dmath::atan2(dy, dx);
                let k = l / adv;
                if ha == 3 {
                    h *= k;
                    laid.map(|q| [q[0] * k, q[1] * k]);
                } else {
                    laid.map(|q| [q[0] * k, q[1]]);
                }
                adv = l;
            }
        } else if ha != 0 || va != 0 {
            refp = ap;
        }
        let off = text::align_offset(ha, va, adv, h);
        let mx = if t.generation & 2 != 0 { -1.0 } else { 1.0 };
        let my = if t.generation & 4 != 0 { -1.0 } else { 1.0 };
        let m = Mat::ocs(t.extrusion)
            .then_from(&Mat::translate([refp[0], refp[1], t.elevation]))
            .then_from(&Mat::rotate_z(rot))
            .then_from(&Mat::scale([mx, my, 1.0]))
            .then_from(&Mat::translate([off[0], off[1], 0.0]));
        if self.fine {
            self.laid(out, p, &m, laid);
        } else {
            self.text_box(out, p, &m, [0.0, -h / 3.0], [adv, h]);
        }
    }

    /// MTEXT's fields (an MTEXT, or a multi-line attribute's).
    #[allow(clippy::too_many_arguments)]
    fn mtext(
        &self,
        out: &mut Geo,
        p: Props,
        insert: P3,
        x_axis: P3,
        normal: P3,
        style: u32,
        height: f64,
        s: &str,
        fr: &Frame,
        bg: Option<(i32, &Color, f64)>,
    ) {
        let st = self.style(style);
        let h = if height > 0.0 {
            height
        } else if st.fixed > 0.0 {
            st.fixed
        } else {
            2.5
        };
        let n = unit(normal).unwrap_or([0.0, 0.0, 1.0]);
        let x = unit(sub(x_axis, mul(n, geom::dot(x_axis, n)))).unwrap_or_else(|| geom::ocs_axes(n).0);
        let y = cross(n, x);
        let m = Mat::from_cols(x, y, n, insert);
        let look = self.look(&st, h, 1.0, st.oblique);
        if !self.fine {
            let plain = text::text_plain(s);
            let lines = plain.lines().count().max(1) as f64;
            let w = if fr.width > 0.0 {
                fr.width
            } else {
                plain.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f64 * 0.9 * h
            };
            let tall = h * (1.0 + (lines - 1.0) * 5.0 / 3.0 * fr.spacing);
            let col = (fr.attachment.clamp(1, 9) - 1) % 3;
            let row = (fr.attachment.clamp(1, 9) - 1) / 3;
            let x0 = [0.0, -w / 2.0, -w][col as usize];
            let y1 = [0.0, tall / 2.0, tall][row as usize];
            self.text_box(out, p, &m, [x0, y1 - tall], [x0 + w, y1]);
            return;
        }
        let laid = text::mtext(self.fonts, s, &look, fr);
        if let Some((flags, c, scale)) = bg
            && flags & 3 != 0
        {
            let g = (scale.max(1.0) - 1.0) * h;
            let (lo, hi) = (laid.lo, laid.hi);
            let r = vec![
                [lo[0] - g, lo[1] - g, 0.0],
                [hi[0] + g, lo[1] - g, 0.0],
                [hi[0] + g, hi[1] + g, 0.0],
                [lo[0] - g, hi[1] + g, 0.0],
            ];
            if flags & 2 != 0 {
                self.fill_m(out, p, &m, vec![r], true);
            } else {
                let c = self.col_of(c, p.layer);
                self.fill_m(out, p.with_col(c), &m, vec![r], false);
            }
        }
        self.laid(out, p, &m, laid);
    }

    #[allow(clippy::too_many_arguments)]
    fn arrow(&mut self, out: &mut Geo, p: Props, tip: P3, from: P3, size: f64, block: u32, scale: f64) {
        let d = sub(tip, from);
        let Some(u) = unit([d[0], d[1], d[2]]) else { return };
        if !pos(size) {
            return;
        }
        if block != 0
            && let Some(Object {
                data: Data::BlockHeader(bh),
                ..
            }) = self.obj(block)
        {
            let rot = dmath::atan2(u[1], u[0]);
            let m = Mat::translate(tip)
                .then_from(&Mat::rotate_z(rot))
                .then_from(&Mat::scale([size; 3]))
                .then_from(&Mat::translate(mul(bh.base, -1.0)));
            self.expand(out, block, &m, &p, scale * size);
            return;
        }
        let side = [-u[1] * size / 6.0, u[0] * size / 6.0, 0.0];
        let base = sub(tip, mul(u, size));
        self.fill_m(
            out,
            p,
            &Mat::ID,
            vec![vec![tip, add(base, side), sub(base, side)]],
            false,
        );
    }

    /// A block's pieces through `m`, with `p` for what is by block or on layer "0".
    fn expand(&mut self, out: &mut Geo, block: u32, m: &Mat, p: &Props, scale: f64) {
        let geo = self.block_geo(block, scale);
        if geo.raws.is_empty() {
            return;
        }
        out.raws.push(Raw {
            p: *p,
            shape: Shape::Insert {
                geo,
                m: Box::new(*m),
                lts: m.mean_scale(),
            },
        });
    }

    /// A block's pieces in its own coordinates, made once per chord step.
    fn block_geo(&mut self, block: u32, scale: f64) -> Rc<Geo> {
        let b = bucket(self.chord / scale.max(1e-300));
        if let Some(r) = self.cache.get(&(block, b)) {
            return r.clone();
        }
        if self.expanding.contains(&block) || self.expanding.len() >= MAX_DEPTH {
            return Rc::new(Geo::default());
        }
        self.expanding.push(block);
        // tessellate at the step's tolerance, in the block's units
        let s = if pow2(b).is_finite() {
            self.chord / pow2(b)
        } else {
            scale
        };
        let list = self.block_list(block);
        let mut g = Geo {
            raws: Vec::with_capacity(list.len()),
            pts: Vec::with_capacity(list.len() * 4),
        };
        for o in list {
            self.entity(o, true, s, &mut g);
        }
        self.expanding.pop();
        let r = Rc::new(g);
        self.cache.insert((block, b), r.clone());
        r
    }

    /// The pieces of entity `h`, in its block's coordinates; `scale` is how much bigger they
    /// end up in the scene (for the chord tolerance).
    fn entity(&mut self, o: &'a Object, in_block: bool, scale: f64, out: &mut Geo) {
        let h = o.handle;
        let Some(e) = &o.ent else { return };
        if e.invisible & 1 != 0 {
            return;
        }
        let p = self.props(e, in_block);
        if !self.shown(p.layer) {
            // an insert on an off (not frozen) layer still shows what is on other layers
            let frozen = match p.layer {
                Lay::At(i) => self.layers[i].frozen,
                Lay::Zero => false,
            };
            if frozen || !matches!(o.data, Data::Insert(_) | Data::MInsert(_)) {
                return;
            }
        }
        let chord = self.chord / scale.max(1e-300);
        let start = out.raws.len();
        self.shapes(h, o, p, in_block, scale, chord, out);
        // drop what is on hidden layers
        // (an insert stays: what in it is on other layers still shows)
        let keep = |b: &Self, r: &Raw| b.shown(r.p.layer) || matches!(r.shape, Shape::Insert { .. });
        if out.raws[start..].iter().any(|r| !keep(self, r)) {
            let mut k = start;
            for i in start..out.raws.len() {
                if keep(self, &out.raws[i]) {
                    out.raws.swap(k, i);
                    k += 1;
                }
            }
            out.raws.truncate(k);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn shapes(
        &mut self,
        h: u32,
        o: &'a Object,
        p: Props,
        in_block: bool,
        scale: f64,
        chord: f64,
        out: &mut Geo,
    ) {
        match &o.data {
            Data::Line(l) => self.path(out, p, [l.start, l.end], false),
            Data::Circle(c) => {
                let mut pts = vec![];
                arc_pts(c.center, c.radius, 0.0, dmath::TAU, chord, &mut pts);
                pts.pop();
                self.path_m(out, p, &Mat::ocs(c.extrusion), pts, true);
            }
            Data::Arc(a) => {
                let mut pts = vec![];
                arc_pts(
                    a.circle.center,
                    a.circle.radius,
                    a.start,
                    ccw_sweep(a.start, a.end),
                    chord,
                    &mut pts,
                );
                self.path_m(out, p, &Mat::ocs(a.circle.extrusion), pts, false);
            }
            Data::Ellipse(el) => {
                let n = unit(el.extrusion).unwrap_or([0.0, 0.0, 1.0]);
                let minor = mul(cross(n, el.major), el.ratio);
                let sweep = ccw_sweep(el.start, el.end);
                let mut pts = vec![];
                ellipse_pts(
                    el.center,
                    el.major,
                    minor,
                    el.start,
                    el.start + sweep,
                    chord,
                    &mut pts,
                );
                let full = sweep >= dmath::TAU - 1e-9;
                if full {
                    pts.pop();
                }
                self.path(out, p, pts, full);
            }
            Data::Spline(s) => {
                let pts = if s.points.len() >= 2 {
                    let w: &[f64] = if s.weighted || s.rational { &s.weights } else { &[] };
                    nurbs_pts(s.degree.max(1) as usize, &s.knots, &s.points, w, chord)
                } else {
                    let t = |v: P3| (v != [0.0; 3]).then_some(v);
                    let mut fit = s.fit.clone();
                    if s.closed && fit.len() > 2 && fit.first() != fit.last() {
                        fit.push(fit[0]);
                    }
                    let mode = match s.knot_param {
                        1 => 1,
                        2 => 2,
                        _ => 0,
                    };
                    interp_pts(&fit, t(s.begin_tangent), t(s.end_tangent), mode, chord)
                };
                self.path(out, p, pts, false);
            }
            Data::LwPolyline(l) => {
                let n = if l.normal == [0.0; 3] {
                    [0.0, 0.0, 1.0]
                } else {
                    l.normal
                };
                let v: Vec<(P3, f64)> = l
                    .points
                    .iter()
                    .enumerate()
                    .map(|(i, q)| ([q[0], q[1], l.elevation], l.bulges.get(i).copied().unwrap_or(0.0)))
                    .collect();
                let widths: Vec<[f64; 2]> = (0..v.len())
                    .map(|i| {
                        if l.const_width > 0.0 {
                            [l.const_width; 2]
                        } else {
                            l.widths.get(i).copied().unwrap_or([0.0; 2])
                        }
                    })
                    .collect();
                self.polyline(
                    out,
                    p,
                    &Mat::ocs(n),
                    &v,
                    &widths,
                    l.flag & 512 != 0,
                    l.flag & 256 != 0,
                    chord,
                );
            }
            Data::Polyline2d(pl) => {
                let vs = self.owned(h, &pl.vertices);
                let spline_fit = pl.flags & 4 != 0;
                let mut v = vec![];
                let mut widths = vec![];
                for vo in vs {
                    if let Data::Vertex2d(x) = &vo.data {
                        // a spline-fit polyline's frame (control) vertices are not drawn
                        if x.flags & 16 != 0 {
                            continue;
                        }
                        v.push((
                            [x.at[0], x.at[1], pl.elevation],
                            if spline_fit { 0.0 } else { x.bulge },
                        ));
                        let mut w = [x.start_width, x.end_width];
                        if w == [0.0; 2] {
                            w = [pl.start_width, pl.end_width];
                        }
                        widths.push(w);
                    }
                }
                self.polyline(
                    out,
                    p,
                    &Mat::ocs(pl.extrusion),
                    &v,
                    &widths,
                    pl.flags & 1 != 0,
                    pl.flags & 128 != 0,
                    chord,
                );
            }
            Data::Polyline3d(pl) => {
                let pts: Vec<P3> = self
                    .owned(h, &pl.vertices)
                    .into_iter()
                    .filter_map(|vo| match &vo.data {
                        Data::Vertex3d(x) if x.flags & 16 == 0 || pl.curve == 0 => Some(x.at),
                        _ => None,
                    })
                    .collect();
                self.path(out, p, pts, pl.flags & 1 != 0);
            }
            Data::PolylinePface(pf) => {
                let vs = self.owned(h, &pf.vertices);
                let pts: Vec<P3> = vs
                    .iter()
                    .filter_map(|vo| match &vo.data {
                        Data::VertexPface(x) => Some(x.0.at),
                        _ => None,
                    })
                    .collect();
                for vo in &vs {
                    if let Data::VertexPfaceFace(f) = &vo.data {
                        let idx: Vec<i16> = f.index.iter().copied().filter(|i| *i != 0).collect();
                        for k in 0..idx.len() {
                            let (a, b) = (idx[k], idx[(k + 1) % idx.len()]);
                            if a < 0 {
                                continue;
                            }
                            let (Some(pa), Some(pb)) =
                                (pts.get(a as usize - 1), pts.get(b.unsigned_abs() as usize - 1))
                            else {
                                continue;
                            };
                            self.path(out, p, [*pa, *pb], false);
                        }
                    }
                }
            }
            Data::PolylineMesh(pm) => {
                let pts: Vec<P3> = self
                    .owned(h, &pm.vertices)
                    .into_iter()
                    .filter_map(|vo| match &vo.data {
                        Data::VertexMesh(x) if x.0.flags & 16 == 0 => Some(x.0.at),
                        _ => None,
                    })
                    .collect();
                let (mut m, mut n) = (pm.m.max(0) as usize, pm.n.max(0) as usize);
                if pm.curve != 0 && pts.len() == (pm.m_density.max(0) * pm.n_density.max(0)) as usize {
                    m = pm.m_density as usize;
                    n = pm.n_density as usize;
                }
                if m * n == 0 || pts.len() < m * n {
                    return;
                }
                for i in 0..m {
                    self.path(
                        out,
                        p,
                        (0..n).map(|j| pts[i * n + j]).collect::<Vec<P3>>(),
                        pm.flags & 32 != 0,
                    );
                }
                for j in 0..n {
                    self.path(
                        out,
                        p,
                        (0..m).map(|i| pts[i * n + j]).collect::<Vec<P3>>(),
                        pm.flags & 1 != 0,
                    );
                }
            }
            Data::Point(pt) => self.point(out, p, pt),
            Data::Solid(s) | Data::Trace(Trace(s)) => {
                let c = s.corners;
                let z = s.elevation;
                let ring = vec![
                    [c[0][0], c[0][1], z],
                    [c[1][0], c[1][1], z],
                    [c[3][0], c[3][1], z],
                    [c[2][0], c[2][1], z],
                ];
                self.fill_m(out, p, &Mat::ocs(s.extrusion), vec![ring], false);
            }
            Data::Face3d(f) => {
                let c = f.corners;
                let n = if c[3] == c[2] { 3 } else { 4 };
                for k in 0..n {
                    if f.invisible & (1 << k) == 0 {
                        self.path(out, p, [c[k], c[(k + 1) % n]], false);
                    }
                }
            }
            Data::Text(t) => {
                let s = t.text.text();
                self.text(out, p, t, &s);
            }
            Data::Attrib(a) => self.attrib(out, p, a, false),
            Data::AttDef(a) => {
                if !in_block {
                    self.attrib(out, p, &a.attrib, true);
                }
            }
            Data::MText(m) => {
                let fr = Frame {
                    width: m.rect_width,
                    attachment: m.attachment,
                    spacing_style: m.linespacing_style,
                    spacing: if m.linespacing_factor > 0.0 {
                        m.linespacing_factor
                    } else {
                        1.0
                    },
                };
                let s = m.text.text();
                self.mtext(
                    out,
                    p,
                    m.insert,
                    m.x_axis,
                    m.extrusion,
                    m.style.handle,
                    m.text_height,
                    &s,
                    &fr,
                    Some((m.bg_flags, &m.bg_color, m.bg_scale)),
                );
            }
            Data::Insert(ins) => self.insert(out, h, p, ins, 1, 1, 0.0, 0.0, in_block, scale),
            Data::MInsert(mi) => self.insert(
                out,
                h,
                p,
                &mi.insert,
                mi.rows.max(1) as usize,
                mi.cols.max(1) as usize,
                mi.row_spacing,
                mi.col_spacing,
                in_block,
                scale,
            ),
            Data::DimOrdinate(x) => self.dim(out, p, &x.dim, scale),
            Data::DimLinear(x) => self.dim(out, p, &x.dim, scale),
            Data::DimAligned(x) => self.dim(out, p, &x.dim, scale),
            Data::DimAng3Pt(x) => self.dim(out, p, &x.dim, scale),
            Data::DimAng2Ln(x) => self.dim(out, p, &x.dim, scale),
            Data::DimRadius(x) => self.dim(out, p, &x.dim, scale),
            Data::DimDiameter(x) => self.dim(out, p, &x.dim, scale),
            Data::ArcDimension(x) => self.dim(out, p, &x.dim, scale),
            Data::LargeRadialDimension(x) => self.dim(out, p, &x.dim, scale),
            Data::Table(t) => {
                if let Some(Object {
                    data: Data::BlockHeader(bh),
                    ..
                }) = self.obj(t.block.handle)
                {
                    let m = Mat::ocs(t.extrusion)
                        .then_from(&Mat::translate(t.insert))
                        .then_from(&Mat::rotate_z(t.rotation))
                        .then_from(&Mat::scale(t.scale))
                        .then_from(&Mat::translate(mul(bh.base, -1.0)));
                    let s = scale * m.max_scale();
                    self.expand(out, t.block.handle, &m, &p, s);
                }
            }
            Data::Leader(l) => self.leader(out, p, l, scale, chord),
            Data::MLeader(ml) => self.mleader(out, p, ml, scale, chord),
            Data::Tolerance(t) => self.tolerance(out, p, t),
            Data::Hatch(ht) => self.hatch(out, p, ht, chord),
            Data::MLine(ml) => self.mline(out, p, ml, chord),
            Data::Wipeout(Wipeout(im)) => {
                let ring = self.image_ring(im);
                if ring.len() >= 3 {
                    self.fill_m(out, p, &Mat::ID, vec![ring.clone()], true);
                    if self.wipeout_frame {
                        self.path(out, p, ring, true);
                    }
                }
            }
            Data::Image(im) => {
                let ring = self.image_ring(im);
                self.path(out, p, ring, true);
            }
            Data::Ray(r) => out.raws.push(Raw {
                p,
                shape: Shape::Ray(Box::new((r.at, r.direction)), false),
            }),
            Data::Xline(Xline(r)) => out.raws.push(Raw {
                p,
                shape: Shape::Ray(Box::new((r.at, r.direction)), true),
            }),
            Data::Shape(sh) => {
                let st = self.style(sh.style.handle);
                let Some(c) = char::from_u32(sh.number as u16 as u32) else {
                    return;
                };
                let look = self.look(
                    &st,
                    sh.scale,
                    if sh.width > 0.0 { sh.width } else { 1.0 },
                    sh.oblique,
                );
                let m = Mat::ocs(sh.extrusion)
                    .then_from(&Mat::translate(sh.at))
                    .then_from(&Mat::rotate_z(sh.rotation));
                if self.fine {
                    let (laid, _) = text::line(self.fonts, &c.to_string(), &look);
                    self.laid(out, p, &m, laid);
                }
            }
            Data::Region(Region(a)) | Data::Body(Body(a)) => self.acis(out, p, a),
            Data::Solid3d(s) => self.acis(out, p, &s.acis),
            Data::Viewport(_) => {}
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn polyline(
        &self,
        out: &mut Geo,
        p: Props,
        m: &Mat,
        v: &[(P3, f64)],
        widths: &[[f64; 2]],
        closed: bool,
        continuous: bool,
        chord: f64,
    ) {
        if v.is_empty() {
            return;
        }
        let wide = widths.iter().any(|w| w[0] > 0.0 || w[1] > 0.0);
        if wide && v.len() >= 2 {
            self.wide_polyline(out, p, m, v, widths, closed, chord);
            return;
        }
        let (pts, at) = bulged(v, closed, chord);
        if pts.len() == 1 {
            // a polyline of one point shows as a dot
            let q = pts[0];
            self.path_m(out, p, m, [q, add(q, [chord.max(1e-9), 0.0, 0.0])], false);
            return;
        }
        let (start, n) = if m.is_id() {
            out.add(pts)
        } else {
            out.add(pts.into_iter().map(|q| m.apply(q)))
        };
        let restarts = if continuous { vec![] } else { at };
        out.raws.push(Raw {
            p,
            shape: Shape::Path {
                at: start,
                n,
                closed,
                restarts,
            },
        });
    }

    /// A polyline with widths: each segment a filled band, mitred at the vertices.
    #[allow(clippy::too_many_arguments)]
    fn wide_polyline(
        &self,
        out: &mut Geo,
        p: Props,
        m: &Mat,
        v: &[(P3, f64)],
        widths: &[[f64; 2]],
        closed: bool,
        chord: f64,
    ) {
        let n = v.len();
        let segs = if closed { n } else { n - 1 };
        // each segment's points and its tangents at the ends
        let mut seg_pts: Vec<Vec<P3>> = vec![];
        for i in 0..segs {
            let mut s = vec![v[i].0];
            geom::bulge_pts(v[i].0, v[(i + 1) % n].0, v[i].1, chord, &mut s);
            seg_pts.push(s);
        }
        let dir = |a: P3, b: P3| -> P2 {
            let d = [b[0] - a[0], b[1] - a[1]];
            let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
            if l > 0.0 { [d[0] / l, d[1] / l] } else { [1.0, 0.0] }
        };
        let tin = |i: usize| -> P2 {
            let s = &seg_pts[i];
            dir(s[s.len() - 2], s[s.len() - 1])
        };
        let tout = |i: usize| -> P2 { dir(seg_pts[i][0], seg_pts[i][1]) };
        // the offset at a vertex between an incoming and outgoing direction, half-width w
        let join = |c: P3, a: Option<P2>, b: Option<P2>, w: f64| -> (P3, P3) {
            let nl = |d: P2| [-d[1], d[0]];
            let (na, nb) = (a.map(nl), b.map(nl));
            let nrm = match (na, nb) {
                (Some(x), Some(y)) => {
                    let s = [x[0] + y[0], x[1] + y[1]];
                    let l = (s[0] * s[0] + s[1] * s[1]).sqrt();
                    let cosh = if l > 1e-9 {
                        (s[0] * x[0] + s[1] * x[1]) / l
                    } else {
                        0.0
                    };
                    if l > 1e-9 && cosh > 0.25 {
                        [s[0] / l / cosh, s[1] / l / cosh]
                    } else {
                        y
                    }
                }
                (Some(x), None) => x,
                (None, Some(y)) => y,
                _ => [0.0, 1.0],
            };
            (
                [c[0] + nrm[0] * w, c[1] + nrm[1] * w, c[2]],
                [c[0] - nrm[0] * w, c[1] - nrm[1] * w, c[2]],
            )
        };
        for i in 0..segs {
            let s = &seg_pts[i];
            let (w0, w1) = (widths[i][0] / 2.0, widths[i][1] / 2.0);
            let prev = if i > 0 {
                Some(tin(i - 1))
            } else if closed {
                Some(tin(segs - 1))
            } else {
                None
            };
            let next = if i + 1 < segs {
                Some(tout(i + 1))
            } else if closed {
                Some(tout(0))
            } else {
                None
            };
            // the joints use the widths meeting there
            let (l0, r0) = join(s[0], prev, Some(tout(i)), w0);
            let (l1, r1) = join(*s.last().unwrap_or(&s[0]), Some(tin(i)), next, w1);
            let k = s.len();
            let mut left = vec![l0];
            let mut right = vec![r0];
            for j in 1..k - 1 {
                let t = j as f64 / (k - 1) as f64;
                let w = w0 + (w1 - w0) * t;
                let d = dir(s[j - 1], s[j + 1]);
                let nn = [-d[1], d[0]];
                left.push([s[j][0] + nn[0] * w, s[j][1] + nn[1] * w, s[j][2]]);
                right.push([s[j][0] - nn[0] * w, s[j][1] - nn[1] * w, s[j][2]]);
            }
            left.push(l1);
            right.push(r1);
            right.reverse();
            left.extend(right);
            if w0 == 0.0 && w1 == 0.0 {
                self.path_m(out, p, m, s.clone(), false);
            } else {
                self.fill_m(out, p, m, vec![left], false);
            }
        }
    }

    fn point(&self, out: &mut Geo, p: Props, pt: &Point) {
        let mode = self.pdmode;
        let size = if self.pdsize > 0.0 {
            self.pdsize
        } else if self.pdsize < 0.0 {
            -self.pdsize / 100.0 * self.view_h
        } else {
            0.05 * self.view_h
        };
        let m = Mat::ocs(pt.extrusion);
        let c = pt.at;
        let r = size / 2.0;
        let q = |dx: f64, dy: f64| [c[0] + dx, c[1] + dy, c[2]];
        let p = p.solid();
        match mode & 7 {
            0 => {
                let d = (self.chord * 0.5).min(size * 1e-3).max(1e-12);
                self.path(out, p, [c, q(d, 0.0)], false)
            }
            2 => {
                self.path(out, p, [q(-r, 0.0), q(r, 0.0)], false);
                self.path(out, p, [q(0.0, -r), q(0.0, r)], false);
            }
            3 => {
                self.path(out, p, [q(-r, -r), q(r, r)], false);
                self.path(out, p, [q(-r, r), q(r, -r)], false);
            }
            4 => self.path(out, p, [c, q(0.0, r)], false),
            _ => {}
        }
        if mode & 32 != 0 {
            let mut pts = vec![];
            arc_pts(c, r, 0.0, dmath::TAU, self.chord, &mut pts);
            pts.pop();
            self.path(out, p, pts, true);
        }
        if mode & 64 != 0 {
            self.path(out, p, [q(-r, -r), q(r, -r), q(r, r), q(-r, r)], true);
        }
        let _ = m;
    }

    fn attrib(&self, out: &mut Geo, p: Props, a: &Attrib, def: bool) {
        if a.flags & 1 != 0 {
            return;
        }
        if matches!(a.kind, 2 | 4) {
            let m = &a.mtext;
            let fr = Frame {
                width: m.rect_width,
                attachment: m.attachment,
                spacing_style: m.linespacing_style,
                spacing: if m.linespacing > 0.0 { m.linespacing } else { 1.0 },
            };
            let s = if def { a.tag.text() } else { m.text.text() };
            let style = if m.style.is_null() {
                a.text.style.handle
            } else {
                m.style.handle
            };
            self.mtext(
                out,
                p,
                m.insert,
                m.x_axis,
                m.extrusion,
                style,
                m.height,
                &s,
                &fr,
                None,
            );
            return;
        }
        let s = if def { a.tag.text() } else { a.text.text.text() };
        self.text(out, p, &a.text, &s);
    }

    #[allow(clippy::too_many_arguments)]
    fn insert(
        &mut self,
        out: &mut Geo,
        h: u32,
        p: Props,
        ins: &Insert,
        rows: usize,
        cols: usize,
        row_sp: f64,
        col_sp: f64,
        in_block: bool,
        scale: f64,
    ) {
        let bh_h = ins.block.handle;
        if let Some(Object {
            data: Data::BlockHeader(bh),
            ..
        }) = self.obj(bh_h)
        {
            let base = Mat::ocs(ins.extrusion)
                .then_from(&Mat::translate(ins.at))
                .then_from(&Mat::rotate_z(ins.rotation));
            let tail = Mat::scale(ins.scale).then_from(&Mat::translate(mul(bh.base, -1.0)));
            let s = scale * base.then_from(&tail).max_scale();
            for r in 0..rows.min(1000) {
                for c in 0..cols.min(1000) {
                    let m = if r == 0 && c == 0 {
                        base.then_from(&tail)
                    } else {
                        base.then_from(&Mat::translate([c as f64 * col_sp, r as f64 * row_sp, 0.0]))
                            .then_from(&tail)
                    };
                    self.expand(out, bh_h, &m, &p, s);
                }
            }
        }
        if ins.has_attribs {
            for a in self.owned(h, &ins.attribs) {
                if let (Data::Attrib(at), Some(e)) = (&a.data, &a.ent) {
                    if e.invisible & 1 != 0 {
                        continue;
                    }
                    let mut ap = self.props(e, in_block);
                    ap.by_block(&p);
                    if self.shown(ap.layer) {
                        self.attrib(out, ap, at, false);
                    }
                }
            }
        }
    }

    fn dim(&mut self, out: &mut Geo, p: Props, d: &DimCommon, scale: f64) {
        let bh = d.block.handle;
        if !matches!(
            self.obj(bh),
            Some(Object {
                data: Data::BlockHeader(_),
                ..
            })
        ) {
            return;
        }
        let sc = if d.ins_scale == [0.0; 3] {
            [1.0; 3]
        } else {
            d.ins_scale
        };
        let m = Mat::translate([d.pt12[0], d.pt12[1], 0.0])
            .then_from(&Mat::rotate_z(d.ins_rotation))
            .then_from(&Mat::scale(sc));
        let s = scale * m.max_scale();
        self.expand(out, bh, &m, &p, s);
    }

    fn arrow_size(&self, dimstyle: u32) -> (f64, u32) {
        match self.dimvars(dimstyle) {
            Some(v) => {
                let s = if v.scale > 0.0 { v.scale } else { 1.0 };
                let ldr = match self.obj(dimstyle) {
                    Some(Object {
                        data: Data::DimStyle(ds),
                        ..
                    }) => ds.ldrblk.handle,
                    _ => 0,
                };
                (v.asz * s, ldr)
            }
            None => (
                header_f(self.d, "DIMASZ", 2.5) * header_f(self.d, "DIMSCALE", 1.0).max(1e-9),
                0,
            ),
        }
    }

    fn leader(&mut self, out: &mut Geo, p: Props, l: &Leader, scale: f64, chord: f64) {
        if l.points.len() < 2 {
            return;
        }
        let pts = if l.path_type == 1 && l.points.len() > 2 {
            interp_pts(&l.points, None, None, 0, chord)
        } else {
            l.points.clone()
        };
        self.path(out, p, pts, false);
        if l.arrowhead {
            let (size, blk) = self.arrow_size(l.dimstyle.handle);
            self.arrow(out, p.solid(), l.points[0], l.points[1], size, blk, scale);
        }
    }

    fn mleader(&mut self, out: &mut Geo, p: Props, ml: &MLeader, scale: f64, chord: f64) {
        let cx = &ml.context;
        let line_col = self.col_of(&ml.line_color, p.layer);
        let wt_of = |w: i32, dflt: Wt| match w {
            -1 => Wt::Layer(p.layer),
            -2 => Wt::ByBlock,
            -3 => Wt::Mm(DEFAULT_WEIGHT),
            w if w >= 0 => Wt::Mm(w as f32 / 100.0),
            _ => dflt,
        };
        let lp = Props {
            col: if line_col == Col::ByBlock { p.col } else { line_col },
            wt: wt_of(ml.line_weight, p.wt),
            lt: if ml.line_type.is_null() {
                p.lt
            } else {
                self.lt_of(ml.line_type.handle, p.layer)
            },
            ..p
        };
        for root in &cx.roots {
            for line in &root.lines {
                let kind = if line.overrides & 1 != 0 {
                    line.leader_type
                } else {
                    ml.leader_type
                };
                if kind == 0 || line.points.is_empty() {
                    continue;
                }
                let mut lp2 = lp;
                if line.overrides & 2 != 0 {
                    let c = self.col_of(&line.color, p.layer);
                    lp2.col = if c == Col::ByBlock { p.col } else { c };
                }
                if line.overrides & 8 != 0 {
                    lp2.wt = wt_of(line.line_weight, lp.wt);
                }
                let mut pts = line.points.clone();
                pts.push(root.connection);
                let drawn = if kind == 2 && pts.len() > 2 {
                    interp_pts(&pts, None, None, 0, chord)
                } else {
                    pts.clone()
                };
                self.path(out, lp2, drawn, false);
                let size = if line.overrides & 16 != 0 {
                    line.arrow_size
                } else {
                    cx.arrow_size
                };
                let blk = if line.overrides & 32 != 0 {
                    line.arrow.handle
                } else {
                    ml.arrow.handle
                };
                self.arrow(out, lp2.solid(), pts[0], pts[1], size, blk, scale);
            }
            if ml.landing && ml.dogleg && root.landing_distance > 0.0 {
                let end = add(root.connection, mul(root.direction, root.landing_distance));
                self.path(out, lp, [root.connection, end], false);
            }
        }
        if let Some(t) = &cx.text {
            let c = self.col_of(&t.color, p.layer);
            let tp = p.with_col(if c == Col::ByBlock { p.col } else { c });
            let h = if t.height > 0.0 { t.height } else { cx.text_height };
            let fr = Frame {
                width: t.width,
                attachment: match t.alignment {
                    2 => 2,
                    3 => 3,
                    _ => 1,
                },
                spacing_style: t.line_spacing_style,
                spacing: if t.line_spacing > 0.0 { t.line_spacing } else { 1.0 },
            };
            let style = if t.style.is_null() {
                ml.text_style.handle
            } else {
                t.style.handle
            };
            let s = t.label.text();
            let bg =
                (t.bg_fill || t.bg_mask).then_some((if t.bg_mask { 2 } else { 1 }, &t.bg_color, t.bg_scale));
            self.mtext(out, tp, t.location, t.direction, t.normal, style, h, &s, &fr, bg);
        }
        if let Some(b) = &cx.block
            && let Some(Object {
                data: Data::BlockHeader(bh),
                ..
            }) = self.obj(b.block.handle)
        {
            let c = self.col_of(&b.color, p.layer);
            let bp = p.with_col(if c == Col::ByBlock { p.col } else { c });
            let sc = if b.scale == [0.0; 3] { [1.0; 3] } else { b.scale };
            let m = Mat::ocs(b.normal)
                .then_from(&Mat::translate(b.location))
                .then_from(&Mat::rotate_z(b.rotation))
                .then_from(&Mat::scale(sc))
                .then_from(&Mat::translate(mul(bh.base, -1.0)));
            let s = scale * m.max_scale();
            self.expand(out, b.block.handle, &m, &bp, s);
        }
    }

    /// TOLERANCE: rows of compartments (split at %%v), each framed, with their text.
    fn tolerance(&self, out: &mut Geo, p: Props, t: &Tolerance) {
        let (h, gap) = match self.dimvars(t.dimstyle.handle) {
            Some(v) => {
                let s = if v.scale > 0.0 { v.scale } else { 1.0 };
                (v.txt * s, v.gap.abs() * s)
            }
            None => (header_f(self.d, "DIMTXT", 2.5), header_f(self.d, "DIMGAP", 0.625)),
        };
        if !pos(h) {
            return;
        }
        let pad = gap.max(h * 0.25);
        let row_h = h + 2.0 * pad;
        let n = unit(t.extrusion).unwrap_or([0.0, 0.0, 1.0]);
        let x = unit(t.x_dir).unwrap_or_else(|| geom::ocs_axes(n).0);
        let m = Mat::from_cols(x, cross(n, x), n, t.insert);
        let st = self.style(match self.obj(t.dimstyle.handle) {
            Some(Object {
                data: Data::DimStyle(ds),
                ..
            }) => ds.txsty.handle,
            _ => 0,
        });
        let look = self.look(&st, h, 1.0, 0.0);
        let text = t.text.text().replace("^J", "\n").replace("%%V", "%%v");
        for (r, row) in text.split('\n').enumerate() {
            let y0 = -(r as f64) * row_h;
            let mut xc = 0.0;
            for cell in row.split("%%v") {
                let plain = text::text_plain(cell);
                if plain.trim().is_empty() {
                    continue;
                }
                let fr = Frame {
                    width: 0.0,
                    attachment: 4,
                    spacing_style: 1,
                    spacing: 1.0,
                };
                let (w, laid) = if self.fine {
                    let l = text::mtext(self.fonts, cell, &look, &fr);
                    (l.hi[0] - l.lo[0], Some(l))
                } else {
                    (plain.chars().count() as f64 * 0.9 * h, None)
                };
                let bw = w + 2.0 * pad;
                let box_ = vec![
                    [xc, y0 - row_h / 2.0, 0.0],
                    [xc + bw, y0 - row_h / 2.0, 0.0],
                    [xc + bw, y0 + row_h / 2.0, 0.0],
                    [xc, y0 + row_h / 2.0, 0.0],
                ];
                self.path_m(out, p, &m, box_, true);
                if let Some(mut l) = laid {
                    let dx = xc + pad;
                    l.map(|q| [q[0] + dx, q[1] + y0]);
                    self.laid(out, p, &m, l);
                }
                xc += bw;
            }
        }
    }

    fn hatch(&self, out: &mut Geo, p: Props, ht: &Hatch, chord: f64) {
        let paths: Vec<&HatchPath> = if ht.style == 2 {
            let ext: Vec<&HatchPath> = ht.paths.iter().filter(|x| x.flag & 1 != 0).collect();
            if ext.is_empty() {
                ht.paths.iter().collect()
            } else {
                ext
            }
        } else {
            ht.paths.iter().collect()
        };
        let rings: Vec<Vec<P2>> = paths
            .iter()
            .map(|x| hatch::ring(x, chord))
            .filter(|r| r.len() >= 3)
            .collect();
        if rings.is_empty() {
            return;
        }
        let m = Mat::ocs(ht.extrusion).then_from(&Mat::translate([0.0, 0.0, ht.elevation]));
        let z = |r: &Vec<P2>| r.iter().map(|q| [q[0], q[1], 0.0]).collect::<Vec<P3>>();
        if ht.gradient != 0 {
            let col = match ht.gradient_colors.first() {
                Some(g) => {
                    let c = Color {
                        index: g.index,
                        rgb: Some(g.rgb as u32),
                        ..Color::default()
                    };
                    match says(&c) {
                        Says::Rgb(x) => Col::Rgb(x),
                        Says::ByBlock => Col::ByBlock,
                        Says::ByLayer => p.col,
                    }
                }
                None => p.col,
            };
            self.fill_m(out, p.with_col(col), &m, rings.iter().map(z).collect(), false);
            return;
        }
        if ht.solid {
            self.fill_m(out, p, &m, rings.iter().map(z).collect(), false);
            return;
        }
        if !self.fine {
            for r in &rings {
                self.path_m(out, p, &m, z(r), true);
            }
            return;
        }
        let segs = hatch::pattern(&ht.lines, &rings, chord * 0.5);
        if !segs.is_empty() {
            let (at, n) = out.add(
                segs.iter()
                    .flat_map(|s| [m.apply([s[0][0], s[0][1], 0.0]), m.apply([s[1][0], s[1][1], 0.0])]),
            );
            out.raws.push(Raw {
                p: p.solid(),
                shape: Shape::Segs { at, n },
            });
        }
    }

    fn mline(&self, out: &mut Geo, p: Props, ml: &MLine, chord: f64) {
        let Some(Object {
            data: Data::MlineStyle(st),
            ..
        }) = self.obj(ml.style.handle)
        else {
            return;
        };
        let nv = ml.vertices.len();
        if nv < 2 {
            return;
        }
        let closed = ml.open_closed & 2 != 0;
        let nl = st.lines.len().min(ml.lines_in_style as usize);
        let at = |i: usize, j: usize| -> P3 {
            let v = &ml.vertices[i];
            let d = v
                .lines
                .get(j)
                .and_then(|l| l.segparms.first())
                .copied()
                .unwrap_or(0.0);
            add(v.at, mul(v.miter, d))
        };
        let el_props = |j: usize| -> Props {
            let l = &st.lines[j];
            let col = match self.col_of(&l.color, p.layer) {
                Col::ByBlock => p.col,
                Col::Layer(_) => p.col,
                c => c,
            };
            let lt = match self.lt_of(l.ltype.handle, p.layer) {
                Lt::Layer(_) => p.lt,
                Lt::ByBlock => p.lt,
                t => t,
            };
            Props { col, lt, ..p }
        };
        for j in 0..nl {
            let ep = el_props(j);
            // the element's pieces: segparms after the first are on/off distances along the
            // segment from its start
            let segs = if closed { nv } else { nv - 1 };
            let mut run: Vec<P3> = vec![];
            for i in 0..segs {
                let a = at(i, j);
                let b = at((i + 1) % nv, j);
                let sp = ml.vertices[i]
                    .lines
                    .get(j)
                    .map(|l| &l.segparms[..])
                    .unwrap_or(&[]);
                if sp.len() <= 2 {
                    if run.is_empty() {
                        run.push(a);
                    }
                    run.push(b);
                    continue;
                }
                if run.len() > 1 {
                    self.path(out, ep, std::mem::take(&mut run), false);
                }
                run.clear();
                let dir = unit(sub(b, a)).unwrap_or([1.0, 0.0, 0.0]);
                let full = len(sub(b, a));
                let ps = &sp[1..];
                let mut k = 0;
                while k < ps.len() {
                    let s0 = ps[k];
                    let s1 = ps.get(k + 1).copied().unwrap_or(full).min(full);
                    if s1 > s0 {
                        self.path(out, ep, [add(a, mul(dir, s0)), add(a, mul(dir, s1))], false);
                    }
                    k += 2;
                }
            }
            if run.len() > 1 {
                if closed && run.len() > 2 && run.first() == run.last() {
                    run.pop();
                    self.path(out, ep, run, true);
                } else {
                    self.path(out, ep, run, false);
                }
            }
        }
        if nl >= 2 {
            let (first, last) = (0, nl - 1);
            let outer = p;
            if st.flags & 1 != 0 {
                let mut ring: Vec<P3> = (0..nv).map(|i| at(i, first)).collect();
                ring.extend((0..nv).rev().map(|i| at(i, last)));
                let c = match self.col_of(&st.fill, p.layer) {
                    Col::ByBlock | Col::Layer(_) => p.col,
                    c => c,
                };
                self.fill_m(out, p.with_col(c), &Mat::ID, vec![ring], false);
            }
            if !closed {
                let cap = |out: &mut Geo, i: usize, square: bool, round: bool, sign: f64| {
                    if square {
                        self.path(out, outer, [at(i, first), at(i, last)], false);
                    }
                    if round {
                        let (a, b) = (at(i, first), at(i, last));
                        let c = mul(add(a, b), 0.5);
                        let r = len(sub(a, c));
                        let a0 = dmath::atan2(a[1] - c[1], a[0] - c[0]);
                        let mut pts = vec![];
                        arc_pts(c, r, a0, sign * dmath::PI, chord, &mut pts);
                        self.path(out, outer, pts, false);
                    }
                };
                cap(out, 0, st.flags & 16 != 0, st.flags & 64 != 0, 1.0);
                cap(out, nv - 1, st.flags & 256 != 0, st.flags & 1024 != 0, -1.0);
            }
            if st.flags & 2 != 0 {
                for i in 1..nv - 1 {
                    self.path(out, outer, [at(i, first), at(i, last)], false);
                }
            }
        }
    }

    /// An IMAGE's or WIPEOUT's clip boundary (or its whole rectangle) in world coordinates.
    fn image_ring(&self, im: &Image) -> Vec<P3> {
        let (w, h) = (im.size[0], im.size[1]);
        let mut c: Vec<P2> = if im.clipping && im.clip.len() >= 2 {
            if im.clip_type == 1 || im.clip.len() == 2 {
                let (a, b) = (im.clip[0], im.clip[1]);
                vec![a, [b[0], a[1]], b, [a[0], b[1]]]
            } else {
                im.clip.clone()
            }
        } else {
            vec![[-0.5, -0.5], [w - 0.5, -0.5], [w - 0.5, h - 0.5], [-0.5, h - 0.5]]
        };
        if c.len() > 2 && c.first() == c.last() {
            c.pop();
        }
        c.iter()
            .map(|q| add(im.insert, add(mul(im.u, q[0] + 0.5), mul(im.v, h - 0.5 - q[1]))))
            .collect()
    }

    fn acis(&self, out: &mut Geo, p: Props, a: &Acis) {
        let Some(wf) = &a.wireframe else { return };
        for w in &wf.wires {
            let pts: Vec<P3> = match &w.transform {
                Some(t) => w
                    .points
                    .iter()
                    .map(|q| {
                        let v = add(add(mul(t.x, q[0]), mul(t.y, q[1])), mul(t.z, q[2]));
                        add(t.translation, mul(v, if t.scale != 0.0 { t.scale } else { 1.0 }))
                    })
                    .collect(),
                None => w.points.clone(),
            };
            self.path(out, p, pts, false);
        }
    }

    // ---- the top

    fn root_props(&self) -> Props {
        Props {
            layer: Lay::At(self.layer0),
            col: Col::Rgb([255; 3]),
            wt: Wt::Mm(DEFAULT_WEIGHT),
            lt: Lt::Solid,
            lts: 1.0,
        }
    }

    /// Resolves pieces into scene items: `m` takes them to the scene, `acc` is what by block
    /// and layer "0" mean here, `lts` how much the inserts scale line type patterns.
    fn finish(&mut self, entity: u32, g: &Geo, m: &Mat, acc: &Props, lts: f64, v: &View) {
        for r in &g.raws {
            let mut q = r.p;
            q.within(acc);
            if let Shape::Insert { geo, m: km, lts: kl } = &r.shape {
                // an insert on a frozen layer hides everything in it; on an off one, only
                // what takes its layer
                if let Lay::At(i) = q.layer
                    && self.layers[i].frozen
                {
                    continue;
                }
                let m2 = m.then_from(km);
                self.finish(entity, geo, &m2, &q, lts * kl, v);
                continue;
            }
            self.emit(entity, q, &r.shape, &g.pts, m, lts, v);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit(&mut self, entity: u32, q: Props, shape: &Shape, all: &[P3], m: &Mat, lts: f64, v: &View) {
        let Lay::At(li) = q.layer else { return };
        if !self.linfo[li].visible || (!v.frozen.is_empty() && v.frozen.contains(&li)) {
            return;
        }
        let rgb = match q.col {
            Col::Rgb(c) => c,
            Col::Layer(Lay::At(i)) => self.linfo[i].rgb,
            _ => [255; 3],
        };
        let by_layer = q.col == Col::Layer(Lay::At(li));
        let weight = match q.wt {
            Wt::Mm(w) => w,
            Wt::Layer(Lay::At(i)) => self.linfo[i].wt,
            _ => DEFAULT_WEIGHT,
        };
        let lt = match q.lt {
            Lt::Layer(Lay::At(i)) => self.linfo[i].lt,
            t => t,
        };
        let id = m.is_id();
        let to2 = |p: &P3| -> P2 {
            if id {
                return [p[0], p[1]];
            }
            let w = m.apply(*p);
            [w[0], w[1]]
        };
        let items = &mut self.items;
        let ext = &mut self.ext;
        let mut push = |prim: Prim| {
            // the extents grow while the points are at hand
            let mut add = |pts: &[P2]| {
                for p in pts {
                    if p[0] < ext[0] {
                        ext[0] = p[0];
                    }
                    if p[0] > ext[2] {
                        ext[2] = p[0];
                    }
                    if p[1] < ext[1] {
                        ext[1] = p[1];
                    }
                    if p[1] > ext[3] {
                        ext[3] = p[1];
                    }
                }
            };
            match &prim {
                Prim::Path { pts, .. } | Prim::Segments { pts } => add(pts),
                Prim::Fill { rings } | Prim::Mask { rings } => rings.iter().for_each(|r| add(r)),
            }
            items.push(Item {
                entity,
                layer: li,
                rgb,
                weight,
                by_layer,
                prim,
            })
        };
        match shape {
            Shape::Path {
                at,
                n,
                closed,
                restarts,
            } => {
                let pts = &all[*at as usize..(*at + *n) as usize];
                let p2: Vec<P2> = pts.iter().map(to2).collect();
                let dashed = match (lt, self.fine) {
                    (Lt::Pattern(h), true) => {
                        let (pattern, period) = &self.ltypes[&h];
                        let scale = self.ltscale * q.lts * lts * v.ltk;
                        // a pattern repeating within a few chord tolerances (a pixel or so
                        // across the whole drawing) can't be seen: drawn solid
                        if period * scale < MIN_PERIOD * self.chord {
                            None
                        } else {
                            let dashes: Vec<f64> = pattern.iter().map(|x| x * scale).collect();
                            let dot = self.chord * 0.25;
                            ltype::dash(&p2, *closed, restarts, &Pattern { dashes, dot })
                        }
                    }
                    _ => None,
                };
                match dashed {
                    Some(pairs) => {
                        let pairs = clip_pairs(pairs, v.clip.as_ref());
                        if !pairs.is_empty() {
                            push(Prim::Segments { pts: pairs });
                        }
                    }
                    None => match &v.clip {
                        Some(c) if !p2.iter().all(|q| c.contains(*q)) => {
                            for part in clip_path(&p2, *closed, c) {
                                if part.len() >= 2 {
                                    push(Prim::Path {
                                        pts: part,
                                        closed: false,
                                    });
                                }
                            }
                        }
                        _ => push(Prim::Path {
                            pts: p2,
                            closed: *closed,
                        }),
                    },
                }
            }
            Shape::Segs { at, n } => {
                let pts = &all[*at as usize..(*at + *n) as usize];
                let p2: Vec<P2> = pts.iter().map(to2).collect();
                let pairs = clip_pairs(p2, v.clip.as_ref());
                if !pairs.is_empty() {
                    push(Prim::Segments { pts: pairs });
                }
            }
            Shape::Fill(rs) | Shape::Mask(rs) => {
                let rings: Vec<Vec<P2>> = rs
                    .iter()
                    .filter_map(|r| {
                        let r2: Vec<P2> = r.iter().map(to2).collect();
                        match &v.clip {
                            Some(c) => clip_ring(&r2, c),
                            None => Some(r2),
                        }
                    })
                    .collect();
                if !rings.is_empty() {
                    push(if matches!(shape, Shape::Mask(_)) {
                        Prim::Mask { rings }
                    } else {
                        Prim::Fill { rings }
                    });
                }
            }
            Shape::Insert { .. } => {}
            Shape::Ray(ray, both) => {
                let (at, dir) = &**ray;
                let a = to2(at);
                let d = m.vec(*dir);
                self.rays.push(PendingRay {
                    entity,
                    layer: li,
                    rgb,
                    weight,
                    at: a,
                    dir: [d[0], d[1]],
                    both: *both,
                    clip: v.clip,
                });
            }
        }
    }

    fn top(&mut self, ents: &[&'a Object], v: &View) {
        let mut g = Geo::default();
        let root = self.root_props();
        for &o in ents {
            let h = o.handle;
            g.clear();
            self.entity(o, false, v.m.max_scale(), &mut g);
            self.finish(h, &g, &v.m, &root, 1.0, v);
        }
    }

    fn model(&mut self) {
        let ents = self.block_list(self.ms);
        // most drawings make about an item per object or two: room for that at once rather
        // than growing (and copying) the list many times
        self.items.reserve(self.d.objects.len() + ents.len());
        let v = View {
            m: Mat::ID,
            ltk: 1.0,
            clip: None,
            frozen: vec![],
        };
        self.top(&ents, &v);
    }

    fn layout(&mut self, lh: u32) {
        let Some(Object {
            data: Data::Layout(lay),
            ..
        }) = self.obj(lh)
        else {
            return;
        };
        let br = lay.block.handle;
        if br == self.ms {
            // the Model layout is model space
            self.model();
            return;
        }
        let ents = self.block_list(br);
        let paper = View {
            m: Mat::ID,
            ltk: 1.0,
            clip: None,
            frozen: vec![],
        };
        let mut first_vp = true;
        let model = self.block_list(self.ms);
        for &o in &ents {
            let h = o.handle;
            let Data::Viewport(vp) = &o.data else {
                self.top(&[o], &paper);
                continue;
            };
            if first_vp {
                // the paper space's own viewport
                first_vp = false;
                continue;
            }
            if vp.status & 0x20000 != 0 || !pos(vp.view_height) || !pos(vp.height) {
                continue;
            }
            let k = vp.height / vp.view_height;
            let (ax, ay, _) = geom::ocs_axes(vp.direction);
            let t = vp.target;
            // model to the view's plane, twisted, scaled, centred on the viewport
            let view = Mat::from_cols(
                [ax[0], ay[0], 0.0],
                [ax[1], ay[1], 0.0],
                [ax[2], ay[2], 0.0],
                [0.0; 3],
            )
            .then_from(&Mat::translate(mul(t, -1.0)));
            let m = Mat::translate(vp.center)
                .then_from(&Mat::scale([k, k, k]))
                .then_from(&Mat::translate([-vp.view_center[0], -vp.view_center[1], 0.0]))
                .then_from(&Mat::rotate_z(-vp.twist))
                .then_from(&view);
            let c = vp.center;
            let clip = Rect {
                lo: [c[0] - vp.width / 2.0, c[1] - vp.height / 2.0],
                hi: [c[0] + vp.width / 2.0, c[1] + vp.height / 2.0],
            };
            let frozen = vp
                .frozen
                .iter()
                .filter_map(|x| self.layer_of.get(&x.handle).copied())
                .collect();
            let mv = View {
                m,
                ltk: if self.psltscale { 1.0 } else { k },
                clip: Some(clip),
                frozen,
            };
            self.top(&model, &mv);
            // the viewport's frame, on its layer
            if let Some(e) = &o.ent {
                let p = self.props(e, false);
                let r = vec![
                    [clip.lo[0], clip.lo[1], 0.0],
                    [clip.hi[0], clip.lo[1], 0.0],
                    [clip.hi[0], clip.hi[1], 0.0],
                    [clip.lo[0], clip.hi[1], 0.0],
                ];
                if self.shown(p.layer) {
                    let mut g = Geo::default();
                    self.path(&mut g, p.solid(), r, true);
                    let root = self.root_props();
                    self.finish(h, &g, &Mat::ID, &root, 1.0, &paper);
                }
            }
        }
    }

    fn run(&mut self, space: Space) {
        match space {
            Space::Model => self.model(),
            Space::Layout(h) => self.layout(h),
        }
    }

    fn scene(self) -> Scene {
        let e = self.ext;
        let mut s = Scene {
            layers: self.layers,
            items: self.items,
            extents: (e[0] <= e[2]).then_some(([e[0], e[1]], [e[2], e[3]])),
        };
        // rays and construction lines: cut to a box well beyond everything else
        let (lo, hi) = s.extents.unwrap_or(([-1.0, -1.0], [1.0, 1.0]));
        let ext = s.extents;
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-9);
        for r in self.rays {
            let mut b = Rect {
                lo: [lo[0] - 2.0 * span, lo[1] - 2.0 * span],
                hi: [hi[0] + 2.0 * span, hi[1] + 2.0 * span],
            };
            if let Some(c) = r.clip {
                b = Rect {
                    lo: [b.lo[0].max(c.lo[0]), b.lo[1].max(c.lo[1])],
                    hi: [b.hi[0].min(c.hi[0]), b.hi[1].min(c.hi[1])],
                };
            }
            let l = (r.dir[0] * r.dir[0] + r.dir[1] * r.dir[1]).sqrt();
            if l < 1e-300 {
                continue;
            }
            let far = 10.0 * (span + (r.at[0] - lo[0]).abs() + (r.at[1] - lo[1]).abs()) / l;
            let a = if r.both {
                [r.at[0] - r.dir[0] * far, r.at[1] - r.dir[1] * far]
            } else {
                r.at
            };
            let e = [r.at[0] + r.dir[0] * far, r.at[1] + r.dir[1] * far];
            if let Some((t0, t1)) = geom::clip_seg(a, e, &b) {
                let p = |t: f64| [a[0] + (e[0] - a[0]) * t, a[1] + (e[1] - a[1]) * t];
                s.items.push(Item {
                    entity: r.entity,
                    layer: r.layer,
                    rgb: r.rgb,
                    weight: r.weight,
                    by_layer: false,
                    prim: Prim::Path {
                        pts: vec![p(t0), p(t1)],
                        closed: false,
                    },
                });
            }
        }
        // the extents leave the rays out: they would only zoom everything away
        s.extents = ext;
        s
    }
}

/// Segments (point pairs) cut to a rectangle.
fn clip_pairs(pairs: Vec<P2>, clip: Option<&Rect>) -> Vec<P2> {
    let Some(c) = clip else { return pairs };
    if pairs.iter().all(|q| c.contains(*q)) {
        return pairs;
    }
    let mut out = Vec::with_capacity(pairs.len());
    for w in pairs.as_chunks::<2>().0 {
        let (a, b) = (w[0], w[1]);
        if let Some((t0, t1)) = geom::clip_seg(a, b, c) {
            let at = |t: f64| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            out.push(at(t0));
            out.push(at(t1));
        }
    }
    out
}

/// The extents the file keeps (EXTMIN and EXTMAX, a layout's own), when they look right.
fn stored_extents(d: &Drawing, space: Space) -> Option<(P2, P2)> {
    let (lo, hi) = match space {
        Space::Model => match (d.header.get("EXTMIN"), d.header.get("EXTMAX")) {
            (Some(crate::header::Val::P3(a)), Some(crate::header::Val::P3(b))) => (*a, *b),
            _ => return None,
        },
        Space::Layout(h) => match d.objects.get(&h) {
            Some(Object {
                data: Data::Layout(l),
                ..
            }) => (l.extmin, l.extmax),
            _ => return None,
        },
    };
    let ok = (0..2).all(|k| lo[k].is_finite() && hi[k].is_finite() && lo[k] < hi[k] && hi[k] - lo[k] < 1e15);
    ok.then_some(([lo[0], lo[1]], [hi[0], hi[1]]))
}

/// The paper's extents to choose a chord from, for a layout: its limits.
fn layout_limits(d: &Drawing, space: Space) -> Option<f64> {
    if let Space::Layout(h) = space
        && let Some(Object {
            data: Data::Layout(l),
            ..
        }) = d.objects.get(&h)
    {
        let w = (l.limmax[0] - l.limmin[0]).max(l.limmax[1] - l.limmin[1]);
        if w > 0.0 && w.is_finite() {
            return Some(w);
        }
    }
    None
}

pub fn plan(d: &Drawing, space: Space, fonts: &dyn Fonts, opts: &Options) -> super::Plan {
    // a rough pass (no text shaping, patterns or dashes) for the extents, which the chord
    // tolerance and point sizes go by
    let ext = match stored_extents(d, space) {
        Some(e) => Some(e),
        None => {
            let mut rough = B::new(d, fonts, false, f64::INFINITY, opts.all_layers);
            rough.run(space);
            rough.scene().extents
        }
    };
    let size = ext.map(|(lo, hi)| ((hi[0] - lo[0]).max(hi[1] - lo[1]), hi[1] - lo[1]));
    let side = size
        .map(|s| s.0)
        .filter(|s| *s > 0.0)
        .or_else(|| layout_limits(d, space))
        .unwrap_or(100.0);
    let chord = if opts.chord > 0.0 { opts.chord } else { side * 1e-4 };
    super::Plan {
        chord,
        view_h: size.map(|s| s.1).filter(|s| *s > 0.0).unwrap_or(side),
    }
}

pub fn build_planned(
    d: &Drawing,
    space: Space,
    fonts: &dyn Fonts,
    opts: &Options,
    plan: &super::Plan,
) -> Scene {
    let mut b = B::new(d, fonts, true, plan.chord, opts.all_layers);
    b.view_h = plan.view_h;
    b.run(space);
    b.scene()
}

pub fn build(d: &Drawing, space: Space, fonts: &dyn Fonts, opts: &Options) -> Scene {
    let p = plan(d, space, fonts, opts);
    build_planned(d, space, fonts, opts, &p)
}

pub fn build_some(
    d: &Drawing,
    space: Space,
    handles: &[u32],
    fonts: &dyn Fonts,
    opts: &Options,
    plan: &super::Plan,
    ext: Option<(P2, P2)>,
) -> Scene {
    let mut b = B::new(d, fonts, true, plan.chord, opts.all_layers);
    b.view_h = plan.view_h;
    if let Some((lo, hi)) = ext {
        b.ext = [lo[0], lo[1], hi[0], hi[1]];
    }
    let block = match space {
        Space::Model => b.ms,
        Space::Layout(h) => match b.obj(h) {
            Some(Object {
                data: Data::Layout(l),
                ..
            }) => l.block.handle,
            _ => b.ps,
        },
    };
    let mut hs: Vec<u32> = handles.to_vec();
    hs.sort_unstable();
    hs.dedup();
    let ents: Vec<&Object> = hs
        .iter()
        .filter_map(|h| b.obj(*h))
        .filter(|o| match &o.ent {
            Some(e) => match e.entmode {
                2 => block == b.ms,
                1 => block == b.ps,
                _ => o.owner.handle == block,
            },
            None => false,
        })
        .collect();
    let v = View {
        m: Mat::ID,
        ltk: 1.0,
        clip: None,
        frozen: vec![],
    };
    b.top(&ents, &v);
    let mut s = b.scene();
    if ext.is_some() {
        s.grow_extents();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(layer: Lay, col: Col) -> Props {
        Props {
            layer,
            col,
            wt: Wt::ByBlock,
            lt: Lt::Layer(layer),
            lts: 1.0,
        }
    }

    #[test]
    fn by_block_and_layer_zero_through_nested_inserts() {
        // a line on "0", by block, in block B; B inserted (on "0", by block) in block A; A
        // inserted on layer 3, red
        let line = p(Lay::Zero, Col::ByBlock);
        let ins_b = p(Lay::Zero, Col::ByBlock);
        let ins_a = Props {
            wt: Wt::Mm(0.5),
            ..p(Lay::At(3), Col::Rgb([255, 0, 0]))
        };
        // flattening one level at a time ...
        let mut once = line;
        once.within(&ins_b);
        once.within(&ins_a);
        // ... or composing the inserts first gives the same
        let mut kid = ins_b;
        kid.within(&ins_a);
        let mut composed = line;
        composed.within(&kid);
        assert_eq!(once, composed);
        assert_eq!(once.layer, Lay::At(3));
        assert_eq!(once.col, Col::Rgb([255, 0, 0]));
        assert_eq!(once.wt, Wt::Mm(0.5));
        assert_eq!(once.lt, Lt::Layer(Lay::At(3)));
        // by layer on "0" takes the insert's layer's colour; on its own layer keeps it
        let mut q = p(Lay::Zero, Col::Layer(Lay::Zero));
        q.within(&ins_a);
        assert_eq!(q.col, Col::Layer(Lay::At(3)));
        let mut q = p(Lay::At(5), Col::Layer(Lay::At(5)));
        q.within(&ins_a);
        assert_eq!((q.layer, q.col), (Lay::At(5), Col::Layer(Lay::At(5))));
        // an attribute keeps its layer, takes only what is by block
        let mut a = p(Lay::Zero, Col::ByBlock);
        a.by_block(&ins_a);
        assert_eq!((a.layer, a.col), (Lay::Zero, Col::Rgb([255, 0, 0])));
    }

    #[test]
    fn chord_buckets() {
        assert_eq!(pow2(bucket(1.0)), 1.0);
        assert_eq!(pow2(bucket(0.3)), 0.25);
        assert_eq!(pow2(bucket(5.0)), 4.0);
        assert!(pow2(bucket(f64::INFINITY)).is_infinite());
    }
}
