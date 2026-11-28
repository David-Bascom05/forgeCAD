//! Editor layer identity is independent of stack position, display name, and output policy.
use super::*;
use crate::electronics::{Call, Fault, Params, Result, fault, measure, string};
use crate::values::Dim;

macro_rules! roles {
    ($name:ident { $($v:ident => $s:literal),+ }) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name { $($v),+ }
        impl $name {
            pub const ALL: &[Self] = &[$(Self::$v),+];
            pub fn name(self) -> &'static str { match self { $(Self::$v => $s),+ } }
            pub(super) fn parse(s: &str) -> Option<Self> { Self::ALL.iter().copied().find(|r| r.name() == s) }
        }
    }
}
roles!(CopperObject { Track => "track", Via => "via", Smd => "smd", Pth => "pth", Region => "region", Pour => "pour" });
roles!(PairRole { Adhesive => "adhesive", Fab => "fab", Assembly => "assembly", Coating => "coating", Gold => "gold", Carbon => "carbon", Silver => "silver", Tin => "tin", Heatsink => "heatsink", Peelable => "peelable", Designator => "designator", Value => "value", Body => "body", Dimensions => "dimensions" });
roles!(DrawingRole { Margin => "margin", Drawings => "drawings", Comments => "comments", Eco1 => "eco1", Eco2 => "eco2", FabNotes => "fab_notes", AssemblyNotes => "assembly_notes", Dimensions => "dimensions", Sheet => "sheet", RoutePath => "route_path", VScore => "v_score" });

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlotLayer {
    Copper(String),
    Film(Side, Film),
    Paired(Side, PairRole),
    Courtyard(Side),
    Center(Side),
    Drawing(DrawingRole),
    User(String),
    Profile,
    PlatedDrill,
    NonPlatedDrill,
    Keepout,
    Airwires,
    DrillGuide,
}
impl PlotLayer {
    pub fn id(&self) -> String {
        match self {
            Self::Copper(s) => format!("copper.{s}"),
            Self::Film(s, f) => format!("{}.{}", s.name(), f.name()),
            Self::Paired(s, r) => format!("{}.{}", s.name(), r.name()),
            Self::Courtyard(s) => format!("{}.courtyard", s.name()),
            Self::Center(s) => format!("{}.center", s.name()),
            Self::Drawing(r) => r.name().into(),
            Self::User(s) => format!("user.{s}"),
            Self::Profile => "profile".into(),
            Self::PlatedDrill => "plated_drill".into(),
            Self::NonPlatedDrill => "nonplated_drill".into(),
            Self::Keepout => "keepout".into(),
            Self::Airwires => "airwires".into(),
            Self::DrillGuide => "drill_guide".into(),
        }
    }
    pub fn group(&self) -> &'static str {
        match self {
            Self::Copper(_) => "Copper",
            Self::Film(..) | Self::Paired(_, PairRole::Adhesive) => "Surface",
            Self::Paired(
                _,
                PairRole::Peelable
                | PairRole::Carbon
                | PairRole::Silver
                | PairRole::Tin
                | PairRole::Heatsink
                | PairRole::Gold
                | PairRole::Coating,
            ) => "Special processes",
            Self::Airwires => "Connectivity",
            Self::DrillGuide | Self::Profile | Self::PlatedDrill | Self::NonPlatedDrill => "Machining",
            Self::Courtyard(_) | Self::Keepout => "Constraints",
            Self::Paired(..) | Self::Center(_) => "Assembly",
            Self::Drawing(_) | Self::User(_) => "Documentation",
        }
    }
    pub fn title(&self, board: &Board) -> String {
        if let Self::User(s) = self {
            return board.layout.user_layers[s].0.clone();
        }
        let label = match self {
            Self::Copper(id) => format!("{id} copper"),
            Self::Film(side, film) => format!(
                "{} {}",
                side.name(),
                match film {
                    Film::Mask => "solder mask",
                    Film::Paste => "solder paste",
                    Film::Silk => "silkscreen",
                }
            ),
            Self::PlatedDrill => "Plated holes".into(),
            Self::NonPlatedDrill => "Nonplated holes".into(),
            Self::Profile => "Board profile".into(),
            Self::Keepout => "Copper keepouts".into(),
            Self::Center(side) => format!("{} placement centers", side.name()),
            _ => self.id().replace(['.', '_'], " "),
        };
        let mut chars = label.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => label,
        }
    }
}

#[derive(Debug)]
pub struct LayerGraphic {
    pub layer: PlotLayer,
    pub shapes: Vec<Shape>,
    pub span: Span,
}

#[derive(Debug)]
pub(super) enum Primitive {
    Dot(Point, i64),
    Region(Vec<Point>),
    Line(Vec<Point>, i64),
    Text {
        at: Point,
        value: String,
        height: i64,
        width: i64,
        angle: f64,
    },
}
pub(super) const DOT: &[&str] = &["layer", "at", "diameter"];
pub(super) const LINE: &[&str] = &["layer", "points", "width"];
pub(super) const REGION: &[&str] = &["layer", "points"];
pub(super) const TEXT: &[&str] = &["layer", "at", "value", "height", "width", "angle"];
pub(super) const PRIMITIVES: &[(&str, &[&str])] = &[
    ("layer_dot", DOT),
    ("layer_line", LINE),
    ("layer_region", REGION),
    ("layer_text", TEXT),
];

impl Primitive {
    pub(super) fn read(c: &mut Call<'_>, p: &Params) -> Result<Self> {
        Ok(match c.name {
            "layer_dot" => Self::Dot(
                reader::coordinate(c.req("at")?, p)?,
                reader::length(c.req("diameter")?, p)?,
            ),
            "layer_text" => Self::Text {
                at: reader::coordinate(c.req("at")?, p)?,
                value: reader::label(c.req("value")?)?,
                height: reader::length(c.req("height")?, p)?,
                width: reader::length(c.req("width")?, p)?,
                angle: c
                    .take("angle")
                    .map(|e| measure(e, p, Dim::Angle))
                    .transpose()?
                    .unwrap_or(0.),
            },
            "layer_line" => {
                let ps = reader::points(c.req("points")?, p)?;
                if ps.len() < 2 {
                    return Err(Fault::new(
                        "InvalidLayerLine",
                        c.span,
                        "a line needs at least two points",
                    ));
                }
                Self::Line(ps, reader::length(c.req("width")?, p)?)
            }
            _ => Self::Region(reader_layout::polygon(c.req("points")?, p)?),
        })
    }
    pub(super) fn shapes(&self, pose: Pose, reference: &str, span: Span) -> Result<Vec<Shape>> {
        let transform = |ps: &[Point]| {
            ps.iter()
                .map(|&p| parts::transform(pose, p, span))
                .collect::<Result<Vec<_>>>()
        };
        Ok(match self {
            Self::Dot(pt, d) => vec![Shape::disk(parts::transform(pose, *pt, span)?, *d)],
            Self::Region(ps) => vec![Shape::Region(transform(ps)?)],
            Self::Line(ps, w) => transform(ps)?
                .windows(2)
                .map(|p| Shape::Stroke {
                    axis: [p[0], p[1]],
                    width: *w,
                })
                .collect(),
            Self::Text {
                at,
                value,
                height,
                width,
                angle,
            } => parts::legend(
                &value.replace("{ref}", reference),
                *height,
                *width,
                Pose {
                    at: parts::transform(pose, *at, span)?,
                    angle: pose.angle + if pose.side == Side::Top { *angle } else { -angle },
                    ..pose
                },
                span,
            )?,
        })
    }
}
pub(super) fn footprint_graphics(c: &mut Call<'_>, p: &Params) -> Result<Vec<(PairRole, Primitive)>> {
    let mut graphics = Vec::new();
    if let Some(e) = c.take("artwork") {
        for el in crate::electronics::list(e)? {
            let mut g = Call::with_signature(el, PRIMITIVES)?;
            let key = g.req("layer")?;
            let role = PairRole::parse(&string(key)?).ok_or_else(|| {
                fault(
                    key,
                    "UnknownFootprintLayer",
                    "choose a paired artwork role, e.g. fab, assembly or adhesive",
                )
            })?;
            graphics.push((role, Primitive::read(&mut g, p)?));
            g.end()?;
        }
    }
    Ok(graphics)
}
pub(super) fn board_graphic(c: &mut Call<'_>, p: &Params, layout: &Layout) -> Result<LayerGraphic> {
    let e = c.req("layer")?;
    let id = string(e)?;
    let unknown = || fault(e, "UnknownDrawingLayer", format!("unknown drawing layer {id}"));
    let layer = if let Some((prefix, role)) = id.split_once('.') {
        if prefix == "user" && layout.user_layers.contains_key(role) {
            PlotLayer::User(role.into())
        } else {
            PlotLayer::Paired(
                Side::parse(prefix).ok_or_else(unknown)?,
                PairRole::parse(role).ok_or_else(unknown)?,
            )
        }
    } else {
        PlotLayer::Drawing(DrawingRole::parse(&id).ok_or_else(unknown)?)
    };
    let primitive = Primitive::read(c, p)?;
    let pose = Pose {
        at: [0, 0],
        side: Side::Top,
        angle: 0.,
    };
    // Board coordinates are fixed. Only the glyph orientation follows the board face.
    let shapes = if let (
        PlotLayer::Paired(Side::Bottom, _),
        Primitive::Text {
            at,
            value,
            height,
            width,
            angle,
        },
    ) = (&layer, &primitive)
    {
        parts::legend(
            value,
            *height,
            *width,
            Pose {
                at: *at,
                side: Side::Bottom,
                angle: *angle,
            },
            c.span,
        )?
    } else {
        primitive.shapes(pose, "{ref}", c.span)?
    };
    Ok(LayerGraphic {
        layer,
        shapes,
        span: c.span,
    })
}
impl Board {
    pub fn plot_layers(&self) -> Vec<PlotLayer> {
        let mut out: Vec<_> = self
            .layers
            .iter()
            .filter(|l| matches!(l.material, LayerMaterial::Copper { .. }))
            .map(|l| PlotLayer::Copper(l.id.clone()))
            .collect();
        for side in Side::BOTH {
            out.extend(Film::ALL.map(|f| PlotLayer::Film(side, f)));
            out.extend(PairRole::ALL.iter().map(|&r| PlotLayer::Paired(side, r)));
            out.extend([PlotLayer::Courtyard(side), PlotLayer::Center(side)]);
        }
        out.extend([
            PlotLayer::Profile,
            PlotLayer::PlatedDrill,
            PlotLayer::NonPlatedDrill,
            PlotLayer::Keepout,
            PlotLayer::Airwires,
            PlotLayer::DrillGuide,
        ]);
        out.extend(DrawingRole::ALL.iter().copied().map(PlotLayer::Drawing));
        out.extend(self.layout.user_layers.keys().cloned().map(PlotLayer::User));
        out
    }
    pub fn plot(&self, layer: &PlotLayer) -> Vec<artwork::Mark<'_>> {
        use artwork::Mark;
        let outline = |id, ps: &[Point], span| {
            geometry::sides(ps)
                .map(|axis| Mark::new(id, None, span, Shape::Stroke { axis, width: 50_000 }))
                .collect::<Vec<_>>()
        };
        match layer {
            PlotLayer::Copper(id) => self.copper(id),
            PlotLayer::Film(s, f) => self.film(*s, *f),
            PlotLayer::PlatedDrill | PlotLayer::NonPlatedDrill => self
                .drills()
                .into_iter()
                .filter(|m| m.net.is_some() == matches!(layer, PlotLayer::PlatedDrill))
                .collect(),
            PlotLayer::Profile => outline("edge", &self.outline, Span::default())
                .into_iter()
                .chain(
                    self.layout
                        .cutouts
                        .iter()
                        .flat_map(|(id, ps)| outline(id, ps, Span::default())),
                )
                .collect(),
            PlotLayer::Courtyard(side) => self
                .layout
                .placements
                .iter()
                .filter(|(_, p)| p.pose.side == *side)
                .flat_map(|(id, p)| match &p.courtyard {
                    Some(Shape::Region(ps)) => outline(id, ps, p.span),
                    _ => Vec::new(),
                })
                .collect(),
            PlotLayer::Center(side) => self
                .layout
                .placements
                .iter()
                .filter(|(_, p)| p.pose.side == *side)
                .flat_map(|(id, p)| {
                    let [x, y] = p.pose.at;
                    [
                        [[x - 250_000, y], [x + 250_000, y]],
                        [[x, y - 250_000], [x, y + 250_000]],
                    ]
                    .map(|axis| Mark::new(id, None, p.span, Shape::Stroke { axis, width: 50_000 }))
                })
                .collect(),
            PlotLayer::Airwires => report::airwires(self)
                .into_iter()
                .map(|(net, a, b, _)| {
                    Mark::new(
                        a,
                        Some(net),
                        self.layout.pads[a].span,
                        Shape::Stroke {
                            axis: [self.layout.pads[a].at, self.layout.pads[b].at],
                            width: 40_000,
                        },
                    )
                })
                .collect(),
            PlotLayer::DrillGuide => self
                .drills()
                .into_iter()
                .flat_map(|m| {
                    let Shape::Stroke { axis: [a, b], width } = m.shape else {
                        unreachable!()
                    };
                    let x = (a[0] + b[0]) / 2;
                    let y = (a[1] + b[1]) / 2;
                    let d = width / 2;
                    [[[x - d, y], [x + d, y]], [[x, y - d], [x, y + d]]]
                        .map(|axis| Mark::new(m.id, None, m.span, Shape::Stroke { axis, width: 50_000 }))
                })
                .collect(),
            PlotLayer::Keepout => self
                .layout
                .areas
                .iter()
                .filter(|(_, a)| a.net.is_none())
                .flat_map(|(id, a)| outline(id, &a.polygon, a.span))
                .collect(),
            _ => self
                .layout
                .layer_graphics
                .iter()
                .filter(|(_, g)| &g.layer == layer)
                .flat_map(|(id, g)| {
                    g.shapes
                        .iter()
                        .cloned()
                        .map(move |s| Mark::new(id, None, g.span, s))
                })
                .collect(),
        }
    }
}
