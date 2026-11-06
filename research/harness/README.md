# Harness maths and measurements

`derive.py` checks, in SymPy, what kernel/forge-harness computes: a corner's tangent arc (tangent
length R tan(θ/2), centre on the bisector, length Σlegs − Σ2t + ΣRθ), the tightest-fit allocation
of tangent lengths along a chain (no leg over-full, no corner grown; 2000 random chains), lead
points off directed nodes, the rigid turn on an arc being the rotation-minimising frame, the
bundle rule (one wire: d; two or more: max(sqrt(K Σd²), d₁ + d₂), K = 1.5; Glenair's table read
as K for comparison), the AWG formula (ASTM B258's definition), and the wire length order with
round-up. Rerun: `~/laya/.venv/bin/python -I research/harness/derive.py` (20/20).

Used by: `kernel/forge-harness/src/lib.rs` (`bundle_d`, `awg_d`, `centreline`, `route`) and
`src/tube.rs`.

## Tube cost (2026-11-06)

`kernel/forge-harness/tests/tubes.rs`, `a_segment_routes_and_sweeps_in_milliseconds`, one segment
of 6 wires, `--release --nocapture`:

```
                         with the self-intersection test    without (sweep_unchecked)
corners 1 (7 pieces)     sweep 2.20 ms                       0.036 ms   mesh 0.36 ms
corners 3 (11 pieces)    sweep 2.57 ms                       0.021 ms   mesh 1.50 ms
corners 6 (17 pieces)    sweep 2.76 ms                       0.047 ms   mesh 0.79 ms
route (paths, bundles, centreline): 0.006-0.009 ms
```

Nearly all of a sweep's cost was `selfx::crossing`. A harness tube's bends are never tighter than
the tube (a tighter one is swept piece by piece), so the tubes use `forge_ops::sweep::sweep_unchecked`,
and the decision is B-rep tubes everywhere (under 1 ms a segment with its mesh), not mesh tubes in
the viewport.
