# Engagement-controlled clearing: first design

**Status: an early design, written 2026-11-24 by the TOOLPATH-QUALITY worker. What was built is
kernel/forge-cam/src/adaptive.rs (`adaptive(..)` in docs/cam.md): a cleared-area simulation
stepped to one capped engagement, one-direction peel passes, links at depth by retrace; this file
keeps the reasoning that led there.**

## 1. Why: what the harness measures today

verify/forge-cam-quality measures every path forge-cam makes (README there). With offset pockets
plus stay-down linking and feeds set from engagement (cam/quality, 2026-11-24), the chip is held at
the cutting data's fz (hex p99 = fz in every milling op, overload time 0%). What offsets cannot
fix is the *shape* of the engagement:

- every offset pocket starts each level with a full slot (φ = 180°) and meets 180° again in every
  concave corner and round every island (well_island φmax 180, mean 119; pocket_islands 180/106;
  thin_wall 180/101). The feed scheduler slows down there, so the chip is right, but
- the axial depth must be chosen for the *worst* engagement (a slot): ap ≤ about 1 D in aluminium
  for a flat end mill, so a 12 mm-deep pocket with a 6 mm tool takes 4 levels at 3 mm. A path whose
  engagement never exceeds, say, φ* = 50° (ae ≈ 0.18 D) could take the whole flute length in one
  level (ap 2–3 D), the chip held by the feed, the load spread along the flute. That is the
  productivity and tool-life gain the commercial "adaptive" strategies sell.

The quantity to control is the engagement angle φ (equivalently ae/D = (1 − cos φ)/2). The feed
then follows from φ (built: forge_cam::engage).

## 2. The algorithm

Per level z (levels from the in-process stock and the op's `from`/`to`/`step` as today):

**State.** C, the cleared region at this level, an exact forge-cam-geom `Region` (lines and arcs
on the 2⁻³² grid): at the start, the in-process stock's material at z subtracted from the stock box
(stock.rs `material(z)`: what earlier ops provably cleared), plus the entry. T = R ⊖ r, the tool
centres' region (R the op's region, r the tool radius plus leave), exactly as pocket() has it (C₀).
The goal: C ⊇ R ∘ B_r, the opening (what an offset pocket clears; Pocket.lean `passes_cover`).

**One target.** A single user-set maximum engagement φ* (default from the cutting data, e.g. 60°),
giving a single radial step a* = r (1 − cos φ*). The angle comes out of the geometry: one number for the user to set
and for the harness to check.

**Entry.** A helix in the largest disk of T not met by material (the medial axis's widest point
of T ∩ uncleared, found exactly as helix_fits does today: a disk test by boolean), or, for an open
region, from outside the stock (C already holds the outside).

**Passes: offsets of the cleared region.** The next pass is the boundary of
Pₖ₊₁ = ((Cₖ ⊖ r) ⊕ B_{a*}) ∩ T,
i.e. the tool centres that keep the tool's edge a* into uncleared material along a straight wall
of the cleared region: there the engagement is exactly φ*. The cut adds the swept band:
Cₖ₊₁ = Cₖ ∪ (∂Pₖ₊₁ ⊕ B_r). Repeat until C ⊇ R ∘ B_r. This is a cleared-area offset; every
step is an exact offset and boolean forge-cam-geom already has (offset.rs, boolean.rs).

**Corners.** At a concave corner of Cₖ the edge meets more than a* of material: engagement above
φ*. Three things keep the load in hand there:
1. a fixed, user-set smoothing radius ρ (default r/2) applied to every pass as a morphological
   opening (offset in by ρ, out by ρ: exact): one exact operation, easy to certify;
2. the feed scheduler (built) slows the tool wherever the measured engagement exceeds φ*, so the
   chip stays at fz; the harness reports the time above φ*;
3. optional, user-set plain trochoids (a circle plus a constant advance, radius and pitch given by
   the user) along the remaining narrow channels.
Later, step 2 may become a hard cap (a local trochoid inserted where the exact engagement exceeds
φ*).

**Links.** Within a level the tool stays down: from the end of one pass to the start of the next
by a straight move that lies in Cₖ ⊖ r (the tool wholly inside what is cleared: no material met)
and in T, tested exactly as pocket()'s link_ok does today (a band about the segment less the
region, empty); else up to the retract height and down again (a helix if the next start is in
material). Staying down saves the retract and plunge, the largest share of
non-cutting time on small pockets.

**Medial axis.** Only to place the entry helix (the plain medial axis of T).

## 3. Certification

Every property below is either proven once in Lean for every path the algorithm can make, or
checked on the posted path because it depends on the input (design.md §8, the owner's rule).

| property | how |
|---|---|
| G1 no gouge | every pass and link lies in T = R ⊖ r by construction (∩ T, exact); the posted path is certified against the part as today (input-dependent: runtime, as now) |
| G2, G3 | as today, on the posted path, against the in-process stock and the fixture |
| G4 the result | the loop ends only when C ⊇ R ∘ B_r, an exact region inclusion (one boolean). Lean: a generalisation of `passes_cover`: if every Pₖ₊₁ ⊆ (Cₖ ⊖ r) ⊕ B_{a} with a ≤ r and the loop stops at C ⊇ R ∘ B_r, the swept bands cover the opening; voids = C as today |
| termination | each step adds area ≥ a* · (perimeter of ∂Pₖ₊₁ ∩ uncleared) > 0 while C ⊉ R ∘ B_r; the region is bounded: Lean, by a measure argument (area of R ∘ B_r \ Cₖ strictly decreasing by a bounded-below amount) — needed so generation cannot hang |
| links | a link segment ⊂ (Cₖ ⊖ r) ∩ T, exact band test (built for pocket()); Lean: the tool on such a segment meets no material and no part |
| G5, G6 | as today: feeds within the machine (runtime on the text), the post's round trip (proven) |
| engagement (new, G7, optional) | the claimed φ* is a *quality* claim, not safety. On straight pieces of an offset of a straight cleared wall it is exact by construction (φ = acos(1 − a*/r)); elsewhere it is measured exactly from C (the arcs of the tool circle outside Cₖ, front half) and reported, never claimed beyond what was measured |

## 4. How we will know it is better

The harness already measures it: φmax and the φ histogram, chip p99/max, MRR, cycle time
(forge-machine timeline), air and links. Targets on the benchmark parts (pocket_islands,
thin_wall, deep_slot, well_island, plus open-pocket and narrow-channel parts to add): φmax ≤ φ*
except where the time above φ* is reported; ap = the flute length in one level; cycle time below
offset pocket + feed scheduling at equal chip. The comparison against an open-source reference
(FreeCAD's Adaptive, run as an external program in verify/ only, if the licence verdict allows)
gives the outside check.
