//! Annotation and fill entities' own fields (chapter 20.4): the dimensions, MTEXT, LEADER,
//! TOLERANCE, MLINE, HATCH, IMAGE and WIPEOUT.

use super::Spec;
use crate::bits::R;
use crate::io::{Color, Href, Io};
use crate::str::Str;
use crate::version::Version;

const UP: [f64; 3] = [0.0, 0.0, 1.0];

/// A BS count, then that many items.
fn bs_list<S: Io, T: Default>(
    s: &mut S,
    v: &mut Vec<T>,
    mut f: impl FnMut(&mut S, &mut T) -> R<()>,
) -> R<()> {
    let mut n = v.len() as i16;
    s.bs(&mut n)?;
    s.items(v, n as i32, &mut f)
}

/// What every dimension has after the entity's common data (20.4.22).
#[derive(Clone, Debug, PartialEq)]
pub struct DimCommon {
    /// R2010+.
    pub version: u8,
    pub extrusion: [f64; 3],
    pub text_mid: [f64; 2],
    pub elevation: f64,
    /// Bit 0 the opposite of the 70 group's 128, bit 1 its 32.
    pub flags1: u8,
    pub user_text: Str,
    pub text_rotation: f64,
    pub horiz_dir: f64,
    /// The anonymous block's insertion scale and rotation.
    pub ins_scale: [f64; 3],
    pub ins_rotation: f64,
    /// R2000+.
    pub attachment: i16,
    pub linespacing_style: i16,
    pub linespacing_factor: f64,
    pub measurement: f64,
    /// R2007+.
    pub unknown: bool,
    pub flip_arrow1: bool,
    pub flip_arrow2: bool,
    pub pt12: [f64; 2],
    pub dimstyle: Href,
    pub block: Href,
}

impl Default for DimCommon {
    fn default() -> DimCommon {
        DimCommon {
            version: 0,
            extrusion: UP,
            text_mid: [0.0; 2],
            elevation: 0.0,
            flags1: 0,
            user_text: Str::default(),
            text_rotation: 0.0,
            horiz_dir: 0.0,
            ins_scale: [1.0; 3],
            ins_rotation: 0.0,
            attachment: 5,
            linespacing_style: 1,
            linespacing_factor: 1.0,
            measurement: 0.0,
            unknown: false,
            flip_arrow1: false,
            flip_arrow2: false,
            pt12: [0.0; 2],
            dimstyle: Href::NULL_HARD,
            block: Href::NULL_HARD,
        }
    }
}

impl DimCommon {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        if v >= Version::R2010 {
            s.rc(&mut self.version)?;
        }
        s.bd3(&mut self.extrusion)?;
        s.rd2(&mut self.text_mid)?;
        s.bd(&mut self.elevation)?;
        s.rc(&mut self.flags1)?;
        s.tv(&mut self.user_text)?;
        s.bd(&mut self.text_rotation)?;
        s.bd(&mut self.horiz_dir)?;
        s.bd3(&mut self.ins_scale)?;
        s.bd(&mut self.ins_rotation)?;
        if v >= Version::R2000 {
            s.bs(&mut self.attachment)?;
            s.bs(&mut self.linespacing_style)?;
            s.bd(&mut self.linespacing_factor)?;
            s.bd(&mut self.measurement)?;
        }
        if v >= Version::R2007 {
            s.b(&mut self.unknown)?;
            s.b(&mut self.flip_arrow1)?;
            s.b(&mut self.flip_arrow2)?;
        }
        s.rd2(&mut self.pt12)
    }

    /// The handles after the entity's: DIMSTYLE, then the anonymous block.
    fn handles<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.h(&mut self.dimstyle)?;
        s.h(&mut self.block)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimOrdinate {
    pub dim: DimCommon,
    pub pt10: [f64; 3],
    pub pt13: [f64; 3],
    pub pt14: [f64; 3],
    /// Bit 0 is the 70 group's 64 (an X-type ordinate).
    pub flags2: u8,
}

impl Spec for DimOrdinate {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.pt10)?;
        s.bd3(&mut self.pt13)?;
        s.bd3(&mut self.pt14)?;
        s.rc(&mut self.flags2)?;
        self.dim.handles(s)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimLinear {
    pub dim: DimCommon,
    pub pt13: [f64; 3],
    pub pt14: [f64; 3],
    pub pt10: [f64; 3],
    pub ext_rotation: f64,
    pub rotation: f64,
}

impl Spec for DimLinear {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.pt13)?;
        s.bd3(&mut self.pt14)?;
        s.bd3(&mut self.pt10)?;
        s.bd(&mut self.ext_rotation)?;
        s.bd(&mut self.rotation)?;
        self.dim.handles(s)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimAligned {
    pub dim: DimCommon,
    pub pt13: [f64; 3],
    pub pt14: [f64; 3],
    pub pt10: [f64; 3],
    pub ext_rotation: f64,
}

impl Spec for DimAligned {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.pt13)?;
        s.bd3(&mut self.pt14)?;
        s.bd3(&mut self.pt10)?;
        s.bd(&mut self.ext_rotation)?;
        self.dim.handles(s)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimAng3Pt {
    pub dim: DimCommon,
    pub pt10: [f64; 3],
    pub pt13: [f64; 3],
    pub pt14: [f64; 3],
    pub pt15: [f64; 3],
}

impl Spec for DimAng3Pt {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.pt10)?;
        s.bd3(&mut self.pt13)?;
        s.bd3(&mut self.pt14)?;
        s.bd3(&mut self.pt15)?;
        self.dim.handles(s)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimAng2Ln {
    pub dim: DimCommon,
    pub pt16: [f64; 2],
    pub pt13: [f64; 3],
    pub pt14: [f64; 3],
    pub pt15: [f64; 3],
    pub pt10: [f64; 3],
}

impl Spec for DimAng2Ln {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.rd2(&mut self.pt16)?;
        s.bd3(&mut self.pt13)?;
        s.bd3(&mut self.pt14)?;
        s.bd3(&mut self.pt15)?;
        s.bd3(&mut self.pt10)?;
        self.dim.handles(s)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimRadius {
    pub dim: DimCommon,
    pub pt10: [f64; 3],
    pub pt15: [f64; 3],
    pub leader_len: f64,
}

impl Spec for DimRadius {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.pt10)?;
        s.bd3(&mut self.pt15)?;
        // the spec says D; a BD as for DIAMETER
        s.bd(&mut self.leader_len)?;
        self.dim.handles(s)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DimDiameter {
    pub dim: DimCommon,
    pub pt15: [f64; 3],
    pub pt10: [f64; 3],
    pub leader_len: f64,
}

impl Spec for DimDiameter {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.pt15)?;
        s.bd3(&mut self.pt10)?;
        s.bd(&mut self.leader_len)?;
        self.dim.handles(s)
    }
}

/// ARC_DIMENSION (a class).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ArcDimension {
    pub dim: DimCommon,
    pub arc_pt: [f64; 3],
    pub ext1: [f64; 3],
    pub ext2: [f64; 3],
    pub center: [f64; 3],
    pub partial: bool,
    pub start: f64,
    pub end: f64,
    pub has_leader: bool,
    pub leader1: [f64; 3],
    pub leader2: [f64; 3],
}

impl Spec for ArcDimension {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.arc_pt)?;
        s.bd3(&mut self.ext1)?;
        s.bd3(&mut self.ext2)?;
        s.bd3(&mut self.center)?;
        s.b(&mut self.partial)?;
        s.bd(&mut self.start)?;
        s.bd(&mut self.end)?;
        s.b(&mut self.has_leader)?;
        s.bd3(&mut self.leader1)?;
        s.bd3(&mut self.leader2)?;
        self.dim.handles(s)
    }
}

/// LARGE_RADIAL_DIMENSION (a class): a jogged radius.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LargeRadialDimension {
    pub dim: DimCommon,
    pub center: [f64; 3],
    pub chord: [f64; 3],
    pub unknown: f64,
    pub override_center: [f64; 3],
    pub jog: [f64; 3],
}

impl Spec for LargeRadialDimension {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dim.io(s)?;
        s.bd3(&mut self.center)?;
        s.bd3(&mut self.chord)?;
        s.bd(&mut self.unknown)?;
        s.bd3(&mut self.override_center)?;
        s.bd3(&mut self.jog)?;
        self.dim.handles(s)
    }
}

/// MTEXT's R2018+ fields when it is not annotative: the redundant copy and the columns.
#[derive(Clone, Debug, PartialEq)]
pub struct MTextContext {
    pub version: i16,
    pub default_flag: bool,
    pub app: Href,
    pub attachment: i32,
    pub x_axis: [f64; 3],
    pub insert: [f64; 3],
    pub rect_width: f64,
    pub rect_height: f64,
    pub ext_width: f64,
    pub ext_height: f64,
    /// 0 none, 1 static, 2 dynamic.
    pub column_type: i16,
    pub column_count: i32,
    pub column_width: f64,
    pub gutter: f64,
    pub auto_height: bool,
    pub flow_reversed: bool,
    /// Dynamic columns without auto height: each one's height.
    pub column_heights: Vec<f64>,
}

impl Default for MTextContext {
    fn default() -> MTextContext {
        MTextContext {
            version: 0,
            default_flag: true,
            app: Href::NULL_HARD,
            attachment: 1,
            x_axis: [1.0, 0.0, 0.0],
            insert: [0.0; 3],
            rect_width: 0.0,
            rect_height: 0.0,
            ext_width: 0.0,
            ext_height: 0.0,
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

#[derive(Clone, Debug, PartialEq)]
pub struct MText {
    pub insert: [f64; 3],
    pub extrusion: [f64; 3],
    pub x_axis: [f64; 3],
    pub rect_width: f64,
    /// R2007+.
    pub rect_height: f64,
    pub text_height: f64,
    /// 1..9: top left to bottom right.
    pub attachment: i16,
    pub drawing_dir: i16,
    pub ext_height: f64,
    pub ext_width: f64,
    pub text: Str,
    pub style: Href,
    /// R2000+.
    pub linespacing_style: i16,
    pub linespacing_factor: f64,
    pub unknown: bool,
    /// R2004+: 1 fill, 2 fill with the drawing's colour, 0x10 text frame (R2018+).
    pub bg_flags: i32,
    pub bg_scale: f64,
    pub bg_color: Color,
    pub bg_transparency: i32,
    /// R2018+: present when the text is not annotative.
    pub context: Option<MTextContext>,
}

impl Default for MText {
    fn default() -> MText {
        MText {
            insert: [0.0; 3],
            extrusion: UP,
            x_axis: [1.0, 0.0, 0.0],
            rect_width: 0.0,
            rect_height: 0.0,
            text_height: 2.5,
            attachment: 1,
            drawing_dir: 1,
            ext_height: 0.0,
            ext_width: 0.0,
            text: Str::default(),
            style: Href::NULL_HARD,
            linespacing_style: 1,
            linespacing_factor: 1.0,
            unknown: false,
            bg_flags: 0,
            bg_scale: 1.5,
            bg_color: Color::index(256),
            bg_transparency: 0,
            context: Some(MTextContext::default()),
        }
    }
}

impl Spec for MText {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        s.bd3(&mut self.insert)?;
        s.bd3(&mut self.extrusion)?;
        s.bd3(&mut self.x_axis)?;
        s.bd(&mut self.rect_width)?;
        if v >= Version::R2007 {
            s.bd(&mut self.rect_height)?;
        }
        s.bd(&mut self.text_height)?;
        s.bs(&mut self.attachment)?;
        s.bs(&mut self.drawing_dir)?;
        s.bd(&mut self.ext_height)?;
        s.bd(&mut self.ext_width)?;
        s.tv(&mut self.text)?;
        s.h(&mut self.style)?;
        if v >= Version::R2000 {
            s.bs(&mut self.linespacing_style)?;
            s.bd(&mut self.linespacing_factor)?;
            s.b(&mut self.unknown)?;
        }
        if v >= Version::R2004 {
            s.bl(&mut self.bg_flags)?;
            if self.bg_flags & 1 != 0 || (v >= Version::R2018 && self.bg_flags & 0x10 != 0) {
                // the spec says BL; a scale (default 1.5) is a BD
                s.bd(&mut self.bg_scale)?;
                s.cmc(&mut self.bg_color)?;
                s.bl(&mut self.bg_transparency)?;
            }
        }
        if v >= Version::R2018 {
            // the bit is "is NOT annotative"
            let mut plain = self.context.is_some();
            s.b(&mut plain)?;
            if S::READ {
                self.context = plain.then(MTextContext::default);
            }
            if let Some(c) = &mut self.context {
                s.bs(&mut c.version)?;
                s.b(&mut c.default_flag)?;
                s.h(&mut c.app)?;
                s.bl(&mut c.attachment)?;
                s.bd3(&mut c.x_axis)?;
                s.bd3(&mut c.insert)?;
                s.bd(&mut c.rect_width)?;
                s.bd(&mut c.rect_height)?;
                s.bd(&mut c.ext_width)?;
                s.bd(&mut c.ext_height)?;
                s.bs(&mut c.column_type)?;
                if c.column_type != 0 {
                    s.bl(&mut c.column_count)?;
                    s.bd(&mut c.column_width)?;
                    s.bd(&mut c.gutter)?;
                    s.b(&mut c.auto_height)?;
                    s.b(&mut c.flow_reversed)?;
                    let n = if !c.auto_height && c.column_type == 2 {
                        c.column_count
                    } else {
                        0
                    };
                    s.items(&mut c.column_heights, n, &mut |s, h| s.bd(h))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Leader {
    pub unknown: bool,
    /// 0 MTEXT, 1 TOLERANCE, 2 INSERT, 3 none.
    pub annot_type: i16,
    pub path_type: i16,
    pub points: Vec<[f64; 3]>,
    pub origin: [f64; 3],
    pub extrusion: [f64; 3],
    pub x_dir: [f64; 3],
    pub block_offset: [f64; 3],
    pub endpt_proj: [f64; 3],
    pub box_height: f64,
    pub box_width: f64,
    pub hook_on_x: bool,
    pub arrowhead: bool,
    /// R2000+: unknown BS, B, B.
    pub unknown_bs: i16,
    pub unknown_b1: bool,
    pub unknown_b2: bool,
    pub annotation: Href,
    pub dimstyle: Href,
}

impl Default for Leader {
    fn default() -> Leader {
        Leader {
            unknown: false,
            annot_type: 3,
            path_type: 0,
            points: vec![],
            origin: [0.0; 3],
            extrusion: UP,
            x_dir: [1.0, 0.0, 0.0],
            block_offset: [0.0; 3],
            endpt_proj: [0.0; 3],
            box_height: 0.0,
            box_width: 0.0,
            hook_on_x: false,
            arrowhead: true,
            unknown_bs: 0,
            unknown_b1: false,
            unknown_b2: false,
            annotation: Href::NULL_HARD,
            dimstyle: Href::NULL_HARD,
        }
    }
}

impl Spec for Leader {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.b(&mut self.unknown)?;
        s.bs(&mut self.annot_type)?;
        s.bs(&mut self.path_type)?;
        s.list(&mut self.points, |s, p| s.bd3(p))?;
        s.bd3(&mut self.origin)?;
        s.bd3(&mut self.extrusion)?;
        s.bd3(&mut self.x_dir)?;
        s.bd3(&mut self.block_offset)?;
        // R14+ (so every version here); R13-R14's DIMGAP is not stored
        s.bd3(&mut self.endpt_proj)?;
        s.bd(&mut self.box_height)?;
        s.bd(&mut self.box_width)?;
        s.b(&mut self.hook_on_x)?;
        s.b(&mut self.arrowhead)?;
        if s.ver() >= Version::R2000 {
            s.bs(&mut self.unknown_bs)?;
            s.b(&mut self.unknown_b1)?;
            s.b(&mut self.unknown_b2)?;
        }
        s.h(&mut self.annotation)?;
        s.h(&mut self.dimstyle)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tolerance {
    pub insert: [f64; 3],
    pub x_dir: [f64; 3],
    pub extrusion: [f64; 3],
    pub text: Str,
    pub dimstyle: Href,
}

impl Default for Tolerance {
    fn default() -> Tolerance {
        Tolerance {
            insert: [0.0; 3],
            x_dir: [1.0, 0.0, 0.0],
            extrusion: UP,
            text: Str::default(),
            dimstyle: Href::NULL_HARD,
        }
    }
}

impl Spec for Tolerance {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bd3(&mut self.insert)?;
        s.bd3(&mut self.x_dir)?;
        s.bd3(&mut self.extrusion)?;
        // the spec says BS; it is a TV
        s.tv(&mut self.text)?;
        s.h(&mut self.dimstyle)
    }
}

/// One of an MLINE's lines at a vertex: where its dashes break, and its fill.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MLineLine {
    pub segparms: Vec<f64>,
    pub fillparms: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MLineVertex {
    pub at: [f64; 3],
    pub dir: [f64; 3],
    pub miter: [f64; 3],
    /// One per line in the style.
    pub lines: Vec<MLineLine>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MLine {
    pub scale: f64,
    /// 0 top, 1 centre, 2 bottom.
    pub just: u8,
    pub base: [f64; 3],
    pub extrusion: [f64; 3],
    /// 1 open, 3 closed.
    pub open_closed: i16,
    pub lines_in_style: u8,
    pub vertices: Vec<MLineVertex>,
    pub style: Href,
}

impl Default for MLine {
    fn default() -> MLine {
        MLine {
            scale: 1.0,
            just: 0,
            base: [0.0; 3],
            extrusion: UP,
            open_closed: 1,
            lines_in_style: 0,
            vertices: vec![],
            style: Href::NULL_HARD,
        }
    }
}

impl Spec for MLine {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bd(&mut self.scale)?;
        s.rc(&mut self.just)?;
        s.bd3(&mut self.base)?;
        s.bd3(&mut self.extrusion)?;
        s.bs(&mut self.open_closed)?;
        s.rc(&mut self.lines_in_style)?;
        let nl = self.lines_in_style as i32;
        bs_list(s, &mut self.vertices, |s, v| {
            s.bd3(&mut v.at)?;
            s.bd3(&mut v.dir)?;
            s.bd3(&mut v.miter)?;
            s.items(&mut v.lines, nl, &mut |s, l| {
                bs_list(s, &mut l.segparms, |s, x| s.bd(x))?;
                bs_list(s, &mut l.fillparms, |s, x| s.bd(x))
            })
        })?;
        s.h(&mut self.style)
    }
}

/// A gradient colour stop (R2004+), as stored.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GradientColor {
    pub value: f64,
    pub index: i16,
    pub rgb: i32,
    /// The colour's flag byte (the spec says ignored).
    pub byte: u8,
}

/// A spline edge's control point and, when rational, its weight.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HatchSpline {
    pub degree: i32,
    pub rational: bool,
    pub periodic: bool,
    pub knots: Vec<f64>,
    pub ctrl: Vec<[f64; 2]>,
    /// One per control point when rational, else empty.
    pub weights: Vec<f64>,
    /// R2010+.
    pub fit: Vec<[f64; 2]>,
    pub start_tangent: [f64; 2],
    pub end_tangent: [f64; 2],
}

/// An edge of a boundary path.
#[derive(Clone, Debug, PartialEq)]
pub enum HatchEdge {
    Line {
        a: [f64; 2],
        b: [f64; 2],
    },
    Arc {
        center: [f64; 2],
        radius: f64,
        start: f64,
        end: f64,
        ccw: bool,
    },
    Ellipse {
        center: [f64; 2],
        /// The major axis's end, from the centre.
        major: [f64; 2],
        ratio: f64,
        start: f64,
        end: f64,
        ccw: bool,
    },
    Spline(HatchSpline),
}

impl Default for HatchEdge {
    fn default() -> HatchEdge {
        HatchEdge::Line {
            a: [0.0; 2],
            b: [0.0; 2],
        }
    }
}

impl HatchEdge {
    fn kind(&self) -> u8 {
        match self {
            HatchEdge::Line { .. } => 1,
            HatchEdge::Arc { .. } => 2,
            HatchEdge::Ellipse { .. } => 3,
            HatchEdge::Spline(_) => 4,
        }
    }

    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let mut kind = self.kind();
        s.rc(&mut kind)?;
        if S::READ {
            *self = match kind {
                1 => HatchEdge::default(),
                2 => HatchEdge::Arc {
                    center: [0.0; 2],
                    radius: 0.0,
                    start: 0.0,
                    end: 0.0,
                    ccw: true,
                },
                3 => HatchEdge::Ellipse {
                    center: [0.0; 2],
                    major: [0.0; 2],
                    ratio: 1.0,
                    start: 0.0,
                    end: 0.0,
                    ccw: true,
                },
                4 => HatchEdge::Spline(HatchSpline::default()),
                k => return crate::bits::bad(format!("hatch edge type {k}")),
            };
        }
        match self {
            HatchEdge::Line { a, b } => {
                s.rd2(a)?;
                s.rd2(b)
            }
            HatchEdge::Arc {
                center,
                radius,
                start,
                end,
                ccw,
            } => {
                s.rd2(center)?;
                s.bd(radius)?;
                s.bd(start)?;
                s.bd(end)?;
                s.b(ccw)
            }
            HatchEdge::Ellipse {
                center,
                major,
                ratio,
                start,
                end,
                ccw,
            } => {
                s.rd2(center)?;
                s.rd2(major)?;
                s.bd(ratio)?;
                s.bd(start)?;
                s.bd(end)?;
                s.b(ccw)
            }
            HatchEdge::Spline(p) => {
                s.bl(&mut p.degree)?;
                s.b(&mut p.rational)?;
                s.b(&mut p.periodic)?;
                let mut nk = p.knots.len() as i32;
                let mut nc = p.ctrl.len() as i32;
                s.bl(&mut nk)?;
                s.bl(&mut nc)?;
                s.items(&mut p.knots, nk, &mut |s, k| s.bd(k))?;
                let rational = p.rational;
                if S::READ {
                    p.weights.clear();
                } else if rational && p.weights.len() != p.ctrl.len() {
                    return crate::bits::bad("hatch spline: a weight per control point");
                }
                let mut ws = std::mem::take(&mut p.weights);
                let mut i = 0;
                s.items(&mut p.ctrl, nc, &mut |s, c| {
                    s.rd2(c)?;
                    if rational {
                        if S::READ {
                            ws.push(0.0);
                        }
                        s.bd(&mut ws[i])?;
                    }
                    i += 1;
                    Ok(())
                })?;
                p.weights = ws;
                if s.ver() >= Version::R2010 {
                    s.list(&mut p.fit, |s, f| s.rd2(f))?;
                    s.rd2(&mut p.start_tangent)?;
                    s.rd2(&mut p.end_tangent)?;
                }
                Ok(())
            }
        }
    }
}

/// A boundary path: edges, or (flag 2) a polyline.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HatchPath {
    /// 1 external, 2 polyline, 4 derived, 8 textbox, 16 outermost.
    pub flag: i32,
    pub edges: Vec<HatchEdge>,
    pub has_bulges: bool,
    pub closed: bool,
    /// Polyline paths: x, y, bulge (0 when no bulges are stored).
    pub vertices: Vec<[f64; 3]>,
    /// The objects the path came from (soft pointers).
    pub boundary: Vec<Href>,
}

impl HatchPath {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.flag)?;
        if self.flag & 2 == 0 {
            s.list(&mut self.edges, |s, e| e.io(s))?;
        } else {
            s.b(&mut self.has_bulges)?;
            s.b(&mut self.closed)?;
            let bulges = self.has_bulges;
            s.list(&mut self.vertices, |s, v| {
                let mut p = [v[0], v[1]];
                s.rd2(&mut p)?;
                v[0] = p[0];
                v[1] = p[1];
                if bulges {
                    s.bd(&mut v[2])?;
                }
                Ok(())
            })?;
        }
        // the count here, the handles after the entity's
        s.list(&mut self.boundary, |s, h| s.h(h))
    }
}

/// A pattern's definition line.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HatchLine {
    pub angle: f64,
    pub base: [f64; 2],
    pub offset: [f64; 2],
    pub dashes: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hatch {
    /// R2004+: gradient fill.
    pub gradient: i32,
    pub reserved: i32,
    pub gradient_angle: f64,
    pub gradient_shift: f64,
    pub single_color: i32,
    pub gradient_tint: f64,
    pub gradient_colors: Vec<GradientColor>,
    pub gradient_name: Str,
    pub elevation: f64,
    pub extrusion: [f64; 3],
    pub name: Str,
    pub solid: bool,
    pub associative: bool,
    pub paths: Vec<HatchPath>,
    /// 0 odd parity, 1 outermost, 2 whole area.
    pub style: i16,
    /// 0 user-defined, 1 predefined, 2 custom.
    pub pattern_type: i16,
    /// Pattern fills only (not solid).
    pub angle: f64,
    pub scale: f64,
    pub double: bool,
    pub lines: Vec<HatchLine>,
    /// When a path is derived (flag 4).
    pub pixel_size: f64,
    pub seeds: Vec<[f64; 2]>,
}

impl Default for Hatch {
    fn default() -> Hatch {
        Hatch {
            gradient: 0,
            reserved: 0,
            gradient_angle: 0.0,
            gradient_shift: 0.0,
            single_color: 0,
            gradient_tint: 0.0,
            gradient_colors: vec![],
            gradient_name: Str::default(),
            elevation: 0.0,
            extrusion: UP,
            name: Str::default(),
            solid: false,
            associative: false,
            paths: vec![],
            style: 0,
            pattern_type: 1,
            angle: 0.0,
            scale: 1.0,
            double: false,
            lines: vec![],
            pixel_size: 0.0,
            seeds: vec![],
        }
    }
}

impl Spec for Hatch {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        if s.ver() >= Version::R2004 {
            s.bl(&mut self.gradient)?;
            s.bl(&mut self.reserved)?;
            s.bd(&mut self.gradient_angle)?;
            s.bd(&mut self.gradient_shift)?;
            s.bl(&mut self.single_color)?;
            s.bd(&mut self.gradient_tint)?;
            s.list(&mut self.gradient_colors, |s, c| {
                s.bd(&mut c.value)?;
                s.bs(&mut c.index)?;
                s.bl(&mut c.rgb)?;
                s.rc(&mut c.byte)
            })?;
            s.tv(&mut self.gradient_name)?;
        }
        s.bd(&mut self.elevation)?;
        s.bd3(&mut self.extrusion)?;
        s.tv(&mut self.name)?;
        s.b(&mut self.solid)?;
        s.b(&mut self.associative)?;
        s.list(&mut self.paths, |s, p| p.io(s))?;
        s.bs(&mut self.style)?;
        s.bs(&mut self.pattern_type)?;
        if !self.solid {
            s.bd(&mut self.angle)?;
            s.bd(&mut self.scale)?;
            s.b(&mut self.double)?;
            bs_list(s, &mut self.lines, |s, l| {
                s.bd(&mut l.angle)?;
                s.bd2(&mut l.base)?;
                s.bd2(&mut l.offset)?;
                bs_list(s, &mut l.dashes, |s, d| s.bd(d))
            })?;
        }
        if self.paths.iter().any(|p| p.flag & 4 != 0) {
            s.bd(&mut self.pixel_size)?;
        }
        s.list(&mut self.seeds, |s, p| s.rd2(p))
    }
}

/// IMAGE (a class); WIPEOUT stores the same.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub class_version: i32,
    pub insert: [f64; 3],
    /// One pixel's step along the image's rows and columns.
    pub u: [f64; 3],
    pub v: [f64; 3],
    /// In pixels.
    pub size: [f64; 2],
    /// 1 show, 2 show unaligned, 4 clip, 8 transparent.
    pub display: i16,
    pub clipping: bool,
    pub brightness: u8,
    pub contrast: u8,
    pub fade: u8,
    /// R2010+: 0 outside, 1 inside.
    pub clip_mode: bool,
    /// 1 rectangle (two corners), 2 polygon.
    pub clip_type: i16,
    pub clip: Vec<[f64; 2]>,
    pub imagedef: Href,
    pub reactor: Href,
}

impl Default for Image {
    fn default() -> Image {
        Image {
            class_version: 0,
            insert: [0.0; 3],
            u: [1.0, 0.0, 0.0],
            v: [0.0, 1.0, 0.0],
            size: [1.0, 1.0],
            display: 7,
            clipping: false,
            brightness: 50,
            contrast: 50,
            fade: 0,
            clip_mode: false,
            clip_type: 1,
            clip: vec![[-0.5, -0.5], [0.5, 0.5]],
            imagedef: Href::NULL_HARD,
            reactor: Href::new(3, 0),
        }
    }
}

impl Spec for Image {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.class_version)?;
        s.bd3(&mut self.insert)?;
        s.bd3(&mut self.u)?;
        s.bd3(&mut self.v)?;
        s.rd2(&mut self.size)?;
        s.bs(&mut self.display)?;
        s.b(&mut self.clipping)?;
        s.rc(&mut self.brightness)?;
        s.rc(&mut self.contrast)?;
        s.rc(&mut self.fade)?;
        if s.ver() >= Version::R2010 {
            s.b(&mut self.clip_mode)?;
        }
        s.bs(&mut self.clip_type)?;
        if self.clip_type == 1 {
            s.items(&mut self.clip, 2, &mut |s, p| s.rd2(p))?;
        } else {
            s.list(&mut self.clip, |s, p| s.rd2(p))?;
        }
        s.h(&mut self.imagedef)?;
        s.h(&mut self.reactor)
    }
}

/// WIPEOUT (a class): an IMAGE's fields.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Wipeout(pub Image);

impl Spec for Wipeout {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.0.io(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{Classes, EntCommon, Object, encode_object, read_object, s as text};
    use crate::types::Data;

    /// Encodes an entity of `make`'s data for every version and reads it back.
    fn round_trip(type_code: u16, make: impl Fn(Version) -> Data) {
        let list = [
            ("IMAGE".to_string(), true),
            ("LARGE_RADIAL_DIMENSION".to_string(), true),
            ("HATCH".to_string(), true),
        ];
        let classes = Classes { list: &list };
        for ver in Version::ALL {
            let o = Object {
                handle: 0x2A,
                type_code,
                eed: vec![],
                owner: Href::NULL_SOFT,
                reactors: vec![Href::new(4, 0x1F)],
                xdic: if ver >= Version::R2004 {
                    None
                } else {
                    Some(Href::NULL_HARD)
                },
                ds_binary: false,
                ent: Some(EntCommon {
                    layer: Href::new(5, 0x10),
                    ..EntCommon::default()
                }),
                data: make(ver),
                trailing: vec![],
                raw: None,
            };
            let (bytes, _) = encode_object(&o, ver).unwrap();
            let back =
                read_object(&bytes, 0, ver, &classes, None).unwrap_or_else(|e| panic!("{ver:?}: {}", e.0));
            assert_eq!(back.data, o.data, "{ver:?}");
            assert_eq!(back.ent, o.ent, "{ver:?}");
        }
    }

    fn dim(ver: Version) -> DimCommon {
        DimCommon {
            text_mid: [3.0, 4.0],
            elevation: 0.5,
            flags1: 1,
            user_text: text("<> mm", ver),
            text_rotation: 0.25,
            measurement: 12.5,
            flip_arrow1: ver >= Version::R2007,
            pt12: [1.0, 2.0],
            dimstyle: Href::new(5, 0x27),
            block: Href::new(5, 0x99),
            ..DimCommon::default()
        }
    }

    #[test]
    fn dimensions_read_back() {
        round_trip(25, |v| {
            Data::DimRadius(DimRadius {
                dim: dim(v),
                pt10: [1.0, 1.0, 0.0],
                pt15: [4.0, 5.0, 0.0],
                leader_len: 2.0,
            })
        });
        round_trip(26, |v| {
            Data::DimDiameter(DimDiameter {
                dim: dim(v),
                pt15: [-4.0, -5.0, 0.0],
                pt10: [4.0, 5.0, 0.0],
                leader_len: 0.0,
            })
        });
        round_trip(23, |v| {
            Data::DimAng3Pt(DimAng3Pt {
                dim: dim(v),
                pt10: [1.0, 2.0, 0.0],
                pt13: [3.0, 0.0, 0.0],
                pt14: [0.0, 3.0, 0.0],
                pt15: [0.0, 0.0, 0.0],
            })
        });
        round_trip(20, |v| {
            Data::DimOrdinate(DimOrdinate {
                dim: dim(v),
                pt10: [0.0; 3],
                pt13: [5.0, 6.0, 0.0],
                pt14: [5.0, 9.0, 0.0],
                flags2: 1,
            })
        });
        round_trip(501, |v| {
            Data::LargeRadialDimension(LargeRadialDimension {
                dim: dim(v),
                center: [0.0; 3],
                chord: [100.0, 0.0, 0.0],
                unknown: 0.0,
                override_center: [40.0, -10.0, 0.0],
                jog: [60.0, -5.0, 0.0],
            })
        });
    }

    #[test]
    fn leader_and_image_read_back() {
        round_trip(45, |_| {
            Data::Leader(Leader {
                annot_type: 0,
                path_type: 1,
                points: vec![[0.0, 0.0, 0.0], [10.0, 5.0, 0.0], [15.0, 5.0, 0.0]],
                box_height: 2.5,
                box_width: 12.0,
                hook_on_x: true,
                annotation: Href::new(5, 0x44),
                dimstyle: Href::new(5, 0x27),
                ..Leader::default()
            })
        });
        round_trip(500, |v| {
            Data::Image(Image {
                size: [640.0, 480.0],
                clip_mode: v >= Version::R2010,
                imagedef: Href::new(5, 0x50),
                reactor: Href::new(3, 0x51),
                ..Image::default()
            })
        });
        round_trip(500, |_| {
            Data::Image(Image {
                clip_type: 2,
                clip: vec![[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]],
                ..Image::default()
            })
        });
    }

    #[test]
    fn mtext_background_and_columns_read_back() {
        round_trip(44, |v| {
            let new = v >= Version::R2004;
            Data::MText(MText {
                text: text("{\\fArial|b0;two}\\Pcolumns", v),
                rect_height: if v >= Version::R2007 { 30.0 } else { 0.0 },
                bg_flags: new as i32,
                bg_color: if new {
                    Color {
                        rgb: Some(0xC2FF_0000),
                        ..Color::index(1)
                    }
                } else {
                    Color::index(256)
                },
                context: Some(if v >= Version::R2018 {
                    MTextContext {
                        app: Href::new(5, 0x12),
                        column_type: 2,
                        column_count: 2,
                        column_width: 40.0,
                        gutter: 5.0,
                        column_heights: vec![20.0, 12.0],
                        ..MTextContext::default()
                    }
                } else {
                    MTextContext::default()
                }),
                ..MText::default()
            })
        });
    }

    fn hatch(v: Version) -> Hatch {
        let (r2004, r2010) = (v >= Version::R2004, v >= Version::R2010);
        let spline = HatchSpline {
            degree: 3,
            rational: true,
            periodic: false,
            knots: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            ctrl: vec![[0.0, 0.0], [1.0, 2.0], [3.0, 2.0], [4.0, 0.0]],
            weights: vec![1.0, 0.5, 0.5, 1.0],
            fit: if r2010 {
                vec![[0.0, 0.0], [4.0, 0.0]]
            } else {
                vec![]
            },
            start_tangent: if r2010 { [1.0, 1.0] } else { [0.0; 2] },
            end_tangent: if r2010 { [1.0, -1.0] } else { [0.0; 2] },
        };
        let edges = vec![
            HatchEdge::Line {
                a: [4.0, 0.0],
                b: [4.0, -2.0],
            },
            HatchEdge::Arc {
                center: [2.0, -2.0],
                radius: 2.0,
                start: 0.0,
                end: std::f64::consts::PI,
                ccw: false,
            },
            HatchEdge::Ellipse {
                center: [0.0, -1.0],
                major: [0.0, 1.0],
                ratio: 0.5,
                start: std::f64::consts::PI,
                end: std::f64::consts::TAU,
                ccw: true,
            },
            HatchEdge::Spline(spline),
        ];
        let stop = |value, index, rgb: u32| GradientColor {
            value,
            index,
            rgb: rgb as i32,
            byte: 0,
        };
        Hatch {
            gradient: r2004 as i32,
            gradient_angle: if r2004 { 0.5 } else { 0.0 },
            gradient_colors: if r2004 {
                vec![stop(0.0, 5, 0xC300_00FF), stop(1.0, 2, 0xC3FF_FF00)]
            } else {
                vec![]
            },
            gradient_name: if r2004 { text("LINEAR", v) } else { Str::default() },
            name: text("ANSI31", v),
            associative: true,
            paths: vec![
                HatchPath {
                    flag: 1 | 4,
                    edges,
                    boundary: vec![Href::new(4, 0x60), Href::new(4, 0x61)],
                    ..HatchPath::default()
                },
                HatchPath {
                    flag: 2 | 16,
                    has_bulges: true,
                    closed: true,
                    vertices: vec![[1.0, -1.0, 0.0], [2.0, -1.0, 1.0], [2.0, -0.5, 0.0]],
                    ..HatchPath::default()
                },
                HatchPath {
                    flag: 2,
                    closed: true,
                    vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
                    boundary: vec![Href::new(4, 0x62)],
                    ..HatchPath::default()
                },
            ],
            style: 1,
            angle: 0.785,
            scale: 2.0,
            double: true,
            lines: vec![
                HatchLine {
                    angle: 0.785,
                    base: [0.0, 0.0],
                    offset: [-2.24, 2.24],
                    dashes: vec![],
                },
                HatchLine {
                    angle: 0.0,
                    base: [0.5, 0.0],
                    offset: [0.0, 3.0],
                    dashes: vec![3.0, -1.5, 0.0, -1.5],
                },
            ],
            pixel_size: 0.125,
            seeds: vec![[1.5, -0.5], [2.0, -3.0]],
            ..Hatch::default()
        }
    }

    #[test]
    fn hatch_read_back() {
        round_trip(78, |v| Data::Hatch(hatch(v)));
        // through its class too, as a solid fill: no pattern, no pixel size
        round_trip(502, |v| {
            let mut h = hatch(v);
            h.solid = true;
            h.name = text("SOLID", v);
            h.lines.clear();
            h.angle = 0.0;
            h.scale = 1.0;
            h.double = false;
            h.pixel_size = 0.0;
            for p in &mut h.paths {
                p.flag &= !4;
            }
            Data::Hatch(h)
        });
    }
}
