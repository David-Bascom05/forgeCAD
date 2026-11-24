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
use crate::feeds::{self, Cut, DrillRow, Group, Stock, Substrate, THIN_CAP};
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
    /// Tool tilt from the surface normal toward the feed, degrees (a ball's contact moves out
    /// to a larger diameter): feeds::contact.
    pub tilt: Option<f64>,
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
            tilt: self.tilt.or(base.tilt),
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
    /// Corner radius, mm: 0 flat, d/2 ball (`ball` implies it), between for a bull nose.
    pub re: f64,
    /// Its substrate (picks the group's carbide or HSS speed; others have no generic speed)
    /// and coating as the record states it (no generic factor is applied for a coating: the
    /// maker's preset carries it).
    pub substrate: Substrate,
    pub coating: &'a str,
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
    /// Power and torque at the upper ends of the material's cited kc1.1/mc ranges (the nominal
    /// values where it cites none).
    pub power_hi: f64,
    pub torque_hi: f64,
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
    milling_on(ask, tool, group, None, spindle, nominal)
}

/// As `milling`, for a specific stock material: its record's kc1.1 and mc at the cut's actual
/// speed (and their measured spread) take the group's place for power and torque.
pub fn milling_on(
    ask: &Ask,
    tool: &ToolData,
    group: Option<&Group>,
    stock: Option<&Stock>,
    spindle: &Spindle,
    nominal: (f64, f64),
) -> Option<Resolved> {
    let d = tool.d;
    let carbide = tool.substrate == Substrate::Carbide;
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
    // where the edge cuts: D_eff and the axial thinning sin κ (flat, bull, ball, tilted ball)
    let re = if tool.ball { 0.5 * d } else { tool.re };
    let ct = feeds::contact(d, re, ap.v, ask.tilt.unwrap_or(0.0));
    let d_eff = ct.d_eff;
    // the width is cut at D_eff on a ball (Sandvik's side-milling form), at D otherwise
    let d_radial = if re >= 0.5 * d { d_eff } else { d };

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
            // the group's generic speeds are per substrate; a coating moves no generic number
            let v = match tool.substrate {
                Substrate::Carbide => g.vc_carbide,
                Substrate::Hss => g.vc_hss,
                Substrate::Other | Substrate::Unknown => return None,
            };
            let coat = if tool.coating.is_empty() {
                String::new()
            } else {
                format!("; coating {} (no generic factor)", tool.coating)
            };
            Val::new(
                v,
                "m/min",
                Source::Material,
                format!("group {} {}: {}{coat}", g.group, tool.substrate.name(), g.source),
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
    let r = (ae.v / d_radial.max(1e-9)).clamp(0.0, 1.0);
    let k = feeds::thinning(r) * ct.sin_kappa;
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
        } else {
            // no chip from any layer: nothing to set the feed from
            let g = group?;
            Val::new(
                g.fz_at(d),
                "mm",
                Source::Material,
                format!("group {}: {}", g.group, g.source),
            )
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
    // from here the cutting force: the stock material's own data at the actual speed where its
    // record has some, else the group's
    let kz: Option<Group> = group.map(|g| match stock {
        Some(st) => st.apply(g, vc_val.v),
        None => g.clone(),
    });
    let group = kz.as_ref();

    // the user's power and torque limits lower the feed (only the feed). P ∝ fz^(1 − mc)
    // at fixed n, ap, ae (kc = kc1.1 hm^(−mc), hm ∝ fz), so the largest fz under a cap is exact.
    if let Some(g) = group {
        let cut = Cut {
            d,
            flutes: tool.flutes,
            ball: tool.ball,
            carbide,
            ap: ap.v,
            ae: ae.v,
        };
        for (cap, what) in [(ask.max_power, "max power"), (ask.max_torque, "max torque")] {
            let Some(cap) = cap else { continue };
            let f = feeds::at_contact(g, &cut, fz.v, n, ct, d_radial);
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
        Some(g) => feeds::at_contact(
            g,
            &Cut {
                d,
                flutes: tool.flutes,
                ball: tool.ball,
                carbide,
                ap: ap.v,
                ae: ae.v,
            },
            fz.v,
            n,
            ct,
            d_radial,
        ),
        None => feeds::Feeds {
            rpm: n,
            vc: vc_val.v,
            fz: fz.v,
            hex: fz.v * k,
            hm: fz.v * ct.sin_kappa * feeds::mean_chip_factor(r, feeds::engagement_angle(r)),
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
    // the same cut at the upper ends of the cited kc1.1/mc ranges
    let (power_hi, torque_hi) = match group {
        Some(g) if g.kc1_1_range.is_some() || g.mc_range.is_some() => {
            let (kc_hi, mc_hi) = g.upper();
            let hi = Group {
                kc1_1: kc_hi,
                mc: mc_hi,
                ..g.clone()
            };
            let f = feeds::at_contact(
                &hi,
                &Cut {
                    d,
                    flutes: tool.flutes,
                    ball: tool.ball,
                    carbide,
                    ap: ap.v,
                    ae: ae.v,
                },
                fz.v,
                n,
                ct,
                d_radial,
            );
            (f.power, f.torque)
        }
        _ => (derived.power, derived.torque),
    };
    for (what, have, hi, avail, unit) in [
        ("power", derived.power, power_hi, power_avail, "kW"),
        ("torque", derived.torque, torque_hi, torque_avail, "N·m"),
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
            (Some(_), Some(lim)) if have <= lim && hi > lim => Check {
                what,
                verdict: Verdict::Warn,
                value: hi,
                limit: lim,
                unit,
                why: format!(
                    "{:.0}% of the spindle's at {n:.0} rpm with the data's nominal kc, {:.0}% at the upper ends of its measured range; {}",
                    100.0 * have / lim,
                    100.0 * hi / lim,
                    kc_note.clone().unwrap_or_default()
                ),
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
        power_hi,
        torque_hi,
        power_avail,
        torque_avail,
        checks,
    })
}

/// The user's turning values (all optional).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurnAsk {
    /// Constant surface speed, m/min (G96); or a fixed spindle speed, rpm (G97; wins).
    pub vc: Option<f64>,
    pub rpm: Option<f64>,
    /// The CSS top speed (G50), rpm.
    pub max_rpm: Option<f64>,
    /// Feed per revolution, mm/rev; depth of cut, mm.
    pub f: Option<f64>,
    pub ap: Option<f64>,
    pub max_power: Option<f64>,
}

/// The turning tool's own cutting data (the library layer) and its entering angle.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnData {
    /// m/min, mm/rev, mm; None where the tool states none.
    pub vc: Option<f64>,
    pub f: Option<f64>,
    pub ap: Option<f64>,
    /// Where the tool's data comes from (its cite or record source).
    pub source: String,
    /// The entering angle κr, degrees (the holder's `kr`); None: not known.
    pub kr: Option<f64>,
}

/// The cut at one diameter of the op's range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurnPoint {
    /// Diameter, mm.
    pub d: f64,
    pub rpm: f64,
    /// The surface speed actually cut at, m/min (under vc where the top speed clamps).
    pub vc: f64,
    /// mm/min.
    pub feed: f64,
    /// kW, N·m (NaN: no kc data).
    pub power: f64,
    pub torque: f64,
    /// Spindle power available at that rpm (None: outside the curve or no curve).
    pub power_avail: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnResolved {
    /// G96 surface speed, or the speed of a fixed rpm at the largest diameter.
    pub vc: Val,
    /// G97 spindle speed, when fixed.
    pub rpm: Option<Val>,
    /// The CSS top speed (G50).
    pub max_rpm: Val,
    pub f: Val,
    pub ap: Val,
    pub kr: Val,
    /// The chip, mm: hm = f sin κr (Sandvik, "Turning formulas": hex = fn sin κr).
    pub hm: f64,
    /// Below this diameter CSS is clamped at `max_rpm` (the surface speed falls), mm.
    pub d_clamp: f64,
    /// The cut at the op's largest diameter, at d_clamp when inside the range, and at its least.
    pub at: Vec<TurnPoint>,
    pub checks: Vec<Check>,
}

/// Resolve a turning op's speed, feed and depth over the diameters it cuts, [d_min, d_max] mm.
/// Layers: library (the tool's own vc, f, ap) → user; the material gives only kc1.1/mc (the
/// milling group's speeds are not turning data and are not used). CSS: n(d) = 1000 vc/(π d),
/// clamped to the top speed; Pc = vc ap f kc/60e3 kW with kc = kc1.1 hm^(−mc), hm = f sin κr
/// (Sandvik Coromant, "Turning formulas and definitions"); Mc = Pc 30e3/(π n). None when no
/// speed, feed or depth is known.
pub fn turning(
    ask: &TurnAsk,
    tool: &TurnData,
    group: Option<&Group>,
    spindle: &Spindle,
    d_range: [f64; 2],
) -> Option<TurnResolved> {
    let lib = |v: Option<f64>, unit: &'static str| {
        v.map(|x| Val::new(x, unit, Source::Library, tool.source.clone()))
    };
    let user = |v: Option<f64>, unit: &'static str| v.map(|x| Val::new(x, unit, Source::User, ""));
    let f = user(ask.f, "mm/rev").or_else(|| lib(tool.f, "mm/rev"))?;
    let ap = user(ask.ap, "mm").or_else(|| lib(tool.ap, "mm"))?;
    let (d_min, d_max) = (d_range[0].max(0.0), d_range[1].max(d_range[0]).max(1e-6));
    let mut checks = Vec::new();
    // the top speed: the user's, held to the machine's; else the machine's own
    let max_rpm = match ask.max_rpm {
        Some(m) if m > spindle.rpm[1] => {
            let why = format!(
                "asked a top speed of {m:.0} rpm; the spindle's is {:.0}",
                spindle.rpm[1]
            );
            checks.push(Check {
                what: "rpm",
                verdict: Verdict::Warn,
                value: m,
                limit: spindle.rpm[1],
                unit: "rpm",
                why: format!("clamped: {why}"),
            });
            Val::new(spindle.rpm[1], "rpm", Source::MachineClamped, why)
        }
        Some(m) => Val::new(m, "rpm", Source::User, "G50"),
        None => Val::new(
            spindle.rpm[1],
            "rpm",
            Source::MachineClamped,
            "the spindle's top speed (G50)",
        ),
    };
    let rpm_fixed = ask.rpm.map(|n| {
        let c = n.clamp(spindle.rpm[0], max_rpm.v);
        if c != n {
            let why = format!(
                "asked {n:.0} rpm; the spindle runs {:.0}–{:.0}",
                spindle.rpm[0], max_rpm.v
            );
            checks.push(Check {
                what: "rpm",
                verdict: Verdict::Warn,
                value: n,
                limit: if n > c { max_rpm.v } else { spindle.rpm[0] },
                unit: "rpm",
                why: format!("clamped: {why}"),
            });
            Val::new(c, "rpm", Source::MachineClamped, why)
        } else {
            Val::new(n, "rpm", Source::User, "G97")
        }
    });
    let vc = match &rpm_fixed {
        Some(n) => Val::new(
            PI * d_max * n.v / 1000.0,
            "m/min",
            n.source,
            "at the largest diameter",
        ),
        None => user(ask.vc, "m/min").or_else(|| lib(tool.vc, "m/min"))?,
    };
    let kr = match tool.kr {
        Some(k) => Val::new(k, "deg", Source::Library, "the holder's entering angle"),
        None => Val::new(
            90.0,
            "deg",
            Source::Default,
            "not known: 90° (sin κr = 1, the largest chip, the most power)",
        ),
    };
    let hm = f.v * forge_math::dmath::sin(kr.v.to_radians());
    let d_clamp = if rpm_fixed.is_none() {
        1000.0 * vc.v / (PI * max_rpm.v)
    } else {
        0.0
    };
    let mut ds = vec![d_max];
    if d_clamp > d_min && d_clamp < d_max {
        ds.push(d_clamp);
    }
    if d_min < d_max {
        ds.push(d_min);
    }
    let kc = group.map(|g| g.kc1_1 * forge_math::dmath::powf(hm.max(1e-4), -g.mc));
    let at: Vec<TurnPoint> = ds
        .iter()
        .map(|&d| {
            let n = match &rpm_fixed {
                Some(n) => n.v,
                None => {
                    let n0 = 1000.0 * vc.v / (PI * d.max(1e-6));
                    // CSS under the top speed; at the large end the spindle's least speed holds
                    n0.min(max_rpm.v).max(spindle.rpm[0])
                }
            };
            let vcd = PI * d * n / 1000.0;
            let power = kc.map_or(f64::NAN, |kc| vcd * ap.v * f.v * kc / 60.0e3);
            TurnPoint {
                d,
                rpm: n,
                vc: vcd,
                feed: f.v * n,
                power,
                torque: if power.is_finite() {
                    torque_of(power, n)
                } else {
                    f64::NAN
                },
                power_avail: power_at(spindle.power, n),
            }
        })
        .collect();
    if rpm_fixed.is_none() && 1000.0 * vc.v / (PI * d_max) < spindle.rpm[0] {
        checks.push(Check {
            what: "rpm",
            verdict: Verdict::Warn,
            value: 1000.0 * vc.v / (PI * d_max),
            limit: spindle.rpm[0],
            unit: "rpm",
            why: format!(
                "at Ø{d_max:.1} CSS asks under the spindle's least speed: the surface speed there is over vc"
            ),
        });
    }
    // power and torque: the worst point against what the curve gives at its rpm
    for (what, unit) in [("power", "kW"), ("torque", "N·m")] {
        let worst = at
            .iter()
            .map(|p| {
                let (have, lim) = if what == "power" {
                    (p.power, p.power_avail)
                } else {
                    (p.torque, p.power_avail.map(|a| torque_of(a, p.rpm)))
                };
                (have, lim, p)
            })
            .max_by(|a, b| {
                let r = |x: &(f64, Option<f64>, &TurnPoint)| x.1.map_or(f64::NEG_INFINITY, |l| x.0 / l);
                r(a).total_cmp(&r(b))
            });
        let check = match (group, worst) {
            (None, _) => Check {
                what,
                verdict: Verdict::NotChecked,
                value: f64::NAN,
                limit: f64::NAN,
                unit,
                why: "no cutting-force data (kc1.1, mc) for the part's material".into(),
            },
            (Some(_), Some((have, Some(lim), p))) if at.iter().all(|q| q.power_avail.is_some()) => Check {
                what,
                verdict: if have > lim { Verdict::Fail } else { Verdict::Pass },
                value: have,
                limit: lim,
                unit,
                why: format!("worst at Ø{:.1}, {:.0} rpm", p.d, p.rpm),
            },
            (Some(_), _) => Check {
                what,
                verdict: Verdict::NotChecked,
                value: at
                    .iter()
                    .map(|p| if what == "power" { p.power } else { p.torque })
                    .fold(f64::NAN, f64::max),
                limit: f64::NAN,
                unit,
                why: if spindle.power.is_empty() {
                    "the machine has no spindle power curve".into()
                } else {
                    "an rpm the op runs at is outside the machine's power curve".into()
                },
            },
        };
        checks.push(check);
    }
    if let Some(cap) = ask.max_power {
        let top = at.iter().map(|p| p.power).fold(f64::NAN, f64::max);
        checks.push(Check {
            what: "power",
            verdict: if top > cap {
                Verdict::Fail
            } else if top.is_finite() {
                Verdict::Pass
            } else {
                Verdict::NotChecked
            },
            value: top,
            limit: cap,
            unit: "kW",
            why: "the user's max_power".into(),
        });
    }
    let top_feed = at.iter().map(|p| p.feed).fold(0.0, f64::max);
    checks.push(if spindle.max_feed > 0.0 {
        Check {
            what: "feed",
            verdict: if top_feed > spindle.max_feed {
                Verdict::Fail
            } else {
                Verdict::Pass
            },
            value: top_feed,
            limit: spindle.max_feed,
            unit: "mm/min",
            why: "f × the highest rpm the op runs at".into(),
        }
    } else {
        Check {
            what: "feed",
            verdict: Verdict::NotChecked,
            value: top_feed,
            limit: f64::NAN,
            unit: "mm/min",
            why: "the machine's maximum feed is not known".into(),
        }
    });
    Some(TurnResolved {
        vc,
        rpm: rpm_fixed,
        max_rpm,
        f,
        ap,
        kr,
        hm,
        d_clamp,
        at,
        checks,
    })
}

/// The peck default and its source: PMPA Craftsman Cribsheet #116, "Twist drills for success"
/// (D. Wynn, 2023-06-01, https://www.pmpa.org/cribsheet-twist-drills-for-success/): pull-outs
/// once the hole is deeper than 3 to 4 diameters; pecks of 3.5 D, then 2 D, then about 1 D
/// (1–1.5 D). A canned peck cycle (G83) takes one Q, so every peck is held at PMPA's 1 D (its
/// first two pecks are deeper: this errs slow), from a depth of 3 D (the low end of 3–4 D).
pub const PECK_RULE: &str = "PMPA Cribsheet #116 (pmpa.org, 2023): peck past 3-4 D, 3.5 D / 2 D / 1 D; held at 1 D for every peck (one G83 Q; errs slow), from 3 D";

/// The user's drilling values (all optional).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrillAsk {
    /// m/min, or rpm (wins).
    pub vc: Option<f64>,
    pub rpm: Option<f64>,
    /// Feed per revolution, mm/rev; or the penetration rate, mm/min.
    pub f: Option<f64>,
    pub feed: Option<f64>,
    /// Peck depth, mm (0: no pecking).
    pub peck: Option<f64>,
    pub max_power: Option<f64>,
    pub max_torque: Option<f64>,
}

/// The drill, as the drilling model needs it.
#[derive(Clone, Copy, Debug)]
pub struct DrillTool<'a> {
    pub d: f64,
    pub flutes: u32,
    /// Point angle, degrees (κr = half of it); 0: not known.
    pub point: f64,
    pub substrate: Substrate,
    /// The maker's data for the group: its `fz` is the maker's feed per REVOLUTION (drill
    /// tables state IPR or mm/rev), its vc the speed.
    pub preset: Option<(&'a Preset, Option<&'a Cite>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrillResolved {
    pub vc: Val,
    pub rpm: Val,
    /// mm/rev and mm/min.
    pub f: Val,
    pub feed: Val,
    pub peck: Val,
    /// The chip per edge, mm: (fn/z) sin κr.
    pub h: f64,
    /// Material removal rate, cm³/min: Dc fn vc / 4.
    pub mrr: f64,
    /// kW, N·m (NaN: no kc data); at the upper ends of a cited range.
    pub power: f64,
    pub torque: f64,
    pub power_hi: f64,
    pub power_avail: Option<f64>,
    pub checks: Vec<Check>,
}

/// Resolve a drilling op (spot, drill, peck), a model of its own (not the milling feed's
/// fraction). Layers: material (the group's `[[drill]]` row for the substrate: vc, and fn by
/// diameter) → library (the drill's preset: vc and fn) → user. Then machine clamps (rpm, feed)
/// and the user's limits, reported. Sandvik Coromant, "Drilling formulas and definitions":
/// n = 1000 vc/(π Dc), vf = fn n, Q = Dc fn vc/4 cm³/min, Pc = fn vc Dc kc/240e3 kW,
/// Mc = Pc 30e3/(π n) N·m, kc at the chip fz sin κr (fz = fn/z; the page prints the exponent as
/// "^mc", read here as Kienzle's −mc, as its milling page has it). Peck: the user's, else
/// `PECK_RULE` (PMPA's schedule, held at its 1 D).
/// None when no speed or feed is known from any layer.
#[allow(clippy::too_many_arguments)]
pub fn drilling(
    ask: &DrillAsk,
    tool: &DrillTool,
    group: Option<&Group>,
    stock: Option<&Stock>,
    row: Option<&DrillRow>,
    spindle: &Spindle,
    depth: f64,
) -> Option<DrillResolved> {
    let d = tool.d.max(1e-6);
    let mut checks = Vec::new();
    let note_of = |c: Option<&Cite>, f: &str| match c {
        Some(c) if !c.source.is_empty() => {
            format!("{}: {}", format!("{:?}", c.status).to_lowercase(), c.source)
        }
        _ => f.to_string(),
    };
    let row_note = |r: &DrillRow, what: &str| {
        let c = r.cites.iter().find(|c| c.path == what);
        format!(
            "group {} {} drilling data; {}",
            r.group,
            r.substrate,
            note_of(c, "uncited")
        )
    };
    // speed
    let (n_asked, mut vc) = if let Some(n) = ask.rpm {
        (
            n,
            Val::new(PI * d * n / 1000.0, "m/min", Source::User, "from the user's rpm"),
        )
    } else {
        let vc = if let Some(v) = ask.vc {
            Val::new(v, "m/min", Source::User, "")
        } else if let Some((p, c)) = tool.preset {
            Val::new(
                p.vc,
                "m/min",
                Source::Library,
                note_of(c, "the drill's preset (uncited)"),
            )
        } else {
            let r = row?;
            Val::new(r.vc, "m/min", Source::Material, row_note(r, "vc"))
        };
        (1000.0 * vc.v / (PI * d), vc)
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
        vc = Val::new(PI * d * n / 1000.0, "m/min", Source::MachineClamped, why.clone());
        Val::new(n, "rpm", Source::MachineClamped, why)
    } else {
        Val::new(
            n,
            "rpm",
            if ask.rpm.is_some() {
                Source::User
            } else {
                vc.source
            },
            "",
        )
    };
    // feed per revolution
    let mut f = if let Some(v) = ask.feed {
        Val::new(v / n, "mm/rev", Source::User, "from the user's feed")
    } else if let Some(v) = ask.f {
        Val::new(v, "mm/rev", Source::User, "")
    } else if let Some((p, c)) = tool.preset {
        Val::new(
            p.fz,
            "mm/rev",
            Source::Library,
            note_of(c, "the drill's preset (uncited)"),
        )
    } else {
        let r = row?;
        let (fv, how) = r.f_at(d);
        Val::new(
            fv,
            "mm/rev",
            Source::Material,
            format!("{} ({how})", row_note(r, "f_rows")),
        )
    };
    let mut feed = Val::new(f.v * n, "mm/min", f.source, f.note.clone());
    // kc at the actual speed (a stock record's rows where it has them)
    let kz: Option<Group> = group.map(|g| match stock {
        Some(st) => st.apply(g, vc.v),
        None => g.clone(),
    });
    let z = tool.flutes.max(1) as f64;
    let sk = if tool.point > 0.0 {
        forge_math::dmath::sin((0.5 * tool.point).to_radians())
    } else {
        1.0
    };
    let pw = |kc1: f64, mc: f64, fv: f64| {
        let h = (fv / z * sk).max(1e-4);
        let kc = kc1 * forge_math::dmath::powf(h, -mc);
        fv * vc.v * d * kc / 240.0e3
    };
    // the user's power/torque limits lower the feed only: P ∝ fn^(1 − mc)
    if let Some(g) = kz.as_ref() {
        for (cap, what) in [(ask.max_power, "max power"), (ask.max_torque, "max torque")] {
            let Some(cap) = cap else { continue };
            let p = pw(g.kc1_1, g.mc, f.v);
            let now = if what == "max power" { p } else { torque_of(p, n) };
            if now > cap && g.mc < 1.0 {
                let s = forge_math::dmath::powf(cap / now, 1.0 / (1.0 - g.mc));
                let why = format!("lowered to hold {what} {cap} (was {now:.3})");
                f = Val::new(f.v * s, "mm/rev", Source::Limited, why.clone());
                feed = Val::new(feed.v * s, "mm/min", Source::Limited, why);
            }
        }
    }
    if spindle.max_feed > 0.0 && feed.v > spindle.max_feed - FEED_MARGIN {
        let to = (spindle.max_feed - FEED_MARGIN).max(0.5 * spindle.max_feed);
        let why = format!(
            "asked {:.1} mm/min; the machine feeds at most {:.1}",
            feed.v, spindle.max_feed
        );
        checks.push(Check {
            what: "feed",
            verdict: Verdict::Warn,
            value: feed.v,
            limit: spindle.max_feed,
            unit: "mm/min",
            why: format!("clamped: {why}"),
        });
        f = Val::new(to / n, "mm/rev", Source::MachineClamped, why.clone());
        feed = Val::new(to, "mm/min", Source::MachineClamped, why);
    }
    let peck = match ask.peck {
        Some(p) => Val::new(p, "mm", Source::User, ""),
        None if depth > 3.0 * d => Val::new(d, "mm", Source::Default, PECK_RULE),
        None => Val::new(0.0, "mm", Source::Default, PECK_RULE),
    };
    let (power, power_hi) = match kz.as_ref() {
        Some(g) => {
            let (kh, mh) = g.upper();
            (pw(g.kc1_1, g.mc, f.v), pw(kh, mh, f.v))
        }
        None => (f64::NAN, f64::NAN),
    };
    let torque = if power.is_finite() {
        torque_of(power, n)
    } else {
        f64::NAN
    };
    let power_avail = power_at(spindle.power, n);
    for (what, have, hi, avail, unit) in [
        ("power", power, power_hi, power_avail, "kW"),
        (
            "torque",
            torque,
            torque_of(power_hi, n),
            power_avail.map(|a| torque_of(a, n)),
            "N·m",
        ),
    ] {
        checks.push(match (have.is_finite(), avail) {
            (false, _) => Check {
                what,
                verdict: Verdict::NotChecked,
                value: f64::NAN,
                limit: avail.unwrap_or(f64::NAN),
                unit,
                why: "no cutting-force data (kc1.1, mc) for the part's material".into(),
            },
            (true, None) => Check {
                what,
                verdict: Verdict::NotChecked,
                value: have,
                limit: f64::NAN,
                unit,
                why: "no spindle power at this rpm (no curve, or outside it)".into(),
            },
            (true, Some(lim)) => Check {
                what,
                verdict: if have > lim {
                    Verdict::Fail
                } else if hi > lim {
                    Verdict::Warn
                } else {
                    Verdict::Pass
                },
                value: if have > lim || hi <= lim { have } else { hi },
                limit: lim,
                unit,
                why: format!(
                    "{:.0}% of the spindle's at {n:.0} rpm ({:.0}% at the data's upper ends)",
                    100.0 * have / lim,
                    100.0 * hi / lim
                ),
            },
        });
    }
    Some(DrillResolved {
        h: f.v / z * sk,
        mrr: d * f.v * vc.v / 4.0,
        vc,
        rpm,
        f,
        feed,
        peck,
        power,
        torque,
        power_hi,
        power_avail,
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
            ..Group::default()
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
            re: 0.0,
            substrate: Substrate::Carbide,
            coating: "",
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

    fn turn_tool() -> TurnData {
        TurnData {
            vc: Some(200.0),
            f: Some(0.3),
            ap: Some(3.0),
            source: "test insert".into(),
            kr: Some(90.0),
        }
    }

    /// Sandvik's turning power Pc = vc ap fn kc / 60e3 at kc = kc1.1 hm^(−mc), hm = fn sin κr:
    /// vc 200, ap 3, fn 0.3, κr 90°, kc1.1 1700, mc 0.25: kc = 1700·0.3^−0.25 = 2297.0,
    /// Pc = 200·3·0.3·2297.0/60e3 = 6.891 kW. CSS clamps at 3000 rpm below Ø21.22.
    #[test]
    fn turning_css_and_power() {
        let mut g = group();
        g.kc1_1 = 1700.0;
        const C: [[f64; 2]; 2] = [[0.0, 15.0], [6000.0, 15.0]];
        let sp = Spindle {
            rpm: [50.0, 6000.0],
            max_feed: 5000.0,
            power: &C,
        };
        let ask = TurnAsk {
            max_rpm: Some(3000.0),
            ..TurnAsk::default()
        };
        let r = turning(&ask, &turn_tool(), Some(&g), &sp, [10.0, 80.0]).unwrap();
        assert_eq!((r.vc.v, r.vc.source), (200.0, Source::Library));
        assert_eq!((r.max_rpm.v, r.max_rpm.source), (3000.0, Source::User));
        assert!((r.d_clamp - 21.2207).abs() < 1e-4, "{}", r.d_clamp);
        assert_eq!(r.at.len(), 3);
        let big = r.at[0];
        assert!((big.rpm - 1000.0 * 200.0 / (PI * 80.0)).abs() < 1e-9);
        assert!((big.power - 6.891).abs() < 5e-4, "{}", big.power);
        // under d_clamp the speed holds at the top speed and the surface speed falls
        let small = r.at[2];
        assert_eq!((small.d, small.rpm), (10.0, 3000.0));
        assert!((small.vc - PI * 10.0 * 3.0).abs() < 1e-9);
        assert!(small.power < big.power);
        // torque is largest at the largest diameter (the lowest rpm)
        assert!(big.torque > r.at[1].torque);
        assert!(
            r.checks.iter().all(|c| c.verdict == Verdict::Pass),
            "{:?}",
            r.checks
        );
        // no top speed given: the machine's, labelled
        let r = turning(&TurnAsk::default(), &turn_tool(), Some(&g), &sp, [10.0, 80.0]).unwrap();
        assert_eq!((r.max_rpm.v, r.max_rpm.source), (6000.0, Source::MachineClamped));
        // a top speed over the machine's is clamped and reported
        let ask = TurnAsk {
            max_rpm: Some(9000.0),
            ..TurnAsk::default()
        };
        let r = turning(&ask, &turn_tool(), Some(&g), &sp, [10.0, 80.0]).unwrap();
        assert_eq!(r.max_rpm.v, 6000.0);
        assert!(
            r.checks
                .iter()
                .any(|c| c.what == "rpm" && c.verdict == Verdict::Warn)
        );
        // an entering angle of 45° thins the chip: hm = f sin 45°
        let mut t = turn_tool();
        t.kr = Some(45.0);
        let r = turning(&TurnAsk::default(), &t, Some(&g), &sp, [10.0, 80.0]).unwrap();
        assert!((r.hm - 0.3 * std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        // a weak spindle: refused at the worst point; no data: not checked
        const W: [[f64; 2]; 2] = [[0.0, 2.0], [6000.0, 2.0]];
        let weak = Spindle { power: &W, ..sp };
        let r = turning(&TurnAsk::default(), &turn_tool(), Some(&g), &weak, [10.0, 80.0]).unwrap();
        assert_eq!(
            r.checks.iter().find(|c| c.what == "power").unwrap().verdict,
            Verdict::Fail
        );
        let r = turning(&TurnAsk::default(), &turn_tool(), None, &weak, [10.0, 80.0]).unwrap();
        assert_eq!(
            r.checks.iter().find(|c| c.what == "power").unwrap().verdict,
            Verdict::NotChecked
        );
        // user G97 rpm wins over CSS
        let ask = TurnAsk {
            rpm: Some(1000.0),
            ..TurnAsk::default()
        };
        let r = turning(&ask, &turn_tool(), Some(&g), &sp, [10.0, 80.0]).unwrap();
        assert!(r.at.iter().all(|p| p.rpm == 1000.0));
        assert!((r.vc.v - PI * 80.0).abs() < 1e-9);
    }

    /// With a cited range, the power is also checked at its upper ends: within the spindle at
    /// the nominal kc but over it at the upper ends warns (never a silent pass).
    #[test]
    fn power_at_the_upper_ends_warns() {
        let mut g = group();
        g.kc1_1_range = Some([350.0, 1400.0]);
        g.mc_range = Some([0.2, 0.3]);
        let ask = Ask {
            rpm: Some(5000.0),
            fz: Some(0.05),
            ae: Some(12.7),
            ap: Some(12.7),
            ..Ask::default()
        };
        let r = milling(&ask, &tool(None), Some(&g), &spindle(), (0.0, 0.0)).unwrap();
        // hm = 2 fz/π; P ∝ kc1.1 hm^(−mc)
        let hm = 2.0 * 0.05 / PI;
        let ratio = 1400.0 * forge_math::dmath::powf(hm, -0.3) / (700.0 * forge_math::dmath::powf(hm, -0.25));
        assert!((r.power_hi / r.feeds.power - ratio).abs() < 1e-9);
        let lim = r.power_avail.unwrap();
        assert!(
            r.feeds.power < lim && r.power_hi > lim,
            "{} {} {lim}",
            r.feeds.power,
            r.power_hi
        );
        let p = r.checks.iter().find(|c| c.what == "power").unwrap();
        assert_eq!(p.verdict, Verdict::Warn);
    }

    /// A ball at shallow depth: speed set at D_eff, and the chip held counting both the axial
    /// (sin κ = D_eff/D) and the radial thinning (ae against D_eff); a tilt moves D_eff out.
    #[test]
    fn ball_and_tilt_through_the_resolver() {
        let g = group();
        let ball = ToolData {
            ball: true,
            ..tool(None)
        };
        let wide = Spindle {
            rpm: [100.0, 60_000.0],
            ..spindle()
        };
        let ask = Ask {
            vc: Some(300.0),
            chip: Some(0.05),
            ap: Some(1.0),
            ae: Some(2.0),
            ..Ask::default()
        };
        // narrower, the combined thinning would pass THIN_CAP: the chip is then held under target
        let thin = milling(&Ask { ae: Some(0.5), ..ask }, &ball, Some(&g), &wide, (0.0, 0.0)).unwrap();
        assert!((thin.fz.v - THIN_CAP * 0.05).abs() < 1e-12 && thin.feeds.hex < 0.05);
        let r = milling(&ask, &ball, Some(&g), &wide, (0.0, 0.0)).unwrap();
        let ct = feeds::contact(12.7, 6.35, 1.0, 0.0);
        assert!((r.feeds.d_eff - ct.d_eff).abs() < 1e-12);
        assert!((r.rpm.v - 1000.0 * 300.0 / (PI * ct.d_eff)).abs() < 1e-9);
        assert!((r.feeds.hex - 0.05).abs() < 1e-12, "{}", r.feeds.hex);
        let k = feeds::thinning(2.0 / ct.d_eff) * ct.sin_kappa;
        assert!((r.fz.v - 0.05 / k).abs() < 1e-12);
        let tilted = milling(
            &Ask {
                tilt: Some(20.0),
                ..ask
            },
            &ball,
            Some(&g),
            &wide,
            (0.0, 0.0),
        )
        .unwrap();
        assert!(tilted.feeds.d_eff > r.feeds.d_eff && tilted.rpm.v < r.rpm.v);
        assert!((tilted.feeds.hex - 0.05).abs() < 1e-12);
    }

    /// Drilling as its own model, Sandvik's page's example cut (Dc 20, vc 200, fn 0.20) with a
    /// 140° two-flute drill, kc1.1 1700, mc 0.25, worked by hand: n = 3183.1, vf = 636.6 mm/min,
    /// Q = 200 cm³/min, h = 0.1 sin 70° = 0.09397, kc = 3070.5, Pc = 10.235 kW, Mc = 30.70 N·m.
    #[test]
    fn drilling_sandvik_example() {
        let mut g = group();
        g.kc1_1 = 1700.0;
        let drill = DrillTool {
            d: 20.0,
            flutes: 2,
            point: 140.0,
            substrate: Substrate::Carbide,
            preset: None,
        };
        const C: [[f64; 2]; 2] = [[0.0, 15.0], [12_000.0, 15.0]];
        let sp = Spindle {
            rpm: [100.0, 12_000.0],
            max_feed: 10_000.0,
            power: &C,
        };
        let ask = DrillAsk {
            vc: Some(200.0),
            f: Some(0.2),
            ..DrillAsk::default()
        };
        let r = drilling(&ask, &drill, Some(&g), None, None, &sp, 70.0).unwrap();
        assert!((r.rpm.v - 3183.1).abs() < 0.05);
        assert!((r.feed.v - 636.62).abs() < 0.01);
        assert!((r.mrr - 200.0).abs() < 1e-9);
        assert!((r.h - 0.093_969).abs() < 1e-6);
        assert!((r.power - 10.235).abs() < 5e-4, "{}", r.power);
        assert!((r.torque - 30.70).abs() < 5e-3, "{}", r.torque);
        // past 3 D deep: pecks of one diameter, labelled as forgeCAD's rule
        assert_eq!((r.peck.v, r.peck.source), (20.0, Source::Default));
        // no data at all: nothing resolved; a group row gives speed and feed by diameter
        assert!(drilling(&DrillAsk::default(), &drill, Some(&g), None, None, &sp, 10.0).is_none());
        let row = DrillRow {
            group: "N".into(),
            substrate: "carbide".into(),
            vc: 309.37,
            d_rows: vec![6.35, 9.525],
            f_rows: vec![0.4064, 0.508],
            ..DrillRow::default()
        };
        let small = DrillTool { d: 6.6, ..drill };
        let fast = Spindle {
            rpm: [100.0, 20_000.0],
            ..sp
        };
        let r = drilling(
            &DrillAsk::default(),
            &small,
            Some(&g),
            None,
            Some(&row),
            &fast,
            10.0,
        )
        .unwrap();
        assert_eq!((r.vc.v, r.vc.source), (309.37, Source::Material));
        let want = 0.4064 + (0.508 - 0.4064) * (6.6 - 6.35) / (9.525 - 6.35);
        assert!((r.f.v - want).abs() < 1e-12 && r.f.source == Source::Material);
        assert_eq!(r.peck.v, 0.0);
        // outside the rows: the lesser extension
        let (f3, how) = row.f_at(3.0);
        assert!(f3 <= 0.4064 * 3.0 / 6.35 + 1e-15 && how.contains("under"));
    }
}
