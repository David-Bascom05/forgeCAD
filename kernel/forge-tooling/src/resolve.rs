//! Feeds and speeds as parameters: each value resolved in layers, each layer overriding the one
//! before it, and each resolved value saying where it came from.
//!
//! Layers, lowest first:
//! 1. `Default`: forgeCAD's own fallback (the op's nominal ae/ap, plunge at 30% of the feed):
//!    no data behind it, and it says so;
//! 2. `Material`: the part's ISO 513 group record (cutting.toml), with that record's source;
//! 3. `Library`: the tool's own cutting data for the group (a `[[tool.preset]]`, cited per value);
//! 4. `User`: a value the user gave, per tool or per op (the op's wins).
//!
//! After the layers, two things may lower a value, and both are reported, never silent:
//! - `MachineClamped`: the machine's rpm range or maximum feed;
//! - `Limited`: one of the user's own limits (max power, max torque, feed range).
//!
//! The formulas are feeds.rs's (chip thinning, mean chip, Kienzle power, torque). Power and
//! torque limits lower the feed only, never ap or ae (patent note: fto-adaptive.md, '607).
use crate::feeds::{self, Cut, Group, THIN_CAP};
use crate::{Cite, Preset};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Default,
    Material,
    Library,
    User,
    MachineClamped,
    Limited,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Default => "default",
            Source::Material => "material",
            Source::Library => "library",
            Source::User => "user",
            Source::MachineClamped => "machine-clamped",
            Source::Limited => "limited",
        }
    }
}

/// One resolved value: what it is, its unit, where it came from, and why (the cite, the
/// default's basis, or the clamp's reason).
#[derive(Clone, Debug, PartialEq)]
pub struct Val {
    pub v: f64,
    pub unit: &'static str,
    pub source: Source,
    pub note: String,
}

impl Val {
    fn new(v: f64, unit: &'static str, source: Source, note: impl Into<String>) -> Val {
        Val {
            v,
            unit,
            source,
            note: note.into(),
        }
    }
}

/// The user's values (all optional). Per-tool values are merged under per-op ones by
/// `Ask::over`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ask {
    /// Cutting speed, m/min; or the spindle speed, rpm (rpm wins when both are given).
    pub vc: Option<f64>,
    pub rpm: Option<f64>,
    /// Chip to hold, mm (the peak chip hex, thinned for ae < D/2); or a programmed feed per
    /// tooth, mm (used as given: not thinned); or the feed itself, mm/min.
    pub chip: Option<f64>,
    pub fz: Option<f64>,
    pub feed: Option<f64>,
    /// Axial depth and radial width, mm; or the largest engagement angle, degrees.
    pub ap: Option<f64>,
    pub ae: Option<f64>,
    pub max_angle: Option<f64>,
    /// Entry feeds, mm/min.
    pub plunge: Option<f64>,
    pub ramp: Option<f64>,
    pub lead_in: Option<f64>,
    /// Limits: kW, N·m at the spindle; the feed range engagement feed stays in, mm/min.
    pub max_power: Option<f64>,
    pub max_torque: Option<f64>,
    pub min_feed: Option<f64>,
    pub max_feed: Option<f64>,
}

impl Ask {
    /// `self`'s values where given, else `base`'s (an op's over its tool's).
    pub fn over(self, base: Ask) -> Ask {
        Ask {
            vc: self.vc.or(base.vc),
            rpm: self.rpm.or(base.rpm),
            chip: self.chip.or(base.chip),
            fz: self.fz.or(base.fz),
            feed: self.feed.or(base.feed),
            ap: self.ap.or(base.ap),
            ae: self.ae.or(base.ae),
            max_angle: self.max_angle.or(base.max_angle),
            plunge: self.plunge.or(base.plunge),
            ramp: self.ramp.or(base.ramp),
            lead_in: self.lead_in.or(base.lead_in),
            max_power: self.max_power.or(base.max_power),
            max_torque: self.max_torque.or(base.max_torque),
            min_feed: self.min_feed.or(base.min_feed),
            max_feed: self.max_feed.or(base.max_feed),
        }
    }
}

/// The tool, as the resolver needs it.
#[derive(Clone, Copy, Debug)]
pub struct ToolData<'a> {
    pub d: f64,
    pub flutes: u32,
    pub ball: bool,
    pub carbide: bool,
    /// The tool's cutting data for the part's group, and its cite.
    pub preset: Option<(&'a Preset, Option<&'a Cite>)>,
}

/// The machine, as the resolver needs it.
#[derive(Clone, Copy, Debug)]
pub struct Spindle<'a> {
    pub rpm: [f64; 2],
    /// mm/min; 0 or less: not known.
    pub max_feed: f64,
    /// (rpm, kW), piecewise linear, rpm increasing; empty: no power data.
    pub power: &'a [[f64; 2]],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    /// Over a limit the op still runs with (reported).
    Warn,
    /// Over a hard limit: the op is refused.
    Fail,
    /// The data to check it is unknown.
    NotChecked,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Warn => "warn",
            Verdict::Fail => "fail",
            Verdict::NotChecked => "not checked",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    /// rpm | feed | power | torque | chip.
    pub what: &'static str,
    pub verdict: Verdict,
    pub value: f64,
    pub limit: f64,
    pub unit: &'static str,
    pub why: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    pub vc: Val,
    pub rpm: Val,
    /// The chip held (peak, mm), the programmed feed per tooth, and the feed.
    pub chip: Val,
    pub fz: Val,
    pub feed: Val,
    pub ap: Val,
    pub ae: Val,
    pub plunge: Val,
    pub ramp: Val,
    pub lead_in: Val,
    /// The feed range engagement feed may move in, mm/min.
    pub min_feed: Val,
    pub max_feed: Val,
    /// The derived numbers at the resolved values (hex, hm, power, torque, ...).
    pub feeds: feeds::Feeds,
    /// The feed holds a chip (the user's `chip`, the tool's or the material's), so engagement
    /// feed may move it along the path to hold that chip; false when the user programmed `fz`
    /// or `feed` (used as given, never moved).
    pub held_chip: bool,
    /// Material removal rate, cm³/min.
    pub mrr: f64,
    /// Spindle power and torque available at the resolved rpm, when the curve covers it.
    pub power_avail: Option<f64>,
    pub torque_avail: Option<f64>,
    pub checks: Vec<Check>,
}

/// How far under the machine's maximum feed a clamped feed is held, mm/min: the runtime limit
/// check (forge-cam-lang `runtime_limits`) compares the posted F against the limit's
/// outward-rounded lower end, so an F exactly at the limit is refused.
pub const FEED_MARGIN: f64 = 1.0;

/// Spindle power at `rpm` from a piecewise-linear (rpm, kW) curve; None outside it (no
/// extrapolation: what the curve does not cover is not known).
pub fn power_at(curve: &[[f64; 2]], rpm: f64) -> Option<f64> {
    if curve.is_empty() || rpm < curve[0][0] || rpm > curve[curve.len() - 1][0] {
        return None;
    }
    if curve.len() == 1 {
        return Some(curve[0][1]);
    }
    for w in curve.windows(2) {
        let ([a, pa], [b, pb]) = (w[0], w[1]);
        if rpm <= b {
            return Some(if b > a {
                pa + (pb - pa) * (rpm - a) / (b - a)
            } else {
                pb
            });
        }
    }
    None
}

/// Torque, N·m, of `kw` at `rpm`: M = P 30e3/(π n) (Sandvik, "Milling formulas").
pub fn torque_of(kw: f64, rpm: f64) -> f64 {
    if rpm > 0.0 { kw * 30.0e3 / (PI * rpm) } else { 0.0 }
}

/// Resolve a milling op's feeds and speeds. `nominal` is the op's own (ae, ap), used when the
/// user gives none. None when there is no speed at all (no user value, no tool or group data).
pub fn milling(
    ask: &Ask,
    tool: &ToolData,
    group: Option<&Group>,
    spindle: &Spindle,
    nominal: (f64, f64),
) -> Option<Resolved> {
    let d = tool.d;
    let z = tool.flutes.max(1) as f64;
    let mut checks = Vec::new();
    let cite_note = |c: Option<&Cite>, fallback: &str| match c {
        Some(c) if !c.source.is_empty() => {
            format!("{}: {}", format!("{:?}", c.status).to_lowercase(), c.source)
        }
        _ => fallback.to_string(),
    };

    // engagement
    let ae = if let Some(v) = ask.ae {
        Val::new(v, "mm", Source::User, "")
    } else if let Some(deg) = ask.max_angle {
        // φ = acos(1 − 2 ae/D)  ⇒  ae = D (1 − cos φ)/2
        let phi = deg.clamp(0.0, 180.0).to_radians();
        Val::new(
            0.5 * d * (1.0 - forge_math::dmath::cos(phi)),
            "mm",
            Source::User,
            format!("from max engagement {deg}°"),
        )
    } else {
        Val::new(nominal.0, "mm", Source::Default, "the op's nominal width")
    };
    let ap = match ask.ap {
        Some(v) => Val::new(v, "mm", Source::User, ""),
        None => Val::new(nominal.1, "mm", Source::Default, "the op's nominal depth"),
    };
    let d_eff = feeds::effective_diameter(d, tool.ball, ap.v);

    // speed
    let preset = tool.preset;
    let (n_asked, mut vc_val) = if let Some(n) = ask.rpm {
        (
            n,
            Val::new(
                n * PI * d_eff / 1000.0,
                "m/min",
                Source::User,
                "from the user's rpm",
            ),
        )
    } else {
        let vc = if let Some(v) = ask.vc {
            Val::new(v, "m/min", Source::User, "")
        } else if let Some((p, c)) = preset {
            Val::new(
                p.vc,
                "m/min",
                Source::Library,
                cite_note(c, "the tool's preset (uncited)"),
            )
        } else {
            let g = group?;
            let v = if tool.carbide { g.vc_carbide } else { g.vc_hss };
            Val::new(
                v,
                "m/min",
                Source::Material,
                format!("group {}: {}", g.group, g.source),
            )
        };
        (1000.0 * vc.v / (PI * d_eff.max(1e-6)), vc)
    };
    let rpm_src = if ask.rpm.is_some() {
        Source::User
    } else {
        vc_val.source
    };
    let n = n_asked.clamp(spindle.rpm[0], spindle.rpm[1]);
    let rpm = if n != n_asked {
        let why = format!(
            "asked {n_asked:.0} rpm; the machine runs {:.0}–{:.0}",
            spindle.rpm[0], spindle.rpm[1]
        );
        checks.push(Check {
            what: "rpm",
            verdict: Verdict::Warn,
            value: n_asked,
            limit: if n_asked > n {
                spindle.rpm[1]
            } else {
                spindle.rpm[0]
            },
            unit: "rpm",
            why: format!("clamped: {why}"),
        });
        vc_val = Val::new(
            n * PI * d_eff / 1000.0,
            "m/min",
            Source::MachineClamped,
            why.clone(),
        );
        Val::new(n, "rpm", Source::MachineClamped, why)
    } else {
        Val::new(
            n,
            "rpm",
            rpm_src,
            if d_eff < d {
                format!("at D_eff {d_eff:.3} mm")
            } else {
                String::new()
            },
        )
    };

    // chip and feed
    let r = (ae.v / d).clamp(0.0, 1.0);
    let k = feeds::thinning(r);
    let (chip, fz) = if let Some(f) = ask.feed {
        let fz = f / (n * z);
        (
            Val::new(fz * k, "mm", Source::User, "from the user's feed"),
            Val::new(fz, "mm", Source::User, "from the user's feed"),
        )
    } else if let Some(fz) = ask.fz {
        (
            Val::new(fz * k, "mm", Source::User, "from the user's fz (not thinned)"),
            Val::new(fz, "mm", Source::User, "programmed as given"),
        )
    } else {
        let chip = if let Some(h) = ask.chip {
            Val::new(h, "mm", Source::User, "")
        } else if let Some((p, c)) = preset {
            Val::new(
                p.fz,
                "mm",
                Source::Library,
                format!(
                    "{}; the tool's {} chip, thinned for this width",
                    cite_note(c, "the tool's preset (uncited)"),
                    if p.name.is_empty() { "preset" } else { &p.name }
                ),
            )
        } else if let Some(g) = group {
            Val::new(
                g.fz_at(d),
                "mm",
                Source::Material,
                format!("group {}: {}", g.group, g.source),
            )
        } else {
            Val::new(0.0, "mm", Source::Default, "no chip data")
        };
        let fz = if k > 0.0 {
            (chip.v / k).min(THIN_CAP * chip.v)
        } else {
            chip.v
        };
        let note = if k < 1.0 && k > 0.0 {
            format!(
                "chip {:.4} mm thinned ×{:.3} (ae/D {:.3})",
                chip.v,
                fz / chip.v.max(1e-12),
                r
            )
        } else {
            String::new()
        };
        let src = chip.source;
        (chip, Val::new(fz, "mm", src, note))
    };
    let mut fz = fz;
    let mut vf = Val::new(fz.v * n * z, "mm/min", fz.source, fz.note.clone());

    // the user's power and torque limits lower the feed (only the feed). P ∝ fz^(1 − mc)
    // at fixed n, ap, ae (kc = kc1.1 hm^(−mc), hm ∝ fz), so the largest fz under a cap is exact.
    if let Some(g) = group {
        let cut = Cut {
            d,
            flutes: tool.flutes,
            ball: tool.ball,
            carbide: tool.carbide,
            ap: ap.v,
            ae: ae.v,
        };
        for (cap, what) in [(ask.max_power, "max power"), (ask.max_torque, "max torque")] {
            let Some(cap) = cap else { continue };
            let f = feeds::at(g, &cut, fz.v, n, d_eff);
            let now = if what == "max power" { f.power } else { f.torque };
            if now > cap && now > 0.0 && g.mc < 1.0 {
                let s = forge_math::dmath::powf(cap / now, 1.0 / (1.0 - g.mc));
                let why = format!("lowered to hold {what} {cap} (was {now:.3})");
                fz = Val::new(fz.v * s, "mm", Source::Limited, why.clone());
                vf = Val::new(vf.v * s, "mm/min", Source::Limited, why);
            }
        }
    }
    let min_feed = match ask.min_feed {
        Some(v) => Val::new(v, "mm/min", Source::User, ""),
        None => Val::new(0.0, "mm/min", Source::Default, "no floor"),
    };
    let max_feed = match ask.max_feed {
        Some(v) => Val::new(v, "mm/min", Source::User, ""),
        None if spindle.max_feed > 0.0 => Val::new(
            spindle.max_feed,
            "mm/min",
            Source::MachineClamped,
            "the machine's maximum feed",
        ),
        None => Val::new(f64::INFINITY, "mm/min", Source::Default, "no ceiling known"),
    };
    // the user's own feed range (the machine's ceiling follows, as a machine clamp)
    let (lo, hi) = (ask.min_feed.unwrap_or(0.0), ask.max_feed.unwrap_or(f64::INFINITY));
    if vf.v > hi || vf.v < lo {
        let lim = vf.v.clamp(lo, hi);
        let why = format!(
            "asked {:.1} mm/min; the user's feed range is {lo:.1}–{hi:.1}",
            vf.v
        );
        fz = Val::new(fz.v * lim / vf.v.max(1e-12), "mm", Source::Limited, why.clone());
        vf = Val::new(lim, "mm/min", Source::Limited, why);
    }
    if spindle.max_feed > 0.0 && vf.v > spindle.max_feed - FEED_MARGIN {
        let to = (spindle.max_feed - FEED_MARGIN).max(0.5 * spindle.max_feed);
        let why = format!(
            "asked {:.1} mm/min; the machine feeds at most {:.1} (held {FEED_MARGIN} under it: a posted F at the limit is refused)",
            vf.v, spindle.max_feed
        );
        checks.push(Check {
            what: "feed",
            verdict: Verdict::Warn,
            value: vf.v,
            limit: spindle.max_feed,
            unit: "mm/min",
            why: format!("clamped: {why}"),
        });
        fz = Val::new(fz.v * to / vf.v, "mm", Source::MachineClamped, why.clone());
        vf = Val::new(to, "mm/min", Source::MachineClamped, why);
    }

    let entry = |v: Option<f64>, frac: f64, what: &str| match v {
        Some(x) => Val::new(x, "mm/min", Source::User, ""),
        None => Val::new(
            frac * vf.v,
            "mm/min",
            Source::Default,
            format!(
                "forgeCAD default: {:.0}% of the cutting feed ({what}); no data behind it",
                100.0 * frac
            ),
        ),
    };
    let plunge = entry(ask.plunge, 0.3, "plunge");
    let ramp = entry(ask.ramp, 0.3, "ramp");
    let lead_in = entry(ask.lead_in, 1.0, "lead-in");

    // derived numbers and checks
    let derived = match group {
        Some(g) => feeds::at(
            g,
            &Cut {
                d,
                flutes: tool.flutes,
                ball: tool.ball,
                carbide: tool.carbide,
                ap: ap.v,
                ae: ae.v,
            },
            fz.v,
            n,
            d_eff,
        ),
        None => feeds::Feeds {
            rpm: n,
            vc: vc_val.v,
            fz: fz.v,
            hex: fz.v * k,
            hm: fz.v * feeds::mean_chip_factor(r, feeds::engagement_angle(r)),
            phi: feeds::engagement_angle(r),
            vf: vf.v,
            power: f64::NAN,
            torque: f64::NAN,
            d_eff,
            rpm_limited: rpm.source == Source::MachineClamped,
        },
    };
    let mrr = ap.v * ae.v * vf.v / 1000.0;
    let power_avail = power_at(spindle.power, n);
    let torque_avail = power_avail.map(|p| torque_of(p, n));
    let kc_note = group.map(|g| format!("kc1.1 {} mc {} (group {}: {})", g.kc1_1, g.mc, g.group, g.source));
    for (what, have, avail, unit) in [
        ("power", derived.power, power_avail, "kW"),
        ("torque", derived.torque, torque_avail, "N·m"),
    ] {
        let check = match (group, avail) {
            (None, _) => Check {
                what,
                verdict: Verdict::NotChecked,
                value: f64::NAN,
                limit: avail.unwrap_or(f64::NAN),
                unit,
                why: "no cutting-force data (kc1.1, mc) for the part's material".into(),
            },
            (Some(_), None) => Check {
                what,
                verdict: Verdict::NotChecked,
                value: have,
                limit: f64::NAN,
                unit,
                why: if spindle.power.is_empty() {
                    "the machine has no spindle power curve".into()
                } else {
                    format!("{n:.0} rpm is outside the machine's power curve")
                },
            },
            (Some(_), Some(lim)) => Check {
                what,
                verdict: if have > lim { Verdict::Fail } else { Verdict::Pass },
                value: have,
                limit: lim,
                unit,
                why: format!(
                    "{:.0}% of the spindle's at {n:.0} rpm; {}",
                    100.0 * have / lim,
                    kc_note.clone().unwrap_or_default()
                ),
            },
        };
        checks.push(check);
    }
    checks.push(if spindle.max_feed > 0.0 {
        Check {
            what: "feed",
            verdict: if vf.v > spindle.max_feed {
                Verdict::Fail
            } else {
                Verdict::Pass
            },
            value: vf.v,
            limit: spindle.max_feed,
            unit: "mm/min",
            why: String::new(),
        }
    } else {
        Check {
            what: "feed",
            verdict: Verdict::NotChecked,
            value: vf.v,
            limit: f64::NAN,
            unit: "mm/min",
            why: "the machine's maximum feed is not known".into(),
        }
    });
    // the chip vs the tool's own data: more than its preset's chip is over the maker's number
    if let Some((p, _)) = preset {
        checks.push(Check {
            what: "chip",
            verdict: if derived.hex > p.fz * (1.0 + 1e-9) {
                Verdict::Warn
            } else {
                Verdict::Pass
            },
            value: derived.hex,
            limit: p.fz,
            unit: "mm",
            why: format!(
                "peak chip vs the tool's {} chip",
                if p.name.is_empty() { "preset" } else { &p.name }
            ),
        });
    } else {
        checks.push(Check {
            what: "chip",
            verdict: Verdict::NotChecked,
            value: derived.hex,
            limit: f64::NAN,
            unit: "mm",
            why: "the tool has no cutting data for this material".into(),
        });
    }

    Some(Resolved {
        vc: vc_val,
        rpm,
        chip,
        fz,
        feed: vf,
        ap,
        ae,
        plunge,
        ramp,
        lead_in,
        min_feed,
        max_feed,
        feeds: derived,
        held_chip: ask.feed.is_none() && ask.fz.is_none(),
        mrr,
        power_avail,
        torque_avail,
        checks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CiteStatus;

    fn group() -> Group {
        Group {
            group: "N".into(),
            name: "al".into(),
            vc_carbide: 400.0,
            vc_hss: 90.0,
            fz_d1: 0.01,
            fz_d20: 0.12,
            kc1_1: 700.0,
            mc: 0.25,
            source: "test group".into(),
            note: String::new(),
        }
    }
    fn preset() -> Preset {
        Preset {
            name: "N slot".into(),
            group: "N".into(),
            vc: 640.08,
            fz: 0.08128,
            ap: None,
            ae: None,
        }
    }
    fn cite() -> Cite {
        Cite {
            path: "preset.N slot".into(),
            status: CiteStatus::Estimate,
            source: "SF_H45AL-3.pdf".into(),
            taken: String::new(),
            value: String::new(),
            basis: String::new(),
            note: String::new(),
        }
    }
    const CURVE: [[f64; 2]; 3] = [[0.0, 0.0], [2000.0, 7.5], [10_000.0, 7.5]];
    fn spindle() -> Spindle<'static> {
        Spindle {
            rpm: [100.0, 10_000.0],
            max_feed: 10_000.0,
            power: &CURVE,
        }
    }
    fn tool<'a>(p: Option<(&'a Preset, Option<&'a Cite>)>) -> ToolData<'a> {
        ToolData {
            d: 12.7,
            flutes: 3,
            ball: false,
            carbide: true,
            preset: p,
        }
    }

    #[test]
    fn layers_material_then_library_then_user() {
        let g = group();
        let (p, c) = (preset(), cite());
        // material only
        let wide = Spindle {
            rpm: [100.0, 20_000.0],
            ..spindle()
        };
        let r = milling(&Ask::default(), &tool(None), Some(&g), &wide, (1.27, 5.0)).unwrap();
        assert_eq!(r.vc.source, Source::Material);
        assert_eq!(r.rpm.source, Source::Material);
        assert_eq!(r.chip.source, Source::Material);
        assert_eq!(r.ae.source, Source::Default);
        assert!((r.chip.v - g.fz_at(12.7)).abs() < 1e-15);
        // the tool's preset over the group
        let r = milling(
            &Ask::default(),
            &tool(Some((&p, Some(&c)))),
            Some(&g),
            &spindle(),
            (1.27, 5.0),
        )
        .unwrap();
        assert_eq!(r.chip.source, Source::Library);
        assert!(r.chip.note.contains("SF_H45AL-3.pdf"));
        assert_eq!(r.chip.v, 0.08128);
        // vc 640 at D 12.7 asks 16,043 rpm: the machine clamps it, and says so
        assert_eq!(r.rpm.source, Source::MachineClamped);
        assert_eq!(r.rpm.v, 10_000.0);
        assert!(
            r.checks
                .iter()
                .any(|k| k.what == "rpm" && k.verdict == Verdict::Warn)
        );
        // the user over both
        let ask = Ask {
            rpm: Some(8000.0),
            chip: Some(0.05),
            ae: Some(2.0),
            ..Ask::default()
        };
        let r = milling(
            &ask,
            &tool(Some((&p, Some(&c)))),
            Some(&g),
            &spindle(),
            (1.27, 5.0),
        )
        .unwrap();
        assert_eq!((r.rpm.source, r.rpm.v), (Source::User, 8000.0));
        assert_eq!((r.chip.source, r.chip.v), (Source::User, 0.05));
        assert_eq!((r.ae.source, r.ae.v), (Source::User, 2.0));
        assert!((r.feeds.hex - 0.05).abs() < 1e-12);
    }

    #[test]
    fn op_over_tool() {
        let tool_ask = Ask {
            vc: Some(300.0),
            fz: Some(0.04),
            ..Ask::default()
        };
        let op_ask = Ask {
            fz: Some(0.05),
            ..Ask::default()
        };
        let a = op_ask.over(tool_ask);
        assert_eq!((a.vc, a.fz), (Some(300.0), Some(0.05)));
    }

    #[test]
    fn user_fz_is_not_thinned_and_user_rpm_clamp_is_reported() {
        let g = group();
        let ask = Ask {
            rpm: Some(20_000.0),
            fz: Some(0.04),
            ..Ask::default()
        };
        let r = milling(&ask, &tool(None), Some(&g), &spindle(), (1.27, 5.0)).unwrap();
        assert_eq!(r.fz.v, 0.04);
        assert_eq!(r.rpm.source, Source::MachineClamped);
        assert!(r.rpm.note.contains("asked 20000 rpm"));
        let k = r.checks.iter().find(|k| k.what == "rpm").unwrap();
        assert_eq!((k.verdict, k.value, k.limit), (Verdict::Warn, 20_000.0, 10_000.0));
    }

    #[test]
    fn power_cap_lowers_the_feed_exactly() {
        let g = group();
        let ask = Ask {
            ae: Some(12.7),
            ap: Some(12.7),
            max_power: Some(2.0),
            ..Ask::default()
        };
        let r = milling(&ask, &tool(None), Some(&g), &spindle(), (0.0, 0.0)).unwrap();
        assert_eq!(r.feed.source, Source::Limited);
        assert!((r.feeds.power - 2.0).abs() < 1e-9, "{}", r.feeds.power);
        // ap and ae stay as the user set them
        assert_eq!((r.ap.v, r.ae.v), (12.7, 12.7));
        let ask = Ask {
            max_torque: Some(1.5),
            ..ask
        };
        let r = milling(&ask, &tool(None), Some(&g), &spindle(), (0.0, 0.0)).unwrap();
        assert!((r.feeds.torque - 1.5).abs() < 1e-9, "{}", r.feeds.torque);
    }

    #[test]
    fn spindle_power_check_and_not_checked() {
        let g = group();
        // a heavy slot at 2,000 rpm: over the 7.5 kW the curve gives there
        let ask = Ask {
            rpm: Some(2000.0),
            fz: Some(0.3),
            ae: Some(12.7),
            ap: Some(25.0),
            ..Ask::default()
        };
        let r = milling(&ask, &tool(None), Some(&g), &spindle(), (0.0, 0.0)).unwrap();
        let p = r.checks.iter().find(|k| k.what == "power").unwrap();
        assert_eq!(p.verdict, Verdict::Fail);
        assert_eq!(p.limit, 7.5);
        let t = r.checks.iter().find(|k| k.what == "torque").unwrap();
        assert!((t.limit - torque_of(7.5, 2000.0)).abs() < 1e-12);
        // no curve: not checked, never pass
        let bare = Spindle {
            rpm: [100.0, 10_000.0],
            max_feed: 0.0,
            power: &[],
        };
        let r = milling(&ask, &tool(None), Some(&g), &bare, (0.0, 0.0)).unwrap();
        for w in ["power", "torque", "feed", "chip"] {
            let k = r.checks.iter().find(|k| k.what == w).unwrap();
            assert_eq!(k.verdict, Verdict::NotChecked, "{w}");
        }
        // no material data: power not checked even with a curve
        let r = milling(&ask, &tool(None), None, &spindle(), (0.0, 0.0)).unwrap();
        assert_eq!(
            r.checks.iter().find(|k| k.what == "power").unwrap().verdict,
            Verdict::NotChecked
        );
        // no speed from anywhere: nothing resolved
        assert!(milling(&Ask::default(), &tool(None), None, &spindle(), (1.0, 1.0)).is_none());
    }

    #[test]
    fn max_angle_gives_ae() {
        let g = group();
        let ask = Ask {
            max_angle: Some(90.0),
            ..Ask::default()
        };
        let r = milling(&ask, &tool(None), Some(&g), &spindle(), (0.0, 1.0)).unwrap();
        assert!((r.ae.v - 6.35).abs() < 1e-12);
        assert!((r.feeds.phi - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn machine_feed_clamp_is_reported() {
        let g = group();
        let ask = Ask {
            feed: Some(15_000.0),
            ..Ask::default()
        };
        let r = milling(&ask, &tool(None), Some(&g), &spindle(), (1.0, 1.0)).unwrap();
        assert_eq!(
            (r.feed.v, r.feed.source),
            (10_000.0 - FEED_MARGIN, Source::MachineClamped)
        );
        assert!(
            r.checks
                .iter()
                .any(|k| k.what == "feed" && k.verdict == Verdict::Warn)
        );
        assert!(r.feed.note.contains("15000.0"));
    }

    #[test]
    fn power_curve_interpolates_and_stops_at_its_ends() {
        assert_eq!(power_at(&CURVE, 1000.0), Some(3.75));
        assert_eq!(power_at(&CURVE, 10_000.0), Some(7.5));
        assert_eq!(power_at(&CURVE, 10_001.0), None);
        assert_eq!(power_at(&[], 1000.0), None);
    }
}
