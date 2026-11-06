//! The writer. The file is laid out as the Open Design Toolkit lays out an R2000 file: the file
//! header with six section locators, the auxiliary header, an empty preview, the header
//! variables, the classes, 0x200 bytes of padding, the objects, the object map, the free-space
//! section, the second file header and the template (MEASUREMENT).
//!
//! The objects are what a new AutoCAD 2000 drawing holds: the nine symbol tables' control objects
//! and the viewport-entity one, their records (layers, line types, the Standard text and
//! dimension styles, the ACAD application, the *Active viewport), *Model_Space and *Paper_Space
//! with their BLOCK and ENDBLK, the named-object dictionary with ACAD_GROUP, ACAD_MLINESTYLE (the
//! Standard multiline style), ACAD_PLOTSETTINGS, ACAD_PLOTSTYLENAME (Normal) and ACAD_LAYOUT
//! (Model, Layout1); then the entities, all in model space.

use crate::bits::{BitWriter, crc16, mc, ms, umc};
use crate::{Doc, Entity, HAlign, Layer, LineType, Shape, Weight, encode};

// fixed handles
const BLOCK_CTRL: u32 = 0x1;
const LAYER_CTRL: u32 = 0x2;
const STYLE_CTRL: u32 = 0x3;
const LTYPE_CTRL: u32 = 0x5;
const VIEW_CTRL: u32 = 0x6;
const UCS_CTRL: u32 = 0x7;
const VPORT_CTRL: u32 = 0x8;
const APPID_CTRL: u32 = 0x9;
const DIMSTYLE_CTRL: u32 = 0xA;
const VX_CTRL: u32 = 0xB;
const NAMED_OBJECTS: u32 = 0xC;
const GROUP_DICT: u32 = 0xD;
const MLINESTYLE_DICT: u32 = 0xE;
/// The Normal plot style, a placeholder; layers point at it.
const NORMAL: u32 = 0xF;
const PLOTSTYLE_DICT: u32 = 0x10;
const PLOTSETTINGS_DICT: u32 = 0x11;
const LAYOUT_DICT: u32 = 0x12;
const MLINESTYLE_STD: u32 = 0x13;
const LT_BYBLOCK: u32 = 0x14;
const LT_BYLAYER: u32 = 0x15;
const LT_CONTINUOUS: u32 = 0x16;
const STYLE_STD: u32 = 0x17;
const APPID_ACAD: u32 = 0x18;
const DIMSTYLE_STD: u32 = 0x19;
const VPORT_ACTIVE: u32 = 0x1A;
const LAYOUT_MODEL: u32 = 0x1B;
const LAYOUT_1: u32 = 0x1C;
const PAPER_SPACE: u32 = 0x1E;
const MODEL_SPACE: u32 = 0x1F;
const MS_BLOCK: u32 = 0x20;
const MS_ENDBLK: u32 = 0x21;
const PS_BLOCK: u32 = 0x22;
const PS_ENDBLK: u32 = 0x23;
/// The first handle the drawing's own records and entities take.
const FIRST_FREE: u32 = 0x24;

// object types
const T_TEXT: i16 = 1;
const T_BLOCK: i16 = 4;
const T_ENDBLK: i16 = 5;
const T_ARC: i16 = 17;
const T_CIRCLE: i16 = 18;
const T_LINE: i16 = 19;
const T_POINT: i16 = 27;
const T_SOLID: i16 = 31;
const T_ELLIPSE: i16 = 35;
const T_SPLINE: i16 = 36;
const T_DICTIONARY: i16 = 42;
const T_BLOCK_CTRL: i16 = 48;
const T_BLOCK_HEADER: i16 = 49;
const T_LAYER_CTRL: i16 = 50;
const T_LAYER: i16 = 51;
const T_STYLE_CTRL: i16 = 52;
const T_STYLE: i16 = 53;
const T_LTYPE_CTRL: i16 = 56;
const T_LTYPE: i16 = 57;
const T_VIEW_CTRL: i16 = 60;
const T_UCS_CTRL: i16 = 62;
const T_VPORT_CTRL: i16 = 64;
const T_VPORT: i16 = 65;
const T_APPID_CTRL: i16 = 66;
const T_APPID: i16 = 67;
const T_DIMSTYLE_CTRL: i16 = 68;
const T_DIMSTYLE: i16 = 69;
const T_VX_CTRL: i16 = 70;
const T_MLINESTYLE: i16 = 73;
const T_LWPOLYLINE: i16 = 77;
// the classes, in the class section's order: types 500, 501, 502
const T_DICTIONARYWDFLT: i16 = 500;
const T_PLACEHOLDER: i16 = 501;
const T_LAYOUT: i16 = 502;
const CLASSES: [(&str, &str); 3] = [
    ("AcDbDictionaryWithDefault", "ACDBDICTIONARYWDFLT"),
    ("AcDbPlaceHolder", "ACDBPLACEHOLDER"),
    ("AcDbLayout", "LAYOUT"),
];

// handle reference codes
const SOFT_OWNER: u8 = 2;
const HARD_OWNER: u8 = 3;
const SOFT_PTR: u8 = 4;
const HARD_PTR: u8 = 5;

/// "AC1015", five zeros and the maintenance version: the first twelve bytes of the file.
const VERSION: [u8; 12] = *b"AC1015\0\0\0\0\0\x06";
const MAINTENANCE: u16 = 6;
const CODEPAGE_ANSI_1252: u16 = 30;

const SN_HEADER: [u8; 16] = [
    0xCF, 0x7B, 0x1F, 0x23, 0xFD, 0xDE, 0x38, 0xA9, 0x5F, 0x7C, 0x68, 0xB8, 0x4E, 0x6D, 0x33, 0x5F,
];
const SN_HEADER_END: [u8; 16] = [
    0x30, 0x84, 0xE0, 0xDC, 0x02, 0x21, 0xC7, 0x56, 0xA0, 0x83, 0x97, 0x47, 0xB1, 0x92, 0xCC, 0xA0,
];
const SN_CLASSES: [u8; 16] = [
    0x8D, 0xA1, 0xC4, 0xB8, 0xC4, 0xA9, 0xF8, 0xC5, 0xC0, 0xDC, 0xF4, 0x5F, 0xE7, 0xCF, 0xB6, 0x8A,
];
const SN_CLASSES_END: [u8; 16] = [
    0x72, 0x5E, 0x3B, 0x47, 0x3B, 0x56, 0x07, 0x3A, 0x3F, 0x23, 0x0B, 0xA0, 0x18, 0x30, 0x49, 0x75,
];
const SN_FILE_HEADER_END: [u8; 16] = [
    0x95, 0xA0, 0x4E, 0x28, 0x99, 0x82, 0x1A, 0xE5, 0x5E, 0x41, 0xE0, 0x5F, 0x9D, 0x3A, 0x4D, 0x00,
];
const SN_PREVIEW: [u8; 16] = [
    0x1F, 0x25, 0x6D, 0x07, 0xD4, 0x36, 0x28, 0x28, 0x9D, 0x57, 0xCA, 0x3F, 0x9D, 0x44, 0x10, 0x2B,
];
const SN_PREVIEW_END: [u8; 16] = [
    0xE0, 0xDA, 0x92, 0xF8, 0x2B, 0xC9, 0xD7, 0xD7, 0x62, 0xA8, 0x35, 0xC0, 0x62, 0xBB, 0xEF, 0xD4,
];
const SN_SECOND: [u8; 16] = [
    0xD4, 0x7B, 0x21, 0xCE, 0x28, 0x93, 0x9F, 0xBF, 0x53, 0x24, 0x40, 0x09, 0x12, 0x3C, 0xAA, 0x01,
];
const SN_SECOND_END: [u8; 16] = [
    0x2B, 0x84, 0xDE, 0x31, 0xD7, 0x6C, 0x60, 0x40, 0xAC, 0xDB, 0xBF, 0xF6, 0xED, 0xC3, 0x55, 0xFE,
];

/// One object being written: its data and its handles, two streams joined at the end.
struct Obj {
    data: BitWriter,
    hs: BitWriter,
}

impl Obj {
    /// The common start: type, the size-in-bits slot, handle and (no) extended data.
    fn new(kind: i16, handle: u32) -> Obj {
        let mut data = BitWriter::new();
        data.bs(kind);
        data.rl(0);
        data.h(0, handle);
        data.bs(0);
        Obj {
            data,
            hs: BitWriter::new(),
        }
    }
    /// A non-entity object, owned by `owner` (and reacting to it when `reactor`).
    fn object(kind: i16, handle: u32, owner: u32, reactor: bool) -> Obj {
        let mut o = Obj::new(kind, handle);
        o.data.bl(reactor as i32);
        o.hs.h(SOFT_PTR, owner);
        if reactor {
            o.hs.h(SOFT_PTR, owner);
        }
        o.hs.h(HARD_OWNER, 0);
        o
    }
    /// An entity: model or paper space, bylayer everything, on `layer`, between `prev` and
    /// `next` in its block (0 for none).
    fn entity(kind: i16, handle: u32, paper: bool, layer: u32, links: (u32, u32)) -> Obj {
        let mut o = Obj::new(kind, handle);
        o.data.b(false); // no graphic
        o.data.bb(if paper { 1 } else { 2 });
        o.data.bl(0); // reactors
        let nolinks = links.0 != 0 && links.0 + 1 == handle && links.1 == handle + 1;
        o.data.b(nolinks);
        o.data.bs(256); // colour bylayer
        o.data.bd(1.0); // line type scale
        o.data.bb(0); // line type bylayer
        o.data.bb(0); // plot style bylayer
        o.data.bs(0); // visible
        o.data.rc(Weight::ByLayer.code());
        o.hs.h(HARD_OWNER, 0);
        if !nolinks {
            o.hs.h(SOFT_PTR, links.0);
            o.hs.h(SOFT_PTR, links.1);
        }
        o.hs.h(HARD_PTR, layer);
        o
    }
    /// A symbol-table record's name and flags (referenced, not from an xref).
    fn entry(&mut self, name: &[u8]) {
        self.data.t(name);
        self.data.b(true);
        self.data.bs(0);
        self.data.b(false);
    }
    /// MS size, the streams (the size in bits patched in), CRC.
    fn finish(mut self) -> Vec<u8> {
        let bits = self.data.len;
        // the RL slot sits after the type's bitshort
        let at = type_bits(&self.data);
        self.data.patch_rl(at, bits as u32);
        self.data.append(&self.hs);
        let mut out = Vec::with_capacity(self.data.bytes.len() + 6);
        ms(&mut out, self.data.bytes.len() as u32);
        out.extend_from_slice(&self.data.bytes);
        let crc = crc16(0xC0C1, &out);
        out.extend_from_slice(&crc.to_le_bytes());
        out
    }
}

/// How many bits the leading type bitshort took.
fn type_bits(w: &BitWriter) -> usize {
    match w.bytes[0] >> 6 {
        0 => 18,
        1 => 10,
        _ => 2,
    }
}

fn control(kind: i16, handle: u32, owned: &[u32]) -> Obj {
    let mut o = Obj::object(kind, handle, 0, false);
    o.data.bl(owned.len() as i32);
    for &h in owned {
        o.hs.h(SOFT_OWNER, h);
    }
    o
}

fn dictionary(handle: u32, owner: u32, entries: &[(&str, u32)]) -> Obj {
    let mut o = Obj::object(T_DICTIONARY, handle, owner, owner != 0);
    dict_body(&mut o, entries);
    o
}

fn dict_body(o: &mut Obj, entries: &[(&str, u32)]) {
    o.data.bl(entries.len() as i32);
    o.data.bs(1); // cloning: keep existing
    o.data.rc(0); // not a hard owner
    for (name, h) in entries {
        o.data.t(name.as_bytes());
        o.hs.h(SOFT_OWNER, *h);
    }
}

/// The handles every object and entity of the drawing gets.
struct Plan {
    line_types: Vec<(u32, LineType)>,
    layers: Vec<(u32, Layer)>,
    entities: Vec<(u32, u32, Shape)>,
    seed: u32,
}

fn plan(doc: &Doc) -> Plan {
    let mut next = FIRST_FREE;
    let mut take = || {
        next += 1;
        next - 1
    };
    let fixed = ["ByBlock", "ByLayer", "Continuous"];
    let line_types: Vec<(u32, LineType)> = doc
        .line_types
        .iter()
        .filter(|t| !fixed.iter().any(|f| f.eq_ignore_ascii_case(&t.name)))
        .map(|t| (take(), t.clone()))
        .collect();
    let mut layers: Vec<Layer> = doc.layers.clone();
    let blank = |name: &str| Layer {
        name: name.into(),
        colour: 7,
        line_type: "Continuous".into(),
        weight: Weight::Default,
    };
    if !layers.iter().any(|l| l.name == "0") {
        layers.insert(0, blank("0"));
    }
    for e in &doc.entities {
        if !layers.iter().any(|l| l.name == e.layer) {
            layers.push(blank(&e.layer));
        }
    }
    let layers: Vec<(u32, Layer)> = layers.into_iter().map(|l| (take(), l)).collect();
    let layer_of = |name: &str| layers.iter().find(|l| l.1.name == name).map_or(0, |l| l.0);
    let entities = doc
        .entities
        .iter()
        .map(|Entity { layer, shape }| (take(), layer_of(layer), shape.clone()))
        .collect();
    Plan {
        line_types,
        layers,
        entities,
        seed: next,
    }
}

/// The drawing as an R2000 DWG file.
pub fn write(doc: &Doc) -> Vec<u8> {
    let p = plan(doc);
    let layer0 = p.layers[0].0;
    let (lo, hi) = extents(doc);
    let mut objs: Vec<(u32, Vec<u8>)> = vec![];
    let mut put = |h: u32, o: Obj| objs.push((h, o.finish()));

    // the symbol tables
    put(BLOCK_CTRL, {
        let mut o = control(T_BLOCK_CTRL, BLOCK_CTRL, &[]);
        o.hs.h(HARD_OWNER, MODEL_SPACE);
        o.hs.h(HARD_OWNER, PAPER_SPACE);
        o
    });
    let layer_hs: Vec<u32> = p.layers.iter().map(|l| l.0).collect();
    put(LAYER_CTRL, control(T_LAYER_CTRL, LAYER_CTRL, &layer_hs));
    put(STYLE_CTRL, control(T_STYLE_CTRL, STYLE_CTRL, &[STYLE_STD]));
    put(LTYPE_CTRL, {
        let mut owned = vec![LT_CONTINUOUS];
        owned.extend(p.line_types.iter().map(|t| t.0));
        let mut o = control(T_LTYPE_CTRL, LTYPE_CTRL, &owned);
        o.hs.h(HARD_OWNER, LT_BYBLOCK);
        o.hs.h(HARD_OWNER, LT_BYLAYER);
        o
    });
    put(VIEW_CTRL, control(T_VIEW_CTRL, VIEW_CTRL, &[]));
    put(UCS_CTRL, control(T_UCS_CTRL, UCS_CTRL, &[]));
    put(VPORT_CTRL, control(T_VPORT_CTRL, VPORT_CTRL, &[VPORT_ACTIVE]));
    put(APPID_CTRL, control(T_APPID_CTRL, APPID_CTRL, &[APPID_ACAD]));
    put(DIMSTYLE_CTRL, {
        let mut o = control(T_DIMSTYLE_CTRL, DIMSTYLE_CTRL, &[DIMSTYLE_STD]);
        o.data.rc(0); // no further (hard owner) handles
        o
    });
    put(VX_CTRL, control(T_VX_CTRL, VX_CTRL, &[]));

    // the dictionaries
    put(
        NAMED_OBJECTS,
        dictionary(
            NAMED_OBJECTS,
            0,
            &[
                ("ACAD_GROUP", GROUP_DICT),
                ("ACAD_LAYOUT", LAYOUT_DICT),
                ("ACAD_MLINESTYLE", MLINESTYLE_DICT),
                ("ACAD_PLOTSETTINGS", PLOTSETTINGS_DICT),
                ("ACAD_PLOTSTYLENAME", PLOTSTYLE_DICT),
            ],
        ),
    );
    put(GROUP_DICT, dictionary(GROUP_DICT, NAMED_OBJECTS, &[]));
    put(
        MLINESTYLE_DICT,
        dictionary(MLINESTYLE_DICT, NAMED_OBJECTS, &[("Standard", MLINESTYLE_STD)]),
    );
    put(NORMAL, Obj::object(T_PLACEHOLDER, NORMAL, PLOTSTYLE_DICT, true));
    put(PLOTSTYLE_DICT, {
        let mut o = Obj::object(T_DICTIONARYWDFLT, PLOTSTYLE_DICT, NAMED_OBJECTS, true);
        dict_body(&mut o, &[("Normal", NORMAL)]);
        o.hs.h(HARD_PTR, NORMAL); // the default
        o
    });
    put(PLOTSETTINGS_DICT, dictionary(PLOTSETTINGS_DICT, NAMED_OBJECTS, &[]));
    put(
        LAYOUT_DICT,
        dictionary(
            LAYOUT_DICT,
            NAMED_OBJECTS,
            &[("Layout1", LAYOUT_1), ("Model", LAYOUT_MODEL)],
        ),
    );
    put(MLINESTYLE_STD, {
        let mut o = Obj::object(T_MLINESTYLE, MLINESTYLE_STD, MLINESTYLE_DICT, true);
        o.data.t(b"Standard");
        o.data.t(b"");
        o.data.bs(0); // flags
        o.data.bs(256); // fill colour bylayer
        o.data.bd(std::f64::consts::FRAC_PI_2);
        o.data.bd(std::f64::consts::FRAC_PI_2);
        o.data.rc(2);
        for off in [0.5, -0.5] {
            o.data.bd(off);
            o.data.bs(256); // bylayer
            o.data.bs(32767); // line type bylayer
        }
        o
    });

    // line types
    let ltype = |h: u32, name: &str, desc: &str, pattern: &[f64]| {
        let mut o = Obj::object(T_LTYPE, h, LTYPE_CTRL, false);
        o.entry(&encode(name));
        o.data.t(&encode(desc));
        o.data.bd(pattern.iter().map(|x| x.abs()).sum());
        o.data.rc(b'A');
        o.data.rc(pattern.len() as u8);
        for &x in pattern {
            o.data.bd(x);
            o.data.bs(0); // shape number
            o.data.rd(0.0); // x offset
            o.data.rd(0.0); // y offset
            o.data.bd(0.0); // scale
            o.data.bd(0.0); // rotation
            o.data.bs(0); // shape flags
        }
        o.data.raw(&[0; 256]); // the strings area
        o.hs.h(HARD_PTR, 0); // xref block
        for _ in pattern {
            o.hs.h(HARD_PTR, 0); // shape file
        }
        o
    };
    put(LT_BYBLOCK, ltype(LT_BYBLOCK, "ByBlock", "", &[]));
    put(LT_BYLAYER, ltype(LT_BYLAYER, "ByLayer", "", &[]));
    put(LT_CONTINUOUS, ltype(LT_CONTINUOUS, "Continuous", "Solid line", &[]));
    for (h, t) in &p.line_types {
        put(*h, ltype(*h, &t.name, &t.description, &t.pattern));
    }
    let ltype_of = |name: &str| {
        p.line_types
            .iter()
            .find(|t| t.1.name.eq_ignore_ascii_case(name))
            .map_or(LT_CONTINUOUS, |t| t.0)
    };

    // layers
    for (h, l) in &p.layers {
        let mut o = Obj::object(T_LAYER, *h, LAYER_CTRL, false);
        o.entry(&encode(&l.name));
        // plotted, the lineweight in bits 5..9; on (a positive colour), thawed, unlocked
        o.data.bs(0x10 | ((l.weight.code() as i16) << 5));
        o.data.bs(l.colour.clamp(1, 255));
        o.hs.h(HARD_PTR, 0); // xref block
        o.hs.h(HARD_PTR, NORMAL); // plot style
        o.hs.h(HARD_PTR, ltype_of(&l.line_type));
        put(*h, o);
    }

    put(STYLE_STD, {
        let mut o = Obj::object(T_STYLE, STYLE_STD, STYLE_CTRL, false);
        o.entry(b"Standard");
        o.data.b(false); // vertical
        o.data.b(false); // a shape file
        o.data.bd(0.0); // fixed height
        o.data.bd(1.0); // width factor
        o.data.bd(0.0); // oblique
        o.data.rc(0); // generation
        o.data.bd(2.5); // last height
        o.data.t(&encode(&doc.font));
        o.data.t(b"");
        o.hs.h(HARD_PTR, 0);
        o
    });
    put(APPID_ACAD, {
        let mut o = Obj::object(T_APPID, APPID_ACAD, APPID_CTRL, false);
        o.entry(b"ACAD");
        o.data.rc(0);
        o.hs.h(HARD_PTR, 0);
        o
    });
    put(DIMSTYLE_STD, {
        let mut o = Obj::object(T_DIMSTYLE, DIMSTYLE_STD, DIMSTYLE_CTRL, false);
        o.entry(b"Standard");
        dim_vars(&mut o.data);
        o.data.b(false); // the 70 group's 1 bit
        o.hs.h(HARD_PTR, 0); // xref block
        o.hs.h(HARD_PTR, STYLE_STD); // DIMTXSTY
        for _ in 0..4 {
            o.hs.h(HARD_PTR, 0); // DIMLDRBLK, DIMBLK, DIMBLK1, DIMBLK2
        }
        o
    });
    put(VPORT_ACTIVE, {
        let mut o = Obj::object(T_VPORT, VPORT_ACTIVE, VPORT_CTRL, false);
        o.entry(b"*Active");
        let (w, h) = ((hi[0] - lo[0]).max(1.0), (hi[1] - lo[1]).max(1.0));
        let d = &mut o.data;
        d.bd(h * 1.05); // view height
        d.bd(w * 1.05); // aspect ratio times the view height
        d.rd2([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]); // view centre
        d.bd3([0.0; 3]); // target
        d.bd3([0.0, 0.0, 1.0]); // direction
        d.bd(0.0); // twist
        d.bd(50.0); // lens
        d.bd(0.0); // front clip
        d.bd(0.0); // back clip
        d.bits(4, 1); // view mode
        d.rc(0); // render mode
        d.rd2([0.0, 0.0]); // lower left
        d.rd2([1.0, 1.0]); // upper right
        d.b(false); // UCSFOLLOW
        d.bs(100); // circle zoom
        d.b(true); // fast zoom
        d.bits(2, 3); // UCSICON
        d.b(false); // grid
        d.rd2([10.0, 10.0]);
        d.b(false); // snap
        d.b(false); // snap style
        d.bs(0); // isopair
        d.bd(0.0); // snap rotation
        d.rd2([0.0, 0.0]); // snap base
        d.rd2([10.0, 10.0]); // snap spacing
        d.b(false);
        d.b(true); // UCS per viewport
        d.bd3([0.0; 3]);
        d.bd3([1.0, 0.0, 0.0]);
        d.bd3([0.0, 1.0, 0.0]);
        d.bd(0.0); // UCS elevation
        d.bs(0); // orthographic type
        o.hs.h(HARD_PTR, 0); // xref block
        o.hs.h(HARD_PTR, 0); // named UCS
        o.hs.h(HARD_PTR, 0); // base UCS
        o
    });

    // model and paper space
    let first = p.entities.first().map_or(0, |e| e.0);
    let last = p.entities.last().map_or(0, |e| e.0);
    for (rec, name, block, end, layout, paper) in [
        (MODEL_SPACE, "*Model_Space", MS_BLOCK, MS_ENDBLK, LAYOUT_MODEL, false),
        (PAPER_SPACE, "*Paper_Space", PS_BLOCK, PS_ENDBLK, LAYOUT_1, true),
    ] {
        let mut o = Obj::object(T_BLOCK_HEADER, rec, BLOCK_CTRL, false);
        o.entry(name.as_bytes());
        for _ in 0..5 {
            o.data.b(false); // anonymous, attributes, xref, overlaid, loaded
        }
        o.data.bd3([0.0; 3]); // base point
        o.data.t(b""); // xref path
        o.data.rc(0); // no inserts
        o.data.t(b""); // description
        o.data.bl(0); // no preview
        o.hs.h(HARD_PTR, 0);
        o.hs.h(HARD_OWNER, block);
        let (f, l) = if paper { (0, 0) } else { (first, last) };
        o.hs.h(SOFT_PTR, f);
        o.hs.h(SOFT_PTR, l);
        o.hs.h(HARD_OWNER, end);
        o.hs.h(HARD_PTR, layout);
        put(rec, o);
        let mut b = Obj::entity(T_BLOCK, block, paper, layer0, (0, 0));
        b.data.t(name.as_bytes());
        put(block, b);
        put(end, Obj::entity(T_ENDBLK, end, paper, layer0, (0, 0)));
    }
    for (h, rec, name, tab, model) in [
        (LAYOUT_MODEL, MODEL_SPACE, "Model", 0, true),
        (LAYOUT_1, PAPER_SPACE, "Layout1", 1, false),
    ] {
        put(h, layout(h, rec, name, tab, model, (lo, hi)));
    }

    // the entities, a chain in model space
    for (k, (h, layer, shape)) in p.entities.iter().enumerate() {
        let prev = if k == 0 { 0 } else { p.entities[k - 1].0 };
        let next = p.entities.get(k + 1).map_or(0, |e| e.0);
        put(*h, entity(*h, *layer, shape, (prev, next)));
    }

    objs.sort_by_key(|o| o.0);
    assemble(doc, &p, objs, (lo, hi), layer0)
}

/// The box round everything drawn, or the limits when nothing is.
fn extents(doc: &Doc) -> ([f64; 2], [f64; 2]) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    let mut add = |q: [f64; 2]| {
        for k in 0..2 {
            lo[k] = lo[k].min(q[k]);
            hi[k] = hi[k].max(q[k]);
        }
    };
    for e in &doc.entities {
        match &e.shape {
            Shape::Line { a, b } => {
                add(*a);
                add(*b);
            }
            Shape::Circle { c, r } | Shape::Arc { c, r, .. } => {
                add([c[0] - r, c[1] - r]);
                add([c[0] + r, c[1] + r]);
            }
            Shape::Ellipse { c, major, .. } => {
                let r = major[0].hypot(major[1]);
                add([c[0] - r, c[1] - r]);
                add([c[0] + r, c[1] + r]);
            }
            Shape::Polyline { points, .. } | Shape::Spline { points, .. } => {
                points.iter().for_each(|q| add(*q))
            }
            Shape::Point { at } | Shape::Text { at, .. } => add(*at),
            Shape::Solid { corners } => corners.iter().for_each(|q| add(*q)),
        }
    }
    if lo[0] > hi[0] {
        return doc.limits;
    }
    (lo, hi)
}

fn entity(h: u32, layer: u32, shape: &Shape, links: (u32, u32)) -> Obj {
    let kind = match shape {
        Shape::Line { .. } => T_LINE,
        Shape::Circle { .. } => T_CIRCLE,
        Shape::Arc { .. } => T_ARC,
        Shape::Ellipse { .. } => T_ELLIPSE,
        Shape::Polyline { .. } => T_LWPOLYLINE,
        Shape::Spline { .. } => T_SPLINE,
        Shape::Point { .. } => T_POINT,
        Shape::Solid { .. } => T_SOLID,
        Shape::Text { .. } => T_TEXT,
    };
    let mut o = Obj::entity(kind, h, false, layer, links);
    let d = &mut o.data;
    let z = |p: [f64; 2]| [p[0], p[1], 0.0];
    const UP: [f64; 3] = [0.0, 0.0, 1.0];
    match shape {
        Shape::Line { a, b } => {
            d.b(true); // the z's are zero
            d.rd(a[0]);
            d.dd(b[0], a[0]);
            d.rd(a[1]);
            d.dd(b[1], a[1]);
            d.bt(0.0);
            d.be(UP);
        }
        Shape::Circle { c, r } => {
            d.bd3(z(*c));
            d.bd(*r);
            d.bt(0.0);
            d.be(UP);
        }
        Shape::Arc { c, r, start, end } => {
            d.bd3(z(*c));
            d.bd(*r);
            d.bt(0.0);
            d.be(UP);
            d.bd(*start);
            d.bd(*end);
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            d.bd3(z(*c));
            d.bd3(z(*major));
            d.bd3(UP);
            d.bd(*ratio);
            d.bd(*t0);
            d.bd(*t1);
        }
        Shape::Polyline { points, closed } => {
            d.bs(if *closed { 512 } else { 0 });
            d.bl(points.len() as i32);
            if let Some((first, rest)) = points.split_first() {
                d.rd2(*first);
                let mut prev = *first;
                for q in rest {
                    d.dd(q[0], prev[0]);
                    d.dd(q[1], prev[1]);
                    prev = *q;
                }
            }
        }
        Shape::Spline {
            degree,
            knots,
            points,
            weights,
        } => {
            d.bl(1); // control points and knots
            d.bl(*degree as i32);
            d.b(weights.is_some()); // rational
            d.b(false); // closed
            d.b(false); // periodic
            d.bd(1e-10); // knot tolerance
            d.bd(1e-10); // control point tolerance
            d.bl(knots.len() as i32);
            d.bl(points.len() as i32);
            d.b(weights.is_some());
            for k in knots {
                d.bd(*k);
            }
            for (i, q) in points.iter().enumerate() {
                d.bd3(z(*q));
                if let Some(w) = weights {
                    d.bd(w.get(i).copied().unwrap_or(1.0));
                }
            }
        }
        Shape::Point { at } => {
            d.bd3(z(*at));
            d.bt(0.0);
            d.be(UP);
            d.bd(0.0); // x axis angle
        }
        Shape::Solid { corners } => {
            d.bt(0.0);
            d.bd(0.0); // elevation
            for q in corners {
                d.rd2(*q);
            }
            d.be(UP);
        }
        Shape::Text {
            at,
            h,
            angle,
            align,
            text,
        } => {
            let ha: i16 = match align {
                HAlign::Left => 0,
                HAlign::Centre => 1,
                HAlign::Right => 2,
            };
            // which values are left at their defaults, and so not written
            let mut flags = 0x01 | 0x04 | 0x10 | 0x20 | 0x80; // elevation, oblique, width, generation, valign
            if ha == 0 {
                flags |= 0x02 | 0x40; // no alignment point, left
            }
            if *angle == 0.0 {
                flags |= 0x08;
            }
            d.rc(flags);
            d.rd2(*at);
            if ha != 0 {
                d.dd(at[0], at[0]);
                d.dd(at[1], at[1]);
            }
            d.be(UP);
            d.bt(0.0);
            if *angle != 0.0 {
                d.rd(*angle);
            }
            d.rd(*h);
            d.t(&encode(text));
            if ha != 0 {
                d.bs(ha);
            }
            o.hs.h(HARD_PTR, STYLE_STD);
        }
    }
    o
}

fn layout(h: u32, rec: u32, name: &str, tab: i32, model: bool, (lo, hi): ([f64; 2], [f64; 2])) -> Obj {
    let mut o = Obj::object(T_LAYOUT, h, LAYOUT_DICT, true);
    let d = &mut o.data;
    // the plot settings: no device, the paper as AutoCAD's defaults have it
    d.t(b""); // page setup
    d.t(b"none_device");
    d.bs(if model { 1712 } else { 688 }); // plot layout flags
    for _ in 0..4 {
        d.bd(0.0); // margins
    }
    d.bd(0.0); // paper width
    d.bd(0.0); // paper height
    d.t(b""); // paper size
    d.bd2([0.0, 0.0]); // origin
    d.bs(1); // millimetres
    d.bs(0); // rotation
    d.bs(if model { 0 } else { 5 }); // plot type: display, layout
    d.bd2([0.0, 0.0]); // window
    d.bd2([0.0, 0.0]);
    d.t(b""); // view name
    d.bd(1.0); // real world units
    d.bd(1.0); // drawing units
    d.t(b""); // style sheet
    d.bs(if model { 0 } else { 16 }); // scale type: custom, 1:1
    d.bd(1.0); // scale factor
    d.bd2([0.0, 0.0]); // paper image origin
    // the layout
    d.t(name.as_bytes());
    d.bl(tab);
    d.bs(1); // flags
    d.bd3([0.0; 3]); // UCS origin
    d.rd2([0.0, 0.0]); // limits
    d.rd2(if model { hi } else { [420.0, 297.0] });
    d.bd3([0.0; 3]); // insertion base
    d.bd3([1.0, 0.0, 0.0]);
    d.bd3([0.0, 1.0, 0.0]);
    d.bd(0.0); // elevation
    d.bs(0); // orthographic type
    if model {
        d.bd3([lo[0], lo[1], 0.0]);
        d.bd3([hi[0], hi[1], 0.0]);
    } else {
        d.bd3([1e20; 3]);
        d.bd3([-1e20; 3]);
    }
    o.hs.h(SOFT_PTR, rec);
    o.hs.h(SOFT_PTR, 0); // last active viewport
    o.hs.h(HARD_PTR, 0); // base UCS
    o.hs.h(HARD_PTR, 0); // named UCS
    o
}

/// The dimension variables, header and DIMSTYLE alike, from DIMSCALE to DIMATFIT (AutoCAD's
/// defaults, as a DXF without them reads).
fn dim_vars(d: &mut BitWriter) {
    for v in [1.0, 0.18, 0.0625, 0.38, 0.18, 0.0, 0.0, 0.0, 0.0] {
        d.bd(v); // DIMSCALE ASZ EXO DLI EXE RND DLE TP TM
    }
    for v in [false, false, true, true, false, false] {
        d.b(v); // DIMTOL LIM TIH TOH SE1 SE2
    }
    for v in [0, 0, 0] {
        d.bs(v); // DIMTAD ZIN AZIN
    }
    for v in [0.18, 0.09, 0.0, 25.4, 1.0, 0.0, 1.0, 0.09, 0.0] {
        d.bd(v); // DIMTXT CEN TSZ ALTF LFAC TVP TFAC GAP ALTRND
    }
    d.b(false); // DIMALT
    d.bs(2); // DIMALTD
    for _ in 0..4 {
        d.b(false); // DIMTOFL SAH TIX SOXD
    }
    for _ in 0..3 {
        d.bs(0); // DIMCLRD CLRE CLRT
    }
    for v in [0, 4, 4, 2, 2, 0, 0, 2, 46, 0, 0] {
        d.bs(v); // DIMADEC DEC TDEC ALTU ALTTD AUNIT FRAC LUNIT DSEP TMOVE JUST
    }
    d.b(false); // DIMSD1
    d.b(false); // DIMSD2
    for v in [1, 0, 0, 0] {
        d.bs(v); // DIMTOLJ TZIN ALTZ ALTTZ
    }
    d.b(false); // DIMUPT
    d.bs(3); // DIMATFIT
}

/// The header variables (chapter 9, R2000), in their order.
fn header_vars(p: &Plan, (lo, hi): ([f64; 2], [f64; 2]), limits: ([f64; 2], [f64; 2]), layer0: u32, guid: &str) -> BitWriter {
    let mut d = BitWriter::new();
    d.bd(412148564080.0);
    d.bd(1.0);
    d.bd(1.0);
    d.bd(1.0);
    d.t(b"m");
    for _ in 0..3 {
        d.t(b"");
    }
    d.bl(24);
    d.bl(0);
    d.h(HARD_PTR, 0); // current viewport entity
    // DIMASO DIMSHO PLINEGEN ORTHOMODE REGENMODE FILLMODE QTEXTMODE PSLTSCALE LIMCHECK USRTIMER
    // SKPOLY ANGDIR SPLFRAME MIRRTEXT WORLDVIEW TILEMODE PLIMCHECK VISRETAIN DISPSILH PELLIPSE
    for v in [1, 1, 0, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 0] {
        d.b(v == 1);
    }
    // PROXYGRAPHICS TREEDEPTH LUNITS LUPREC AUNITS AUPREC ATTMODE PDMODE USERI1-5 SPLINESEGS SURFU
    // SURFV SURFTYPE SURFTAB1 SURFTAB2 SPLINETYPE SHADEDGE SHADEDIF UNITMODE MAXACTVP ISOLINES
    // CMLJUST TEXTQLTY
    for v in [1, 3020, 2, 4, 0, 0, 1, 0, 0, 0, 0, 0, 0, 8, 6, 6, 6, 6, 6, 6, 3, 70, 0, 64, 4, 0, 50] {
        d.bs(v);
    }
    // LTSCALE TEXTSIZE TRACEWID SKETCHINC FILLETRAD THICKNESS ANGBASE PDSIZE PLINEWID USERR1-5
    // CHAMFERA-D FACETRES CMLSCALE CELTSCALE
    for v in [
        1.0, 2.5, 0.05, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5,
        1.0, 1.0,
    ] {
        d.bd(v);
    }
    d.t(b"acad"); // MENUNAME
    for _ in 0..8 {
        d.bl(0); // TDCREATE TDUPDATE TDINDWG TDUSRTIMER: days, milliseconds
    }
    d.bs(256); // CECOLOR bylayer
    d.h(0, p.seed); // HANDSEED
    d.h(HARD_PTR, layer0); // CLAYER
    d.h(HARD_PTR, STYLE_STD); // TEXTSTYLE
    d.h(HARD_PTR, LT_BYLAYER); // CELTYPE
    d.h(HARD_PTR, DIMSTYLE_STD);
    d.h(HARD_PTR, MLINESTYLE_STD);
    d.bd(0.0); // PSVPSCALE
    // paper space: INSBASE EXTMIN EXTMAX LIMMIN LIMMAX ELEVATION UCSORG UCSXDIR UCSYDIR UCSNAME,
    // its orthographic UCSs
    d.bd3([0.0; 3]);
    d.bd3([1e20; 3]);
    d.bd3([-1e20; 3]);
    d.rd2([0.0, 0.0]);
    d.rd2([420.0, 297.0]);
    let ucs = |d: &mut BitWriter| {
        d.bd(0.0);
        d.bd3([0.0; 3]);
        d.bd3([1.0, 0.0, 0.0]);
        d.bd3([0.0, 1.0, 0.0]);
        d.h(HARD_PTR, 0); // UCSNAME
        d.h(HARD_PTR, 0); // UCSORTHOREF
        d.bs(0); // UCSORTHOVIEW
        d.h(HARD_PTR, 0); // UCSBASE
        for _ in 0..6 {
            d.bd3([0.0; 3]); // UCSORGTOP BOTTOM LEFT RIGHT FRONT BACK
        }
    };
    ucs(&mut d);
    // model space
    d.bd3([0.0; 3]);
    d.bd3([lo[0], lo[1], 0.0]);
    d.bd3([hi[0], hi[1], 0.0]);
    d.rd2(limits.0);
    d.rd2(limits.1);
    ucs(&mut d);
    d.t(b""); // DIMPOST
    d.t(b""); // DIMAPOST
    dim_vars(&mut d);
    d.h(HARD_PTR, STYLE_STD); // DIMTXSTY
    for _ in 0..4 {
        d.h(HARD_PTR, 0); // DIMLDRBLK DIMBLK DIMBLK1 DIMBLK2
    }
    d.bs(-2); // DIMLWD byblock
    d.bs(-2); // DIMLWE
    for h in [
        BLOCK_CTRL,
        LAYER_CTRL,
        STYLE_CTRL,
        LTYPE_CTRL,
        VIEW_CTRL,
        UCS_CTRL,
        VPORT_CTRL,
        APPID_CTRL,
        DIMSTYLE_CTRL,
        VX_CTRL,
    ] {
        d.h(HARD_OWNER, h);
    }
    d.h(HARD_PTR, GROUP_DICT);
    d.h(HARD_PTR, MLINESTYLE_DICT);
    d.h(HARD_OWNER, NAMED_OBJECTS);
    d.bs(1); // TSTACKALIGN
    d.bs(70); // TSTACKSIZE
    d.t(b""); // HYPERLINKBASE
    d.t(b""); // STYLESHEET
    d.h(HARD_PTR, LAYOUT_DICT);
    d.h(HARD_PTR, PLOTSETTINGS_DICT);
    d.h(HARD_PTR, PLOTSTYLE_DICT);
    // CELWEIGHT bylayer, lineweights not displayed, extended names, named plot styles
    d.bl(0x2A1D);
    d.bs(4); // INSUNITS mm
    d.bs(0); // CEPSNTYPE bylayer
    d.t(guid.as_bytes()); // FINGERPRINTGUID
    d.t(guid.as_bytes()); // VERSIONGUID
    d.h(HARD_PTR, PAPER_SPACE);
    d.h(HARD_PTR, MODEL_SPACE);
    d.h(HARD_PTR, LT_BYLAYER);
    d.h(HARD_PTR, LT_BYBLOCK);
    d.h(HARD_PTR, LT_CONTINUOUS);
    for _ in 0..4 {
        d.bs(-1); // unknown
    }
    d
}

/// A sentinel-wrapped section: start sentinel, RL size, the data, its CRC, end sentinel.
fn section(start: &[u8; 16], data: &[u8], end: &[u8; 16]) -> Vec<u8> {
    let mut out = start.to_vec();
    let mut body = (data.len() as u32).to_le_bytes().to_vec();
    body.extend_from_slice(data);
    let crc = crc16(0xC0C1, &body);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(end);
    out
}

/// A name-derived GUID for the drawing, so the same drawing makes the same file.
fn guid(objs: &[(u32, Vec<u8>)]) -> String {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    let mut g: u64 = 0x8422_2325_CBF2_9CE4;
    for (_, o) in objs {
        for &b in o {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01B3);
            g = (g ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3);
        }
    }
    format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:012X}}}",
        h >> 32,
        (h >> 16) & 0xFFFF,
        h & 0xFFF,
        0x8000 | (g >> 48) & 0x3FFF,
        g & 0xFFFF_FFFF_FFFF
    )
}

/// The object map: (handle, file offset) steps in sections of at most 2032 bytes, each starting
/// again from handle 0 and offset 0.
fn object_map(locs: &[(u32, u64)]) -> Vec<u8> {
    let mut out = vec![];
    let flush = |data: &[u8], out: &mut Vec<u8>| {
        let mut s = ((data.len() + 2) as u16).to_be_bytes().to_vec();
        s.extend_from_slice(data);
        let crc = crc16(0xC0C1, &s);
        out.extend_from_slice(&s);
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let (mut data, mut last_h, mut last_loc) = (vec![], 0u32, 0u64);
    for &(h, loc) in locs {
        let mut step = vec![];
        umc(&mut step, h - last_h);
        mc(&mut step, loc as i64 - last_loc as i64);
        if data.len() + step.len() + 2 > 2032 {
            flush(&data, &mut out);
            data.clear();
            step.clear();
            umc(&mut step, h);
            mc(&mut step, loc as i64);
        }
        data.extend_from_slice(&step);
        last_h = h;
        last_loc = loc;
    }
    if !data.is_empty() {
        flush(&data, &mut out);
    }
    flush(&[], &mut out);
    out
}

fn assemble(
    doc: &Doc,
    p: &Plan,
    objs: Vec<(u32, Vec<u8>)>,
    ext: ([f64; 2], [f64; 2]),
    layer0: u32,
) -> Vec<u8> {
    const RECORDS: usize = 6;
    let file_header_len = 0x19 + 9 * RECORDS + 2 + 16;
    let guid = guid(&objs);

    let aux = aux_header(p.seed);
    let mut preview = SN_PREVIEW.to_vec();
    preview.extend_from_slice(&1u32.to_le_bytes()); // overall size
    preview.push(0); // no images
    preview.extend_from_slice(&SN_PREVIEW_END);
    let mut vars = header_vars(p, ext, doc.limits, layer0, &guid);
    // the header ends on a byte
    while vars.len % 8 != 0 {
        vars.b(false);
    }
    let header = section(&SN_HEADER, &vars.bytes, &SN_HEADER_END);
    let mut cls = BitWriter::new();
    for (k, (cpp, dxf)) in CLASSES.iter().enumerate() {
        cls.bs(500 + k as i16);
        cls.bs(0); // version / proxy flags
        cls.t(b"ObjectDBX Classes");
        cls.t(cpp.as_bytes());
        cls.t(dxf.as_bytes());
        cls.b(false); // was a zombie
        cls.bs(0x1F3); // makes objects
    }
    let classes = section(&SN_CLASSES, &cls.bytes, &SN_CLASSES_END);

    let aux_at = file_header_len;
    let preview_at = aux_at + aux.len();
    let header_at = preview_at + preview.len();
    let classes_at = header_at + header.len();
    let objects_at = classes_at + classes.len() + 0x200;

    let mut body = vec![0u8; 0x200]; // the padding after the classes
    let mut locs = vec![];
    let mut at = objects_at;
    for (h, o) in &objs {
        locs.push((*h, at as u64));
        at += o.len();
        body.extend_from_slice(o);
    }
    let map_at = at;
    let map = object_map(&locs);
    let free_at = map_at + map.len();
    let mut free = vec![];
    free.extend_from_slice(&0u32.to_le_bytes());
    free.extend_from_slice(&(objs.len() as u32).to_le_bytes()); // about how many objects
    free.extend_from_slice(&[0; 8]); // TDUPDATE
    free.extend_from_slice(&(objects_at as u32).to_le_bytes());
    free.push(4);
    for v in [0x32u32, 0, 0x64, 0, 0x200, 0, 0xFFFF_FFFF, 0] {
        free.extend_from_slice(&v.to_le_bytes());
    }
    let second_at = free_at + free.len();
    let sections = [
        (0u8, header_at as u32, header.len() as u32),
        (1, classes_at as u32, classes.len() as u32),
        (2, map_at as u32, map.len() as u32),
        (3, free_at as u32, free.len() as u32),
    ];
    let second = second_header(second_at as u32, &sections, aux_at as u32, aux.len() as u32, p.seed);
    let template_at = second_at + second.len();
    let template = [0u8, 0, 1, 0]; // no description; MEASUREMENT metric

    let mut out = Vec::with_capacity(template_at + 4);
    out.extend_from_slice(&VERSION);
    out.push(1);
    out.extend_from_slice(&(preview_at as u32).to_le_bytes()); // image seeker
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&CODEPAGE_ANSI_1252.to_le_bytes());
    out.extend_from_slice(&(RECORDS as u32).to_le_bytes());
    for (n, a, s) in sections
        .iter()
        .copied()
        .chain([(4, template_at as u32, 4), (5, aux_at as u32, aux.len() as u32)])
    {
        out.push(n);
        out.extend_from_slice(&a.to_le_bytes());
        out.extend_from_slice(&s.to_le_bytes());
    }
    let crc = crc16(0, &out) ^ 0x8461; // six records
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&SN_FILE_HEADER_END);
    debug_assert_eq!(out.len(), file_header_len);
    out.extend_from_slice(&aux);
    out.extend_from_slice(&preview);
    out.extend_from_slice(&header);
    out.extend_from_slice(&classes);
    out.extend_from_slice(&body);
    out.extend_from_slice(&map);
    out.extend_from_slice(&free);
    out.extend_from_slice(&second);
    out.extend_from_slice(&template);
    out
}

/// The auxiliary header (chapter 27).
fn aux_header(seed: u32) -> Vec<u8> {
    let mut a = vec![0xFF, 0x77, 0x01];
    let rs = |a: &mut Vec<u8>, v: u16| a.extend_from_slice(&v.to_le_bytes());
    let rl = |a: &mut Vec<u8>, v: u32| a.extend_from_slice(&v.to_le_bytes());
    rs(&mut a, 23); // AC1015
    rs(&mut a, MAINTENANCE);
    rl(&mut a, 1); // saves
    rl(&mut a, 0xFFFF_FFFF);
    rs(&mut a, 1);
    rs(&mut a, 0);
    rl(&mut a, 0);
    for _ in 0..2 {
        rs(&mut a, 23);
        rs(&mut a, MAINTENANCE);
    }
    for v in [5, 0x893, 5, 0x893, 0, 1] {
        rs(&mut a, v);
    }
    for _ in 0..5 {
        rl(&mut a, 0);
    }
    for _ in 0..4 {
        rl(&mut a, 0); // TDCREATE, TDUPDATE
    }
    rl(&mut a, if seed < 0x7FFF_FFFF { seed } else { 0xFFFF_FFFF });
    rl(&mut a, 0); // educational plot stamp
    rs(&mut a, 0);
    rs(&mut a, 1);
    for _ in 0..3 {
        rl(&mut a, 0);
    }
    rl(&mut a, 1); // saves
    for _ in 0..4 {
        rl(&mut a, 0);
    }
    a
}

/// The second file header (chapter 26): the section locators again and the main handles.
fn second_header(at: u32, sections: &[(u8, u32, u32)], aux_at: u32, aux_len: u32, seed: u32) -> Vec<u8> {
    let mut b = BitWriter::new();
    b.rl(0); // the size, patched below
    b.bl(at as i32);
    b.raw(&VERSION);
    for _ in 0..4 {
        b.b(false);
    }
    b.rc(0x10);
    b.raw(&[0x5C, 0x18, 0xB4, 0x01]);
    b.rc(6);
    for &(n, a, s) in sections.iter().chain(&[(4, 0, 1), (5, aux_at, aux_len)]) {
        b.rc(n);
        b.bl(a as i32);
        b.bl(s as i32);
    }
    let hs = [
        seed,
        BLOCK_CTRL,
        LAYER_CTRL,
        STYLE_CTRL,
        LTYPE_CTRL,
        VIEW_CTRL,
        UCS_CTRL,
        VPORT_CTRL,
        APPID_CTRL,
        DIMSTYLE_CTRL,
        VX_CTRL,
        NAMED_OBJECTS,
        MLINESTYLE_DICT,
        GROUP_DICT,
    ];
    b.bs(hs.len() as i16);
    for (k, h) in hs.iter().enumerate() {
        let bytes = h.to_be_bytes();
        let skip = bytes.iter().take_while(|&&x| x == 0).count().min(3);
        b.rc((4 - skip) as u8);
        b.rc(k as u8);
        b.raw(&bytes[skip..]);
    }
    while b.len % 8 != 0 {
        b.b(false);
    }
    // the size runs from itself to the end sentinel: these bytes, the CRC and 8 more
    let size = (b.bytes.len() + 2 + 8) as u32;
    b.patch_rl(0, size);
    let crc = crc16(0xC0C1, &b.bytes);
    let mut out = SN_SECOND.to_vec();
    out.extend_from_slice(&b.bytes);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    out.extend_from_slice(&SN_SECOND_END);
    out
}
