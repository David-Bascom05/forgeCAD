# The area along a ruled wall's straight lines

On a surface straight in v (`v_degree() == Some(1)`: ruled walls, wrapped walls, spline extrusions,
screws of a line) `mass.rs` integrates along v with Gauss on 3 points, exact for the volume and moments
(polynomials of degree 4 in v). The area's integrand |Su x Sv| is not: Su is linear in v and Sv constant,
so |n|^2 is a quadratic q(v) and |n| its square root, a polynomial only on a developable wall.

Until this change the area kept the 3-point value and reported its difference from the 2-point rule
as the error. That difference is the 2-point rule's error, not the 3-point rule's.

Now (`root_line`, `root_piece` in mass.rs) the 3 values of |n| already computed give q exactly. Its
root is integrated by Gauss on 7 points, halved until Trefethen's bound for an integrand analytic in the
Bernstein ellipse through q's complex roots (SIAM Review 50, 2008, Theorem 4.5: 64 M / (15 (rho^2 - 1)
rho^(2n)) for n + 1 points) is below rounding. No extra surface evaluations: the old error estimate
evaluated the surface twice more per point, this evaluates only square roots.

## Measured (`area.rs` against `reference.py`, `results.txt`)

Four bodies: the wrapped disc of research/sheet-metal/arc/ (a hole across a bend), the hyperboloid of
kernel/forge-model/tests/ruled_area.rs, a square lofted to a smaller turned square (four hyperbolic
paraboloids), and forge-ops glyph's ring wrapped round a cylinder (wrapped parabola walls). Reference:
40-digit quadrature of |Su x Sv| over each wall's parameter square plus the caps in closed form,
independent of the kernel.

```
                 before                       after
body             |area miss|  area err        |area miss|  area err
plug             2.7e-9       3.9e-6          1.2e-14      2.1e-13
hyperboloid      4.1e-3       1.7e-2          2.6e-12      2.4e-11
twisted-square   3.4e-3       2.1e-1          2.2e-14      8.3e-11
glyph-ring       4.5e-9       4.8e-6          1.6e-14      3.7e-10
```

The values were the 3-point rule's and were off by up to 2.2e-6 of the area on the twisted walls; they
are now right to rounding. What is left of the reported error on the last two is the loop integrals'
own estimate (Gauss-Kronrod differences along the edges), the same with the walls' share set to zero:
not this rule's.

## Rerun

```
cp research/mass-props/ruled-area/area.rs kernel/forge-model/examples/area_scratch.rs
fbuild cargo build --release -p forge-model --example area_scratch && target/release/examples/area_scratch
fbuild <python with mpmath> research/mass-props/ruled-area/reference.py
```
