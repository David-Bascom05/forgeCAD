//! Cutting data per ISO 513 material group and the feeds engine.
//!
//! The formulas are the standard ones (libraries-machines-posts-feeds.md §3.2):
//! - spindle n = 1000 vc / (π D_eff) rpm, D_eff = D, or 2 √(ap (D − ap)) for a ball at depth ap;
//! - radial chip thinning: for ae < D/2 the mean chip is thinner than fz by
//!   k = 2 √(ae/D − (ae/D)²), so the feed per tooth that keeps the chip is fz / k (capped);
//! - feed vf = fz n z (mm/min);
//! - power Pc = ap ae vf kc / 6e7 kW, kc = kc1.1 hm^(−mc), hm the mean chip thickness.
//!
//! The data is per group, in our own numbers restated from public handbook ranges; it is a
//! starting point. Every value is marked "verify with the tool maker".
use crate::LibError;
use crate::toml::{Table, num, text};

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// P M K N S H.
    pub group: String,
    pub name: String,
    /// Cutting speed for carbide and for HSS, m/min.
    pub vc_carbide: f64,
    pub vc_hss: f64,
    /// Feed per tooth at 1 mm and at 20 mm diameter (linear between, clamped outside), mm.
    pub fz_d1: f64,
    pub fz_d20: f64,
    /// Specific cutting force at 1 mm chip, N/mm², and its exponent.
    pub kc1_1: f64,
    pub mc: f64,
    pub source: String,
    pub note: String,
}

impl Group {
    pub fn from_table(t: &Table) -> Result<Group, LibError> {
        let g = text(t, "group")
            .ok_or_else(|| LibError::Missing {
                record: "[[group]]".into(),
                field: "group",
                fix: "give the ISO 513 letter".into(),
            })?
            .to_string();
        let rec = format!("cutting group {g}");
        let n = |k: &'static str| {
            num(t, k).ok_or_else(|| LibError::Missing {
                record: rec.clone(),
                field: k,
                fix: format!("give `{k}`"),
            })
        };
        Ok(Group {
            name: text(t, "name").unwrap_or("").to_string(),
            vc_carbide: n("vc_carbide")?,
            vc_hss: n("vc_hss")?,
            fz_d1: n("fz_d1")?,
            fz_d20: n("fz_d20")?,
            kc1_1: n("kc1_1")?,
            mc: n("mc")?,
            source: text(t, "source").unwrap_or("").to_string(),
            note: text(t, "note").unwrap_or("").to_string(),
            group: g,
        })
    }

    pub fn fz_at(&self, d: f64) -> f64 {
        let f = ((d - 1.0) / 19.0).clamp(0.0, 1.0);
        self.fz_d1 + f * (self.fz_d20 - self.fz_d1)
    }
}

/// What the engine was asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    pub d: f64,
    pub flutes: u32,
    pub ball: bool,
    pub carbide: bool,
    /// Axial and radial engagement, mm.
    pub ap: f64,
    pub ae: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feeds {
    pub rpm: f64,
    pub vc: f64,
    /// Programmed feed per tooth (after thinning) and the chip it gives.
    pub fz: f64,
    pub hex: f64,
    /// mm/min.
    pub vf: f64,
    /// kW at the spindle.
    pub power: f64,
    /// The effective diameter the speed was set for.
    pub d_eff: f64,
    /// rpm limited by the machine (the speed is then lower than asked).
    pub rpm_limited: bool,
}

/// The largest factor chip thinning may raise the feed per tooth by.
pub const THIN_CAP: f64 = 3.0;

pub fn feeds(g: &Group, c: &Cut, rpm_range: [f64; 2]) -> Feeds {
    let d_eff = if c.ball && c.ap < 0.5 * c.d {
        2.0 * (c.ap * (c.d - c.ap)).max(0.0).sqrt()
    } else {
        c.d
    };
    let vc0 = if c.carbide { g.vc_carbide } else { g.vc_hss };
    let n0 = 1000.0 * vc0 / (std::f64::consts::PI * d_eff.max(1e-6));
    let rpm = n0.clamp(rpm_range[0], rpm_range[1]);
    let vc = rpm * std::f64::consts::PI * d_eff / 1000.0;
    let target = g.fz_at(c.d);
    let r = (c.ae / c.d).clamp(0.0, 1.0);
    let k = if r < 0.5 {
        2.0 * (r - r * r).max(0.0).sqrt()
    } else {
        1.0
    };
    let fz = if k > 0.0 {
        (target / k).min(THIN_CAP * target)
    } else {
        target
    };
    let hex = fz * k;
    let vf = fz * rpm * c.flutes.max(1) as f64;
    let hm = hex.max(1e-4);
    let kc = g.kc1_1 * forge_math::dmath::powf(hm, -g.mc);
    let power = c.ap * c.ae * vf * kc / 6.0e7;
    Feeds {
        rpm,
        vc,
        fz,
        hex,
        vf,
        power,
        d_eff,
        rpm_limited: rpm != n0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinning_and_ball_diameter() {
        let g = Group {
            group: "N".into(),
            name: "al".into(),
            vc_carbide: 300.0,
            vc_hss: 100.0,
            fz_d1: 0.01,
            fz_d20: 0.1,
            kc1_1: 700.0,
            mc: 0.25,
            source: String::new(),
            note: String::new(),
        };
        let full = feeds(
            &g,
            &Cut {
                d: 10.0,
                flutes: 3,
                ball: false,
                carbide: true,
                ap: 5.0,
                ae: 10.0,
            },
            [0.0, 1e9],
        );
        let light = feeds(
            &g,
            &Cut {
                d: 10.0,
                flutes: 3,
                ball: false,
                carbide: true,
                ap: 5.0,
                ae: 1.0,
            },
            [0.0, 1e9],
        );
        assert!((full.rpm - 9549.296585513721).abs() < 1e-6);
        // ae/D = 0.1: k = 2 sqrt(0.09) = 0.6
        assert!((light.fz / full.fz - 1.0 / 0.6).abs() < 1e-12);
        assert!((light.hex - full.fz).abs() < 1e-12);
        let ball = feeds(
            &g,
            &Cut {
                d: 10.0,
                flutes: 2,
                ball: true,
                carbide: true,
                ap: 1.0,
                ae: 1.0,
            },
            [0.0, 12_000.0],
        );
        assert!((ball.d_eff - 6.0).abs() < 1e-12 && ball.rpm_limited && ball.rpm == 12_000.0);
    }
}
