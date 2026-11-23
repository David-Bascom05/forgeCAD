# What a tapered extrude is due to give

This is the spec for the taper fix (taper_collapses.fg, on claude/repo-review-mbdrlj-import).
DESIGN.md §2 has the reasoning. The Lean files are:

- ForgeExact/Swept/Taper.lean: the theorems;
- ForgeExact/Swept/TaperRef.lean: the exact rule, executable.

`sweep-diff` checks the kernel against both.

## The set

Let t = sin/cos of |angle|, using the kernel's own `dmath::sin_cos`, and let σ be the sign of the
angle. The section at height h is the **mitred offset** of the profile by δ = σ·h·t:

- every edge's line moves δ to its material side (the left of each normalised loop);
- every corner is where its two moved lines meet.

That corner is `q + δ·w`, where w = (n + n') / (1 + n·n') is the mitre of the corner's two unit
left normals. It lies on both moved lines and is the only point that does (`corner_on_both`,
`corner_unique`). So every corner moves linearly in δ.

## Events

All three quantities below are **linear in δ** (`edge_length`, `contact_distance`):

- an edge's length: ℓ + δ·(w' − w)·u;
- a corner's signed distance to another edge's moved line: n·(a − b) + δ·(n·w − 1);
- its position along that edge, measured from either end.

So the first event is the least positive root over these linear functions, an exact
computation. `TaperRef.candidates` does it over ℚ; over f64 it is a handful of dot products per
pair. Two closed segments that move continuously first touch with an end of one on the other, so
corners against edges are all the contacts there are.

The fix replaces `next_event`, which takes 256 uniform samples plus 60 bisections, assumes
wipe-out is monotone, and uses `GONE`. It also gives a contact event (a corner reaching another
edge) for the first time; there is no such check today.

## The rule

Let D = |d|·t be the whole offset. Step from τ = 0, taking the next event τ' in (τ, D]. At τ',
drop the edges whose length is 0, merging their corners, then:

1. **No event at all**: the one drafted solid (the draft path).
2. **The outer loop's area is ≤ 0** (fewer than 3 corners, or zero area): the outline closes at
   h* = τ'/t. The sections are empty past h*, so the exact set ends there. **Due: the solid up to
   h* (the roof), with a lint "taper closes at depth h* before d" carrying a fix.** This is the
   master's ruling, and it is the slab path's answer today: `sweep-diff` finds it exact in every
   case the kernel builds. A refusal is due only where nothing remains.
3. **A hole's area is ≥ 0** (it has closed): drop it and go on.
4. **The loops that are left are not simple and apart** (a corner has met an edge it is not on):
   **refuse, `TaperError::WallsCross { at: τ'/t }`.**
5. **Otherwise** (edge events only): a slab from τ to τ', then go on from τ'.

**An event at exactly D** is processed like any other. A collapse at exactly D is case 2: the
solid ends in its apex or ridge exactly at the depth.

For taper_collapses.fg, the exact hexagon (`hexagon_side`: every side is 2 − 2δ/√3 long) collapses
at δ = √3, so h = √3/tan 80° = 0.30541 against a depth of 0.5. **Due: the exact hexagonal pyramid
pit, 0.30541 deep, with the lint.** The fix-draft session made it build that way (3bc9dc1) and
corrected the comment to 0.305. The fixture's dyadic √3 makes the sides differ by an ulp: one
pair goes first (case 5), then the rest close (case 2). The result is the same.

## What the kernel does today, measured

`sweep-diff` ran 400 random skyline tapers: rational edge lengths, either sign of angle and
depth, some with holes.

| verdict | n | what |
|---|---|---|
| agree | 202 | the kernel's tool is the definition exactly (under the boolean's snap) |
| roof (counted in agree since the ruling) | 26 | the outline closes before the depth; the kernel built the cut-off solid, exactly |
| refused_ok | 164 | the rule refuses too |
| refused_valid | 8 | the kernel refused a taper the rule builds: outward tapers with edge events, the slab path failing its own `check()` (`Draft(Invalid)`) |
| built_invalid | 0 | no skyline built a wrong body |

The wrong body (EdgeDegenerate) comes only from the **draft path accepted without an event check**
(taper.rs:416–417), as in the hexagon. Skylines reach the slab path because their drafts fail
first.

The kernel's refusals are typed wrongly. Where the rule says roof or WallsCross, the kernel
says:

- `Draft(BadAngle)` (97): `slab()` maps a failed `offset()`, two carriers that no longer meet, to
  BadAngle;
- `Draft(Invalid)` (44);
- `Draft(SelfIntersecting)` (23).

None of these is at the locus or names the cause.

**Also:** with lo = 0, h = 0 makes `slab()` build a zero-height slab, and its
`expect("the walls rise")` can panic. Under the rule an event at 0 cannot happen: the profile is
simple at 0, and every root is taken in (τ, D].

## Arcs

The set is the same: arcs keep their centres, their radius changes by ∓δ, and the walls are
cones. Tangent joints (rounded rectangles, slots) still move linearly. Non-tangent arc–line and
arc–arc joints move on a circle–line meet, so their events are roots of quadratics. The exact
executable covers lines with rational lengths today; arcs and irrational lengths are the
algebraic-number step. The f64 implementation is the same rule with those roots.
