# Ends on a surface: the exact arguments

`crates/forge-model/src/upto.rs` picks an end by sampling (rays) and then proves it exactly; the
proofs are booleans or closed arguments, never samples. They were reasoned out in 0f6ca23 and
05ce566 and live as comments at the code; this is the record in one place.

## The arguments

- **Next face** (`verify_next`): the earlier tools near the sweep meet none of it or hold all of
  it, and the part's skin lies under all of its end. Both are booleans on exact solids.
- **Shortcuts** that skip those booleans:
  - one convex solid (a ball, a prism of a convex polygon) holding the whole sweep's box is the
    part there; the end, on its surface and inside it, lies on its skin (`convex_holds`);
  - from outside, solids whose boxes only touch the sweep's share none of it; if one convex solid
    is left and the sweep was cut off at its own surface (a ball's sphere, a convex prism's cap
    with the sweep beyond it and the end over it), nothing is inside and the end is on its skin
    (`skin_under`).
- **A torus met at a slant** (`faces_ahead`): every face of the tool on the torus must face the
  travel. The outline where the normal turns is a loop round the axis; when the axis misses the
  sweep's box no such loop fits inside a face, so the sign can only change across the face's
  edges, and interval bounds of the normal along the travel on the edges settle it.
  The normal along v is a positive multiple of (rho - R)(x vx + y vy)/rho + z vz.
- `ray_torus`: the torus implicit along a ray is a quartic in the ray parameter.

## How to rerun

```
fbuild python3 research/upto-proofs/check.py    # needs sympy
fbuild cargo test --release -p forge-model --test upto
```
`check.py` (recreated 2026-11-01) checks the normal formula against the implicit's gradient on the
torus (ratio 8R rho, positive) and the ray quartic's coefficients. The tests exercise each shortcut
and refusal.
