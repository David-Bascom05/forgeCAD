//! Nominal laminates and the outside coatings are specified independently of plots.
use super::*;
use crate::electronics::{Call, Params, Result, fault, measure, string};
use crate::values::Dim;

#[derive(Debug, Default)]
pub struct Laminate {
    pub usage: Option<String>,
    pub dk: Option<f64>,
    pub df: Option<f64>,
    pub frequency_mhz: Option<f64>,
}
#[derive(Debug)]
pub struct Finish {
    pub material: String,
    pub color: Option<String>,
    pub thickness_nm: i64,
}
pub(super) fn read(c: &mut Call<'_>, p: &Params) -> Result<Laminate> {
    let usage = c
        .take("kind")
        .map(|e| {
            let name = string(e)?;
            let options = if c.name == "copper" {
                &["signal", "plane", "mixed"][..]
            } else {
                &["core", "prepreg"][..]
            };
            if !options.contains(&name.as_str()) {
                return Err(fault(e, "UnknownLayerKind", options.join(", ")));
            }
            Ok(name)
        })
        .transpose()?;
    let mut ratio = |k, minimum, inclusive: bool| {
        c.take(k)
            .map(|e| {
                let n = measure(e, p, Dim::Ratio)?;
                if n < minimum || (!inclusive && n == minimum) {
                    return Err(fault(e, "InvalidLaminateProperty", k));
                }
                Ok(n)
            })
            .transpose()
    };
    Ok(Laminate {
        usage,
        dk: ratio("dk", 1., true)?,
        df: ratio("df", 0., true)?,
        frequency_mhz: ratio("frequency_mhz", 0., false)?,
    })
}
pub(super) fn finish(c: &mut Call<'_>, p: &Params) -> Result<((Side, Film), Finish)> {
    let e = c.req("side")?;
    let side =
        Side::parse(&string(e)?).ok_or_else(|| fault(e, "UnknownBoardSide", "choose top or bottom"))?;
    let e = c.req("kind")?;
    let film = match string(e)?.as_str() {
        "mask" => Film::Mask,
        "silk" => Film::Silk,
        _ => return Err(fault(e, "UnknownSurfaceFinish", "choose mask or silk")),
    };
    Ok((
        (side, film),
        Finish {
            material: reader::label(c.req("material")?)?,
            color: c.take("color").map(reader::label).transpose()?,
            thickness_nm: reader::length(c.req("thickness")?, p)?,
        },
    ))
}
