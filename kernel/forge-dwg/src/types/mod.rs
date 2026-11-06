//! Each object type's own fields (chapter 20.4), one description for reading and writing.

mod entities;
mod objects;
mod objects_b;
mod tables;

pub use entities::*;
pub use objects::*;
pub use objects_b::*;
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
        Block = 4,
        EndBlk = 5,
        Arc = 17,
        Circle = 18,
        Line = 19,
        Point = 27,
        Solid = 31,
        Ellipse = 35,
        Spline = 36,
        Dictionary = 42,
        BlockControl = 48,
        BlockHeader = 49,
        LayerControl = 50,
        Layer = 51,
        StyleControl = 52,
        Style = 53,
        LtypeControl = 56,
        Ltype = 57,
        ViewControl = 60,
        View = 61,
        UcsControl = 62,
        Ucs = 63,
        VportControl = 64,
        Vport = 65,
        AppIdControl = 66,
        AppId = 67,
        DimStyleControl = 68,
        DimStyle = 69,
        VxControl = 70,
        Vx = 71,
        Group = 72,
        MlineStyle = 73,
        LwPolyline = 77,
        Xrecord = 79,
        Placeholder = 80,
        Layout = 82,
    }
    only_classes {
        DictionaryWdflt,
        DictionaryVar,
        Scale,
        IdBuffer,
        LayerIndex,
        SpatialFilter,
        SpatialIndex,
        SortEntsTable,
        ImageDef,
        ImageDefReactor,
        RasterVariables,
        WipeoutVariables,
        Field,
        FieldList,
        GeoData,
        PlotSettings,
    }
    classes {
        "ACDBDICTIONARYWDFLT" => DictionaryWdflt,
        "ACDBPLACEHOLDER" => Placeholder,
        "DICTIONARYVAR" => DictionaryVar,
        "FIELD" => Field,
        "FIELDLIST" => FieldList,
        "GEODATA" => GeoData,
        "IDBUFFER" => IdBuffer,
        "IMAGEDEF" => ImageDef,
        "IMAGEDEF_REACTOR" => ImageDefReactor,
        "LAYER_INDEX" => LayerIndex,
        "PLOTSETTINGS" => PlotSettings,
        "RASTERVARIABLES" => RasterVariables,
        "SCALE" => Scale,
        "SORTENTSTABLE" => SortEntsTable,
        "SPATIAL_FILTER" => SpatialFilter,
        "SPATIAL_INDEX" => SpatialIndex,
        "WIPEOUTVARIABLES" => WipeoutVariables,
        "XRECORD" => Xrecord,
        "LAYOUT" => Layout,
        "LWPOLYLINE" => LwPolyline,
    }
}
