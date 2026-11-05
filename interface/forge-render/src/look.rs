//! How the view looks besides the parts' own shading: Inventor's colour schemes, background
//! (1 colour, gradient or an image), lighting styles, Realistic's environments, and the ground
//! (its plane, reflections, shadows). The lists are data: `data/color_schemes.csv`,
//! `data/lighting_styles.csv`, `data/environments.csv`.

use std::sync::OnceLock;

const SCHEMES_CSV: &str = include_str!("../data/color_schemes.csv");
const LIGHTING_CSV: &str = include_str!("../data/lighting_styles.csv");
const ENVIRONMENTS_CSV: &str = include_str!("../data/environments.csv");

/// A hex sRGB colour in linear, or none for "theme".
fn hex(s: &str) -> Option<[f32; 3]> {
    let v = u32::from_str_radix(s.trim(), 16).ok()?;
    Some(crate::viewcube::rgb(v))
}

fn vec3(s: &str) -> [f32; 3] {
    let mut v = [0.0f32; 3];
    for (k, w) in s.split_whitespace().take(3).enumerate() {
        v[k] = w.parse().unwrap_or(0.0);
    }
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n > 0.0 {
        v.map(|x| x / n)
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn num(s: &str) -> f32 {
    s.trim().parse().unwrap_or(0.0)
}

/// A file's rows past its comments and header, split on commas.
fn rows(csv: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    csv.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .skip(1)
        .map(|l| l.split(',').collect())
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scheme {
    pub id: String,
    pub name: String,
    /// None: the UI theme's own.
    pub solid: Option<[f32; 3]>,
    pub top: Option<[f32; 3]>,
    pub bottom: Option<[f32; 3]>,
    pub floor: Option<[f32; 3]>,
    pub hover: [f32; 3],
    pub selection: [f32; 3],
    pub sketch: Option<u32>,
    pub projected: Option<u32>,
}

pub fn schemes() -> &'static [Scheme] {
    static ALL: OnceLock<Vec<Scheme>> = OnceLock::new();
    ALL.get_or_init(|| {
        rows(SCHEMES_CSV)
            .filter(|r| r.len() >= 10)
            .map(|r| Scheme {
                id: r[0].into(),
                name: r[1].into(),
                solid: hex(r[2]),
                top: hex(r[3]),
                bottom: hex(r[4]),
                floor: hex(r[5]),
                hover: hex(r[6]).unwrap_or([0.2, 0.5, 0.95]),
                selection: hex(r[7]).unwrap_or([0.745, 0.361, 0.011]),
                sketch: u32::from_str_radix(r[8].trim(), 16).ok(),
                projected: u32::from_str_radix(r[9].trim(), 16).ok(),
            })
            .collect()
    })
}

pub fn scheme(id: &str) -> &'static Scheme {
    let all = schemes();
    all.iter().find(|s| s.id == id).unwrap_or(&all[0])
}

/// A light: its direction, whether in the eye's frame (right up back) or the world's, its colour
/// and strength.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub dir: [f32; 3],
    pub eye: bool,
    pub colour: [f32; 3],
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lighting {
    pub id: String,
    pub name: String,
    pub key: Light,
    pub fill: Light,
    pub room_low: [f32; 3],
    pub room_high: [f32; 3],
    pub spec: f32,
    pub shine: f32,
}

pub fn lightings() -> &'static [Lighting] {
    static ALL: OnceLock<Vec<Lighting>> = OnceLock::new();
    ALL.get_or_init(|| {
        rows(LIGHTING_CSV)
            .filter(|r| r.len() >= 14)
            .map(|r| {
                let light = |k: usize| Light {
                    eye: r[k].trim() == "eye",
                    dir: vec3(r[k + 1]),
                    colour: hex(r[k + 2]).unwrap_or([1.0; 3]),
                    strength: num(r[k + 3]),
                };
                Lighting {
                    id: r[0].into(),
                    name: r[1].into(),
                    key: light(2),
                    fill: light(6),
                    room_low: hex(r[10]).unwrap_or([0.26; 3]),
                    room_high: hex(r[11]).unwrap_or([0.56; 3]),
                    spec: num(r[12]),
                    shine: num(r[13]).max(1.0),
                }
            })
            .collect()
    })
}

pub fn lighting(id: &str) -> &'static Lighting {
    let all = lightings();
    all.iter().find(|s| s.id == id).unwrap_or(&all[0])
}

#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub sky_low: [f32; 3],
    pub sky_high: [f32; 3],
    pub ground_low: [f32; 3],
    pub ground_high: [f32; 3],
    /// Horizon band, side softboxes, overhead softbox, grid lines.
    pub lights: [f32; 4],
    pub sun_dir: [f32; 3],
    pub sun_colour: [f32; 3],
    pub sun: f32,
    pub exposure: f32,
}

pub fn environments() -> &'static [Environment] {
    static ALL: OnceLock<Vec<Environment>> = OnceLock::new();
    ALL.get_or_init(|| {
        rows(ENVIRONMENTS_CSV)
            .filter(|r| r.len() >= 14)
            .map(|r| Environment {
                id: r[0].into(),
                name: r[1].into(),
                sky_low: hex(r[2]).unwrap_or([0.5; 3]),
                sky_high: hex(r[3]).unwrap_or([1.0; 3]),
                ground_low: hex(r[4]).unwrap_or([0.1; 3]),
                ground_high: hex(r[5]).unwrap_or([0.3; 3]),
                lights: [num(r[6]), num(r[7]), num(r[8]), num(r[9])],
                sun_dir: vec3(r[10]),
                sun_colour: hex(r[11]).unwrap_or([1.0; 3]),
                sun: num(r[12]),
                exposure: num(r[13]).max(0.05),
            })
            .collect()
    })
}

pub fn environment(id: &str) -> &'static Environment {
    let all = environments();
    all.iter().find(|s| s.id == id).unwrap_or(&all[0])
}

/// The background, Inventor's three.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backdrop {
    Solid,
    #[default]
    Gradient,
    Image,
}

/// How a background image fills the view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFit {
    #[default]
    Stretch,
    Tile,
    Centre,
}

/// The View tab's quick switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    GroundPlane,
    Reflections,
    GroundShadows,
    ObjectShadows,
    AmbientShadows,
}

/// Everything the renderer needs of it, resolved: colours linear.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub backdrop: Backdrop,
    pub fit: ImageFit,
    pub top: [f32; 3],
    pub bottom: [f32; 3],
    pub floor: [f32; 3],
    pub hover: [f32; 3],
    pub selection: [f32; 3],
    pub lighting: Lighting,
    pub environment: Environment,
    pub ground_plane: bool,
    pub reflections: bool,
    pub ground_shadows: bool,
    pub object_shadows: bool,
    /// Occlusion on the parts and the contact shadow under them on the ground.
    pub ambient_shadows: bool,
}

impl Default for Look {
    /// A renderer on its own (tests, shots of forge-render): one colour, no ground, the studio.
    fn default() -> Look {
        let dark = scheme("dark");
        Look {
            backdrop: Backdrop::Solid,
            fit: ImageFit::Stretch,
            top: dark.top.unwrap_or_default(),
            bottom: dark.bottom.unwrap_or_default(),
            floor: dark.floor.unwrap_or_default(),
            hover: dark.hover,
            selection: dark.selection,
            lighting: lightings()[0].clone(),
            environment: environments()[0].clone(),
            ground_plane: false,
            reflections: false,
            ground_shadows: false,
            object_shadows: false,
            ambient_shadows: true,
        }
    }
}

/// A PNG or JPEG file as RGBA (sRGB) and its size, if it reads.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_image(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.starts_with(b"\x89PNG") {
        let mut d = png::Decoder::new(std::io::Cursor::new(&bytes));
        d.set_transformations(
            png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
        );
        let mut r = d.read_info().ok()?;
        let mut buf = vec![0; r.output_buffer_size()?];
        let info = r.next_frame(&mut buf).ok()?;
        let (w, h) = (info.width, info.height);
        let px = (w * h) as usize;
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf[..px * 4].to_vec(),
            png::ColorType::GrayscaleAlpha => buf[..px * 2]
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|[g, a]| [*g, *g, *g, *a])
                .collect(),
            _ => return None,
        };
        return Some((w, h, rgba));
    }
    use zune_core::{colorspace::ColorSpace, options::DecoderOptions};
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut d =
        zune_jpeg::JpegDecoder::new_with_options(zune_core::bytestream::ZCursor::new(&bytes), options);
    let rgba = d.decode().ok()?;
    let info = d.info()?;
    let (w, h) = (info.width as u32, info.height as u32);
    (rgba.len() >= (w * h * 4) as usize).then_some((w, h, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_list_reads_whole_with_unique_words() {
        for (n, ids) in [
            (12, schemes().iter().map(|s| s.id.as_str()).collect::<Vec<_>>()),
            (8, lightings().iter().map(|s| s.id.as_str()).collect()),
            (7, environments().iter().map(|s| s.id.as_str()).collect()),
        ] {
            let unique: std::collections::HashSet<_> = ids.iter().collect();
            assert_eq!((ids.len(), unique.len()), (n, n), "{ids:?}");
        }
        assert_eq!(scheme("theme").top, None, "the theme's own gradient");
        assert!(scheme("winter_night").top.is_some());
        assert_eq!(scheme("nonsense").id, "theme");
        let studio = lighting("studio");
        assert!(studio.key.eye && studio.fill.strength == 0.0);
        assert!(!lighting("grid_light").key.eye);
        assert!(environment("desert_sun").sun > 1.0);
    }
}
