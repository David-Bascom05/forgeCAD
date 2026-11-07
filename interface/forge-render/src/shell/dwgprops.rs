//! Editing through the palettes: the Properties palette's fields (an entity's layer, colour,
//! line type, lineweight and its main geometry; the common ones for many), the Layers palette's
//! switches, colours, new and deleted layers, and the current layer (CLAYER). Each change is one
//! undo step through the editing core, redrawn as a command's is.

use super::dwgdoc::{DwgDoc, colour_name, entry_name, lineweight_name};
use forge_dwg::bits::R;
use forge_dwg::edit::History;
use forge_dwg::file::Drawing;
use forge_dwg::header::Val;
use forge_dwg::io::{Color, Href};
use forge_dwg::types::Data;

/// A layer's switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    On,
    Freeze,
    Lock,
}

/// A row of the Layers palette, from the drawing's layer table.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerRow {
    pub handle: u32,
    pub name: String,
    pub aci: i16,
    pub rgb: [u8; 3],
    pub on: bool,
    pub frozen: bool,
    pub locked: bool,
    pub plot: bool,
    pub current: bool,
    pub ltype: String,
    pub weight: String,
}

/// The lineweights AutoCAD offers (hundredths of a millimetre), by their stored index.
const WEIGHTS: usize = 24;

fn number(v: &str) -> Result<f64, String> {
    let t = v.trim().trim_end_matches('\u{b0}').trim();
    t.parse::<f64>().map_err(|_| format!("\"{v}\" is not a number"))
}

/// A colour as typed: ByLayer, ByBlock, a name (Red..White) or an index 1..255.
pub fn parse_colour(v: &str) -> Option<i16> {
    let w = v.trim().to_ascii_lowercase();
    Some(match w.as_str() {
        "bylayer" => 256,
        "byblock" => 0,
        "red" => 1,
        "yellow" => 2,
        "green" => 3,
        "cyan" => 4,
        "blue" => 5,
        "magenta" => 6,
        "white" => 7,
        _ => match w.parse::<i16>() {
            Ok(k) if (1..=255).contains(&k) => k,
            _ => return None,
        },
    })
}

/// A layer's colour index: R2004 on keep it in the colour's method byte (0xC3), the index field
/// 0.
pub fn layer_aci(l: &forge_dwg::types::Layer) -> i16 {
    match l.color.rgb {
        Some(c) if c >> 24 == 0xC3 => (c & 0xFF) as i16,
        _ => l.color.index.abs(),
    }
}

/// Is a layer on? R2000 says off by a negative colour, R2004 on by a flag too.
pub fn layer_on(l: &forge_dwg::types::Layer) -> bool {
    l.color.index >= 0 && l.flags & 2 == 0
}

impl DwgDoc {
    /// One palette edit as an undo step, redrawn.
    pub fn edit(&mut self, f: impl FnOnce(&mut Drawing, &mut History) -> R<()>) -> Result<(), String> {
        if self.ed.active() {
            self.cancel_command();
        }
        self.ed.hist.begin();
        let r = f(&mut self.drawing, &mut self.ed.hist);
        let touched = self.ed.hist.touched();
        self.ed.hist.end();
        self.sync(&touched);
        self.refresh_grips();
        r.map_err(|e| e.0)
    }

    /// The layers, in the table's order.
    pub fn layer_rows(&self) -> Vec<LayerRow> {
        let d = &self.drawing;
        let current = self.current_layer();
        let hs: Vec<u32> = match d
            .header
            .handle("LAYER_CONTROL")
            .and_then(|h| d.objects.get(&h))
            .map(|o| &o.data)
        {
            Some(Data::LayerControl(c)) => c.0.entries.iter().map(|x| x.handle).collect(),
            _ => vec![],
        };
        hs.into_iter()
            .filter_map(|h| match d.objects.get(&h).map(|o| &o.data) {
                Some(Data::Layer(l)) => Some(LayerRow {
                    handle: h,
                    name: l.entry.name.text(),
                    aci: layer_aci(l),
                    rgb: match l.color.rgb {
                        Some(c) if c >> 24 == 0xC2 => [(c >> 16) as u8, (c >> 8) as u8, c as u8],
                        _ => forge_dwg::scene::aci_rgb(layer_aci(l)),
                    },
                    on: layer_on(l),
                    frozen: l.flags & 1 != 0,
                    locked: l.flags & 8 != 0,
                    plot: l.flags & 16 != 0,
                    current: h == current,
                    ltype: entry_name(d, l.ltype).unwrap_or_else(|| "Continuous".into()),
                    weight: lineweight_name(((l.flags as u16 >> 5) & 31) as u8),
                }),
                _ => None,
            })
            .collect()
    }

    /// Flips a layer's switch (the current layer can't be frozen).
    pub fn layer_switch(&mut self, h: u32, which: Switch) -> Result<(), String> {
        if which == Switch::Freeze && h == self.current_layer() {
            let row = self.layer_rows().into_iter().find(|r| r.handle == h);
            if row.is_some_and(|r| !r.frozen) {
                self.say("Cannot freeze the current layer.");
                return Err("current layer".into());
            }
        }
        self.edit(|d, hist| {
            d.modify(hist, h, |o| {
                if let Data::Layer(l) = &mut o.data {
                    match which {
                        Switch::On => {
                            let on = layer_on(l);
                            // R2004 on: the flag; R2000: the colour's sign as well
                            let cmc = l.color.rgb.is_some();
                            if on {
                                l.flags |= 2;
                                if !cmc {
                                    l.color.index = -l.color.index.abs().max(1);
                                }
                            } else {
                                l.flags &= !2;
                                l.color.index = l.color.index.abs();
                            }
                        }
                        Switch::Freeze => l.flags ^= 1,
                        Switch::Lock => l.flags ^= 8,
                    }
                }
            })
        })
    }

    /// A layer's colour (1..255).
    pub fn layer_colour(&mut self, h: u32, aci: i16) -> Result<(), String> {
        self.edit(|d, hist| {
            d.modify(hist, h, |o| {
                if let Data::Layer(l) = &mut o.data {
                    match &mut l.color.rgb {
                        // R2004 on: the index in the method byte
                        Some(rgb) => *rgb = 0xC300_0000 | aci as u32,
                        None => {
                            let off = l.color.index < 0;
                            l.color.index = if off { -aci } else { aci };
                        }
                    }
                }
            })
        })
    }

    /// A new layer, "Layer1" or the next free number, made current.
    pub fn new_layer(&mut self) -> Result<u32, String> {
        let names: Vec<String> = self
            .layer_rows()
            .iter()
            .map(|r| r.name.to_ascii_lowercase())
            .collect();
        let name = (1..)
            .map(|k| format!("Layer{k}"))
            .find(|n| !names.contains(&n.to_ascii_lowercase()))
            .unwrap_or_default();
        let mut made = 0;
        self.edit(|d, hist| {
            made = d.add_layer(hist, &name, 7)?;
            Ok(())
        })?;
        self.say(format!("Layer \"{name}\" created."));
        Ok(made)
    }

    /// Renames a layer (not 0 or DEFPOINTS).
    pub fn rename_layer(&mut self, h: u32, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty()
            || name.contains(['<', '>', '/', '\\', '"', ':', ';', '?', '*', '|', ',', '=', '`'])
        {
            return Err("Invalid layer name".into());
        }
        if self.drawing.layer(name).is_some_and(|x| x != h) {
            return Err(format!("A layer named \"{name}\" already exists"));
        }
        let old = self
            .layer_rows()
            .into_iter()
            .find(|r| r.handle == h)
            .map(|r| r.name);
        if old
            .as_deref()
            .is_some_and(|o| o == "0" || o.eq_ignore_ascii_case("defpoints"))
        {
            return Err("Layer 0 and DEFPOINTS can't be renamed".into());
        }
        let s = self.drawing.s(name);
        self.edit(|d, hist| {
            d.modify(hist, h, |o| {
                if let Data::Layer(l) = &mut o.data {
                    l.entry.name = s;
                }
            })
        })
    }

    /// Deletes a layer that nothing is on (not 0, DEFPOINTS or the current one).
    pub fn delete_layer(&mut self, h: u32) -> Result<(), String> {
        let Some(row) = self.layer_rows().into_iter().find(|r| r.handle == h) else {
            return Err("No such layer".into());
        };
        if row.name == "0" || row.name.eq_ignore_ascii_case("defpoints") || row.current {
            let why = format!(
                "Layer \"{}\" can't be deleted (0, DEFPOINTS or current).",
                row.name
            );
            self.say(why.clone());
            return Err(why);
        }
        let used = self
            .drawing
            .objects
            .values()
            .any(|o| o.ent.as_ref().is_some_and(|e| e.layer.handle == h));
        if used {
            let why = format!("Layer \"{}\" has objects on it: not deleted.", row.name);
            self.say(why.clone());
            return Err(why);
        }
        let ctrl = self.drawing.header.handle("LAYER_CONTROL");
        self.edit(|d, hist| {
            if let Some(c) = ctrl {
                d.modify(hist, c, |o| {
                    if let Data::LayerControl(lc) = &mut o.data {
                        lc.0.entries.retain(|x| x.handle != h);
                    }
                })?;
            }
            hist.save(d, h);
            d.objects.remove(&h);
            Ok(())
        })?;
        self.say(format!("Layer \"{}\" deleted.", row.name));
        Ok(())
    }

    /// CLAYER: the layer new objects go on.
    pub fn set_current_layer(&mut self, h: u32) -> Result<(), String> {
        self.edit(|d, hist| {
            hist.save_header(d);
            match d.header.get_mut("CLAYER") {
                Some(Val::H(x)) => x.handle = h,
                _ => return forge_dwg::bits::bad("no CLAYER"),
            }
            Ok(())
        })
    }

    /// The ribbon's layer dropdown: the current layer, and what's chosen moved to the layer
    /// picked.
    pub fn layer_picked(&mut self, h: u32) -> Result<(), String> {
        if self.selected.is_empty() {
            return self.set_current_layer(h);
        }
        let name = self
            .layer_rows()
            .into_iter()
            .find(|r| r.handle == h)
            .map(|r| r.name)
            .unwrap_or_default();
        self.set_prop("Layer", &name)
    }

    /// The values a field can take, when it is a choice.
    pub fn prop_choices(&self, key: &str) -> Option<Vec<String>> {
        let d = &self.drawing;
        Some(match key {
            "Layer" => self.layer_rows().into_iter().map(|r| r.name).collect(),
            "Color" => [
                "ByLayer", "ByBlock", "Red", "Yellow", "Green", "Cyan", "Blue", "Magenta", "White",
            ]
            .iter()
            .map(|s| s.to_string())
            .chain((8..=9).map(|k| k.to_string()))
            .chain((250..=255).map(|k| k.to_string()))
            .collect(),
            "Linetype" => {
                let mut v = vec![
                    "ByLayer".to_string(),
                    "ByBlock".to_string(),
                    "Continuous".to_string(),
                ];
                for o in d.objects.values() {
                    if let Data::Ltype(l) = &o.data {
                        let n = l.entry.name.text();
                        if !["bylayer", "byblock", "continuous"].contains(&n.to_ascii_lowercase().as_str()) {
                            v.push(n);
                        }
                    }
                }
                v
            }
            "Lineweight" => [29u8, 30, 31]
                .into_iter()
                .chain(0..WEIGHTS as u8)
                .map(lineweight_name)
                .collect(),
            "Closed" => vec!["Yes".into(), "No".into()],
            _ => return None,
        })
    }

    /// Can the Properties palette change this field of what's chosen?
    pub fn prop_editable(&self, key: &str) -> bool {
        if matches!(
            key,
            "Color" | "Layer" | "Linetype" | "Linetype scale" | "Lineweight"
        ) {
            return !self.selected.is_empty();
        }
        if self.selected.len() != 1 {
            return false;
        }
        let Some(o) = self.selected.first().and_then(|h| self.drawing.objects.get(h)) else {
            return false;
        };
        let keys: &[&str] = match &o.data {
            Data::Line(_) => &[
                "Start X", "Start Y", "Start Z", "End X", "End Y", "End Z", "Length", "Angle",
            ],
            Data::Circle(_) => &[
                "Center X",
                "Center Y",
                "Center Z",
                "Radius",
                "Diameter",
                "Circumference",
                "Area",
            ],
            Data::Arc(_) => &[
                "Center X",
                "Center Y",
                "Center Z",
                "Radius",
                "Start angle",
                "End angle",
            ],
            Data::Text(_) => &[
                "Contents",
                "Height",
                "Rotation",
                "Width factor",
                "Position X",
                "Position Y",
            ],
            Data::MText(_) => &[
                "Contents",
                "Text height",
                "Defined width",
                "Position X",
                "Position Y",
                "Position Z",
            ],
            Data::Insert(_) => &[
                "Position X",
                "Position Y",
                "Position Z",
                "Scale X",
                "Scale Y",
                "Scale Z",
                "Rotation",
            ],
            Data::LwPolyline(_) => &["Vertex X", "Vertex Y", "Global width", "Elevation", "Closed"],
            Data::Point(_) => &["Position X", "Position Y", "Position Z"],
            _ => &[],
        };
        keys.contains(&key)
    }

    /// Sets a field of what's chosen from what was typed or picked.
    pub fn set_prop(&mut self, key: &str, value: &str) -> Result<(), String> {
        let sel: Vec<u32> = self.selected.iter().copied().collect();
        if sel.is_empty() {
            return Err("Nothing chosen".into());
        }
        let r = match key {
            "Color" => {
                let c = parse_colour(value).ok_or_else(|| format!("\"{value}\" is not a colour"))?;
                self.edit_ents(&sel, |e| e.color = Color::index(c))
            }
            "Layer" => {
                let h = self
                    .drawing
                    .layer(value.trim())
                    .ok_or_else(|| format!("No layer \"{value}\""))?;
                self.edit_ents(&sel, |e| e.layer = Href::new(5, h))
            }
            "Linetype" => {
                let v = value.trim().to_ascii_lowercase();
                let (flags, lt) = match v.as_str() {
                    "bylayer" => (0, 0),
                    "byblock" => (1, 0),
                    "continuous" => (2, 0),
                    _ => {
                        let h = self
                            .drawing
                            .objects
                            .iter()
                            .find_map(|(h, o)| match &o.data {
                                Data::Ltype(l) if l.entry.name.text().eq_ignore_ascii_case(&v) => Some(*h),
                                _ => None,
                            })
                            .ok_or_else(|| format!("No line type \"{value}\""))?;
                        (3, h)
                    }
                };
                self.edit_ents(&sel, |e| {
                    e.ltype_flags = flags;
                    e.ltype = Href::new(5, lt);
                })
            }
            "Linetype scale" => {
                let k = number(value)?;
                if k <= 0.0 {
                    return Err("Requires a positive number".into());
                }
                self.edit_ents(&sel, |e| e.ltype_scale = k)
            }
            "Lineweight" => {
                let i = [29u8, 30, 31]
                    .into_iter()
                    .chain(0..WEIGHTS as u8)
                    .find(|i| lineweight_name(*i).eq_ignore_ascii_case(value.trim()))
                    .ok_or_else(|| format!("\"{value}\" is not a lineweight"))?;
                self.edit_ents(&sel, |e| e.lineweight = i)
            }
            _ => {
                let h = sel[0];
                let data = self
                    .drawing
                    .objects
                    .get(&h)
                    .map(|o| o.data.clone())
                    .ok_or("gone")?;
                let new = geometry_field(&self.drawing, data, key, value)?;
                self.edit(|d, hist| d.modify(hist, h, |o| o.data = new))
            }
        };
        if r.is_ok() {
            self.say(format!("{key} = {}", value.trim()));
        }
        r
    }

    fn edit_ents(
        &mut self,
        sel: &[u32],
        f: impl Fn(&mut forge_dwg::object::EntCommon),
    ) -> Result<(), String> {
        self.edit(|d, hist| {
            for &h in sel {
                d.modify(hist, h, |o| {
                    if let Some(e) = o.ent.as_mut() {
                        f(e);
                    }
                })?;
            }
            Ok(())
        })
    }

    /// The General rows for several entities: a value they share, or *VARIES*.
    pub fn common_props(&self) -> Vec<(String, String)> {
        let d = &self.drawing;
        let mut rows: Vec<(String, Vec<String>)> =
            ["Color", "Layer", "Linetype", "Linetype scale", "Lineweight"]
                .iter()
                .map(|k| (k.to_string(), vec![]))
                .collect();
        for h in &self.selected {
            let Some(e) = d.objects.get(h).and_then(|o| o.ent.as_ref()) else {
                continue;
            };
            let lt = match e.ltype_flags {
                0 => "ByLayer".into(),
                1 => "ByBlock".into(),
                2 => "Continuous".into(),
                _ => entry_name(d, e.ltype).unwrap_or_else(|| "?".into()),
            };
            let vals = [
                colour_name(&e.color),
                entry_name(d, e.layer).unwrap_or_else(|| "0".into()),
                lt,
                format!("{:.4}", e.ltype_scale),
                lineweight_name(e.lineweight),
            ];
            for (r, v) in rows.iter_mut().zip(vals) {
                if !r.1.contains(&v) {
                    r.1.push(v);
                }
            }
        }
        rows.into_iter()
            .map(|(k, v)| {
                let s = if v.len() == 1 {
                    v[0].clone()
                } else {
                    "*VARIES*".into()
                };
                (k, s)
            })
            .collect()
    }
}

/// An entity's data with one geometry field set, as the Properties palette shows them.
fn geometry_field(d: &Drawing, mut data: Data, key: &str, value: &str) -> Result<Data, String> {
    let bad = || format!("{key} can't be set here");
    match &mut data {
        Data::Line(l) => {
            let (dx, dy) = (l.end[0] - l.start[0], l.end[1] - l.start[1]);
            let len = (dx * dx + dy * dy).sqrt();
            let ang = forge_math::dmath::atan2(dy, dx);
            match key {
                "Start X" => l.start[0] = number(value)?,
                "Start Y" => l.start[1] = number(value)?,
                "Start Z" => l.start[2] = number(value)?,
                "End X" => l.end[0] = number(value)?,
                "End Y" => l.end[1] = number(value)?,
                "End Z" => l.end[2] = number(value)?,
                "Length" => {
                    let n = number(value)?;
                    if n <= 0.0 {
                        return Err("Requires a positive length".into());
                    }
                    l.end[0] = l.start[0] + n * forge_math::dmath::cos(ang);
                    l.end[1] = l.start[1] + n * forge_math::dmath::sin(ang);
                }
                "Angle" => {
                    let a = number(value)?.to_radians();
                    l.end[0] = l.start[0] + len * forge_math::dmath::cos(a);
                    l.end[1] = l.start[1] + len * forge_math::dmath::sin(a);
                }
                _ => return Err(bad()),
            }
        }
        Data::Circle(c) => match key {
            "Center X" => c.center[0] = number(value)?,
            "Center Y" => c.center[1] = number(value)?,
            "Center Z" => c.center[2] = number(value)?,
            "Radius" | "Diameter" | "Circumference" | "Area" => {
                let n = number(value)?;
                let r = match key {
                    "Radius" => n,
                    "Diameter" => n / 2.0,
                    "Circumference" => n / std::f64::consts::TAU,
                    _ => (n / std::f64::consts::PI).sqrt(),
                };
                if r.is_nan() || r <= 0.0 || !r.is_finite() {
                    return Err("Requires a positive size".into());
                }
                c.radius = r;
            }
            _ => return Err(bad()),
        },
        Data::Arc(a) => match key {
            "Center X" => a.circle.center[0] = number(value)?,
            "Center Y" => a.circle.center[1] = number(value)?,
            "Center Z" => a.circle.center[2] = number(value)?,
            "Radius" => {
                let r = number(value)?;
                if r <= 0.0 {
                    return Err("Requires a positive radius".into());
                }
                a.circle.radius = r;
            }
            "Start angle" => a.start = number(value)?.to_radians().rem_euclid(std::f64::consts::TAU),
            "End angle" => a.end = number(value)?.to_radians().rem_euclid(std::f64::consts::TAU),
            _ => return Err(bad()),
        },
        Data::Text(t) => match key {
            "Contents" => t.text = d.s(value),
            "Height" => {
                let h = number(value)?;
                if h <= 0.0 {
                    return Err("Requires a positive height".into());
                }
                t.height = h;
            }
            "Rotation" => t.rotation = number(value)?.to_radians().rem_euclid(std::f64::consts::TAU),
            "Width factor" => t.width = number(value)?,
            "Position X" => {
                let x = number(value)?;
                if let Some(a) = &mut t.align_pt {
                    a[0] += x - t.insert[0];
                }
                t.insert[0] = x;
            }
            "Position Y" => {
                let y = number(value)?;
                if let Some(a) = &mut t.align_pt {
                    a[1] += y - t.insert[1];
                }
                t.insert[1] = y;
            }
            _ => return Err(bad()),
        },
        Data::MText(t) => {
            let set_ctx = |t: &mut forge_dwg::types::MText| {
                if let Some(c) = &mut t.context {
                    c.insert = t.insert;
                    c.rect_width = t.rect_width;
                }
            };
            match key {
                "Contents" => t.text = d.s(value),
                "Text height" => t.text_height = number(value)?,
                "Defined width" => t.rect_width = number(value)?.max(0.0),
                "Position X" => t.insert[0] = number(value)?,
                "Position Y" => t.insert[1] = number(value)?,
                "Position Z" => t.insert[2] = number(value)?,
                _ => return Err(bad()),
            }
            set_ctx(t);
        }
        Data::Insert(i) => match key {
            "Position X" => i.at[0] = number(value)?,
            "Position Y" => i.at[1] = number(value)?,
            "Position Z" => i.at[2] = number(value)?,
            "Scale X" => i.scale[0] = number(value)?,
            "Scale Y" => i.scale[1] = number(value)?,
            "Scale Z" => i.scale[2] = number(value)?,
            "Rotation" => i.rotation = number(value)?.to_radians().rem_euclid(std::f64::consts::TAU),
            _ => return Err(bad()),
        },
        Data::LwPolyline(p) => match key {
            "Vertex X" => *p.points.first_mut().map(|q| &mut q[0]).ok_or_else(bad)? = number(value)?,
            "Vertex Y" => *p.points.first_mut().map(|q| &mut q[1]).ok_or_else(bad)? = number(value)?,
            "Global width" => p.const_width = number(value)?.max(0.0),
            "Elevation" => p.elevation = number(value)?,
            "Closed" => {
                if value.trim().eq_ignore_ascii_case("yes") {
                    p.flag |= 512;
                } else {
                    p.flag &= !512;
                }
            }
            _ => return Err(bad()),
        },
        Data::Point(p) => match key {
            "Position X" => p.at[0] = number(value)?,
            "Position Y" => p.at[1] = number(value)?,
            "Position Z" => p.at[2] = number(value)?,
            _ => return Err(bad()),
        },
        _ => return Err(bad()),
    }
    Ok(data)
}
