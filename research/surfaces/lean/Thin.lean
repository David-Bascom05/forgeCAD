import ForgeExact.Fillet.Check

/-!
# Thin walls and thicken: the spec (L1), on the fillet spec's offset definitions

For verify/forge-exact on claude/repo-review-mbdrlj-fillet-spec, which owns `ForgeExact.Fillet.Check`
(`Surf`, `onOffset`, `onSurf`, the snaps `tau`, `tauA`, `delta`). This file is written against that
module and is in no lakefile here. It has not been built: this container has no Lean toolchain and
its policy refused installing one. The theorems are stated as `Prop`s, with no `sorry`, for the
session that owns the proofs to prove.

One code path makes both: a thin wall **is** `thicken(surface extrude of the chain, t, side)`
(kernel/forge-model/src/sheets.rs, `Sheet::tool`). So one spec covers both. It reads what
kernel/forge-ops/src/thicken.rs builds:

* **skins**: each face `F` of the sheet, surface `s`, gives two skins, at signed distances `lo` and
  `hi` from `s` along its front (`side: "out"`: `0, t`; `"in"`: `-t, 0`; `"both"`: `-t/2, t/2`).
  A skin is `s` offset exactly: plane to plane, cylinder to cylinder of radius `r + d`, cone, sphere
  and torus in closed form, a spline's extruded surface by `blend::Offset`.
* **walls**: along each boundary edge of the sheet, the ruled face swept by the normals between the
  two skins. At an extruded sheet's caps, that wall lies on the cap plane.
* **corners**: two skins meeting at a crease of the sheet meet in their offsets' intersection: a
  sharp mitre (the outer offsets extended until they meet, the inner trimmed where they cross).
  On the rim, two walls on one plane facing one way join into one face.
* **refusals** (typed, at the locus): `Collapses` where an offset passes a centre of curvature
  (an arc of radius `r ≤ -d`), `Crease` where rim walls neither share a plane nor meet in a line
  through the corner, `Rebuild(Vertex | Collapsed)` where a corner cannot be solved, and `Invalid`
  where the result fails `check` (never returned as a part).

The executable judge below decides each claim exactly, over ℚ, with the fillet spec's `onOffset`,
so a distance is compared on squares and no square root is taken.
-/

namespace ForgeExact.Thin

open ForgeExact.Fillet.Check

/-- A thicken's two distances, `lo < hi`, from the sheet along its front. -/
structure Sides where
  lo : Rat
  hi : Rat

/-- `side:` as the language takes it, for thickness `t > 0`. -/
def sides (side : String) (t : Rat) : Sides :=
  match side with
  | "in" => ⟨-t, 0⟩
  | "both" => ⟨-t / 2, t / 2⟩
  | _ => ⟨0, t⟩

/-- A sampled point of the result and what the spec says it must lie on. -/
inductive Claim where
  /-- a skin point: on face surface `s` offset by `d` (`d` is `lo` or `hi`) -/
  | skin (s : Surf) (d : Rat) (p : Q3)
  /-- a mitre corner: on both creased neighbours' offsets by the same `d` -/
  | mitre (a b : Surf) (d : Rat) (p : Q3)
  /-- a wall point on a straight extrusion's cap: on the cap plane -/
  | cap (plane : Surf) (p : Q3)
  /-- a square end of an open chain: on the end's plane, square to the chain there -/
  | squareEnd (plane : Surf) (p : Q3)

/-- The judge: `some true` where the claim holds within `δ`, `some false` where it fails, `none`
where the fillet spec's `onOffset` does not judge that surface (cones, tori, pipes: judged by kind
only). -/
def judge : Claim → Option Bool
  | .skin s d p => onOffset s d p
  | .mitre a b d p => do
      let x ← onOffset a d p
      let y ← onOffset b d p
      return x && y
  | .cap s p | .squareEnd s p => onSurf s p

/-- A run's verdict: no claim the judge decides is false. -/
def allHold (cs : List Claim) : Bool := cs.all fun c => judge c != some false

/-! ## The cross-section of a straight thin wall, in closed form

A thin wall extruded `h` along the sketch normal from a chain of lines and arcs: its volume is `h`
times the area of its cross-section, the band between the chain's offsets at `lo` and `hi`,
mitred at the creases and square at an open chain's ends. Over any ordered field:

* a line of length `L` contributes `L · (hi − lo)`;
* an arc of radius `r` turning by `φ` contributes `φ · ((r + hi)² − (r + lo)²) / 2` (the band of
  an annulus). `φ` is positive where the arc turns away from its front (counter-clockwise: the
  front, the right of the chain, is outside it), so the front's offset grows its radius;
* a crease turning by `θ` between two lines (`0 < |θ| < π`), with `k = tan(θ/2)` (`k > 0` where
  it turns away from the front: a convex corner of a counter-clockwise loop), contributes
  `k · (hi² − lo²)`: the mitre triangle added on the side it turns from, trimmed on the other;
* a closed chain's turning sums to `2π` (counter-clockwise, as the kernel turns every loop).

For a closed polygon of perimeter `P` whose creases all turn by right angles (`k = ±1`), with
`c` convex and `r` reflex corners: area `= P · t + (c − r) · t²` for `side: "out"` and
`P · t − (c − r) · t²` for `"in"`. (The L-shaped loop of research/surfaces' tests: P = 80,
c = 5, r = 1, t = 1, h = 5: volumes 420 and 380, as the kernel gives.)
-/

section Area
variable {K : Type} [Lean.Grind.Field K] [LE K] [LT K] [Std.IsLinearOrder K]
  [Std.LawfulOrderLT K] [Lean.Grind.OrderedRing K]

/-- A chain piece's band, for the closed form: a line by its length, an arc by its radius and
signed turn (`phi`, in a field holding the turn as a number: ℝ, or ℚ for rational turns). -/
inductive Piece (K : Type) where
  | line (len : K)
  | arc (r phi : K)

/-- A crease between two lines, by `k = tan(θ/2)` of its signed turn. -/
structure Crease (K : Type) where
  k : K

def pieceArea (lo hi : K) : Piece K → K
  | .line len => len * (hi - lo)
  | .arc r phi => phi * ((r + hi) * (r + hi) - (r + lo) * (r + lo)) / 2

def creaseArea (lo hi : K) (c : Crease K) : K := c.k * (hi * hi - lo * lo)

/-- The cross-section's area, as the closed form gives it. -/
def bandArea (lo hi : K) (ps : List (Piece K)) (cs : List (Crease K)) : K :=
  (ps.map (pieceArea lo hi)).foldl (· + ·) 0 + (cs.map (creaseArea lo hi)).foldl (· + ·) 0

/-- **Sides add** (to prove): the band of `[lo, hi]` is the band of `[lo, m]` and of `[m, hi]`
together, piece by piece and crease by crease. So `"both"` is `"in"` at `t/2` and `"out"` at `t/2`
of the same chain, and a thickening split at the sheet adds up. -/
def SidesAdd : Prop :=
  ∀ (lo m hi : K) (ps : List (Piece K)) (cs : List (Crease K)),
    bandArea lo hi ps cs = bandArea lo m ps cs + bandArea m hi ps cs

/-- **A right-angled loop** (to prove): perimeter `P`, `c` convex and `r` reflex corners, thickened
out by `t`: `P t + (c − r) t²`. -/
def RightLoop : Prop :=
  ∀ (t : K) (lens : List K) (ks : List K),
    (∀ k ∈ ks, k = 1 ∨ k = -1) →
      bandArea 0 t (lens.map Piece.line) (ks.map Crease.mk) =
        lens.foldl (· + ·) 0 * t + ks.foldl (· + ·) 0 * (t * t)

end Area

/-! ## What the kernel's thin walls must satisfy (to prove, against the kernel's output)

These are the differential test's claims, as `research/surfaces` runs them: for each thin wall
and thicken built, `verify/fillet-spec`'s writer (extended for thicken) emits the claims above,
every f64 exact, and `allHold` is the verdict.
-/

/-- **Skins are exact offsets**: every skin point the kernel samples lies on its face's surface
offset by `lo` or `hi`, within δ, where `onOffset` judges that surface. -/
def SkinsExact (run : List Claim) : Prop :=
  ∀ c ∈ run, (∃ s d p, c = .skin s d p) → judge c ≠ some false

/-- **Mitres meet both neighbours**: every corner of a crease lies on both creased faces'
offsets by the same side's distance. -/
def MitresExact (run : List Claim) : Prop :=
  ∀ c ∈ run, (∃ a b d p, c = .mitre a b d p) → judge c ≠ some false

/-- **The cap walls are the caps**: on a straight extrusion, every rim wall point at a cap lies
on that cap's plane, and every square end on its end plane. -/
def CapsExact (run : List Claim) : Prop :=
  ∀ c ∈ run, (∃ s p, c = .cap s p ∨ c = .squareEnd s p) → judge c ≠ some false

end ForgeExact.Thin
