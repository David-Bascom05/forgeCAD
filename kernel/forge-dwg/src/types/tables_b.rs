//! Multileaders and tables: MULTILEADER and its annotation context, MLEADERSTYLE, ACAD_TABLE,
//! TABLECONTENT, TABLEGEOMETRY, TABLESTYLE, CELLSTYLEMAP, and the pieces they share (values,
//! custom data, content formats, cell styles). Chapter 20.4.48, 86-87, 96-103.

use super::Spec;
use crate::bits::R;
use crate::io::{Color, Href, Io};
use crate::str::Str;
use crate::version::Version;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MLeaderStyle {
    /// R2010+: 2.
    pub version: i16,
    pub content_type: i16,
    pub draw_mleader_order: i16,
    pub draw_leader_order: i16,
    pub max_points: i32,
    pub first_angle: f64,
    pub second_angle: f64,
    pub leader_type: i16,
    pub line_color: Color,
    pub line_type: Href,
    pub line_weight: i32,
    pub landing: bool,
    pub landing_gap: f64,
    pub dogleg: bool,
    pub landing_distance: f64,
    pub description: Str,
    pub arrow: Href,
    pub arrow_size: f64,
    pub text_default: Str,
    pub text_style: Href,
    pub left_attach: i16,
    pub right_attach: i16,
    pub text_angle_type: i16,
    pub text_align: i16,
    pub text_color: Color,
    pub text_height: f64,
    pub text_frame: bool,
    pub align_left: bool,
    pub align_space: f64,
    pub block: Href,
    pub block_color: Color,
    pub block_scale: [f64; 3],
    pub block_scale_on: bool,
    pub block_rotation: f64,
    pub block_rotation_on: bool,
    pub block_connection: i16,
    pub scale: f64,
    pub changed: bool,
    pub annotative: bool,
    pub break_size: f64,
    /// R2010+.
    pub attach_direction: i16,
    pub top_attach: i16,
    pub bottom_attach: i16,
}

impl Spec for MLeaderStyle {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        if v >= Version::R2010 {
            s.bs(&mut self.version)?;
        }
        s.bs(&mut self.content_type)?;
        s.bs(&mut self.draw_mleader_order)?;
        s.bs(&mut self.draw_leader_order)?;
        s.bl(&mut self.max_points)?;
        s.bd(&mut self.first_angle)?;
        s.bd(&mut self.second_angle)?;
        s.bs(&mut self.leader_type)?;
        s.cmc(&mut self.line_color)?;
        s.h(&mut self.line_type)?;
        s.bl(&mut self.line_weight)?;
        s.b(&mut self.landing)?;
        s.bd(&mut self.landing_gap)?;
        s.b(&mut self.dogleg)?;
        s.bd(&mut self.landing_distance)?;
        s.tv(&mut self.description)?;
        s.h(&mut self.arrow)?;
        s.bd(&mut self.arrow_size)?;
        s.tv(&mut self.text_default)?;
        s.h(&mut self.text_style)?;
        s.bs(&mut self.left_attach)?;
        s.bs(&mut self.right_attach)?;
        // The spec stores this and `align_left` only in the "new format" (R2010+, or an
        // ACAD_MLEADERVER EED entry); R2000 files from AutoCAD and ODA have both regardless.
        s.bs(&mut self.text_angle_type)?;
        s.bs(&mut self.text_align)?;
        s.cmc(&mut self.text_color)?;
        s.bd(&mut self.text_height)?;
        s.b(&mut self.text_frame)?;
        s.b(&mut self.align_left)?;
        s.bd(&mut self.align_space)?;
        s.h(&mut self.block)?;
        s.cmc(&mut self.block_color)?;
        s.bd3(&mut self.block_scale)?;
        s.b(&mut self.block_scale_on)?;
        s.bd(&mut self.block_rotation)?;
        s.b(&mut self.block_rotation_on)?;
        s.bs(&mut self.block_connection)?;
        s.bd(&mut self.scale)?;
        s.b(&mut self.changed)?;
        s.b(&mut self.annotative)?;
        s.bd(&mut self.break_size)?;
        if v >= Version::R2010 {
            s.bs(&mut self.attach_direction)?;
            s.bs(&mut self.top_attach)?;
            s.bs(&mut self.bottom_attach)?;
        }
        Ok(())
    }
}

/// TC: a colour as R2004+ CMC stores it, in every version (table objects only).
pub fn tc_io<S: Io>(s: &mut S, c: &mut Color) -> R<()> {
    s.bs(&mut c.index)?;
    let mut rgb = c.rgb.unwrap_or(0) as i32;
    s.bl(&mut rgb)?;
    c.rgb = Some(rgb as u32);
    let mut flags = c.name.is_some() as u8 | (c.book_name.is_some() as u8) << 1;
    s.rc(&mut flags)?;
    for (bit, x) in [(1, &mut c.name), (2, &mut c.book_name)] {
        if flags & bit != 0 {
            let mut t = x.take().unwrap_or_default();
            s.tv(&mut t)?;
            *x = Some(t);
        } else if S::READ {
            *x = None;
        }
    }
    Ok(())
}

/// A content format (20.4.101.3): how a cell's content shows, and which of that overrides.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ContentFormat {
    pub overrides: i32,
    /// Only the auto scale bit (0x100).
    pub flags: i32,
    pub data_type: i32,
    pub unit_type: i32,
    pub format: Str,
    pub rotation: f64,
    pub block_scale: f64,
    pub alignment: i32,
    pub color: Color,
    pub text_style: Href,
    pub text_height: f64,
}

pub fn content_format_io<S: Io>(s: &mut S, f: &mut ContentFormat) -> R<()> {
    s.bl(&mut f.overrides)?;
    s.bl(&mut f.flags)?;
    s.bl(&mut f.data_type)?;
    s.bl(&mut f.unit_type)?;
    s.tv(&mut f.format)?;
    s.bd(&mut f.rotation)?;
    s.bd(&mut f.block_scale)?;
    s.bl(&mut f.alignment)?;
    tc_io(s, &mut f.color)?;
    s.h(&mut f.text_style)?;
    s.bd(&mut f.text_height)
}

/// A cell border's overrides.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Border {
    /// 1 top, 2 right, 4 bottom, 8 left, 0x10 inside vertical, 0x20 inside horizontal.
    pub edges: i32,
    pub overrides: i32,
    /// 1 single, 2 double.
    pub kind: i32,
    pub color: Color,
    pub line_weight: i32,
    pub line_type: Href,
    pub invisible: i32,
    pub double_spacing: f64,
}

/// A cell style's own data, when present.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellStyleData {
    pub overrides: i32,
    pub merge_flags: i32,
    pub background: Color,
    /// 1 flow, 2 stacked horizontal, 4 stacked vertical.
    pub layout: i32,
    pub content: ContentFormat,
    /// Vertical, horizontal, bottom, right, horizontal spacing, vertical spacing.
    pub margins: Option<[f64; 6]>,
    pub borders: Vec<Border>,
}

/// A cell style (20.4.101.4): for a cell, row, column, the table, or a style's named cell style.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellStyle {
    /// 1 cell, 2 row, 3 column, 4 formatted table data, 5 table.
    pub kind: i32,
    pub data: Option<CellStyleData>,
}

pub fn cell_style_io<S: Io>(s: &mut S, c: &mut CellStyle) -> R<()> {
    s.bl(&mut c.kind)?;
    let mut present = c.data.is_some() as i16;
    s.bs(&mut present)?;
    if present == 0 {
        c.data = None;
        return Ok(());
    }
    let d = c.data.get_or_insert_with(CellStyleData::default);
    s.bl(&mut d.overrides)?;
    s.bl(&mut d.merge_flags)?;
    tc_io(s, &mut d.background)?;
    s.bl(&mut d.layout)?;
    content_format_io(s, &mut d.content)?;
    let mut mflags = d.margins.is_some() as i16;
    s.bs(&mut mflags)?;
    if mflags & 1 != 0 {
        let m = d.margins.get_or_insert([0.0; 6]);
        for x in m {
            s.bd(x)?;
        }
    } else {
        d.margins = None;
    }
    s.list(&mut d.borders, |s, b| {
        s.bl(&mut b.edges)?;
        if b.edges != 0 {
            s.bl(&mut b.overrides)?;
            s.bl(&mut b.kind)?;
            tc_io(s, &mut b.color)?;
            s.bl(&mut b.line_weight)?;
            s.h(&mut b.line_type)?;
            s.bl(&mut b.invisible)?;
            s.bd(&mut b.double_spacing)?;
        }
        Ok(())
    })
}

/// A named cell style: the style, its id (1 title, 2 header, 3 data, 4 table, 101.. custom),
/// class (1 data, 2 label) and name.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NamedCellStyle {
    pub style: CellStyle,
    pub id: i32,
    pub class: i32,
    pub name: Str,
}

fn named_cell_style_io<S: Io>(s: &mut S, c: &mut NamedCellStyle) -> R<()> {
    cell_style_io(s, &mut c.style)?;
    s.bl(&mut c.id)?;
    s.bl(&mut c.class)?;
    s.tv(&mut c.name)
}

/// The cell styles of a table style (20.4.102).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellStyleMap {
    pub styles: Vec<NamedCellStyle>,
}

impl Spec for CellStyleMap {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.list(&mut self.styles, named_cell_style_io)
    }
}

/// A table border as TABLESTYLE until R21 has it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RowBorder {
    pub line_weight: i16,
    pub visible: bool,
    pub color: Color,
}

/// A row style of TABLESTYLE until R21: data, title or header.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RowStyle {
    pub text_style: Href,
    pub text_height: f64,
    pub alignment: i16,
    pub text_color: Color,
    pub fill_color: Color,
    pub fill: bool,
    /// Top, horizontal inside, bottom, left, vertical inside, right.
    pub borders: [RowBorder; 6],
    /// R2007+.
    pub data_type: i32,
    pub unit_type: i32,
    pub format: Str,
}

/// R2010+ (R24): the style's cell styles, kept with it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableStyleCells {
    pub unknown1: u8,
    pub unknown2: i32,
    pub unknown3: i32,
    pub unknown_h: Href,
    /// The "Table" cell style.
    pub table: NamedCellStyle,
    /// Data, title and header.
    pub styles: Vec<NamedCellStyle>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableStyle {
    pub description: Str,
    /// 0 down, 1 up.
    pub flow: i16,
    pub flags: i16,
    pub margin_h: f64,
    pub margin_v: f64,
    pub suppress_title: bool,
    pub suppress_header: bool,
    /// Data, title, header.
    pub rows: Box<[RowStyle; 3]>,
    /// R2010+: the format since, in place of all the above but the description.
    pub cells: Box<TableStyleCells>,
}

impl Spec for TableStyle {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        if v >= Version::R2010 {
            let c = &mut self.cells;
            s.rc(&mut c.unknown1)?;
            s.tv(&mut self.description)?;
            s.bl(&mut c.unknown2)?;
            s.bl(&mut c.unknown3)?;
            s.h(&mut c.unknown_h)?;
            named_cell_style_io(s, &mut c.table)?;
            return s.list(&mut c.styles, named_cell_style_io);
        }
        s.tv(&mut self.description)?;
        s.bs(&mut self.flow)?;
        s.bs(&mut self.flags)?;
        s.bd(&mut self.margin_h)?;
        s.bd(&mut self.margin_v)?;
        s.b(&mut self.suppress_title)?;
        s.b(&mut self.suppress_header)?;
        // The spec has CMC for the colours; they are TC (BS, BL, RC) even in R2000.
        for r in &mut self.rows {
            s.h(&mut r.text_style)?;
            s.bd(&mut r.text_height)?;
            s.bs(&mut r.alignment)?;
            tc_io(s, &mut r.text_color)?;
            tc_io(s, &mut r.fill_color)?;
            s.b(&mut r.fill)?;
            for b in &mut r.borders {
                s.bs(&mut b.line_weight)?;
                s.b(&mut b.visible)?;
                tc_io(s, &mut b.color)?;
            }
            if v >= Version::R2007 {
                s.bl(&mut r.data_type)?;
                s.bl(&mut r.unit_type)?;
                s.tv(&mut r.format)?;
            }
        }
        Ok(())
    }
}

/// Where a cell content sits in its cell (20.4.98).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ContentGeometry {
    pub to_top_left: [f64; 3],
    pub to_center: [f64; 3],
    pub content_width: f64,
    pub content_height: f64,
    pub width: f64,
    pub height: f64,
    pub flags: i32,
}

pub fn content_geometry_io<S: Io>(s: &mut S, g: &mut ContentGeometry) -> R<()> {
    s.bd3(&mut g.to_top_left)?;
    s.bd3(&mut g.to_center)?;
    s.bd(&mut g.content_width)?;
    s.bd(&mut g.content_height)?;
    s.bd(&mut g.width)?;
    s.bd(&mut g.height)?;
    s.bl(&mut g.flags)
}

/// A cell's geometry in TABLEGEOMETRY.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellGeometry {
    pub flags: i32,
    pub width: f64,
    pub height: f64,
    pub unknown: Href,
    pub contents: Vec<ContentGeometry>,
}

/// A table's geometry (20.4.103): its cells row by row.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableGeometry {
    pub rows: i32,
    pub columns: i32,
    pub cells: Vec<CellGeometry>,
}

impl Spec for TableGeometry {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        s.bl(&mut self.rows)?;
        s.bl(&mut self.columns)?;
        s.list(&mut self.cells, |s, c| {
            s.bl(&mut c.flags)?;
            s.bd(&mut c.width)?;
            s.bd(&mut c.height)?;
            s.h(&mut c.unknown)?;
            // The spec ends each content with a BD here, 20.4.98 with a BL; the files have 0,
            // the same two bits either way.
            s.list(&mut c.contents, content_geometry_io)
        })
    }
}

/// A table value (20.4.99): a number, text, date, point or object, with its format.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Value {
    /// R2007+: 1 the value is kGeneral (and no data is stored).
    pub flags: i32,
    /// 0 unknown, 1 long, 2 double, 4 string, 8 date, 16 point, 32 3D point, 64 object,
    /// 512 general.
    pub data_type: i32,
    /// Types 0 and 1.
    pub long: i32,
    pub double: f64,
    pub text: Str,
    /// Types 8, 16, 32, 512: the bytes as stored (an Int64 date, 2 or 3 RD, a general buffer).
    pub bytes: Vec<u8>,
    pub object: Href,
    /// R2007+: 0 none, 1 distance, 2 angle, 4 area, 8 volume.
    pub unit_type: i32,
    pub format: Str,
    pub value_text: Str,
}

pub fn value_io<S: Io>(s: &mut S, x: &mut Value) -> R<()> {
    let v = s.ver();
    if v >= Version::R2007 {
        s.bl(&mut x.flags)?;
    }
    s.bl(&mut x.data_type)?;
    if v < Version::R2007 || x.flags & 1 == 0 {
        match x.data_type {
            0 | 1 => s.bl(&mut x.long)?,
            2 => s.bd(&mut x.double)?,
            4 => s.tv(&mut x.text)?,
            64 => s.h(&mut x.object)?,
            8 | 16 | 32 | 512 => {
                let mut n = x.bytes.len() as i32;
                s.bl(&mut n)?;
                s.bytes(&mut x.bytes, n.max(0) as usize)?;
            }
            t => return crate::bits::bad(format!("value type {t}")),
        }
    }
    if v >= Version::R2007 {
        s.bl(&mut x.unit_type)?;
        s.tv(&mut x.format)?;
        s.tv(&mut x.value_text)?;
    }
    Ok(())
}

/// A custom data item of a cell, row or column (20.4.100).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CustomItem {
    pub name: Str,
    pub value: Value,
}

/// Custom data: a 32-bit value and the key/value items.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Custom {
    pub data: i32,
    pub items: Vec<CustomItem>,
}

fn custom_io<S: Io>(s: &mut S, c: &mut Custom) -> R<()> {
    s.bl(&mut c.data)?;
    s.list(&mut c.items, |s, it| {
        s.tv(&mut it.name)?;
        value_io(s, &mut it.value)
    })
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Column {
    pub name: Str,
    pub custom: Custom,
    pub style: CellStyle,
    /// The base cell style in the table style; 0 none.
    pub style_id: i32,
    pub width: f64,
}

/// A cell content's attribute value (a block content's).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellAttr {
    pub attdef: Href,
    pub value: Str,
    /// From 1.
    pub index: i32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellContent {
    /// 0 unknown, 1 value, 2 field, 4 block.
    pub kind: i32,
    pub value: Value,
    /// The field or block record.
    pub object: Href,
    pub attrs: Vec<CellAttr>,
    /// Non-zero: `format` is stored.
    pub has_format: i16,
    pub format: ContentFormat,
}

/// A cell's linked data.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellLink {
    pub data_link: Href,
    pub rows: i32,
    pub columns: i32,
    pub unknown: i32,
}

/// The cell's further data, when its flag is set.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellExtra {
    pub flag: i32,
    pub unknown: i32,
    pub d40: f64,
    pub d41: f64,
    pub geometry_flags: i32,
    pub unknown_h: Href,
    pub geometry: ContentGeometry,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Cell {
    /// 1 content locked, 2 content read-only, 4 linked, 8 content modified, 0x10 format locked,
    /// 0x20 format read-only, 0x40 format modified.
    pub state: i32,
    pub tooltip: Str,
    pub custom: Custom,
    pub link: Option<CellLink>,
    pub contents: Vec<CellContent>,
    pub style: CellStyle,
    pub style_id: i32,
    pub extra: Option<CellExtra>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub custom: Custom,
    pub style: CellStyle,
    pub style_id: i32,
    pub height: f64,
}

fn cell_io<S: Io>(s: &mut S, c: &mut Cell) -> R<()> {
    s.bl(&mut c.state)?;
    s.tv(&mut c.tooltip)?;
    custom_io(s, &mut c.custom)?;
    let mut linked = c.link.is_some() as i32;
    s.bl(&mut linked)?;
    if linked != 0 {
        let l = c.link.get_or_insert_with(CellLink::default);
        s.h(&mut l.data_link)?;
        s.bl(&mut l.rows)?;
        s.bl(&mut l.columns)?;
        s.bl(&mut l.unknown)?;
    } else {
        c.link = None;
    }
    s.list(&mut c.contents, |s, k| {
        s.bl(&mut k.kind)?;
        match k.kind {
            1 => value_io(s, &mut k.value)?,
            2 | 4 => s.h(&mut k.object)?,
            _ => {}
        }
        s.list(&mut k.attrs, |s, a| {
            s.h(&mut a.attdef)?;
            s.tv(&mut a.value)?;
            s.bl(&mut a.index)
        })?;
        s.bs(&mut k.has_format)?;
        if k.has_format != 0 {
            content_format_io(s, &mut k.format)?;
        }
        Ok(())
    })?;
    cell_style_io(s, &mut c.style)?;
    s.bl(&mut c.style_id)?;
    let mut flag = c.extra.as_ref().map_or(0, |e| e.flag);
    s.bl(&mut flag)?;
    if flag != 0 {
        let e = c.extra.get_or_insert_with(CellExtra::default);
        e.flag = flag;
        s.bl(&mut e.unknown)?;
        s.bd(&mut e.d40)?;
        s.bd(&mut e.d41)?;
        s.bl(&mut e.geometry_flags)?;
        s.h(&mut e.unknown_h)?;
        if e.geometry_flags != 0 {
            content_geometry_io(s, &mut e.geometry)?;
        }
    } else {
        c.extra = None;
    }
    Ok(())
}

/// A table's content (20.4.97): linked data, linked table data (columns, rows, cells), formatted
/// table data (the table's cell style, merged ranges), and the table style.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableContent {
    pub name: Str,
    pub description: Str,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// The fields cells refer to (hard owner).
    pub fields: Vec<Href>,
    pub style: CellStyle,
    /// Top row, left column, bottom row, right column.
    pub merged: Vec<[i32; 4]>,
    pub table_style: Href,
}

pub fn table_content_io<S: Io>(s: &mut S, t: &mut TableContent) -> R<()> {
    s.tv(&mut t.name)?;
    s.tv(&mut t.description)?;
    s.list(&mut t.columns, |s, c| {
        s.tv(&mut c.name)?;
        custom_io(s, &mut c.custom)?;
        cell_style_io(s, &mut c.style)?;
        s.bl(&mut c.style_id)?;
        s.bd(&mut c.width)
    })?;
    s.list(&mut t.rows, |s, r| {
        s.list(&mut r.cells, cell_io)?;
        custom_io(s, &mut r.custom)?;
        cell_style_io(s, &mut r.style)?;
        s.bl(&mut r.style_id)?;
        s.bd(&mut r.height)
    })?;
    s.list(&mut t.fields, |s, h| s.h(h))?;
    cell_style_io(s, &mut t.style)?;
    s.list(&mut t.merged, |s, m| {
        for x in m {
            s.bl(x)?;
        }
        Ok(())
    })?;
    s.h(&mut t.table_style)
}

impl Spec for TableContent {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        table_content_io(s, self)
    }
}

/// A leader line's breaks on one segment: start and end points.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LineBreak {
    pub segment: i32,
    pub pairs: Vec<[[f64; 3]; 2]>,
}

/// A leader line of a multileader.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LeaderLine {
    pub points: Vec<[f64; 3]>,
    pub breaks: Vec<LineBreak>,
    pub index: i32,
    /// R2010+: 0 invisible, 1 straight, 2 spline.
    pub leader_type: i16,
    pub color: Color,
    pub line_type: Href,
    pub line_weight: i32,
    pub arrow_size: f64,
    pub arrow: Href,
    /// 1 type, 2 colour, 4 line type, 8 weight, 16 arrow size, 32 arrow.
    pub overrides: i32,
}

/// A leader root: one or more leader lines attached at one side of the content.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LeaderRoot {
    pub content_valid: bool,
    pub unknown: bool,
    pub connection: [f64; 3],
    pub direction: [f64; 3],
    pub breaks: Vec<[[f64; 3]; 2]>,
    pub index: i32,
    pub landing_distance: f64,
    pub lines: Vec<LeaderLine>,
    /// R2010+: 0 horizontal, 1 vertical.
    pub attach_direction: i16,
}

/// A multileader's text content.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MLeaderText {
    pub label: Str,
    pub normal: [f64; 3],
    pub style: Href,
    pub location: [f64; 3],
    pub direction: [f64; 3],
    pub rotation: f64,
    pub width: f64,
    pub height: f64,
    pub line_spacing: f64,
    /// 1 at least, 2 exactly.
    pub line_spacing_style: i16,
    pub color: Color,
    /// 1 left, 2 center, 3 right.
    pub alignment: i16,
    /// 1 horizontal, 3 vertical, 6 by style.
    pub flow: i16,
    pub bg_color: Color,
    pub bg_scale: f64,
    pub bg_transparency: i32,
    pub bg_fill: bool,
    pub bg_mask: bool,
    pub column_type: i16,
    pub auto_height: bool,
    pub column_width: f64,
    pub column_gutter: f64,
    pub column_flow_reversed: bool,
    pub column_sizes: Vec<f64>,
    pub word_break: bool,
    pub unknown: bool,
}

/// A multileader's block content.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MLeaderBlock {
    pub block: Href,
    pub normal: [f64; 3],
    pub location: [f64; 3],
    pub scale: [f64; 3],
    pub rotation: f64,
    pub color: Color,
    /// The block's transform, row by row.
    pub matrix: [f64; 16],
}

/// A multileader's content and leaders (MLeaderAnnotContext, 20.4.86), as MULTILEADER embeds it:
/// without the context data and scale handle the spec lists first (R2000 has none of them).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MLeaderContext {
    pub roots: Vec<LeaderRoot>,
    pub scale: f64,
    pub base: [f64; 3],
    pub text_height: f64,
    pub arrow_size: f64,
    pub landing_gap: f64,
    pub left_attach: i16,
    pub right_attach: i16,
    /// 0 left, 1 center, 2 right.
    pub text_align: i16,
    /// 0 content extents, 1 insertion point.
    pub attach_type: i16,
    pub text: Option<Box<MLeaderText>>,
    pub block: Option<Box<MLeaderBlock>>,
    pub base_point: [f64; 3],
    pub base_direction: [f64; 3],
    pub base_vertical: [f64; 3],
    pub normal_reversed: bool,
    /// R2010+.
    pub top_attach: i16,
    pub bottom_attach: i16,
}

fn pairs_io<S: Io>(s: &mut S, v: &mut Vec<[[f64; 3]; 2]>) -> R<()> {
    s.list(v, |s, p| {
        s.bd3(&mut p[0])?;
        s.bd3(&mut p[1])
    })
}

fn leader_line_io<S: Io>(s: &mut S, l: &mut LeaderLine) -> R<()> {
    s.list(&mut l.points, |s, p| s.bd3(p))?;
    // The spec lists the break count, segment index and pairs flat; the file has the count, then
    // per break its segment index and pairs (a single line with no breaks is BL 0, BL index).
    s.list(&mut l.breaks, |s, b| {
        s.bl(&mut b.segment)?;
        pairs_io(s, &mut b.pairs)
    })?;
    s.bl(&mut l.index)?;
    if s.ver() >= Version::R2010 {
        s.bs(&mut l.leader_type)?;
        s.cmc(&mut l.color)?;
        s.h(&mut l.line_type)?;
        s.bl(&mut l.line_weight)?;
        s.bd(&mut l.arrow_size)?;
        s.h(&mut l.arrow)?;
        s.bl(&mut l.overrides)?;
    }
    Ok(())
}

fn leader_root_io<S: Io>(s: &mut S, r: &mut LeaderRoot) -> R<()> {
    s.b(&mut r.content_valid)?;
    s.b(&mut r.unknown)?;
    s.bd3(&mut r.connection)?;
    s.bd3(&mut r.direction)?;
    pairs_io(s, &mut r.breaks)?;
    s.bl(&mut r.index)?;
    s.bd(&mut r.landing_distance)?;
    s.list(&mut r.lines, leader_line_io)?;
    if s.ver() >= Version::R2010 {
        s.bs(&mut r.attach_direction)?;
    }
    Ok(())
}

fn mtext_io<S: Io>(s: &mut S, t: &mut MLeaderText) -> R<()> {
    s.tv(&mut t.label)?;
    s.bd3(&mut t.normal)?;
    s.h(&mut t.style)?;
    s.bd3(&mut t.location)?;
    s.bd3(&mut t.direction)?;
    s.bd(&mut t.rotation)?;
    s.bd(&mut t.width)?;
    s.bd(&mut t.height)?;
    s.bd(&mut t.line_spacing)?;
    s.bs(&mut t.line_spacing_style)?;
    s.cmc(&mut t.color)?;
    s.bs(&mut t.alignment)?;
    s.bs(&mut t.flow)?;
    s.cmc(&mut t.bg_color)?;
    s.bd(&mut t.bg_scale)?;
    s.bl(&mut t.bg_transparency)?;
    s.b(&mut t.bg_fill)?;
    s.b(&mut t.bg_mask)?;
    s.bs(&mut t.column_type)?;
    s.b(&mut t.auto_height)?;
    s.bd(&mut t.column_width)?;
    s.bd(&mut t.column_gutter)?;
    s.b(&mut t.column_flow_reversed)?;
    s.list(&mut t.column_sizes, |s, x| s.bd(x))?;
    s.b(&mut t.word_break)?;
    s.b(&mut t.unknown)
}

fn mblock_io<S: Io>(s: &mut S, b: &mut MLeaderBlock) -> R<()> {
    s.h(&mut b.block)?;
    s.bd3(&mut b.normal)?;
    s.bd3(&mut b.location)?;
    s.bd3(&mut b.scale)?;
    s.bd(&mut b.rotation)?;
    s.cmc(&mut b.color)?;
    for x in &mut b.matrix {
        s.bd(x)?;
    }
    Ok(())
}

pub fn mleader_context_io<S: Io>(s: &mut S, c: &mut MLeaderContext) -> R<()> {
    s.list(&mut c.roots, leader_root_io)?;
    s.bd(&mut c.scale)?;
    s.bd3(&mut c.base)?;
    s.bd(&mut c.text_height)?;
    s.bd(&mut c.arrow_size)?;
    s.bd(&mut c.landing_gap)?;
    s.bs(&mut c.left_attach)?;
    s.bs(&mut c.right_attach)?;
    s.bs(&mut c.text_align)?;
    s.bs(&mut c.attach_type)?;
    let mut has_text = c.text.is_some();
    s.b(&mut has_text)?;
    if has_text {
        mtext_io(s, c.text.get_or_insert_with(Box::default))?;
        c.block = None;
    } else {
        c.text = None;
        let mut has_block = c.block.is_some();
        s.b(&mut has_block)?;
        if has_block {
            mblock_io(s, c.block.get_or_insert_with(Box::default))?;
        } else {
            c.block = None;
        }
    }
    s.bd3(&mut c.base_point)?;
    s.bd3(&mut c.base_direction)?;
    s.bd3(&mut c.base_vertical)?;
    s.b(&mut c.normal_reversed)?;
    if s.ver() >= Version::R2010 {
        s.bs(&mut c.top_attach)?;
        s.bs(&mut c.bottom_attach)?;
    }
    Ok(())
}

/// A block attribute's label in a multileader.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BlockLabel {
    pub attdef: Href,
    pub text: Str,
    pub ui_index: i16,
    pub width: f64,
}

/// MULTILEADER (20.4.48).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MLeader {
    /// R2010+: 2.
    pub version: i16,
    pub context: MLeaderContext,
    pub style: Href,
    /// Which of the following override the style.
    pub overrides: i32,
    pub leader_type: i16,
    pub line_color: Color,
    pub line_type: Href,
    pub line_weight: i32,
    pub landing: bool,
    pub dogleg: bool,
    pub landing_distance: f64,
    pub arrow: Href,
    pub arrow_size: f64,
    pub content_type: i16,
    pub text_style: Href,
    pub left_attach: i16,
    pub right_attach: i16,
    pub text_angle_type: i16,
    pub unknown: i16,
    pub text_color: Color,
    pub text_frame: bool,
    pub block: Href,
    pub block_color: Color,
    pub block_scale: [f64; 3],
    pub block_rotation: f64,
    pub attach_type: i16,
    pub annotative: bool,
    /// (is default, arrow head block).
    pub arrows: Vec<(bool, Href)>,
    pub labels: Vec<BlockLabel>,
    pub text_dir_negative: bool,
    pub ipe_align: i16,
    /// 1 left, 2 center, 3 right.
    pub justification: i16,
    pub scale: f64,
    /// R2010+.
    pub attach_direction: i16,
    pub top_attach: i16,
    pub bottom_attach: i16,
    /// R2013+.
    pub extend_to_text: bool,
}

impl Spec for MLeader {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        if v >= Version::R2010 {
            s.bs(&mut self.version)?;
        }
        mleader_context_io(s, &mut self.context)?;
        s.h(&mut self.style)?;
        s.bl(&mut self.overrides)?;
        s.bs(&mut self.leader_type)?;
        s.cmc(&mut self.line_color)?;
        s.h(&mut self.line_type)?;
        s.bl(&mut self.line_weight)?;
        s.b(&mut self.landing)?;
        s.b(&mut self.dogleg)?;
        s.bd(&mut self.landing_distance)?;
        s.h(&mut self.arrow)?;
        s.bd(&mut self.arrow_size)?;
        s.bs(&mut self.content_type)?;
        s.h(&mut self.text_style)?;
        s.bs(&mut self.left_attach)?;
        s.bs(&mut self.right_attach)?;
        s.bs(&mut self.text_angle_type)?;
        s.bs(&mut self.unknown)?;
        s.cmc(&mut self.text_color)?;
        s.b(&mut self.text_frame)?;
        s.h(&mut self.block)?;
        s.cmc(&mut self.block_color)?;
        s.bd3(&mut self.block_scale)?;
        s.bd(&mut self.block_rotation)?;
        s.bs(&mut self.attach_type)?;
        s.b(&mut self.annotative)?;
        // The spec marks these "-R2007"; R2000 files have them.
        s.list(&mut self.arrows, |s, a| {
            s.b(&mut a.0)?;
            s.h(&mut a.1)
        })?;
        s.list(&mut self.labels, |s, l| {
            s.h(&mut l.attdef)?;
            s.tv(&mut l.text)?;
            s.bs(&mut l.ui_index)?;
            s.bd(&mut l.width)
        })?;
        s.b(&mut self.text_dir_negative)?;
        s.bs(&mut self.ipe_align)?;
        s.bs(&mut self.justification)?;
        s.bd(&mut self.scale)?;
        if v >= Version::R2010 {
            s.bs(&mut self.attach_direction)?;
            s.bs(&mut self.top_attach)?;
            s.bs(&mut self.bottom_attach)?;
        }
        if v >= Version::R2013 {
            s.b(&mut self.extend_to_text)?;
        }
        Ok(())
    }
}

/// A cell's property overrides in ACAD_TABLE until R21.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellOverrides {
    /// Which of the following are stored.
    pub flags: i32,
    /// The edges whose overrides the adjacent cell holds: 1 top, 2 right, 4 bottom, 8 left.
    pub virtual_edges: u8,
    pub alignment: i16,
    pub bg_fill_none: bool,
    pub bg_color: Color,
    pub content_color: Color,
    pub text_style: Href,
    pub text_height: f64,
    /// Top, right, bottom, left.
    pub edge_color: [Color; 4],
    pub edge_weight: [i16; 4],
    pub edge_visible: [i16; 4],
}

/// A block cell's attribute value.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableAttr {
    pub index: i16,
    pub text: Str,
    pub attdef: Href,
}

/// A cell of ACAD_TABLE until R21.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableCell {
    /// 1 text, 2 block.
    pub kind: i16,
    /// The edges with overrides: 1 top, 2 right, 4 bottom, 8 left.
    pub edges: u8,
    pub merged: bool,
    pub autofit: bool,
    pub merged_width: i32,
    pub merged_height: i32,
    pub rotation: f64,
    pub text: Str,
    pub block_scale: f64,
    pub has_attrs: bool,
    pub attrs: Vec<TableAttr>,
    /// The text (344) or the block record (340).
    pub object: Href,
    pub overrides: Option<CellOverrides>,
    /// R2007+.
    pub unknown: i32,
    pub value: Value,
}

/// The whole table's overrides of its style, until R21.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableOverrides {
    pub flags: i32,
    pub title_suppressed: bool,
    /// 0 down, 1 up.
    pub flow: i16,
    pub margin_h: f64,
    pub margin_v: f64,
    /// Title, header, data.
    pub row_color: [Color; 3],
    pub fill_none: [bool; 3],
    pub fill_color: [Color; 3],
    pub alignment: [i16; 3],
    pub text_style: [Href; 3],
    pub row_height: [f64; 3],
}

/// Border overrides, until R21: the flags, then a value per border (title, header, data rows;
/// horizontal top, inside, bottom; vertical left, inside, right) whose bit is set.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BorderOverrides<T> {
    pub flags: i32,
    pub values: [T; 18],
}

/// R2010+: a table break's manual position.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BreakHeight {
    pub position: [f64; 3],
    pub height: f64,
    pub flags: i32,
}

/// R2010+: how a table breaks into parts.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableBreaks {
    pub options: i32,
    /// 1 right, 2 vertical, 4 left.
    pub flow: i32,
    pub spacing: f64,
    pub unknown1: i32,
    pub unknown2: i32,
    pub heights: Vec<BreakHeight>,
}

/// R2010+: a range of rows in one part.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RowRange {
    pub position: [f64; 3],
    pub start: i32,
    pub end: i32,
}

/// R2010+ ACAD_TABLE: the content inline, and the breaks.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TableR24 {
    pub unknown_rc: u8,
    pub unknown_h: Href,
    pub unknown_bl: i32,
    /// R2010 only.
    pub unknown_b: bool,
    /// R2013+.
    pub unknown_bl2: i32,
    pub content: TableContent,
    pub unknown_bs: i16,
    pub direction: [f64; 3],
    pub breaks: Option<TableBreaks>,
    pub ranges: Vec<RowRange>,
}

/// ACAD_TABLE (20.4.96): an INSERT of the table's anonymous block with the cells, until R21; the
/// table content inline since (`r24`).
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub insert: [f64; 3],
    pub scale: [f64; 3],
    pub rotation: f64,
    pub extrusion: [f64; 3],
    pub has_attribs: bool,
    /// 2|4 has block, 0x10 direction down, 0x20 title suppressed.
    pub flags: i16,
    pub direction: [f64; 3],
    pub column_widths: Vec<f64>,
    pub row_heights: Vec<f64>,
    /// Row by row.
    pub cells: Vec<TableCell>,
    pub overrides: Option<Box<TableOverrides>>,
    pub border_colors: Option<Box<BorderOverrides<Color>>>,
    pub border_weights: Option<Box<BorderOverrides<i16>>>,
    pub border_visible: Option<Box<BorderOverrides<i16>>>,
    pub block: Href,
    /// R2000 and before: the first and last attribute; R2004+ all of them.
    pub attribs: Vec<Href>,
    pub seqend: Href,
    pub style: Href,
    pub r24: Box<TableR24>,
}

impl Default for Table {
    fn default() -> Table {
        Table {
            insert: [0.0; 3],
            scale: [1.0; 3],
            rotation: 0.0,
            extrusion: [0.0, 0.0, 1.0],
            has_attribs: false,
            flags: 6,
            direction: [1.0, 0.0, 0.0],
            column_widths: vec![],
            row_heights: vec![],
            cells: vec![],
            overrides: None,
            border_colors: None,
            border_weights: None,
            border_visible: None,
            block: Href::NULL_HARD,
            attribs: vec![],
            seqend: Href::NULL_HARD,
            style: Href::NULL_HARD,
            r24: Box::default(),
        }
    }
}

/// An INSERT's scale: the BB saying which of x, y, z are stored, then those.
fn scale_io<S: Io>(s: &mut S, v: &mut [f64; 3]) -> R<()> {
    let mut f = match *v {
        [1.0, 1.0, 1.0] => 3,
        [1.0, _, _] => 1,
        [x, y, z] if y == x && z == x => 2,
        _ => 0,
    };
    s.bb(&mut f)?;
    match f {
        3 => *v = [1.0; 3],
        1 => {
            v[0] = 1.0;
            s.dd(&mut v[1], 1.0)?;
            s.dd(&mut v[2], 1.0)?;
        }
        2 => {
            s.rd(&mut v[0])?;
            v[1] = v[0];
            v[2] = v[0];
        }
        _ => {
            s.rd(&mut v[0])?;
            let x = v[0];
            s.dd(&mut v[1], x)?;
            s.dd(&mut v[2], x)?;
        }
    }
    Ok(())
}

/// One stored value per set bit of `flags` (from `bit0` up), walked by `f`.
fn masked<S: Io, T>(
    s: &mut S,
    flags: i32,
    bit0: u32,
    v: &mut [T],
    mut f: impl FnMut(&mut S, &mut T) -> R<()>,
) -> R<()> {
    for (k, x) in v.iter_mut().enumerate() {
        if flags & (1 << (bit0 + k as u32)) != 0 {
            f(s, x)?;
        }
    }
    Ok(())
}

fn border_overrides_io<S: Io, T: Default + Clone>(
    s: &mut S,
    o: &mut Option<Box<BorderOverrides<T>>>,
    f: impl FnMut(&mut S, &mut T) -> R<()>,
) -> R<()> {
    let mut has = o.is_some();
    s.b(&mut has)?;
    if !has {
        *o = None;
        return Ok(());
    }
    let b = o.get_or_insert_with(|| {
        Box::new(BorderOverrides {
            flags: 0,
            values: std::array::from_fn(|_| T::default()),
        })
    });
    s.bl(&mut b.flags)?;
    masked(s, b.flags, 0, &mut b.values, f)
}

fn cell_overrides_io<S: Io>(s: &mut S, o: &mut CellOverrides) -> R<()> {
    s.bl(&mut o.flags)?;
    s.rc(&mut o.virtual_edges)?;
    let f = o.flags;
    if f & 0x01 != 0 {
        s.rs(&mut o.alignment)?;
    }
    if f & 0x02 != 0 {
        s.b(&mut o.bg_fill_none)?;
    }
    if f & 0x04 != 0 {
        s.cmc(&mut o.bg_color)?;
    }
    if f & 0x08 != 0 {
        s.cmc(&mut o.content_color)?;
    }
    // The spec's handle list says 0x08 for the text style; 0x08 is the content colour's.
    if f & 0x10 != 0 {
        s.h(&mut o.text_style)?;
    }
    if f & 0x20 != 0 {
        s.bd(&mut o.text_height)?;
    }
    for k in 0..4 {
        if f & (0x40 << k) != 0 {
            s.cmc(&mut o.edge_color[k])?;
        }
        if f & (0x400 << k) != 0 {
            s.bs(&mut o.edge_weight[k])?;
        }
        if f & (0x4000 << k) != 0 {
            s.bs(&mut o.edge_visible[k])?;
        }
    }
    Ok(())
}

fn table_cell_io<S: Io>(s: &mut S, c: &mut TableCell) -> R<()> {
    s.bs(&mut c.kind)?;
    s.rc(&mut c.edges)?;
    s.b(&mut c.merged)?;
    s.b(&mut c.autofit)?;
    s.bl(&mut c.merged_width)?;
    s.bl(&mut c.merged_height)?;
    s.bd(&mut c.rotation)?;
    s.h(&mut c.object)?;
    match c.kind {
        // the text only when the cell has no 344 object
        1 if c.object.is_null() => s.tv(&mut c.text)?,
        2 => {
            s.bd(&mut c.block_scale)?;
            s.b(&mut c.has_attrs)?;
            if c.has_attrs {
                let mut n = c.attrs.len() as i16;
                s.bs(&mut n)?;
                s.items(&mut c.attrs, n as i32, &mut |s, a| {
                    s.h(&mut a.attdef)?;
                    s.bs(&mut a.index)?;
                    s.tv(&mut a.text)
                })?;
            }
        }
        _ => {}
    }
    let mut has = c.overrides.is_some();
    s.b(&mut has)?;
    if has {
        cell_overrides_io(s, c.overrides.get_or_insert_with(CellOverrides::default))?;
    } else {
        c.overrides = None;
    }
    if s.ver() >= Version::R2007 {
        s.bl(&mut c.unknown)?;
        value_io(s, &mut c.value)?;
    }
    Ok(())
}

fn table_overrides_io<S: Io>(s: &mut S, t: &mut TableOverrides) -> R<()> {
    s.bl(&mut t.flags)?;
    let f = t.flags;
    if f & 0x01 != 0 {
        s.b(&mut t.title_suppressed)?;
    }
    if f & 0x04 != 0 {
        s.bs(&mut t.flow)?;
    }
    if f & 0x08 != 0 {
        s.bd(&mut t.margin_h)?;
    }
    if f & 0x10 != 0 {
        s.bd(&mut t.margin_v)?;
    }
    masked(s, f, 5, &mut t.row_color, |s, c| s.cmc(c))?;
    masked(s, f, 8, &mut t.fill_none, |s, b| s.b(b))?;
    masked(s, f, 11, &mut t.fill_color, |s, c| s.cmc(c))?;
    masked(s, f, 14, &mut t.alignment, |s, a| s.bs(a))?;
    masked(s, f, 17, &mut t.text_style, |s, h| s.h(h))?;
    masked(s, f, 20, &mut t.row_height, |s, x| s.bd(x))
}

fn table_r24_io<S: Io>(s: &mut S, t: &mut TableR24) -> R<()> {
    let v = s.ver();
    s.rc(&mut t.unknown_rc)?;
    s.h(&mut t.unknown_h)?;
    s.bl(&mut t.unknown_bl)?;
    if v == Version::R2010 {
        s.b(&mut t.unknown_b)?;
    } else {
        s.bl(&mut t.unknown_bl2)?;
    }
    table_content_io(s, &mut t.content)?;
    s.bs(&mut t.unknown_bs)?;
    s.bd3(&mut t.direction)?;
    let mut has = t.breaks.is_some() as i32;
    s.bl(&mut has)?;
    if has != 0 {
        let b = t.breaks.get_or_insert_with(TableBreaks::default);
        s.bl(&mut b.options)?;
        s.bl(&mut b.flow)?;
        s.bd(&mut b.spacing)?;
        s.bl(&mut b.unknown1)?;
        s.bl(&mut b.unknown2)?;
        s.list(&mut b.heights, |s, h| {
            s.bd3(&mut h.position)?;
            s.bd(&mut h.height)?;
            s.bl(&mut h.flags)
        })?;
    } else {
        t.breaks = None;
    }
    s.list(&mut t.ranges, |s, r| {
        s.bd3(&mut r.position)?;
        s.bl(&mut r.start)?;
        s.bl(&mut r.end)
    })
}

impl Spec for Table {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        if v >= Version::R2010 {
            return table_r24_io(s, &mut self.r24);
        }
        s.bd3(&mut self.insert)?;
        scale_io(s, &mut self.scale)?;
        s.bd(&mut self.rotation)?;
        s.bd3(&mut self.extrusion)?;
        s.b(&mut self.has_attribs)?;
        let mut owned = self.attribs.len() as i32;
        if v >= Version::R2004 {
            s.bl(&mut owned)?;
        }
        s.h(&mut self.block)?;
        if self.has_attribs {
            let n = if v >= Version::R2004 { owned } else { 2 };
            s.items(&mut self.attribs, n, &mut |s, h| s.h(h))?;
            s.h(&mut self.seqend)?;
        } else if S::READ {
            self.attribs.clear();
        }
        s.h(&mut self.style)?;
        s.bs(&mut self.flags)?;
        s.bd3(&mut self.direction)?;
        let mut ncols = self.column_widths.len() as i32;
        let mut nrows = self.row_heights.len() as i32;
        s.bl(&mut ncols)?;
        s.bl(&mut nrows)?;
        s.items(&mut self.column_widths, ncols, &mut |s, x| s.bd(x))?;
        s.items(&mut self.row_heights, nrows, &mut |s, x| s.bd(x))?;
        let n = ncols
            .checked_mul(nrows)
            .ok_or(crate::bits::Bad("table size".into()))?;
        s.items(&mut self.cells, n, &mut table_cell_io)?;
        let mut has = self.overrides.is_some();
        s.b(&mut has)?;
        if has {
            table_overrides_io(s, self.overrides.get_or_insert_with(Box::default))?;
        } else {
            self.overrides = None;
        }
        border_overrides_io(s, &mut self.border_colors, |s, c| s.cmc(c))?;
        border_overrides_io(s, &mut self.border_weights, |s, x| s.bs(x))?;
        border_overrides_io(s, &mut self.border_visible, |s, x| s.bs(x))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::BitReader;
    use crate::io::{Reader, Writer};
    use std::fmt::Debug;

    /// Writes `x` for `ver`, reads it back, and checks it is the same and every stream is used up.
    fn round<T: Spec + Default + PartialEq + Debug + Clone>(ver: Version, x: &T) {
        let mut w = Writer::new(ver);
        x.clone().io(&mut w).unwrap();
        let mut r = Reader {
            ver,
            data: BitReader::new(&w.data.bytes, 0),
            strs: (ver >= Version::R2007).then(|| BitReader::new(&w.strs.bytes, 0)),
            hdl: BitReader::new(&w.hdl.bytes, 0),
            base: 0,
        };
        let mut y = T::default();
        y.io(&mut r).unwrap();
        assert_eq!(&y, x, "{ver:?}");
        assert_eq!(r.data.p, w.data.len, "{ver:?} data");
        assert_eq!(r.hdl.p, w.hdl.len, "{ver:?} handles");
        assert_eq!(r.strs.map_or(0, |s| s.p), w.strs.len, "{ver:?} strings");
    }

    fn st(t: &str, v: Version) -> Str {
        Str::new(t, v >= Version::R2007)
    }

    fn h(n: u32) -> Href {
        Href::new(5, n)
    }

    /// A CMC (TC) as R2004+ reads it back.
    fn col(i: i16, v: Version) -> Color {
        Color {
            index: i,
            rgb: Some(0xC200_0000 | i as u32),
            name: Some(st("red", v)),
            ..Color::default()
        }
    }

    fn value(v: Version) -> Value {
        let mut x = Value {
            data_type: 4,
            text: st("12.5", v),
            ..Value::default()
        };
        if v >= Version::R2007 {
            x.unit_type = 1;
            x.format = st("%lu2", v);
            x.value_text = st("12.50", v);
        }
        x
    }

    fn style(v: Version) -> CellStyle {
        CellStyle {
            kind: 1,
            data: Some(CellStyleData {
                overrides: 0x200,
                merge_flags: 0,
                background: col(3, v),
                layout: 1,
                content: ContentFormat {
                    overrides: 0x40,
                    format: st("%au0", v),
                    color: col(7, v),
                    text_style: h(0x11),
                    text_height: 2.5,
                    ..ContentFormat::default()
                },
                margins: Some([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
                borders: vec![
                    Border::default(),
                    Border {
                        edges: 1,
                        overrides: 8,
                        kind: 1,
                        color: col(1, v),
                        line_weight: 25,
                        line_type: h(0x14),
                        invisible: 0,
                        double_spacing: 0.5,
                    },
                ],
            }),
        }
    }

    fn content(v: Version) -> TableContent {
        let custom = Custom {
            data: 0,
            items: vec![CustomItem {
                name: st("ACAD_ROUNDTRIP_2008_CELL_CHECKSUM", v),
                value: value(v),
            }],
        };
        TableContent {
            name: st("t", v),
            description: st("", v),
            columns: vec![Column {
                name: st("A", v),
                custom: custom.clone(),
                style: style(v),
                style_id: 3,
                width: 63.5,
            }],
            rows: vec![Row {
                cells: vec![Cell {
                    state: 1,
                    tooltip: st("tip", v),
                    custom,
                    link: Some(CellLink {
                        data_link: h(0x30),
                        rows: 1,
                        columns: 1,
                        unknown: 0,
                    }),
                    contents: vec![
                        CellContent {
                            kind: 1,
                            value: value(v),
                            has_format: 1,
                            format: style(v).data.unwrap().content,
                            ..CellContent::default()
                        },
                        CellContent {
                            kind: 4,
                            object: h(0x40),
                            attrs: vec![CellAttr {
                                attdef: h(0x41),
                                value: st("x", v),
                                index: 1,
                            }],
                            ..CellContent::default()
                        },
                    ],
                    style: CellStyle { kind: 1, data: None },
                    style_id: 0,
                    extra: Some(CellExtra {
                        flag: 1,
                        unknown: 2,
                        d40: 0.25,
                        d41: 0.5,
                        geometry_flags: 1,
                        unknown_h: h(0x42),
                        geometry: ContentGeometry {
                            to_top_left: [1.0, 2.0, 0.0],
                            to_center: [3.0, 4.0, 0.0],
                            content_width: 5.0,
                            content_height: 6.0,
                            width: 7.0,
                            height: 8.0,
                            flags: 0,
                        },
                    }),
                }],
                custom: Custom::default(),
                style: style(v),
                style_id: 1,
                height: 9.0,
            }],
            fields: vec![h(0x50)],
            style: CellStyle { kind: 4, data: None },
            merged: vec![[0, 0, 1, 1]],
            table_style: h(0x60),
        }
    }

    fn named(v: Version, id: i32, name: &str) -> NamedCellStyle {
        NamedCellStyle {
            style: style(v),
            id,
            class: 2,
            name: st(name, v),
        }
    }

    fn table(v: Version) -> Table {
        if v >= Version::R2010 {
            return Table {
                r24: Box::new(TableR24 {
                    unknown_rc: 0,
                    unknown_h: h(0x70),
                    unknown_bl: 0,
                    unknown_b: v == Version::R2010,
                    unknown_bl2: 0,
                    content: content(v),
                    unknown_bs: 38,
                    direction: [1.0, 0.0, 0.0],
                    breaks: Some(TableBreaks {
                        options: 1,
                        flow: 1,
                        spacing: 2.0,
                        unknown1: 0,
                        unknown2: 0,
                        heights: vec![BreakHeight {
                            position: [1.0, 2.0, 3.0],
                            height: 4.0,
                            flags: 0,
                        }],
                    }),
                    ranges: vec![RowRange {
                        position: [0.0; 3],
                        start: 0,
                        end: 1,
                    }],
                }),
                ..Table::default()
            };
        }
        let text = TableCell {
            kind: 1,
            merged_width: 1,
            merged_height: 1,
            text: st("abc", v),
            object: Href::NULL_HARD,
            overrides: Some(CellOverrides {
                flags: 0x3_FFFF,
                virtual_edges: 2,
                alignment: 5,
                bg_fill_none: true,
                bg_color: col(2, v),
                content_color: col(3, v),
                text_style: h(0x11),
                text_height: 3.0,
                edge_color: [col(1, v), col(2, v), col(3, v), col(4, v)],
                edge_weight: [13, 18, 25, 35],
                edge_visible: [1, 0, 1, 0],
            }),
            unknown: 0,
            value: if v >= Version::R2007 {
                value(v)
            } else {
                Value::default()
            },
            ..TableCell::default()
        };
        let block = TableCell {
            kind: 2,
            merged_width: 1,
            merged_height: 1,
            block_scale: 1.5,
            has_attrs: true,
            attrs: vec![TableAttr {
                index: 1,
                text: st("v", v),
                attdef: h(0x21),
            }],
            object: h(0x20),
            value: Value {
                format: st("", v),
                value_text: st("", v),
                ..Value::default()
            },
            ..TableCell::default()
        };
        Table {
            insert: [1.0, 2.0, 0.0],
            scale: [2.0, 3.0, 2.0],
            rotation: 0.5,
            has_attribs: true,
            direction: [1.0, 0.0, 0.0],
            column_widths: vec![10.0, 20.0],
            row_heights: vec![5.0],
            cells: vec![text, block],
            overrides: Some(Box::new(TableOverrides {
                flags: 0x7F_FFFF,
                title_suppressed: true,
                flow: 1,
                margin_h: 0.1,
                margin_v: 0.2,
                row_color: [col(1, v), col(2, v), col(3, v)],
                fill_none: [true, false, true],
                fill_color: [col(4, v), col(5, v), col(6, v)],
                alignment: [1, 2, 3],
                text_style: [h(0x11), h(0x12), h(0x13)],
                row_height: [1.0, 2.0, 3.0],
            })),
            border_colors: Some(Box::new(BorderOverrides {
                flags: 0x21,
                values: std::array::from_fn(|k| {
                    if k == 0 || k == 5 {
                        col(k as i16, v)
                    } else {
                        Color::default()
                    }
                }),
            })),
            border_weights: Some(Box::new(BorderOverrides {
                flags: 0x2_0000,
                values: std::array::from_fn(|k| if k == 17 { 50 } else { 0 }),
            })),
            border_visible: None,
            block: h(0x80),
            attribs: vec![h(0x81), h(0x82)],
            seqend: h(0x83),
            style: h(0x84),
            ..Table::default()
        }
    }

    fn mleader(v: Version) -> MLeader {
        let line = LeaderLine {
            points: vec![[1.0, 2.0, 0.0], [3.0, 4.0, 0.0]],
            breaks: vec![LineBreak {
                segment: 0,
                pairs: vec![[[1.5, 2.5, 0.0], [1.7, 2.7, 0.0]]],
            }],
            index: 0,
            ..LeaderLine::default()
        };
        let mut line = line;
        if v >= Version::R2010 {
            line.leader_type = 1;
            line.color = col(256, v);
            line.line_type = h(0x15);
            line.line_weight = -2;
            line.arrow_size = 4.0;
            line.arrow = h(0x16);
            line.overrides = 3;
        }
        let root = LeaderRoot {
            content_valid: true,
            unknown: true,
            connection: [5.0, 6.0, 0.0],
            direction: [1.0, 0.0, 0.0],
            breaks: vec![[[0.0; 3], [1.0; 3]]],
            index: 0,
            landing_distance: 0.36,
            lines: vec![line],
            attach_direction: (v >= Version::R2010) as i16,
        };
        let mut c = MLeaderContext {
            roots: vec![root],
            scale: 1.0,
            base: [7.0, 8.0, 0.0],
            text_height: 4.0,
            arrow_size: 4.0,
            landing_gap: 2.0,
            left_attach: 1,
            right_attach: 6,
            text_align: 0,
            attach_type: 0,
            text: None,
            block: Some(Box::new(MLeaderBlock {
                block: h(0x17),
                normal: [0.0, 0.0, 1.0],
                location: [1.0, 1.0, 0.0],
                scale: [1.0; 3],
                rotation: 0.0,
                color: col(0, v),
                matrix: std::array::from_fn(|k| (k % 5 == 0) as i32 as f64),
            })),
            base_point: [7.0, 8.0, 0.0],
            base_direction: [1.0, 0.0, 0.0],
            base_vertical: [0.0, 1.0, 0.0],
            normal_reversed: false,
            ..MLeaderContext::default()
        };
        if v >= Version::R2010 {
            c.top_attach = 9;
            c.bottom_attach = 9;
        }
        let mut m = MLeader {
            context: c,
            style: h(0x18),
            overrides: 0x4_4000,
            leader_type: 1,
            line_color: col(0, v),
            line_type: h(0x19),
            line_weight: -2,
            landing: true,
            dogleg: true,
            landing_distance: 0.36,
            arrow: h(0x1A),
            arrow_size: 4.0,
            content_type: 1,
            text_style: h(0x1B),
            left_attach: 1,
            right_attach: 6,
            text_angle_type: 1,
            unknown: 0,
            text_color: col(0, v),
            text_frame: false,
            block: h(0x1C),
            block_color: col(0, v),
            block_scale: [1.0; 3],
            block_rotation: 0.0,
            attach_type: 0,
            annotative: false,
            arrows: vec![(true, h(0x1D))],
            labels: vec![BlockLabel {
                attdef: h(0x1E),
                text: st("L", v),
                ui_index: 0,
                width: 1.0,
            }],
            text_dir_negative: false,
            ipe_align: 0,
            justification: 1,
            scale: 1.0,
            ..MLeader::default()
        };
        if v >= Version::R2010 {
            m.version = 2;
            m.attach_direction = 1;
            m.top_attach = 9;
            m.bottom_attach = 10;
        }
        if v >= Version::R2013 {
            m.extend_to_text = true;
        }
        m
    }

    fn mleader_text(v: Version) -> MLeader {
        let mut m = mleader(v);
        m.context.block = None;
        m.context.text = Some(Box::new(MLeaderText {
            label: st("xx\\Pxx", v),
            normal: [0.0, 0.0, 1.0],
            style: h(0x11),
            location: [1.0, 2.0, 0.0],
            direction: [1.0, 0.0, 0.0],
            line_spacing: 1.0,
            line_spacing_style: 1,
            color: col(256, v),
            alignment: 1,
            flow: 5,
            bg_color: col(256, v),
            bg_scale: 1.5,
            column_sizes: vec![10.0, 12.0],
            ..MLeaderText::default()
        }));
        m
    }

    fn mleader_style(v: Version) -> MLeaderStyle {
        let mut m = MLeaderStyle {
            content_type: 2,
            max_points: 2,
            first_angle: 0.1,
            leader_type: 1,
            line_color: col(256, v),
            line_type: h(0x14),
            line_weight: -2,
            landing: true,
            landing_gap: 2.0,
            dogleg: true,
            landing_distance: 0.36,
            description: st("d", v),
            arrow: h(0x15),
            arrow_size: 4.0,
            text_default: st("", v),
            text_style: h(0x11),
            text_color: col(256, v),
            text_height: 4.0,
            block_color: col(0, v),
            block_scale: [1.0; 3],
            scale: 1.0,
            break_size: 3.75,
            ..MLeaderStyle::default()
        };
        if v >= Version::R2010 {
            m.version = 2;
            m.top_attach = 9;
            m.bottom_attach = 9;
        }
        m
    }

    fn table_style(v: Version) -> TableStyle {
        if v >= Version::R2010 {
            return TableStyle {
                description: st("Standard", v),
                cells: Box::new(TableStyleCells {
                    unknown_h: h(0x90),
                    table: named(v, 4, "Table"),
                    styles: vec![
                        named(v, 3, "_DATA"),
                        named(v, 1, "_TITLE"),
                        named(v, 2, "_HEADER"),
                    ],
                    ..TableStyleCells::default()
                }),
                ..TableStyle::default()
            };
        }
        let row = |k: i16| RowStyle {
            text_style: h(0x11),
            text_height: 4.5,
            alignment: k,
            text_color: col(k, v),
            fill_color: col(k + 1, v),
            fill: true,
            borders: std::array::from_fn(|b| RowBorder {
                line_weight: -2,
                visible: b % 2 == 0,
                color: col(b as i16, v),
            }),
            data_type: if v >= Version::R2007 { 512 } else { 0 },
            unit_type: 0,
            format: st(if v >= Version::R2007 { "%x" } else { "" }, v),
        };
        TableStyle {
            description: st("Standard", v),
            flow: 0,
            flags: 0,
            margin_h: 0.06,
            margin_v: 0.06,
            suppress_title: false,
            suppress_header: true,
            rows: Box::new([row(1), row(2), row(5)]),
            ..TableStyle::default()
        }
    }

    /// Every version past R2000 (which the corpus checks) writes and reads back the same.
    #[test]
    fn later_versions_read_back() {
        for v in &Version::ALL[1..] {
            let v = *v;
            round(v, &mleader_style(v));
            round(v, &mleader(v));
            round(v, &mleader_text(v));
            round(v, &table(v));
            round(v, &content(v));
            round(v, &table_style(v));
            round(
                v,
                &CellStyleMap {
                    styles: vec![named(v, 1, "_TITLE"), named(v, 101, "custom")],
                },
            );
            round(
                v,
                &TableGeometry {
                    rows: 1,
                    columns: 1,
                    cells: vec![CellGeometry {
                        flags: 0,
                        width: 10.0,
                        height: 5.0,
                        unknown: Href::NULL_SOFT,
                        contents: vec![content(v).rows[0].cells[0].extra.clone().unwrap().geometry],
                    }],
                },
            );
        }
    }

    /// R2000 shapes not in the corpus: a block cell, every override, the scale's forms.
    #[test]
    fn r2000_table_overrides_read_back() {
        let v = Version::R2000;
        let mut t = table(Version::R2000);
        // R2000 CMC is the index alone
        let plain = |c: &mut Color| *c = Color::index(c.index);
        for c in &mut t.cells {
            if let Some(o) = &mut c.overrides {
                plain(&mut o.bg_color);
                plain(&mut o.content_color);
                o.edge_color.iter_mut().for_each(plain);
            }
        }
        let o = t.overrides.as_mut().unwrap();
        o.row_color
            .iter_mut()
            .chain(o.fill_color.iter_mut())
            .for_each(plain);
        t.border_colors
            .as_mut()
            .unwrap()
            .values
            .iter_mut()
            .for_each(plain);
        for scale in [[1.0; 3], [1.0, 2.0, 3.0], [2.0; 3], [2.0, 3.0, 2.0]] {
            t.scale = scale;
            round(v, &t);
        }
    }
}
