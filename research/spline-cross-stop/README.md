# Where the leaf Newton of `cross_sections` stops

`kernel/forge-geom/src/spline/meet.rs`, `cross_sections`: the stop of the leaf's Newton.

## The bug

The leaf accepted a crossing only once a step measured `|ds| |a'| + |dt| |b'| <= 1e-13`, a fixed
length. Once Newton has converged, each step is rounding: the two points' evaluation error, divided
by the sine of the crossing angle. That error grows with the coordinates and with the parameters
(a point at parameter t is off by about ulp(t) |c'|). About 300 mm out the floor is above 1e-13, so
the steps circle above the stop for all 12 iterations. The leaf is then dropped with no error, and
a crossing goes missing.

## What was measured

`xs_probe.rs` builds random closed blob pairs (7 to 10 point closed cubics, 5 to 40 mm), translates
each pair to 0, 30, 300, 1e3, 1e4 and 1e5 mm out, and for each crossing a fine polyline finds:

- counts the crossings `cross_sections` does not return (within 1e-3 in the parameter);
- runs the leaf's Newton from the crossing and records the floor (iterations 6 to 12) of the step
  measure and of the residual |q - p| before the step, in units of eps * scale, with
  scale = 1 + max |p| + |s| |a'| + |t| |b'|.

`SHALLOW=1` instead mirrors each blob across a line through one of its points, phi/2 off its
tangent (phi from 3e-6 to 1e-2), which gives one crossing at that point at angle phi.

Results, old stop and new, are in `results.txt`. In short:

```
normal set, 3174 crossings per distance      0    30   300  1000  1e4  1e5
  dropped, old stop                          2     2    33   274  449  424
  dropped, new stop                          2     2     2     2    2    2
  residual floor / (eps scale), worst        0.9  1.1  2.3  2.1  2.8  2.7
  step floor / (eps scale), worst            6    17   44   67   44   35
shallow set, 600 crossings per distance
  dropped, old stop                          0     1     0     0    0    0
  dropped, new stop                          0     0     0     0    0    0
  residual floor, worst, sin < 1e-3          0.9  1.5  2.1  1.5  2.0  2.0
  step floor / (eps scale), worst            777  2155 1740 1271 2057 2736
```

The 2 left in the normal set are the same 2 at every distance. They are errors in the reference: two
shallow crossings (sine 0.024) that the polyline places 2e-3 off in the parameter. The kernel
returns both.

So the residual floor stays under 3 eps * scale at every distance and every angle down to 3e-6.
The step floor is 1/sin(angle) larger and has no bound worth stopping on. Without the parameter
terms in the scale, the residual floor near the origin read 19 eps * scale, because the parameter's
rounding (t up to a few hundred) dominates there.

## The rule

The leaf stops once the residual before the step is at most `64 eps * scale`, over 20 times the
worst floor measured. At 1e5 mm that is 1.4e-9 mm, far below `tol::LINEAR`, so a residual that
small is a real crossing. The step is still taken, so the parameter keeps the last correction.

`xs_find.rs` searched round-number blob pairs near (300, 150) for one the old stop gets wrong. It
gives the pair in `kernel/forge-geom/tests/cross_sections_far.rs`.

## Rerun

Copy either file to `kernel/forge-geom/examples/` and run
`PARAM=1 [SHALLOW=1] fbuild cargo run --release -p forge-geom --example xs_probe`, or
`--example xs_find`. `xs_find` carries its own copy of the old leaf. For the probe's old-stop column,
put the old stop back in `meet.rs`.
