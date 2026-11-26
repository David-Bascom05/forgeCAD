//! Resolve service geometry in the same registry and units as other vendor work.
use super::*;
use crate::electronics::{Call, Fault, Params, Result, fault, string};
use std::path::Path;

#[derive(Debug)]
pub struct SelectedService {
    pub vendor: String,
    pub service: forge_vendor::PcbService,
    pub span: Span,
}

impl SelectedService {
    pub(super) fn check_stack(&self, layers: &[Layer]) -> Result<()> {
        let copper: Vec<_> = layers
            .iter()
            .filter_map(|l| match l.material {
                LayerMaterial::Copper { weight_oz_ft2 } => Some(weight_oz_ft2),
                _ => None,
            })
            .collect();
        let s = &self.service;
        let external =
            |l: Option<&Layer>| l.is_some_and(|l| matches!(l.material, LayerMaterial::Copper { .. }));
        if copper.len() != s.layers
            || !external(layers.first())
            || !external(layers.last())
            || copper.iter().enumerate().any(|(i, &weight)| {
                Some(weight)
                    != if i == 0 || i + 1 == copper.len() {
                        Some(s.outer_oz)
                    } else {
                        s.inner_oz
                    }
            })
        {
            return Err(Fault::new(
                "VendorStackMismatch",
                self.span,
                format!(
                    "{} / {} requires {} copper layers, {} oz outer{}",
                    self.vendor,
                    s.id,
                    s.layers,
                    s.outer_oz,
                    s.inner_oz.map(|n| format!(", {n} oz inner")).unwrap_or_default()
                ),
            ));
        }
        Ok(())
    }
}

/// The normal vendor precedence in a local session. A fenced project can supply
/// its own regular TOML files, inside the fence, ahead of the built-ins.
pub fn profiles(dir: Option<&Path>, root: Option<&Path>) -> std::sync::Arc<forge_vendor::Profiles> {
    let Some(root) = root else {
        return crate::vendor::document_profiles(dir, false);
    };
    let mut profiles = forge_vendor::Profiles::builtin();
    let Some((fence, folder)) = forge_dirs::fs::canonicalize(root)
        .ok()
        .zip(dir.map(forge_vendor::project_dir))
    else {
        return profiles.into();
    };
    let Ok(folder) = forge_dirs::fs::canonicalize(folder) else {
        return profiles.into();
    };
    if !folder.starts_with(&fence) {
        return profiles.into();
    }
    let Ok(entries) = forge_dirs::fs::read_dir(folder) else {
        return profiles.into();
    };
    let mut paths: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|s| s == "toml"))
        .collect();
    paths.sort();
    let mut project = Vec::new();
    for p in paths {
        let Ok(path) = forge_dirs::fs::canonicalize(p) else {
            continue;
        };
        if !path.starts_with(&fence) {
            continue;
        }
        let Ok(bytes) = crate::deps::read_plain(&path, 4 << 20) else {
            continue;
        };
        crate::deps::read(&path, Some(&bytes));
        if let Ok(src) = std::str::from_utf8(&bytes) {
            match forge_vendor::Vendor::parse(src) {
                Ok(vendor) => project.push(forge_vendor::Entry {
                    lib: forge_vendor::Lib::Project,
                    file: Some(path),
                    vendor,
                }),
                Err(err) => profiles.errors.push((path, err)),
            }
        }
    }
    project.append(&mut profiles.list);
    profiles.list = project;
    profiles.into()
}

pub(super) fn read(
    c: &mut Call<'_>,
    id: &str,
    params: &Params,
    dir: Option<&Path>,
    root: Option<&Path>,
) -> Result<Process> {
    let mut scale = 1.;
    let selected = if let Some(e) = c.take("vendor") {
        let profiles = profiles(dir, root);
        let name = string(e)?;
        let v = &profiles
            .find(&name)
            .ok_or_else(|| fault(e, "UnknownPcbVendor", format!("no vendor {name}")))?
            .vendor;
        let service_expr = c.req("service")?;
        let service_id = string(service_expr)?;
        let service = v
            .pcb
            .iter()
            .find(|s| s.id == service_id)
            .ok_or_else(|| {
                fault(
                    service_expr,
                    "UnknownPcbService",
                    format!(
                        "{} offers [{}]",
                        v.name,
                        v.pcb.iter().map(|s| s.id.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                )
            })?
            .clone();
        scale = v.units.mm();
        Some(SelectedService {
            vendor: v.id.clone(),
            service,
            span: e.span,
        })
    } else {
        None
    };
    let s = selected.as_ref().map(|v| &v.service);
    let span = c.span;
    let mut limit = |key, floor: Option<f64>| -> Result<Option<i64>> {
        let preset = floor
            .map(|v| {
                let nm = scale * v * 1e6;
                if !nm.is_finite() || !(0.0..=500_000_000.).contains(&nm) {
                    return Err(Fault::new(
                        "VendorRuleRange",
                        c.span,
                        format!("invalid service {key}"),
                    ));
                }
                Ok((nm - 1e-7).ceil() as i64)
            })
            .transpose()?;
        let own = c.take(key).map(|e| reader::length(e, params)).transpose()?;
        Ok(own.max(preset))
    };
    let min_slot_nm = limit("min_slot", s.and_then(|s| s.min_slot))?;
    let inner_drill_clearance_nm = limit("inner_drill_clearance", s.and_then(|s| s.inner_drill_clearance))?;
    let min_edge_nm = limit("min_edge", s.and_then(|s| s.min_edge))?;
    let min_mask_web_nm = limit("min_mask_web", s.and_then(|s| s.min_mask_web))?;
    let hole_clearance_nm = limit("hole_clearance", None)?;
    let silk_clearance_nm = limit("silk_clearance", None)?;
    let mut required = |k, v| {
        limit(k, v)?.ok_or_else(|| Fault::new("MissingArgument", span, format!("process requires {k}")))
    };
    Ok(Process {
        id: id.into(),
        min_slot_nm,
        inner_drill_clearance_nm,
        min_edge_nm,
        min_mask_web_nm,
        hole_clearance_nm,
        silk_clearance_nm,
        min_width_nm: required("min_width", s.map(|s| s.min_width))?,
        min_spacing_nm: required("min_spacing", s.map(|s| s.min_spacing))?,
        min_drill_nm: required("min_drill", s.map(|s| s.min_drill))?,
        min_annular_ring_nm: required("min_annular_ring", s.map(|s| s.min_annular_ring))?,
        selected,
    })
}
