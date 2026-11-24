# M.ct (torus × cylinder event search): what is proven, what it costs (CURVED-SITES, 2026-11-24)

The shipped search (`meet::intersect`, `Form::Poly4`) finds events by a scaled discriminant sampled at
64 places, plus #33's `thin_window` root-count scan (spacing half the tube's radius) when a double root
is flagged. Oracle (meet_cert with the mean-value form, tests and the decision log only): mixed, the
gate parts and fuzz 1–300 seq, 2,491 torus × cylinder calls: **2,491 Certified, 0 Missed** with
today's code. No theorem says it finds every event (M.ct UNKNOWN): a near-tangent loop can be
narrower than any fixed spacing.

## The sampling rule tried (branch proof/curved-sites-mct)

Every loop that does not wind round the cylinder bounds a disc on it whose rim is at distance r
(the tube's radius) from the core circle C, so the distance has an extremum inside: at a point of C
(C pierces the cylinder: roots of g(θ) = ρ², ≤ 4), on the torus's axis (≤ 2), or on the radial line
through a critical θ of g(θ) = dist²(C(θ), axis) (≤ 4). The smooth step is proven in Lean
(verify/forge-curved MctSample.lean: `critical_of_radial`, `hasDerivAt_distSq_axis`). So every loop
holds one of ≤ 10 angles computable in closed form. A ruling that meets the curve nowhere is in no
loop (skipped, provably).

What it does not give: the loop's *events* (its two turning points). From an angle inside a loop
the code must find the root count's nearest change each side, which is again sampling:

| variant | oracle (2,491 calls) | band vs merge-train/1 | proof |
|---|---|---|---|
| today (discriminant + thin_window) | all Certified | — | none for completeness (M.ct UNKNOWN) |
| critical angles + doubling walk + bisection | all Certified (fuzz295 7/7) | **total +2.38%**, mixed seq +3.26%, nary1 +5.19%, insert app +77.7%, manifold app +18% | the walk can step over a pair of changes (M.ct.walk open) |
| critical angles + walk over known events | **misses** seed 295's 1.918, 2.097 | mixed seq about +0.26% (quick count) | unsound: two changes between samples are invisible |

A "cheap walk, fall back when it can't certify" form has no sound fall-back test: on seed 295 the
cheap walk stops at a known event whose count change looks like the explanation while two changes
lie between its samples.

## Options for David

1. Keep today's search (no runtime cost), the certifier as test/fuzz oracle (Missed → a bug in
   the code), M.ct stays UNKNOWN in the ledger with the oracle's corpus evidence.
2. Ship the critical-angle rule (+2.4% total, +3.3% mixed, up to +78% on a bench part) — it is
   still not a complete proof (M.ct.walk, M.ct.enc, M.ct.count open).
3. Further research for a provable and cheap form: bracket each loop's turning points directly
   (F = F_s = 0 near the loop's interior point, a 2D certified solve per loop), cost unknown.

Ledger rows: M.ct, M.cert, M.ct.crit, M.ct.enc, M.ct.walk, M.ct.count.

## Decision (David, 2026-11-24)

Option 1: today's search stays shipped; the certifier runs as a CI oracle (poly4-oracle.yml) over
the gate parts, mixed and fuzz 1–300; M.ct stays UNKNOWN. Option 3 only if the oracle ever disagrees.
The critical-angle rule and its Lean step stay on branch proof/curved-sites-mct.
