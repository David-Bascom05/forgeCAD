# CURVED-SITES handoff

The worker that held curved surface–surface sites (exact closed forms where the tracer was slow or
refused, their Lean proofs and ledger rows, and the speed of curved intersection). Wound down on
2026-11-25 at David's direction.

## Branches

| Branch | State |
|---|---|
| `proof/curved-sites-screw-flip` | In train 8. Pre-verified with tool 13d78f06, UX OK'd. It holds the anti-parallel screw flip (`pair.rs`, ScrewFlip.lean, ledger C.flip), a thread's band on its own bore made Coincident (`coaxial.rs`, ledger C.band), HOSTED-DESIGN's repro tests (`asm_thread_repros.rs`), the fg_demo refused-preview fixture, and extent refusals in words (`error.rs` `extent_text`, UX-LEAD's clauses). |
| `perf/ellipse-foot` | The ellipse closest-point rewrite (below). The second review found 2 blockers and 1 minor; all are fixed, and the re-review is SAFE. Pre-verified for train 9, pending a ruling on volute's codegen band. |

## The ellipse closest point (`perf/ellipse-foot`)

`Ellipse::closest` sampled the distance's slope 64 times and bisected each sign change: about 100
`sin_cos` per call, and 26% of gearbox's regen in FALLBACK's profile. Now `ellipse_square` gives
every angle at which the ellipse is square to the point. Those are the roots of Joachimsthal's G,
at most four, one per bracket, each found by a held Newton with no trig in the loop. The nearest
of those inside the range, and of the range's ends, is taken. The old sampler stays behind
forge-geom's `ellipse-foot-sampled` for one train. Remove it in the train after this one lands.

The proof is `verify/forge-curved/ForgeCurved/EllipseFoot.lean`, with 22 theorems that use the
standard axioms only; the ledger row is C.efoot. The f64 side is open and covered by tests: the
differential against the sampler, a brute-force check, and the square-to-the-point test in
`curve.rs`.

**Gate**, from callgrind on one VM against main c135b274. The strict total is −1.51%. gearbox app
−20.6%, edit −20.7%, cli −9.9%. quadarm −12 to −16%. bracket −3.5 to −5%. frame −1.1 to −2.1%.
crank −0.6 to −1%. valve −0.1 to −0.6%. volute app/cli/edit are +0.045 to +0.058%, over the 0.03%
band, while its kernel lines are −0.04%. volute never calls the ellipse code. Its calls are
identical in number, and the extra instructions are in `skinned::Data::point`'s machine code and
the inlining around `screw::remembered`'s thread-local. That is code-generation noise from editing
`curve.rs` in the same crate (forge-geom is one codegen unit). `#[inline(never)]` on the new
solver and removing the feature both left volute unchanged to the instruction. Bits move in 21
gate parts. In every one, `bits measures` and `benchparts measures` (84 lines: validity, meshed
volume to 1e-6, area, face count) are identical to main's. So the moves are last-bit differences
in the foot point's angle, where an exact solve replaced a bisection.

A bug the tests caught: where U ≫ V, the bracket end p₀ = d·cu/(cu + cv) rounds onto the pole at
e = 0, G is infinite there, and the solver read the wrong direction. Each bracket's direction now
comes from the proof, and G is never evaluated at the ends. Keep it that way.

## Not started (handed on)

- **volute**: `skinned::Data::point` is ~37.6% self; `dmath::sin_cos` makes 873k calls from it.
  `IntCurve::eval` costs ~3.6k Ir per call on skinned surfaces against ~320 on analytic ones (a
  Hermite point, then Newton onto both surfaces). `Skinned::implicit` costs 3.4k Ir per call.
  These figures come from FLAT-SITES' trim profile on main c135b274.
- **jack**: `Screw::value` under `implicit_gradient` is ~25%. A closed form for the gradient, or a
  cached frame, is the likely lever.
- **FLAT-SITES needs a sound bound e on |hermite − eval| per chain segment** for its cheap
  pre-prune. This is a proof task, a Hermite remainder against the traced curve's own curvature
  bounds. It is not started.
- **TR160 #167334**: 29 s, then refused. 90% of the time is `trace_marched`, 76% of that is
  `trace_past_stalls`, and 92% inside that is `Nurbs::nearest`/`eval`. FALLBACK's nurbs-stall
  gets it to 8.6 s/2.9 s, still refused. Coordinate with IMPORT-SPEED, whose NURBS eval rewrite
  halves the cost, on the tracer's use of nearest.
- **Open ledger items:** η_trace for C.2910; the M.ct topology steps; C.g5.apexplane once
  proof/apex-near-tangent lands.
- **A pre-existing kernel bug on main**: part_tools.fg with `hole1.d` = 4.9999 (or 4.99997) and a
  tapped M5 hole with a modelled thread gives "internal error: inconsistent classification". A
  hair-gap between the thread and a near-coincident bore. It was reported to the coordinator, not
  fixed.

## Risks

- **C.band is a tolerance decision**: |gap| ≤ LINEAR counts as the same surface. A thread whose
  band sits a hair off its bore, as in the 4.9999 case above, falls on the other side of that
  line and reaches the tracer.
- **Ellipse ties.** For a point exactly on an axis, two mirror points are equally near. The new
  code may return the other one from the old sampler (200 of 13,640 test cases). Both answers
  are correct, but downstream bits can move. The gate bits diff on the branch records which.
- **Cusp conditioning.** At the evolute's cusps, three roots merge and the angle is
  ill-conditioned (error ~ ε^⅓). The distance stays right to rounding, as the brute-force test
  checks. Callers that use the angle itself, and not the point, see that jitter.
