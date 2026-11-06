//! Non-graphical objects: dictionaries, layouts, styles (chapter 20.4).

use super::Spec;
use crate::bits::R;
use crate::io::{Color, Href, Io};
use crate::str::Str;
use crate::version::Version;

#[derive(Clone, Debug, PartialEq)]
pub struct Dictionary {
    pub cloning: i16,
    pub hard_owner: u8,
    pub items: Vec<(Str, Href)>,
}

impl Default for Dictionary {
    fn default() -> Dictionary {
        Dictionary {
            cloning: 1,
            hard_owner: 0,
            items: vec![],
        }
    }
}

impl Spec for Dictionary {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let mut n = self.items.len() as i32;
        s.bl(&mut n)?;
        s.bs(&mut self.cloning)?;
        s.rc(&mut self.hard_owner)?;
        s.items(&mut self.items, n, &mut |s, it| s.tv(&mut it.0))?;
        for it in &mut self.items {
            s.h(&mut it.1)?;
        }
        Ok(())
    }
}

/// A dictionary with a default entry (ACAD_PLOTSTYLENAME's).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DictionaryWdflt {
    pub dict: Dictionary,
    pub default: Href,
}

impl Spec for DictionaryWdflt {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        self.dict.io(s)?;
        s.h(&mut self.default)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Placeholder;

impl Spec for Placeholder {
    fn io<S: Io>(&mut self, _: &mut S) -> R<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MlineLine {
    pub offset: f64,
    pub color: Color,
    pub ltype_index: i16,
    pub ltype: Href,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MlineStyle {
    pub name: Str,
    pub description: Str,
    pub flags: i16,
    pub fill: Color,
    pub start_angle: f64,
    pub end_angle: f64,
    pub lines: Vec<MlineLine>,
}

impl Spec for MlineStyle {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        s.tv(&mut self.name)?;
        s.tv(&mut self.description)?;
        s.bs(&mut self.flags)?;
        s.cmc(&mut self.fill)?;
        s.bd(&mut self.start_angle)?;
        s.bd(&mut self.end_angle)?;
        let mut n = self.lines.len() as u8;
        s.rc(&mut n)?;
        s.items(&mut self.lines, n as i32, &mut |s, l| {
            s.bd(&mut l.offset)?;
            s.cmc(&mut l.color)?;
            if v >= Version::R2018 {
                s.h(&mut l.ltype)
            } else {
                s.bs(&mut l.ltype_index)
            }
        })
    }
}

/// A layout's plot settings (PLOTSETTINGS has the same).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PlotSettings {
    pub page_setup: Str,
    pub printer: Str,
    pub flags: i16,
    pub margins: [f64; 4],
    pub paper: [f64; 2],
    pub paper_size: Str,
    pub origin: [f64; 2],
    pub paper_units: i16,
    pub rotation: i16,
    pub plot_type: i16,
    pub window_min: [f64; 2],
    pub window_max: [f64; 2],
    /// R2000 and before; a handle since.
    pub view_name: Str,
    pub real_units: f64,
    pub drawing_units: f64,
    pub style_sheet: Str,
    pub scale_type: i16,
    pub scale: f64,
    pub image_origin: [f64; 2],
    pub shade_mode: i16,
    pub shade_level: i16,
    pub shade_dpi: i16,
    pub view: Href,
}

pub fn plot_settings_io<S: Io>(p: &mut PlotSettings, s: &mut S) -> R<()> {
    let v = s.ver();
    s.tv(&mut p.page_setup)?;
    s.tv(&mut p.printer)?;
    s.bs(&mut p.flags)?;
    for m in &mut p.margins {
        s.bd(m)?;
    }
    s.bd2(&mut p.paper)?;
    s.tv(&mut p.paper_size)?;
    s.bd2(&mut p.origin)?;
    s.bs(&mut p.paper_units)?;
    s.bs(&mut p.rotation)?;
    s.bs(&mut p.plot_type)?;
    s.bd2(&mut p.window_min)?;
    s.bd2(&mut p.window_max)?;
    if v <= Version::R2000 {
        s.t8(&mut p.view_name)?;
    }
    s.bd(&mut p.real_units)?;
    s.bd(&mut p.drawing_units)?;
    s.tv(&mut p.style_sheet)?;
    s.bs(&mut p.scale_type)?;
    s.bd(&mut p.scale)?;
    s.bd2(&mut p.image_origin)?;
    if v >= Version::R2004 {
        s.bs(&mut p.shade_mode)?;
        s.bs(&mut p.shade_level)?;
        s.bs(&mut p.shade_dpi)?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Layout {
    pub plot: PlotSettings,
    pub name: Str,
    pub tab_order: i32,
    pub flags: i16,
    pub ucs_origin: [f64; 3],
    pub limmin: [f64; 2],
    pub limmax: [f64; 2],
    pub insbase: [f64; 3],
    pub ucs_x: [f64; 3],
    pub ucs_y: [f64; 3],
    pub elevation: f64,
    pub ortho: i16,
    pub extmin: [f64; 3],
    pub extmax: [f64; 3],
    pub visual_style: Href,
    pub block: Href,
    pub active_viewport: Href,
    pub base_ucs: Href,
    pub named_ucs: Href,
    pub viewports: Vec<Href>,
}

impl Spec for Layout {
    fn io<S: Io>(&mut self, s: &mut S) -> R<()> {
        let v = s.ver();
        plot_settings_io(&mut self.plot, s)?;
        s.tv(&mut self.name)?;
        s.bl(&mut self.tab_order)?;
        s.bs(&mut self.flags)?;
        s.bd3(&mut self.ucs_origin)?;
        s.rd2(&mut self.limmin)?;
        s.rd2(&mut self.limmax)?;
        s.bd3(&mut self.insbase)?;
        s.bd3(&mut self.ucs_x)?;
        s.bd3(&mut self.ucs_y)?;
        s.bd(&mut self.elevation)?;
        s.bs(&mut self.ortho)?;
        s.bd3(&mut self.extmin)?;
        s.bd3(&mut self.extmax)?;
        let mut nvp = self.viewports.len() as i32;
        if v >= Version::R2004 {
            // a BL, though the spec says RL
            s.bl(&mut nvp)?;
            s.h(&mut self.plot.view)?;
        }
        if v >= Version::R2007 {
            s.h(&mut self.visual_style)?;
        }
        s.h(&mut self.block)?;
        s.h(&mut self.active_viewport)?;
        s.h(&mut self.base_ucs)?;
        s.h(&mut self.named_ucs)?;
        if v >= Version::R2004 {
            s.items(&mut self.viewports, nvp, &mut |s, h| s.h(h))?;
        }
        Ok(())
    }
}
