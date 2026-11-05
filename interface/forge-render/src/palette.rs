//! Part colours: Inventor's default grey for every part, or a pastel per part file (every
//! occurrence of one part the same). The pastels are research/part-colours/palette.py's: one
//! OKLCH lightness, hues clear of the preselect and selection blues, each at least CIEDE2000 7
//! from the others and 8 from every colour the app draws marks, sketches and the origin in.

/// Inventor's default appearance: a light neutral grey.
pub const GREY: [f32; 4] = [0.706, 0.706, 0.706, 0.0];

/// In this order consecutive entries are far apart (see palette.py).
pub const PASTEL: [[f32; 3]; 12] = [
    [0.688, 0.731, 0.936],
    [0.909, 0.686, 0.590],
    [0.540, 0.808, 0.728],
    [0.898, 0.668, 0.761],
    [0.711, 0.776, 0.564],
    [0.775, 0.702, 0.906],
    [0.867, 0.713, 0.536],
    [0.507, 0.803, 0.817],
    [0.919, 0.670, 0.670],
    [0.618, 0.798, 0.638],
    [0.848, 0.679, 0.844],
    [0.799, 0.746, 0.527],
];

/// A part's colour: grey, or its pastel by its file's name (FNV-1a, so it never depends on
/// what was loaded first or where the file is).
pub fn part_colour(file: &str, pastel: bool) -> [f32; 4] {
    if !pastel {
        return GREY;
    }
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let name = name
        .strip_suffix(".fg")
        .or_else(|| name.strip_suffix(".fc"))
        .unwrap_or(name);
    let mut h: u32 = 0x811c_9dc5;
    for b in name.bytes() {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    let c = PASTEL[h as usize % PASTEL.len()];
    [c[0], c[1], c[2], 0.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_keeps_its_colour_wherever_its_file_is() {
        assert_eq!(
            part_colour("../hw/bolt.fg", true),
            part_colour("/x/y/bolt.fg", true)
        );
        assert_eq!(part_colour("a.fg", false), GREY);
        let all: std::collections::HashSet<[u32; 3]> =
            ["base_pan", "top_pan", "rail", "post", "carrier", "hub", "yoke"]
                .iter()
                .map(|n| {
                    let c = part_colour(n, true);
                    [c[0].to_bits(), c[1].to_bits(), c[2].to_bits()]
                })
                .collect();
        assert!(all.len() >= 5, "seven parts in at least five colours");
    }
}
