//! ISO 286 fits: a tolerance class (`H7`, `g6`, `JS8`) on a nominal size, to its limit deviations.
//! The numbers are data/iso286.txt's, which says where they come from.

const TABLE: &str = include_str!("../data/iso286.txt");

#[derive(Clone, Debug, PartialEq)]
pub enum FitError {
    /// Not a letter (or two) and a grade: `H7`, `js6`.
    NotAClass,
    /// A letter or grade the table does not hold.
    NotInTable { class: String },
    /// Over 500 mm or not above 0.
    Size { size: f64 },
}

impl FitError {
    pub fn text(&self) -> String {
        match self {
            FitError::NotAClass => "a fit class is a letter and a grade: H7, g6, JS8".into(),
            FitError::NotInTable { class } => {
                format!(
                    "{class} is not in the ISO 286 table (holes D-H, JS, K, M, N, P, R, S; shafts d-h, js, k, m, n, p, r, s; IT4-IT12)"
                )
            }
            FitError::Size { size } => format!("{size} mm is outside ISO 286's 0-500 mm"),
        }
    }
}

struct Table {
    sizes: Vec<f64>,
    rows: Vec<(String, Vec<f64>)>,
    splits: Vec<(char, f64, f64, f64)>,
}

fn table() -> Table {
    let mut t = Table {
        sizes: vec![],
        rows: vec![],
        splits: vec![],
    };
    for line in TABLE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let mut w = line.split_whitespace();
        let head = w.next().unwrap_or("");
        let nums: Vec<&str> = w.collect();
        let num = |s: &str| s.parse::<f64>().expect("iso286.txt holds numbers");
        match head {
            "sizes" => t.sizes = nums.iter().map(|s| num(s)).collect(),
            "split" => t.splits.push((
                nums[0].chars().next().expect("a letter"),
                num(nums[1]),
                num(nums[2]),
                num(nums[3]),
            )),
            _ => t
                .rows
                .push((head.to_string(), nums.iter().map(|s| num(s)).collect())),
        }
    }
    t
}

impl Table {
    fn range(&self, size: f64) -> Option<usize> {
        (0..self.sizes.len() - 1).find(|&k| size > self.sizes[k] && size <= self.sizes[k + 1])
    }
    fn row(&self, name: &str, r: usize) -> Option<f64> {
        self.rows.iter().find(|x| x.0 == name).map(|x| x.1[r])
    }
    fn split(&self, letter: char, size: f64) -> Option<f64> {
        self.splits
            .iter()
            .find(|s| s.0 == letter && size > s.1 && size <= s.2)
            .map(|s| s.3)
    }
    /// A shaft's fundamental deviation: (es, None) for d..h, (None, ei) for k..s.
    fn shaft(&self, l: char, grade: u32, size: f64, r: usize) -> Option<(Option<f64>, Option<f64>)> {
        match l {
            'd' | 'e' | 'f' | 'g' | 'h' => Some((Some(self.row(&format!("es_{l}"), r)?), None)),
            'k' => Some((
                None,
                Some(if (4..=7).contains(&grade) {
                    self.row("ei_k", r)?
                } else {
                    0.0
                }),
            )),
            'm' | 'n' | 'p' => Some((None, Some(self.row(&format!("ei_{l}"), r)?))),
            'r' | 's' => Some((None, Some(self.split(l, size)?))),
            _ => None,
        }
    }
}

/// The class's (upper, lower) deviations from `size`, in millimetres.
pub fn deviations(size: f64, class: &str) -> Result<(f64, f64), FitError> {
    let letters: String = class.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let grade: u32 = class[letters.len()..].parse().map_err(|_| FitError::NotAClass)?;
    if letters.is_empty() || letters.len() > 2 {
        return Err(FitError::NotAClass);
    }
    let t = table();
    let r = t.range(size).ok_or(FitError::Size { size })?;
    let missing = || FitError::NotInTable {
        class: class.to_string(),
    };
    let it = t.row(&format!("IT{grade}"), r).ok_or_else(missing)?;
    let um = |x: f64| x / 1000.0;
    let hole = letters.chars().next().is_some_and(|c| c.is_ascii_uppercase());
    let low = letters.to_ascii_lowercase();
    if low == "js" {
        return Ok((um(it / 2.0), um(-it / 2.0)));
    }
    let l = low.chars().next().ok_or(FitError::NotAClass)?;
    if low.len() != 1 {
        return Err(missing());
    }
    let (es, ei) = t.shaft(l, grade, size, r).ok_or_else(missing)?;
    if !hole {
        return Ok(match (es, ei) {
            (Some(es), _) => (um(es), um(es - it)),
            (_, Some(ei)) => (um(ei + it), um(ei)),
            _ => unreachable!("a deviation"),
        });
    }
    // holes: D..H mirror the shafts; K..S by ISO 286-1 9.2 with its delta
    if let Some(es) = es {
        let ei = -es;
        return Ok((um(ei + it), um(ei)));
    }
    let ei = ei.expect("k..s give ei");
    let delta = || -> Result<f64, FitError> {
        if size <= 3.0 {
            return Ok(0.0);
        }
        Ok(it - t.row(&format!("IT{}", grade - 1), r).ok_or_else(missing)?)
    };
    let es_hole = match l {
        'k' if grade <= 8 => -ei + delta()?,
        'k' => 0.0,
        'm' if grade <= 8 => -ei + delta()?,
        'm' => -ei,
        'n' if grade <= 8 => -ei + delta()?,
        'n' => {
            if size <= 3.0 {
                -ei
            } else {
                0.0
            }
        }
        _ if grade <= 7 => -ei + delta()?,
        _ => -ei,
    };
    Ok((um(es_hole), um(es_hole - it)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn um(size: f64, class: &str) -> (i64, i64) {
        let (a, b) = deviations(size, class).unwrap();
        ((a * 1000.0).round() as i64, (b * 1000.0).round() as i64)
    }

    #[test]
    fn known_fits() {
        // checked against ISO 286-2's printed tables
        assert_eq!(um(20.0, "H7"), (21, 0));
        assert_eq!(um(20.0, "g6"), (-7, -20));
        assert_eq!(um(20.0, "h6"), (0, -13));
        assert_eq!(um(20.0, "k6"), (15, 2));
        assert_eq!(um(20.0, "p6"), (35, 22));
        assert_eq!(um(20.0, "s6"), (48, 35));
        assert_eq!(um(50.0, "f7"), (-25, -50));
        assert_eq!(um(10.0, "js6"), (5, -5));
        assert_eq!(um(20.0, "K7"), (6, -15));
        assert_eq!(um(20.0, "N7"), (-7, -28));
        assert_eq!(um(20.0, "P7"), (-14, -35));
        assert_eq!(um(100.0, "H8"), (54, 0));
        assert_eq!(um(70.0, "r6"), (62, 43));
        assert!(matches!(deviations(20.0, "Z7"), Err(FitError::NotInTable { .. })));
        assert!(matches!(deviations(600.0, "H7"), Err(FitError::Size { .. })));
        assert!(matches!(deviations(20.0, "7H"), Err(FitError::NotAClass)));
    }
}
