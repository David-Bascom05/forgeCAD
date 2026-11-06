//! Colours: AutoCAD's 256-entry index palette, and what a stored colour means (an index, a true
//! colour, by layer or by block).

use crate::io::Color;

/// The AutoCAD colour index's RGB. 1..9 are the named colours, 10..249 24 hues at five
/// brightnesses, each full and half-saturated, 250..255 greys. 0 and 256 (by block and by layer)
/// have no colour of their own: white here, as is 7 (the canvas is dark).
pub fn aci_rgb(i: i16) -> [u8; 3] {
    match i {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        8 => [128, 128, 128],
        9 => [192, 192, 192],
        10..=249 => {
            let k = i - 10;
            let hue = (k / 10) as f64 * 15.0;
            let shade = k % 10;
            let v = [255.0, 204.0, 153.0, 127.0, 76.0][(shade / 2) as usize];
            // the hue at full saturation, as fractions
            let f = |n: f64| -> f64 {
                // HSV with s = 1: channel n is 1 - max(0, min(k, 4 - k, 1)), k = (n + h/60) mod 6
                let k = (n + hue / 60.0) % 6.0;
                1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
            };
            let (r, g, b) = (f(5.0), f(3.0), f(1.0));
            let ch = |x: f64| -> u8 {
                let full = v * x;
                let c = if shade % 2 == 1 {
                    full + (v - full) / 2.0
                } else {
                    full
                };
                c.floor() as u8
            };
            [ch(r), ch(g), ch(b)]
        }
        250 => [51, 51, 51],
        251 => [91, 91, 91],
        252 => [132, 132, 132],
        253 => [173, 173, 173],
        254 => [214, 214, 214],
        _ => [255, 255, 255],
    }
}

/// What a colour says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Says {
    ByLayer,
    ByBlock,
    Rgb([u8; 3]),
}

/// A stored colour's meaning: R2004+'s method byte when there is one (0xC0 by layer, 0xC1 by
/// block, 0xC2 RGB, 0xC3 an index), else the index (0 by block, 256 by layer, negative an off
/// layer's).
pub fn says(c: &Color) -> Says {
    if let Some(v) = c.rgb {
        match v >> 24 {
            0xC0 => return Says::ByLayer,
            0xC1 => return Says::ByBlock,
            0xC2 => return Says::Rgb([(v >> 16) as u8, (v >> 8) as u8, v as u8]),
            0xC3 => {
                let i = (v & 0xFF) as i16;
                return index_says(if i == 0 { c.index } else { i });
            }
            0xC8 => return Says::ByLayer,
            _ => {}
        }
    }
    index_says(c.index)
}

pub fn index_says(i: i16) -> Says {
    match i {
        0 => Says::ByBlock,
        256 | 257 => Says::ByLayer,
        i => Says::Rgb(aci_rgb(i.abs())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette() {
        assert_eq!(aci_rgb(1), [255, 0, 0]);
        assert_eq!(aci_rgb(7), [255, 255, 255]);
        assert_eq!(aci_rgb(10), [255, 0, 0]);
        assert_eq!(aci_rgb(11), [255, 127, 127]);
        assert_eq!(aci_rgb(12), [204, 0, 0]);
        assert_eq!(aci_rgb(13), [204, 102, 102]);
        assert_eq!(aci_rgb(18), [76, 0, 0]);
        assert_eq!(aci_rgb(19), [76, 38, 38]);
        assert_eq!(aci_rgb(20), [255, 63, 0]);
        assert_eq!(aci_rgb(21), [255, 159, 127]);
        assert_eq!(aci_rgb(23), [204, 127, 102]);
        assert_eq!(aci_rgb(27), [127, 79, 63]);
        assert_eq!(aci_rgb(30), [255, 127, 0]);
        assert_eq!(aci_rgb(40), [255, 191, 0]);
        assert_eq!(aci_rgb(41), [255, 223, 127]);
        assert_eq!(aci_rgb(50), [255, 255, 0]);
        assert_eq!(aci_rgb(90), [0, 255, 0]);
        assert_eq!(aci_rgb(130), [0, 255, 255]);
        assert_eq!(aci_rgb(170), [0, 0, 255]);
        assert_eq!(aci_rgb(210), [255, 0, 255]);
        assert_eq!(aci_rgb(240), [255, 0, 63]);
        assert_eq!(aci_rgb(250), [51, 51, 51]);
    }

    #[test]
    fn methods() {
        let mut c = Color::index(0);
        c.rgb = Some(0xC300_0007);
        assert_eq!(says(&c), Says::Rgb([255, 255, 255]));
        c.rgb = Some(0xC200_8040);
        assert_eq!(says(&c), Says::Rgb([0, 128, 64]));
        c.rgb = Some(0xC100_0000);
        assert_eq!(says(&c), Says::ByBlock);
        c.rgb = Some(0xC000_0000);
        assert_eq!(says(&c), Says::ByLayer);
        assert_eq!(says(&Color::index(256)), Says::ByLayer);
        assert_eq!(says(&Color::index(-3)), Says::Rgb([0, 255, 0]));
    }
}
