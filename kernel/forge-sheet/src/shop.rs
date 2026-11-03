//! What a shop can make, as data: stock thicknesses, the bend rule for each, tooled radii and the
//! minimum distances its checks will read. Only the bend allowance reaches geometry.
use forge_math::dmath;

/// How a bend's flat length is found. Angles are the bend's turn (0 flat), in radians.
#[derive(Clone, Debug, PartialEq)]
pub enum BendRule {
    /// The neutral layer at K t from the inside: BA = angle (R + K t).
    K(f64),
    /// K by R/t, interpolated linearly between rows (rows by increasing R/t).
    KTable(Vec<KRow>),
    /// Measured allowances for one radius, interpolated linearly in angle (rows by increasing angle).
    Allowance(Vec<AllowanceRow>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KRow {
    pub r_over_t: f64,
    pub k: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AllowanceRow {
    pub radius: f64,
    pub angle: f64,
    pub ba: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RuleError {
    /// K must lie in [0, 1].
    KRange { k: f64, lo: f64, hi: f64 },
    /// R/t outside the table's rows.
    OffTable { r_over_t: f64, lo: f64, hi: f64 },
    /// No allowance row for this radius and angle (`nearest` is the closest radius tabled).
    NoRow { radius: f64, angle: f64, nearest: f64 },
    /// Table rows out of order at `row`.
    TableOrder { row: usize },
    /// The rule gave an allowance that is not positive.
    Allowance { ba: f64 },
}

/// Radii in a table match to this.
const RADIUS_MATCH: f64 = 1e-9;

impl BendRule {
    /// The bend allowance for inside radius `radius`, thickness `t` and turn `angle` (> 0).
    pub fn allowance(&self, t: f64, radius: f64, angle: f64) -> Result<f64, RuleError> {
        let ba = match self {
            BendRule::K(k) => angle * (radius + checked_k(*k)? * t),
            BendRule::KTable(rows) => {
                let q = radius / t;
                let k = interpolate(rows.iter().map(|r| (r.r_over_t, r.k)), q).map_err(|e| {
                    e.off_table(
                        q,
                        rows.first().map(|r| r.r_over_t),
                        rows.last().map(|r| r.r_over_t),
                    )
                })?;
                angle * (radius + checked_k(k)? * t)
            }
            BendRule::Allowance(rows) => {
                let mine: Vec<(f64, f64)> = rows
                    .iter()
                    .filter(|r| (r.radius - radius).abs() <= RADIUS_MATCH)
                    .map(|r| (r.angle, r.ba))
                    .collect();
                let nearest = rows
                    .iter()
                    .map(|r| r.radius)
                    .min_by(|a, b| (a - radius).abs().total_cmp(&(b - radius).abs()))
                    .unwrap_or(f64::NAN);
                let no_row = RuleError::NoRow {
                    radius,
                    angle,
                    nearest,
                };
                interpolate(mine.into_iter(), angle).map_err(|e| match e {
                    Interp::Order(row) => RuleError::TableOrder { row },
                    Interp::Outside => no_row,
                })?
            }
        };
        if ba > 0.0 {
            Ok(ba)
        } else {
            Err(RuleError::Allowance { ba })
        }
    }
}

fn checked_k(k: f64) -> Result<f64, RuleError> {
    if (0.0..=1.0).contains(&k) {
        Ok(k)
    } else {
        Err(RuleError::KRange { k, lo: 0.0, hi: 1.0 })
    }
}

enum Interp {
    Order(usize),
    Outside,
}

impl Interp {
    fn off_table(self, q: f64, lo: Option<f64>, hi: Option<f64>) -> RuleError {
        match self {
            Interp::Order(row) => RuleError::TableOrder { row },
            Interp::Outside => RuleError::OffTable {
                r_over_t: q,
                lo: lo.unwrap_or(f64::NAN),
                hi: hi.unwrap_or(f64::NAN),
            },
        }
    }
}

/// Linear interpolation in rows (x, y) by increasing x; a single row answers only its own x.
fn interpolate(rows: impl Iterator<Item = (f64, f64)>, x: f64) -> Result<f64, Interp> {
    let rows: Vec<(f64, f64)> = rows.collect();
    for (i, w) in rows.windows(2).enumerate() {
        if w[1].0 <= w[0].0 {
            return Err(Interp::Order(i + 1));
        }
    }
    for w in rows.windows(2) {
        if x >= w[0].0 && x <= w[1].0 {
            let f = (x - w[0].0) / (w[1].0 - w[0].0);
            return Ok(w[0].1 + f * (w[1].1 - w[0].1));
        }
    }
    match rows.as_slice() {
        [(x0, y)] if (x - x0).abs() <= RADIUS_MATCH => Ok(*y),
        _ => Err(Interp::Outside),
    }
}

/// A length as multiples of thickness and of the bend's radius plus a constant, the way shops
/// state them (hole to bend 2.5 t + R, say).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scaled {
    pub per_t: f64,
    pub per_r: f64,
    pub plus: f64,
}

impl Scaled {
    pub const NONE: Scaled = Scaled {
        per_t: 0.0,
        per_r: 0.0,
        plus: 0.0,
    };
    /// For thickness t at a bend of inside radius r (0 where no bend is involved).
    pub fn at(self, t: f64, r: f64) -> f64 {
        self.per_t * t + self.per_r * r + self.plus
    }
}

/// One stocked thickness and what the shop can do with it.
#[derive(Clone, Debug, PartialEq)]
pub struct Stock {
    pub thickness: f64,
    pub rule: BendRule,
    /// Inside radii the press brake has tooling for; the first is the default.
    pub radii: Vec<f64>,
    /// The shortest flange the die can hold, from the mold line.
    pub min_flange: Scaled,
    pub hole_to_bend: Scaled,
    pub hole_to_edge: Scaled,
    pub min_slot: Scaled,
    /// The smallest round hole across.
    pub min_hole: Scaled,
    /// The relief rule for this stock (its kerf taken from the shop's).
    pub relief: crate::relief::ReliefRule,
    /// The sheet's hardness, Rockwell B, where the shop states it (hardware needs it).
    pub hardness: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ShopProfile {
    pub stock: Vec<Stock>,
    /// The hardware the shop fits.
    pub hardware: Vec<crate::hardware::Hardware>,
    /// The cutting width: a tear relief's.
    pub kerf: f64,
}

impl ShopProfile {
    /// The stock of this thickness, to the rounding a thickness is written with.
    pub fn stock(&self, t: f64) -> Option<&Stock> {
        self.stock
            .iter()
            .find(|s| (s.thickness - t).abs() <= 1e-9 * t.abs().max(1.0))
    }

    /// The relief rule for a sheet of thickness `t`, if the shop stocks it, with the shop's kerf.
    pub fn relief_rule(&self, t: f64) -> Option<crate::relief::ReliefRule> {
        self.stock(t).map(|s| crate::relief::ReliefRule {
            kerf: self.kerf,
            ..s.relief
        })
    }

    /// The stocked thickness nearest `t`.
    pub fn nearest_thickness(&self, t: f64) -> Option<f64> {
        self.stock
            .iter()
            .map(|s| s.thickness)
            .min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
    }
}

/// The bend deduction's half, the setback from mold line to tangent line on a skin of radius
/// `rho`, for a turn of `angle` below pi: rho tan(angle / 2), as rho sin / (1 + cos).
pub fn setback(rho: f64, angle: f64) -> f64 {
    let (s, c) = dmath::sin_cos(angle);
    rho * s / (1.0 + c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k_rule_is_the_neutral_arc() {
        let ba = BendRule::K(0.4).allowance(2.0, 3.0, dmath::PI / 2.0).unwrap();
        assert_eq!(ba, dmath::PI / 2.0 * (3.0 + 0.8));
        assert_eq!(
            BendRule::K(1.5).allowance(2.0, 3.0, 1.0),
            Err(RuleError::KRange {
                k: 1.5,
                lo: 0.0,
                hi: 1.0
            })
        );
    }

    #[test]
    fn tables_interpolate_and_refuse_outside() {
        let t = BendRule::KTable(vec![
            KRow {
                r_over_t: 0.5,
                k: 0.3,
            },
            KRow {
                r_over_t: 2.0,
                k: 0.45,
            },
        ]);
        let ba = t.allowance(2.0, 2.0, 1.0).unwrap();
        assert!((ba - (2.0 + 0.35 * 2.0)).abs() < 1e-15);
        assert!(matches!(
            t.allowance(2.0, 8.0, 1.0),
            Err(RuleError::OffTable { .. })
        ));
        let a = BendRule::Allowance(vec![
            AllowanceRow {
                radius: 2.0,
                angle: 1.0,
                ba: 3.0,
            },
            AllowanceRow {
                radius: 2.0,
                angle: 2.0,
                ba: 5.0,
            },
            AllowanceRow {
                radius: 4.0,
                angle: 1.0,
                ba: 5.0,
            },
        ]);
        assert_eq!(a.allowance(2.0, 2.0, 1.5), Ok(4.0));
        assert_eq!(
            a.allowance(2.0, 3.0, 1.5),
            Err(RuleError::NoRow {
                radius: 3.0,
                angle: 1.5,
                nearest: 2.0
            })
        );
    }
}
