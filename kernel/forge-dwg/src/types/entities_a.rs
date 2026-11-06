//! Entities' own fields (chapter 20.4): attributes, inserts, the old-style polylines and their
//! vertices, faces, shapes, viewports, the ACIS solids, rays, OLE frames and proxies.

use super::Spec;
use super::entities::{Solid, Text, text_io};
use crate::bits::{R, bad};
use crate::io::{Color, Href, Io};
use crate::str::Str;
use crate::version::Version;

const UP: [f64; 3] = [0.0, 0.0, 1.0];
const X: [f64; 3] = [1.0, 0.0, 0.0];
const Y: [f64; 3] = [0.0, 1.0, 0.0];
const HARD_OWNER: Href = Href { code: 3, handle: 0 };

/// What a complex entity owns (an INSERT's attributes, a POLYLINE's vertices) and its SEQEND.
#[derive(Clone, Debug, PartialEq)]
pub struct Owned {
    /// R2000: the first and last sub-entity.
    pub first: Href,
    pub last: Href,
    /// R2004+: every sub-entity.
    pub items: Vec<Href>,
    pub seqend: Href,
}

impl Default for Owned {
    fn default() -> Owned {
        Owned {
            first: Href::NULL_SOFT,
            last: Href::NULL_SOFT,
            items: vec![],
            seqend: HARD_OWNER,
        }
    }
}

impl Owned {
    /// R2004+: the owned object count, in the data.
    fn count<S: Io>(&self, s: &mut S) -> R<i32> {
        let mut n = self.items.len() as i32;
        if s.ver() >= Version::R2004 {
            s.bl(&mut n)?;
        }
        Ok(n)
    }
    fn handles<S: Io>(&mut self, s: &mut S, n: i32) -> R<()> {
        if s.ver() >= Version::R2004 {
            s.items(&mut self.items, n, &mut |s, h| s.h(h))?;
        } else {
            s.h(&mut self.first)?;
            s.h(&mut self.last)?;
        }
        s.h(&mut self.seqend)
    }
}

/// MTEXT's own fields, as an R2018 multi-line attribute embeds them.
#[derive(Clone, Debug, PartialEq)]
pub struct MTextBody {
    pub insert: [f64; 3],
    pub extrusion: [f64; 3],
    pub x_axis: [f64; 3],
    pub rect_width: f64,
    /// R2007+.
    pub rect_height: f64,
    pub height: f64,
    pub attachment: i16,
    pub direction: i16,
    pub extents_height: f64,
    pub extents_width: f64,
    pub text: Str,
    pub style: Href,
    pub linespacing_style: i16,
    pub linespacing: f64,
    pub unknown: bool,
    /// R2004+: 1 fill, 2 fill with the drawing's colour, 0x10 frame (R2018+).
    pub background: i32,
    pub bg_scale: f64,
    pub bg_color: Color,
    pub bg_alpha: i32,
    /// R2018+.
    pub annotative: bool,
    pub version: i16,
    pub default_flag: bool,
    pub app: Href,
    pub attachment_l: i32,
    pub x_axis2: [f64; 3],
    pub insert2: [f64; 3],
    pub rect_width2: f64,
    pub rect_height2: f64,
    pub extents_width2: f64,
    pub extents_height2: f64,
    /// 0 none, 1 static, 2 dynamic.
    pub column_type: i16,
    pub column_count: i32,
    pub column_width: f64,
    pub gutter: f64,
    pub auto_height: bool,
    pub flow_reversed: bool,
    pub column_heights: Vec<f64>,
}

impl Default for MTextBody {
    fn default() -> MTextBody {
        MTextBody {
            insert: [0.0; 3],
            extrusion: UP,
            x_axis: X,
            rect_width: 0.0,
            rect_height: 0.0,
            height: 2.5,
            attachment: 1,
            direction: 1,
            extents_height: 0.0,
            extents_width: 0.0,
            text: Str::default(),
            style: Href::NULL_HARD,
            linespacing_style: 1,
            linespacing: 1.0,
            unknown: false,
            background: 0,
            bg_scale: 1.5,
            bg_color: Color::index(256),
            bg_alpha: 0,
            annotative: false,
            version: 0,
            default_flag: true,
            app: Href::NULL_HARD,
            attachment_l: 1,
            x_axis2: X,
            insert2: [0.0; 3],
            rect_width2: 0.0,
            rect_height2: 0.0,
            extents_width2: 0.0,
            extents_height2: 0.0,
            column_type: 0,
            column_count: 0,
            column_width: 0.0,
            gutter: 0.0,
            auto_height: false,
            flow_reversed: false,
            column_heights: vec![],
        }
    }
}

impl MTextBody {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        s.bd3(&mut self.insert)?;
        s.bd3(&mut self.extrusion)?;
        s.bd3(&mut self.x_axis)?;
        s.bd(&mut self.rect_width)?;
        if v >= Version::R2007 {
            s.bd(&mut self.rect_height)?;
        }
        s.bd(&mut self.height)?;
        s.bs(&mut self.attachment)?;
        s.bs(&mut self.direction)?;
        s.bd(&mut self.extents_height)?;
        s.bd(&mut self.extents_width)?;
        s.tv(&mut self.text)?;
        s.h(&mut self.style)?;
        s.bs(&mut self.linespacing_style)?;
        s.bd(&mut self.linespacing)?;
        s.b(&mut self.unknown)?;
        if v >= Version::R2004 {
            s.bl(&mut self.background)?;
            if self.background & 1 != 0 || (v >= Version::R2018 && self.background & 0x10 != 0) {
                // the spec says BL, but the default is 1.5
                s.bd(&mut self.bg_scale)?;
                s.cmc(&mut self.bg_color)?;
                s.bl(&mut self.bg_alpha)?;
            }
        }
        if v < Version::R2018 {
            return Ok(());
        }
        let mut not_annotative = !self.annotative;
        s.b(&mut not_annotative)?;
        self.annotative = !not_annotative;
        if self.annotative {
            return Ok(());
        }
        s.bs(&mut self.version)?;
        s.b(&mut self.default_flag)?;
        s.h(&mut self.app)?;
        s.bl(&mut self.attachment_l)?;
        s.bd3(&mut self.x_axis2)?;
        s.bd3(&mut self.insert2)?;
        s.bd(&mut self.rect_width2)?;
        s.bd(&mut self.rect_height2)?;
        s.bd(&mut self.extents_width2)?;
        s.bd(&mut self.extents_height2)?;
        s.bs(&mut self.column_type)?;
        if self.column_type == 0 {
            return Ok(());
        }
        let mut n = self.column_heights.len() as i32;
        if !S::READ && n != 0 {
            self.column_count = n;
        }
        s.bl(&mut self.column_count)?;
        s.bd(&mut self.column_width)?;
        s.bd(&mut self.gutter)?;
        s.b(&mut self.auto_height)?;
        s.b(&mut self.flow_reversed)?;
        if !self.auto_height && self.column_type == 2 {
            n = self.column_count;
        } else {
            n = 0;
        }
        s.items(&mut self.column_heights, n, &mut |s, h| s.bd(h))
    }
}

/// ATTRIB: TEXT's fields, then the attribute's.
#[derive(Clone, Debug, PartialEq)]
pub struct Attrib {
    pub text: Text,
    /// R2010+.
    pub version: u8,
    /// R2018+: 1 single line, 2 multi-line attribute, 4 multi-line definition.
    pub kind: u8,
    /// R2018+ multi-line: the embedded MTEXT.
    pub mtext: MTextBody,
    /// R2018+ multi-line: annotative data, its application and a short (0).
    pub annot: Vec<u8>,
    pub annot_app: Href,
    pub annot_short: i16,
    pub tag: Str,
    /// Unused.
    pub field_length: i16,
    /// 1 invisible, 2 constant, 4 verify, 8 preset.
    pub flags: u8,
    /// R2007+.
    pub lock_position: bool,
}

impl Default for Attrib {
    fn default() -> Attrib {
        Attrib {
            text: Text::default(),
            version: 0,
            kind: 1,
            mtext: MTextBody::default(),
            annot: vec![],
            annot_app: Href::NULL_HARD,
            annot_short: 0,
            tag: Str::default(),
            field_length: 0,
            flags: 0,
            lock_position: false,
        }
    }
}

fn attrib_io<S: Io>(a: &mut Attrib, s: &mut S) -> R<()> {
    let v = s.ver();
    text_io(&mut a.text, s)?;
    s.h(&mut a.text.style)?;
    if v >= Version::R2010 {
        s.rc(&mut a.version)?;
    }
    if v >= Version::R2018 {
        s.rc(&mut a.kind)?;
        if a.kind & (2 | 4) != 0 {
            a.mtext.io(s)?;
            let mut n = a.annot.len() as i16;
            s.bs(&mut n)?;
            if n > 0 {
                s.bytes(&mut a.annot, n as usize)?;
                s.h(&mut a.annot_app)?;
                s.bs(&mut a.annot_short)?;
            } else if S::READ {
                a.annot.clear();
            }
        }
    }
    s.tv(&mut a.tag)?;
    s.bs(&mut a.field_length)?;
    s.rc(&mut a.flags)?;
    if v >= Version::R2007 {
        s.b(&mut a.lock_position)?;
    }
    Ok(())
}

impl Spec for Attrib {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        attrib_io(self, s)
    }
}

/// ATTDEF: an ATTRIB's fields and the prompt.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AttDef {
    pub attrib: Attrib,
    /// R2010+.
    pub version: u8,
    pub prompt: Str,
}

impl Spec for AttDef {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        attrib_io(&mut self.attrib, s)?;
        if s.ver() >= Version::R2010 {
            s.rc(&mut self.version)?;
        }
        s.tv(&mut self.prompt)
    }
}

/// SEQEND: ends an INSERT's attributes or a POLYLINE's vertices.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SeqEnd;

impl Spec for SeqEnd {
    fn io<S: Io>(&mut self, _: &mut S) -> R<()> {
        Ok(())
    }
}

/// INSERT: a block reference.
#[derive(Clone, Debug, PartialEq)]
pub struct Insert {
    pub at: [f64; 3],
    pub scale: [f64; 3],
    pub rotation: f64,
    pub extrusion: [f64; 3],
    pub has_attribs: bool,
    /// The BLOCK_HEADER.
    pub block: Href,
    /// The ATTRIBs and SEQEND, when `has_attribs`.
    pub attribs: Owned,
}

impl Default for Insert {
    fn default() -> Insert {
        Insert {
            at: [0.0; 3],
            scale: [1.0; 3],
            rotation: 0.0,
            extrusion: UP,
            has_attribs: false,
            block: Href::NULL_HARD,
            attribs: Owned::default(),
        }
    }
}

impl Insert {
    /// The data up to the owned count, which it returns.
    fn head<S: Io>(&mut self, s: &mut S) -> R<i32> {
        s.bd3(&mut self.at)?;
        let c = &mut self.scale;
        let mut flags = match () {
            _ if S::READ => 0,
            _ if *c == [1.0; 3] => 3,
            _ if c[0] == 1.0 => 1,
            _ if c[1] == c[0] && c[2] == c[0] => 2,
            _ => 0,
        };
        s.bb(&mut flags)?;
        match flags {
            3 => *c = [1.0; 3],
            1 => {
                c[0] = 1.0;
                s.dd(&mut c[1], 1.0)?;
                s.dd(&mut c[2], 1.0)?;
            }
            2 => {
                s.rd(&mut c[0])?;
                c[1] = c[0];
                c[2] = c[0];
            }
            _ => {
                s.rd(&mut c[0])?;
                let x = c[0];
                s.dd(&mut c[1], x)?;
                s.dd(&mut c[2], x)?;
            }
        }
        s.bd(&mut self.rotation)?;
        s.bd3(&mut self.extrusion)?;
        s.b(&mut self.has_attribs)?;
        // the R2004+ count is there only when attributes follow (the spec has it always)
        if self.has_attribs { self.attribs.count(s) } else { Ok(0) }
    }
    fn tail<S: Io>(&mut self, s: &mut S, n: i32) -> R<()> {
        s.h(&mut self.block)?;
        if self.has_attribs {
            self.attribs.handles(s, n)?;
        }
        Ok(())
    }
}

impl Spec for Insert {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let n = self.head(s)?;
        self.tail(s, n)
    }
}

/// MINSERT: an INSERT repeated in rows and columns.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MInsert {
    pub insert: Insert,
    pub cols: i16,
    pub rows: i16,
    pub col_spacing: f64,
    pub row_spacing: f64,
}

impl Spec for MInsert {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let n = self.insert.head(s)?;
        s.bs(&mut self.cols)?;
        s.bs(&mut self.rows)?;
        s.bd(&mut self.col_spacing)?;
        s.bd(&mut self.row_spacing)?;
        self.insert.tail(s, n)
    }
}

/// VERTEX (2D): its Z is the polyline's elevation.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Vertex2d {
    pub flags: u8,
    pub at: [f64; 3],
    pub start_width: f64,
    pub end_width: f64,
    pub bulge: f64,
    /// R2010+.
    pub id: i32,
    pub tangent: f64,
}

impl Spec for Vertex2d {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.rc(&mut self.flags)?;
        s.bd3(&mut self.at)?;
        // equal non-zero widths are stored once, negated
        let mut w = self.start_width;
        if !S::READ && w == self.end_width && w > 0.0 {
            w = -w;
        }
        s.bd(&mut w)?;
        if w < 0.0 {
            self.start_width = -w;
            self.end_width = -w;
        } else {
            self.start_width = w;
            s.bd(&mut self.end_width)?;
        }
        s.bd(&mut self.bulge)?;
        if s.ver() >= Version::R2010 {
            s.bl(&mut self.id)?;
        }
        s.bd(&mut self.tangent)
    }
}

/// VERTEX (3D); the mesh and face-list vertices are the same.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Vertex3d {
    pub flags: u8,
    pub at: [f64; 3],
}

impl Spec for Vertex3d {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.rc(&mut self.flags)?;
        s.bd3(&mut self.at)
    }
}

/// VERTEX (MESH).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VertexMesh(pub Vertex3d);

impl Spec for VertexMesh {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

/// VERTEX (PFACE): a face list's point.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VertexPface(pub Vertex3d);

impl Spec for VertexPface {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

/// VERTEX (PFACE FACE): a face's 1-based vertex indices, negative for an invisible edge.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VertexPfaceFace {
    pub index: [i16; 4],
}

impl Spec for VertexPfaceFace {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        for i in &mut self.index {
            s.bs(i)?;
        }
        Ok(())
    }
}

/// POLYLINE (2D).
#[derive(Clone, Debug, PartialEq)]
pub struct Polyline2d {
    pub flags: i16,
    /// Curve and smooth surface type.
    pub curve: i16,
    pub start_width: f64,
    pub end_width: f64,
    pub thickness: f64,
    pub elevation: f64,
    pub extrusion: [f64; 3],
    pub vertices: Owned,
}

impl Default for Polyline2d {
    fn default() -> Polyline2d {
        Polyline2d {
            flags: 0,
            curve: 0,
            start_width: 0.0,
            end_width: 0.0,
            thickness: 0.0,
            elevation: 0.0,
            extrusion: UP,
            vertices: Owned::default(),
        }
    }
}

impl Spec for Polyline2d {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bs(&mut self.flags)?;
        s.bs(&mut self.curve)?;
        s.bd(&mut self.start_width)?;
        s.bd(&mut self.end_width)?;
        s.bt(&mut self.thickness)?;
        s.bd(&mut self.elevation)?;
        s.be(&mut self.extrusion)?;
        let n = self.vertices.count(s)?;
        self.vertices.handles(s, n)
    }
}

/// POLYLINE (3D).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Polyline3d {
    /// 1 splined quadratic (75 = 5), 2 splined cubic (75 = 6).
    pub curve: u8,
    /// 1 closed.
    pub flags: u8,
    pub vertices: Owned,
}

impl Spec for Polyline3d {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.rc(&mut self.curve)?;
        s.rc(&mut self.flags)?;
        let n = self.vertices.count(s)?;
        self.vertices.handles(s, n)
    }
}

/// POLYLINE (PFACE): a face list, its points' VERTEXes then its faces'.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PolylinePface {
    pub nverts: i16,
    pub nfaces: i16,
    pub vertices: Owned,
}

impl Spec for PolylinePface {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bs(&mut self.nverts)?;
        s.bs(&mut self.nfaces)?;
        let n = self.vertices.count(s)?;
        self.vertices.handles(s, n)
    }
}

/// POLYLINE (MESH): an M by N mesh.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PolylineMesh {
    pub flags: i16,
    pub curve: i16,
    pub m: i16,
    pub n: i16,
    pub m_density: i16,
    pub n_density: i16,
    pub vertices: Owned,
}

impl Spec for PolylineMesh {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bs(&mut self.flags)?;
        s.bs(&mut self.curve)?;
        s.bs(&mut self.m)?;
        s.bs(&mut self.n)?;
        s.bs(&mut self.m_density)?;
        s.bs(&mut self.n_density)?;
        let n = self.vertices.count(s)?;
        self.vertices.handles(s, n)
    }
}

/// 3DFACE.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Face3d {
    pub corners: [[f64; 3]; 4],
    /// Invisible edges: 1, 2, 4, 8 for the first to the fourth.
    pub invisible: i16,
}

impl Spec for Face3d {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let mut no_flags = self.invisible == 0;
        let mut z_zero = self.corners[0][2] == 0.0;
        s.b(&mut no_flags)?;
        s.b(&mut z_zero)?;
        let c = &mut self.corners;
        s.rd(&mut c[0][0])?;
        s.rd(&mut c[0][1])?;
        if !z_zero {
            s.rd(&mut c[0][2])?;
        } else {
            c[0][2] = 0.0;
        }
        for k in 1..4 {
            let prev = c[k - 1];
            s.dd3(&mut c[k], prev)?;
        }
        if !no_flags {
            s.bs(&mut self.invisible)?;
        } else {
            self.invisible = 0;
        }
        Ok(())
    }
}

/// TRACE: SOLID's fields.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Trace(pub Solid);

impl Spec for Trace {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

/// SHAPE: a shape file's shape by number.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub at: [f64; 3],
    pub scale: f64,
    pub rotation: f64,
    pub width: f64,
    pub oblique: f64,
    pub thickness: f64,
    pub number: i16,
    pub extrusion: [f64; 3],
    /// The shape file's STYLE.
    pub style: Href,
}

impl Default for Shape {
    fn default() -> Shape {
        Shape {
            at: [0.0; 3],
            scale: 1.0,
            rotation: 0.0,
            width: 1.0,
            oblique: 0.0,
            thickness: 0.0,
            number: 0,
            extrusion: UP,
            style: Href::NULL_HARD,
        }
    }
}

impl Spec for Shape {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bd3(&mut self.at)?;
        s.bd(&mut self.scale)?;
        s.bd(&mut self.rotation)?;
        s.bd(&mut self.width)?;
        s.bd(&mut self.oblique)?;
        s.bd(&mut self.thickness)?;
        s.bs(&mut self.number)?;
        s.bd3(&mut self.extrusion)?;
        s.h(&mut self.style)
    }
}

/// VIEWPORT (the entity): a paper space window on model space.
#[derive(Clone, Debug, PartialEq)]
pub struct Viewport {
    pub center: [f64; 3],
    pub width: f64,
    pub height: f64,
    pub target: [f64; 3],
    pub direction: [f64; 3],
    pub twist: f64,
    pub view_height: f64,
    pub lens: f64,
    pub front_clip: f64,
    pub back_clip: f64,
    pub snap_angle: f64,
    pub view_center: [f64; 2],
    pub snap_base: [f64; 2],
    pub snap_spacing: [f64; 2],
    pub grid_spacing: [f64; 2],
    pub circle_zoom: i16,
    /// R2007+.
    pub grid_major: i16,
    pub status: i32,
    pub style_sheet: Str,
    pub render_mode: u8,
    pub ucs_at_origin: bool,
    pub ucs_per_viewport: bool,
    pub ucs_origin: [f64; 3],
    pub ucs_x: [f64; 3],
    pub ucs_y: [f64; 3],
    pub ucs_elevation: f64,
    pub ucs_ortho: i16,
    /// R2004+.
    pub shadeplot_mode: i16,
    /// R2007+.
    pub default_lights: bool,
    pub lighting_type: u8,
    pub brightness: f64,
    pub contrast: f64,
    pub ambient: Color,
    pub frozen: Vec<Href>,
    pub clip_boundary: Href,
    /// R2000 only.
    pub header: Href,
    pub named_ucs: Href,
    pub base_ucs: Href,
    /// R2007+.
    pub background: Href,
    pub visual_style: Href,
    pub shadeplot: Href,
    pub sun: Href,
}

impl Default for Viewport {
    fn default() -> Viewport {
        Viewport {
            center: [0.0; 3],
            width: 0.0,
            height: 0.0,
            target: [0.0; 3],
            direction: UP,
            twist: 0.0,
            view_height: 0.0,
            lens: 50.0,
            front_clip: 0.0,
            back_clip: 0.0,
            snap_angle: 0.0,
            view_center: [0.0; 2],
            snap_base: [0.0; 2],
            snap_spacing: [10.0; 2],
            grid_spacing: [10.0; 2],
            circle_zoom: 1000,
            grid_major: 5,
            status: 0,
            style_sheet: Str::default(),
            render_mode: 0,
            ucs_at_origin: true,
            ucs_per_viewport: true,
            ucs_origin: [0.0; 3],
            ucs_x: X,
            ucs_y: Y,
            ucs_elevation: 0.0,
            ucs_ortho: 0,
            shadeplot_mode: 0,
            default_lights: true,
            lighting_type: 1,
            brightness: 0.0,
            contrast: 0.0,
            ambient: Color::index(250),
            frozen: vec![],
            clip_boundary: Href::NULL_SOFT,
            header: Href::NULL_HARD,
            named_ucs: Href::NULL_HARD,
            base_ucs: Href::NULL_HARD,
            background: Href::NULL_SOFT,
            visual_style: Href::NULL_HARD,
            shadeplot: Href::NULL_SOFT,
            sun: HARD_OWNER,
        }
    }
}

impl Spec for Viewport {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        s.bd3(&mut self.center)?;
        s.bd(&mut self.width)?;
        s.bd(&mut self.height)?;
        s.bd3(&mut self.target)?;
        s.bd3(&mut self.direction)?;
        s.bd(&mut self.twist)?;
        s.bd(&mut self.view_height)?;
        s.bd(&mut self.lens)?;
        s.bd(&mut self.front_clip)?;
        s.bd(&mut self.back_clip)?;
        s.bd(&mut self.snap_angle)?;
        s.rd2(&mut self.view_center)?;
        s.rd2(&mut self.snap_base)?;
        s.rd2(&mut self.snap_spacing)?;
        s.rd2(&mut self.grid_spacing)?;
        s.bs(&mut self.circle_zoom)?;
        if v >= Version::R2007 {
            s.bs(&mut self.grid_major)?;
        }
        let mut nfrozen = self.frozen.len() as i32;
        s.bl(&mut nfrozen)?;
        s.bl(&mut self.status)?;
        s.tv(&mut self.style_sheet)?;
        s.rc(&mut self.render_mode)?;
        s.b(&mut self.ucs_at_origin)?;
        s.b(&mut self.ucs_per_viewport)?;
        s.bd3(&mut self.ucs_origin)?;
        s.bd3(&mut self.ucs_x)?;
        s.bd3(&mut self.ucs_y)?;
        s.bd(&mut self.ucs_elevation)?;
        s.bs(&mut self.ucs_ortho)?;
        if v >= Version::R2004 {
            s.bs(&mut self.shadeplot_mode)?;
        }
        if v >= Version::R2007 {
            s.b(&mut self.default_lights)?;
            s.rc(&mut self.lighting_type)?;
            s.bd(&mut self.brightness)?;
            s.bd(&mut self.contrast)?;
            s.cmc(&mut self.ambient)?;
        }
        s.items(&mut self.frozen, nfrozen, &mut |s, h| s.h(h))?;
        s.h(&mut self.clip_boundary)?;
        if v == Version::R2000 {
            s.h(&mut self.header)?;
        }
        s.h(&mut self.named_ucs)?;
        s.h(&mut self.base_ucs)?;
        if v >= Version::R2007 {
            s.h(&mut self.background)?;
            s.h(&mut self.visual_style)?;
            s.h(&mut self.shadeplot)?;
            s.h(&mut self.sun)?;
        }
        Ok(())
    }
}

/// The end of a binary ACIS (SAB) stream, and of a text one.
const SAB_END: &[u8] = b"End\x0E\x02of\x0E\x04ACIS\x0D\x04data";
const SAT_END: &[u8] = b"End-of-ACIS-data";

/// ACIS data as stored.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AcisData {
    pub unknown: bool,
    /// 1: `blocks` of enciphered SAT text; 2: a SAT or SAB file in `file`.
    pub version: i16,
    /// Version 1: each block's bytes as stored, characters 0x21 to 0x7E as 0x9F - c (the spec
    /// says from 0x20, but spaces are stored as they are).
    pub blocks: Vec<Vec<u8>>,
    /// Version 2.
    pub file: Vec<u8>,
}

impl AcisData {
    /// After the empty bit (0).
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.b(&mut self.unknown)?;
        s.bs(&mut self.version)?;
        if self.version == 1 {
            if S::READ {
                self.blocks.clear();
                loop {
                    let mut n = 0i32;
                    s.bl(&mut n)?;
                    if n == 0 {
                        return Ok(());
                    }
                    if !(0..1 << 28).contains(&n) {
                        return bad(format!("SAT block of {n} bytes"));
                    }
                    let mut b = vec![];
                    s.bytes(&mut b, n as usize)?;
                    self.blocks.push(b);
                }
            }
            for b in &mut self.blocks {
                let mut n = b.len() as i32;
                s.bl(&mut n)?;
                s.bytes(b, n as usize)?;
            }
            return s.bl(&mut 0);
        }
        // no length: the file runs to its end mark
        if S::READ {
            self.file.clear();
            while !self.file.ends_with(SAB_END) && !self.file.ends_with(SAT_END) {
                if self.file.len() >= 1 << 28 {
                    return bad("ACIS file without an end");
                }
                let mut c = 0u8;
                s.rc(&mut c)?;
                self.file.push(c);
            }
            Ok(())
        } else {
            let n = self.file.len();
            s.bytes(&mut self.file, n)
        }
    }

    /// Version 1's SAT text, deciphered.
    pub fn sat(&self) -> Vec<u8> {
        let un = |c: u8| match c {
            0x21..=0x7E => 0x9F - c,
            c => c,
        };
        self.blocks.iter().flatten().map(|&c| un(c)).collect()
    }
}

/// A wire of an ACIS entity's wireframe.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Wire {
    pub kind: u8,
    pub selection_marker: i32,
    pub color: i16,
    pub acis_index: i32,
    pub points: Vec<[f64; 3]>,
    pub transform: Option<WireTransform>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WireTransform {
    pub x: [f64; 3],
    pub y: [f64; 3],
    pub z: [f64; 3],
    pub translation: [f64; 3],
    pub scale: f64,
    pub rotation: bool,
    pub reflection: bool,
    pub shear: bool,
}

fn wire_io<S: Io>(s: &mut S, w: &mut Wire) -> R<()> {
    s.rc(&mut w.kind)?;
    s.bl(&mut w.selection_marker)?;
    s.bs(&mut w.color)?;
    s.bl(&mut w.acis_index)?;
    s.list(&mut w.points, |s, p| s.bd3(p))?;
    let mut has = w.transform.is_some();
    s.b(&mut has)?;
    if S::READ {
        w.transform = has.then(WireTransform::default);
    }
    if let Some(t) = &mut w.transform {
        s.bd3(&mut t.x)?;
        s.bd3(&mut t.y)?;
        s.bd3(&mut t.z)?;
        s.bd3(&mut t.translation)?;
        s.bd(&mut t.scale)?;
        s.b(&mut t.rotation)?;
        s.b(&mut t.reflection)?;
        s.b(&mut t.shear)?;
    }
    Ok(())
}

/// A silhouette of an ACIS entity in a viewport.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Silhouette {
    pub viewport: i32,
    pub target: [f64; 3],
    pub direction: [f64; 3],
    pub up: [f64; 3],
    pub perspective: bool,
    pub wires: Vec<Wire>,
}

/// An ACIS entity's wireframe.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Wireframe {
    pub point: Option<[f64; 3]>,
    pub isolines: i32,
    pub isolines_present: bool,
    pub wires: Vec<Wire>,
    pub silhouettes: Vec<Silhouette>,
}

/// REGION, 3DSOLID and BODY: ACIS data, a wireframe and silhouettes.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Acis {
    /// None when the empty bit is set.
    pub data: Option<AcisData>,
    pub wireframe: Option<Wireframe>,
    /// More ACIS data after the wireframe (the second empty bit 0).
    pub more: Option<AcisData>,
    /// R2007+.
    pub unknown: i32,
    /// R2007+.
    pub history: Href,
}

fn acis_opt<S: Io>(s: &mut S, d: &mut Option<AcisData>) -> R<()> {
    let mut empty = d.is_none();
    s.b(&mut empty)?;
    if S::READ {
        *d = (!empty).then(AcisData::default);
    }
    match d {
        Some(d) => d.io(s),
        None => Ok(()),
    }
}

impl Spec for Acis {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        acis_opt(s, &mut self.data)?;
        let mut has = self.wireframe.is_some();
        s.b(&mut has)?;
        if S::READ {
            self.wireframe = has.then(Wireframe::default);
        }
        if let Some(w) = &mut self.wireframe {
            let mut has = w.point.is_some();
            s.b(&mut has)?;
            if S::READ {
                w.point = has.then_some([0.0; 3]);
            }
            if let Some(p) = &mut w.point {
                s.bd3(p)?;
            }
            s.bl(&mut w.isolines)?;
            s.b(&mut w.isolines_present)?;
            s.list(&mut w.wires, wire_io)?;
            s.list(&mut w.silhouettes, |s, x| {
                s.bl(&mut x.viewport)?;
                s.bd3(&mut x.target)?;
                s.bd3(&mut x.direction)?;
                s.bd3(&mut x.up)?;
                s.b(&mut x.perspective)?;
                s.list(&mut x.wires, wire_io)
            })?;
        }
        acis_opt(s, &mut self.more)?;
        if s.ver() >= Version::R2007 {
            s.bl(&mut self.unknown)?;
            s.h(&mut self.history)?;
        }
        Ok(())
    }
}

/// REGION.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Region(pub Acis);

/// 3DSOLID.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Solid3d(pub Acis);

/// BODY.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Body(pub Acis);

impl Spec for Region {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

impl Spec for Solid3d {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

impl Spec for Body {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

/// RAY: from a point, one way.
#[derive(Clone, Debug, PartialEq)]
pub struct Ray {
    pub at: [f64; 3],
    pub direction: [f64; 3],
}

impl Default for Ray {
    fn default() -> Ray {
        Ray {
            at: [0.0; 3],
            direction: X,
        }
    }
}

impl Spec for Ray {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bd3(&mut self.at)?;
        s.bd3(&mut self.direction)
    }
}

/// XLINE: a RAY both ways.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Xline(pub Ray);

impl Spec for Xline {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

/// OLE2FRAME: an embedded OLE object.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Ole2Frame {
    pub flags: i16,
    pub mode: i16,
    pub data: Vec<u8>,
    pub unknown: u8,
}

impl Spec for Ole2Frame {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bs(&mut self.flags)?;
        s.bs(&mut self.mode)?;
        let mut n = self.data.len() as i32;
        s.bl(&mut n)?;
        if !(0..1 << 28).contains(&n) {
            return bad(format!("OLE data of {n} bytes"));
        }
        s.bytes(&mut self.data, n as usize)?;
        s.rc(&mut self.unknown)
    }
}

/// ACAD_PROXY_ENTITY and ACAD_PROXY_OBJECT: an unknown class's data, kept as stored.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Proxy {
    pub class_id: i32,
    /// Before R2018: the version, and the maintenance version shifted 16 to the left.
    pub format: i32,
    /// R2018+.
    pub version: i32,
    pub maintenance: i32,
    /// 0 from DWG, 1 from DXF.
    pub from_dxf: bool,
    /// The rest of the data, `bits` long (strings in an R2007+ string stream are not kept).
    pub data: Vec<u8>,
    pub bits: usize,
    /// Every handle that follows.
    pub handles: Vec<Href>,
}

impl Spec for Proxy {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.class_id)?;
        if s.ver() >= Version::R2018 {
            s.bl(&mut self.version)?;
            s.bl(&mut self.maintenance)?;
        } else {
            s.bl(&mut self.format)?;
        }
        s.b(&mut self.from_dxf)?;
        s.rest(&mut self.data, &mut self.bits)?;
        if S::READ {
            self.handles.clear();
            while s.more_handles() {
                let mut h = Href::default();
                s.h(&mut h)?;
                self.handles.push(h);
            }
            return Ok(());
        }
        for h in &mut self.handles {
            s.h(h)?;
        }
        Ok(())
    }
}

/// ACAD_PROXY_ENTITY.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProxyEntity(pub Proxy);

/// ACAD_PROXY_OBJECT.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProxyObject(pub Proxy);

impl Spec for ProxyEntity {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

impl Spec for ProxyObject {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::BitReader;
    use crate::io::{Reader, Writer};
    use crate::object::s as st;
    use std::fmt::Debug;

    /// Writes `x` at `ver` and reads it back: the same value, the data all used.
    fn round<T: Spec + Default + PartialEq + Debug + Clone>(x: &T, ver: Version) {
        let mut w = Writer::new(ver);
        x.clone().io(&mut w).unwrap();
        let mut r = Reader {
            ver,
            data: BitReader::new(&w.data.bytes, 0),
            strs: (ver >= Version::R2007).then(|| BitReader::new(&w.strs.bytes, 0)),
            hdl: BitReader::new(&w.hdl.bytes, 0),
            base: 0,
            data_end: w.data.len,
        };
        let mut y = T::default();
        y.io(&mut r).unwrap();
        assert_eq!(r.data.p, w.data.len, "{ver:?}");
        assert_eq!(&y, x, "{ver:?}");
    }

    fn h(code: u8, handle: u32) -> Href {
        Href::new(code, handle)
    }

    fn color(ver: Version) -> Color {
        Color {
            rgb: (ver >= Version::R2004).then_some(0xC2_10_20_30),
            ..Color::index(5)
        }
    }

    fn attrib(ver: Version, multi: bool) -> Attrib {
        let mut a = Attrib {
            text: Text {
                insert: [1.0, 2.0],
                height: 3.5,
                text: st("value", ver),
                style: h(5, 0x11),
                ..Text::default()
            },
            tag: st("TAG", ver),
            flags: 9,
            lock_position: ver >= Version::R2007,
            version: (ver >= Version::R2010) as u8,
            ..Attrib::default()
        };
        if multi {
            a.kind = 2;
            a.mtext = MTextBody {
                insert: [1.0, 2.0, 0.0],
                rect_width: 40.0,
                rect_height: 10.0,
                text: st("two\\Plines", ver),
                style: h(5, 0x11),
                background: 1,
                bg_color: color(ver),
                bg_alpha: 0x0200_0040,
                app: h(5, 0x12),
                column_type: 2,
                column_count: 2,
                column_width: 20.0,
                gutter: 1.0,
                column_heights: vec![5.0, 6.0],
                ..MTextBody::default()
            };
            a.annot = vec![1, 2, 3];
            a.annot_app = h(5, 0x12);
        }
        a
    }

    fn owned(ver: Version) -> Owned {
        if ver >= Version::R2004 {
            Owned {
                items: vec![h(3, 0x31), h(3, 0x32)],
                seqend: h(3, 0x33),
                ..Owned::default()
            }
        } else {
            Owned {
                first: h(4, 0x31),
                last: h(4, 0x32),
                seqend: h(3, 0x33),
                ..Owned::default()
            }
        }
    }

    fn wire(transform: bool) -> Wire {
        Wire {
            kind: 1,
            selection_marker: 7,
            color: 256,
            acis_index: 3,
            points: vec![[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]],
            transform: transform.then(|| WireTransform {
                x: X,
                y: Y,
                z: UP,
                translation: [1.0, 2.0, 3.0],
                scale: 1.0,
                rotation: true,
                ..WireTransform::default()
            }),
        }
    }

    #[test]
    fn unvalidated_types_read_back() {
        use Version::*;
        for ver in [R2000, R2004, R2007, R2010, R2013, R2018] {
            round(&attrib(ver, false), ver);
            round(
                &AttDef {
                    attrib: attrib(ver, ver >= R2018),
                    version: (ver >= R2010) as u8,
                    prompt: st("Prompt?", ver),
                },
                ver,
            );
            let insert = Insert {
                at: [1.0, 2.0, 3.0],
                scale: [2.0, 3.0, 4.0],
                rotation: 0.5,
                has_attribs: true,
                block: h(5, 0x20),
                attribs: owned(ver),
                ..Insert::default()
            };
            round(&insert, ver);
            for scale in [[1.0, 2.0, 2.0], [2.0, 2.0, 2.0], [2.0, 2.0, 3.0]] {
                round(
                    &Insert {
                        scale,
                        ..insert.clone()
                    },
                    ver,
                );
            }
            round(
                &MInsert {
                    insert: insert.clone(),
                    cols: 3,
                    rows: 2,
                    col_spacing: 10.0,
                    row_spacing: 5.0,
                },
                ver,
            );
            for (start_width, end_width) in [(0.5, 0.5), (0.5, 1.0), (0.0, 0.0)] {
                let v = Vertex2d {
                    flags: 1,
                    at: [1.0, 2.0, 0.0],
                    start_width,
                    end_width,
                    bulge: 0.25,
                    id: (ver >= R2010) as i32 * 7,
                    tangent: 0.1,
                };
                round(&v, ver);
            }
            round(
                &VertexMesh(Vertex3d {
                    flags: 64,
                    at: [1.0, 2.0, 3.0],
                }),
                ver,
            );
            round(
                &Polyline2d {
                    flags: 1,
                    curve: 5,
                    start_width: 1.0,
                    end_width: 2.0,
                    thickness: 0.5,
                    elevation: 3.0,
                    vertices: owned(ver),
                    ..Polyline2d::default()
                },
                ver,
            );
            round(
                &PolylineMesh {
                    flags: 16,
                    curve: 6,
                    m: 3,
                    n: 4,
                    m_density: 6,
                    n_density: 6,
                    vertices: owned(ver),
                },
                ver,
            );
            let solid = Solid {
                thickness: 1.0,
                elevation: 2.0,
                corners: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                ..Solid::default()
            };
            round(&Trace(solid), ver);
            round(
                &Face3d {
                    corners: [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 2.0], [1.0, 1.0, 2.0]],
                    invisible: 5,
                },
                ver,
            );
            round(
                &Shape {
                    at: [1.0, 2.0, 0.0],
                    scale: 2.0,
                    number: 132,
                    style: h(5, 0x40),
                    ..Shape::default()
                },
                ver,
            );
            round(
                &Viewport {
                    width: 100.0,
                    height: 50.0,
                    status: 32800,
                    style_sheet: st("monochrome.ctb", ver),
                    shadeplot_mode: (ver >= R2004) as i16 * 2,
                    ambient: if ver >= R2007 {
                        color(ver)
                    } else {
                        Color::index(250)
                    },
                    frozen: vec![h(5, 0x10), h(5, 0x11)],
                    header: if ver == R2000 {
                        h(5, 0x2D3)
                    } else {
                        Href::NULL_HARD
                    },
                    sun: if ver >= R2007 { h(3, 0x50) } else { HARD_OWNER },
                    ..Viewport::default()
                },
                ver,
            );
            let acis = Acis {
                data: Some(AcisData {
                    unknown: true,
                    version: 1,
                    blocks: vec![b"knn ".to_vec(), b"oo\n".to_vec()],
                    file: vec![],
                }),
                wireframe: Some(Wireframe {
                    point: Some([1.0, 2.0, 3.0]),
                    isolines: 4,
                    isolines_present: true,
                    wires: vec![wire(false), wire(true)],
                    silhouettes: vec![Silhouette {
                        viewport: 2,
                        direction: UP,
                        up: Y,
                        wires: vec![wire(true)],
                        ..Silhouette::default()
                    }],
                }),
                more: Some(AcisData {
                    version: 2,
                    file: [b"ACIS BinaryFile..".as_slice(), SAB_END].concat(),
                    ..AcisData::default()
                }),
                unknown: 0,
                history: if ver >= R2007 { h(5, 0x60) } else { Href::default() },
            };
            round(&Body(acis.clone()), ver);
            round(
                &Region(Acis {
                    data: None,
                    wireframe: None,
                    more: None,
                    ..acis
                }),
                ver,
            );
            round(
                &Ole2Frame {
                    flags: 2,
                    mode: 1,
                    data: vec![0xD0, 0xCF, 0x11, 0xE0],
                    unknown: 0,
                },
                ver,
            );
            let proxy = Proxy {
                class_id: 500,
                format: if ver < R2018 { 0x1F_0000 | 21 } else { 0 },
                version: if ver >= R2018 { 33 } else { 0 },
                maintenance: if ver >= R2018 { 29 } else { 0 },
                from_dxf: false,
                data: vec![0xA5, 0xC0],
                bits: 11,
                handles: vec![h(3, 0x70), h(4, 0x71)],
            };
            round(&ProxyEntity(proxy.clone()), ver);
            round(&ProxyObject(proxy), ver);
        }
    }

    #[test]
    fn sat_text_is_deciphered() {
        let d = AcisData {
            version: 1,
            blocks: vec![vec![
                0x9F - b'4',
                0x9F - b'0',
                0x9F - b'0',
                b' ',
                0x9F - b'2',
                b'\n',
            ]],
            ..AcisData::default()
        };
        assert_eq!(d.sat(), b"400 2\n");
    }
}
