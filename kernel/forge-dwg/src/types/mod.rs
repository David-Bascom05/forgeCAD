//! Each object type's own fields (chapter 20.4), one description for reading and writing.

mod entities;
mod entities_a;
mod entities_b;
mod objects;
mod tables;

pub use entities::*;
pub use entities_a::*;
pub use entities_b::*;
pub use objects::*;
pub use tables::*;

use crate::bits::R;
use crate::io::Io;
use crate::object::Classes;

/// A type's fields after the common ones, walked in file order.
pub trait Spec {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()>;
}

macro_rules! data {
    (
        fixed { $($fv:ident = $code:literal),* $(,)? }
        only_classes { $($ov:ident),* $(,)? }
        classes { $($name:literal => $cv:ident),* $(,)? }
    ) => {
        /// An object's own fields, by type.
        #[derive(Clone, Debug, PartialEq)]
        pub enum Data {
            $($fv($fv),)*
            $($ov($ov),)*
            /// A type this does not decode: the object is kept as read.
            Raw(u16),
        }

        impl Data {
            /// An empty value of the type a number (and the classes) names.
            pub fn of(type_code: u16, classes: &Classes) -> Data {
                match type_code {
                    $($code => Data::$fv(Default::default()),)*
                    t => match classes.name(t) {
                        $(Some($name) => Data::$cv(Default::default()),)*
                        _ => Data::Raw(t),
                    },
                }
            }
            pub fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
                match self {
                    $(Data::$fv(x) => x.io(s),)*
                    $(Data::$ov(x) => x.io(s),)*
                    Data::Raw(_) => Ok(()),
                }
            }
            /// The type's DXF-ish name, for reports.
            pub fn name(&self) -> String {
                match self {
                    $(Data::$fv(_) => stringify!($fv).to_string(),)*
                    $(Data::$ov(_) => stringify!($ov).to_string(),)*
                    Data::Raw(t) => format!("raw {t}"),
                }
            }
        }
    };
}

data! {
    fixed {
        Text = 1,
        Attrib = 2,
        AttDef = 3,
        Block = 4,
        EndBlk = 5,
        SeqEnd = 6,
        Insert = 7,
        MInsert = 8,
        Vertex2d = 10,
        Vertex3d = 11,
        VertexMesh = 12,
        VertexPface = 13,
        VertexPfaceFace = 14,
        Polyline2d = 15,
        Polyline3d = 16,
        Arc = 17,
        Circle = 18,
        Line = 19,
        DimOrdinate = 20,
        DimLinear = 21,
        DimAligned = 22,
        DimAng3Pt = 23,
        DimAng2Ln = 24,
        DimRadius = 25,
        DimDiameter = 26,
        Point = 27,
        Face3d = 28,
        PolylinePface = 29,
        PolylineMesh = 30,
        Solid = 31,
        Trace = 32,
        Shape = 33,
        Viewport = 34,
        Ellipse = 35,
        Spline = 36,
        Region = 37,
        Solid3d = 38,
        Body = 39,
        Ray = 40,
        Xline = 41,
        Dictionary = 42,
        MText = 44,
        Leader = 45,
        Tolerance = 46,
        MLine = 47,
        BlockControl = 48,
        BlockHeader = 49,
        LayerControl = 50,
        Layer = 51,
        StyleControl = 52,
        Style = 53,
        LtypeControl = 56,
        Ltype = 57,
        ViewControl = 60,
        UcsControl = 62,
        VportControl = 64,
        Vport = 65,
        AppIdControl = 66,
        AppId = 67,
        DimStyleControl = 68,
        DimStyle = 69,
        VxControl = 70,
        MlineStyle = 73,
        Ole2Frame = 74,
        LwPolyline = 77,
        Hatch = 78,
        Placeholder = 80,
        Layout = 82,
        ProxyEntity = 498,
        ProxyObject = 499,
    }
    only_classes {
        DictionaryWdflt,
        ArcDimension,
        LargeRadialDimension,
        Image,
        Wipeout,
    }
    classes {
        "ACAD_PROXY_ENTITY" => ProxyEntity,
        "ACAD_PROXY_OBJECT" => ProxyObject,
        "ACDBDICTIONARYWDFLT" => DictionaryWdflt,
        "ACDBPLACEHOLDER" => Placeholder,
        "ARC_DIMENSION" => ArcDimension,
        "HATCH" => Hatch,
        "IMAGE" => Image,
        "LARGE_RADIAL_DIMENSION" => LargeRadialDimension,
        "LAYOUT" => Layout,
        "LWPOLYLINE" => LwPolyline,
        "WIPEOUT" => Wipeout,
    }
}
