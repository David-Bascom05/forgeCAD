# Tangent cuts and almost perpendicular sections

Recovered from reliab-20 (73691ec5), rebased onto 8bb1f9bb. The regressions are
`a_cut_a_hair_off_tangent_to_two_pillars`,
`a_cylinder_through_a_cusp_tangent_to_both_sides` (forge-agent), and the
closest-approach and nearly-perpendicular-section tests in forge-geom.

## Conic turning points

`conic_quadric` substitutes a conic into a quadric, obtaining

    Q(t) = k0 + k1 cos(t) + k2 sin(t)
           + k3 cos(t)^2 + k4 cos(t) sin(t) + k5 sin(t)^2.

The half-angle polynomial is Q(t) times (1+u^2)^2, where u=tan(t/2).
Its extrema need not be Q's extrema; its interval endpoints are also returned
as near-zero candidates. A near-tangent candidate must be refined in t:

    Q'(t)  = -k1 sin(t) + k2 cos(t)
             + (k5-k3) sin(2t) + k4 cos(2t)
    Q''(t) = -k1 cos(t) - k2 sin(t)
             + 2(k5-k3) cos(2t) - 2k4 sin(2t).

These are obtained by differentiating the displayed Q by hand. Newton on Q'
uses the coefficients from the unrotated half. The inherited eight iterations
and 0.05-radian displacement cap bound this local refinement; they are guards,
not a convergence proof. Nonfinite or distant steps keep the original candidate.
The refined point is accepted only while it and the interval back to the
candidate remain inside the surface's LINEAR band. Test both ordinary roots
and ABSORB_HAIR_ROOTS, the boolean retry that replaces nearby roots by a touch.

## Plane through cylinder

For unit cylinder axis a and plane normal n, c=abs(a dot n), the section's
semiaxes are r/c and r. Replacing it by a circle when r(1/c-1)<=LINEAR can
move a shallow edge crossing much farther than LINEAR. Keep the ellipse unless
its relative axis difference is at most the inherited 1e-12 rounding threshold.
The regression substitutes section points into both defining equations;
a perpendicular plane must still produce a circle.

## Tangent face pairs at a boundary

Two surfaces touching generally add no cut. At a smooth or cusped boundary of
one body, the other surface can nevertheless cross the body's material. The
recovered test finds an entire boundary edge on the touching curve, with
parallel neighbouring face normals, and samples the other face on either side.
Only opposite inside/outside classifications require the touching curve to be
trimmed as a cut. The inherited normal threshold (1e-6) and face-diagonal probe
fractions (0.001, 0.01, 0.05) are search heuristics, not bounds. Both probes must
be inside the other face; boundary or inconclusive results do not assert a cut.
Both sequential and n-ary face-pair loops use this decision; the original branch
changed only the sequential loop.

The search first rejects points more than twice LINEAR from a line (perpendicular
distance), or from a circle's plane or radius. These are lower bounds on distance
to the curve; the factor two leaves rounding slack. Points that survive still use
the original closest-point test at LINEAR. This avoids projecting every unrelated
edge of wrapped text onto every touching curve. Profiling the jack prefixes placed
the extra cost at its final wrapped-text feature; earlier features were unchanged
to about ten thousand instructions. Geometry-only and untouched-main controls
separated this boundary search from the conic refinements.
The other face's conservative box rejects midpoints before those checks, since
the final decision requires that midpoint to be inside that face.

## Rerun

From the worktree root, through the shared resource limiter:

    fbuild cargo test -p forge-geom --lib intersect::tests
    fbuild cargo test -p forge-agent --test build_failures

To repeat the speed diagnosis, build `forge-agent --example benchparts --release`
on the baseline and candidate in separate targets. Resolve the input once with
`benchparts resolve jack .scratch/jack.fc`, then run:

    python3 research/tangent-cuts/bench.py BASE/target/release/examples/benchparts CANDIDATE/target/release/examples/benchparts .scratch/jack.fc .scratch/prefixes

On 8bb1f9bb the whole jack took about 332.08M sequential instructions. The first
port took 334.71M; geometry alone took 332.31M. The bounds and distance rejections
bring the port to 332.64M. Against the shared saved baseline, the final gate's jack
rows are +0.47% sequential and +0.45% n-ary, below its 0.5% limit. Counts include
process startup and vary slightly between invocations.

The part tests check both sequential and n-ary validity and compare their mesh
volumes. Analytic area/volume assertions independently check the intended solid.
