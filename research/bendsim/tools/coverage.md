# What the bend simulator can simulate today

2026-11-24, BENDSIM-TOOLS, bendsim/tools @1d058319. David: "Are we able to sim step bends? Or more
complex stuff? Basically the tools that we have, can we sim them all?"

## 1. The catalogue's 316 tools

- **full**: every working value published, and the body is a drawn section (punch) or published
  width and height (die). Every check runs against the real shape.
- **partial**: simulated, every check runs, but the body is an envelope built from published values
  that errs large (may report a collision that is not there, never misses one); some also have no
  published load rating, so tonnage is "not checked" for them.
- **refused**: a working value is unknown (`UnknownGeometry`, naming the field); the planner never
  picks it and asking for it by id says which fields are missing.

| | full | partial | refused | total |
|---|---|---|---|---|
| punches | 8 | 73 | 87 | 168 |
| dies | 47 | 48 | 53 | 148 |
| **all** | **55** | **121** | **140** | **316** |

Why refused (fields unknown): punch body width 57 (Amada 17, Wilson 33+, Mate 8), gooseneck throat
depth 26 (WILA 14, Rolleri 8, Mate 5; Wilson 3 with width), die shoulder radius + height + width 21
(all Amada), die width 11, die shoulder radius 10, die V + shoulder radius 8, one Wilson die whose
published width (18 mm) is below its own V (a data error, refused as inconsistent).
Why partial: punch body an envelope 73; die width or height an estimate 48; no load rating 28.

| maker | tool | full | partial | refused |
|---|---|---|---|---|
| Amada | die | 0 | 0 | 21 |
| Amada | punch straight | 0 | 0 | 17 |
| Bystronic | die | 14 | 8 | 0 |
| Bystronic | punch acute | 0 | 2 | 0 |
| Bystronic | punch gooseneck | 0 | 9 | 0 |
| Bystronic | punch straight | 0 | 4 | 0 |
| Mate Precision Tooling | die | 0 | 12 | 0 |
| Mate Precision Tooling | punch gooseneck | 0 | 0 | 5 |
| Mate Precision Tooling | punch straight | 0 | 8 | 8 |
| Rolleri S.p.A. | die | 1 | 4 | 12 |
| Rolleri S.p.A. | punch acute | 0 | 3 | 0 |
| Rolleri S.p.A. | punch gooseneck | 0 | 1 | 8 |
| Rolleri S.p.A. | punch straight | 0 | 2 | 0 |
| Trumpf | die | 18 | 0 | 0 |
| Trumpf | punch acute | 1 | 5 | 1 |
| Trumpf | punch gooseneck | 7 | 2 | 1 |
| Trumpf | punch straight | 0 | 2 | 0 |
| WILA | die | 0 | 24 | 0 |
| WILA | punch acute | 0 | 10 | 0 |
| WILA | punch gooseneck | 0 | 0 | 14 |
| WILA | punch straight | 0 | 23 | 0 |
| Wilson Tool International | die | 14 | 0 | 20 |
| Wilson Tool International | punch acute | 0 | 0 | 6 |
| Wilson Tool International | punch gooseneck | 0 | 2 | 12 |
| Wilson Tool International | punch straight | 0 | 0 | 15 |

## 2. Operations

| operation | status | what is missing |
|---|---|---|
| air bending (any angle the tools reach) | **supported**, certified through the motion | |
| acute bends (< 90 deg included) | **supported**: acute punches and dies, chosen when the angle rules need them | |
| multi-station / stage bending | **supported** (`stations auto`, locks) | timing of the moves between stations (no published times) |
| radius tools (a punch radius = the bend's radius) | **partial**: the punch-radius rule holds the tip radius to the inside radius; a radius punch in the catalogue simulates as any punch | few radius punches in the catalogue (Trumpf R3, R4); no large-radius tools |
| bottoming / coining | **not supported** | the rules assume air bending (legs on the shoulders, the angle from the stroke); bottoming needs the die angle = the bend angle, the punch through to the V's floor, and a force model (several times air bending: a cited table needed) |
| hem (pre-bend, then flatten) | **not supported** (`HemNotSimulated`) | a two-step program: acute pre-bend (exists) + a flattening step (flat punch/die, the flange closed to the hem's gap, a near-180 deg refold), flattening dies in the catalogue, the flattening force (WILA publishes a tonnage chart) |
| Z / joggle (offset tools) | **not supported** | a step that makes two bends in one stroke (coupled angles in the stroke), offset punch/die sets in the catalogue (Wilson lists 6, 7.9, 11.1 mm) |
| step / bump bending (a large radius from many small bends) | **not supported** | the planner emitting n small bends along one zone, and a rule on the facets' deviation from the radius (closed form) |
| offset dies | **not supported** | as Z tools (same two-bend step) |
| moving tools (folders, rotary benders, panel benders) | **not supported** | new machine types: research/bendsim/tools/gap.md ranks 7, 9, 10 |

## 3. Cheapest ways to cover the biggest gaps

| gap | gain | cost | how |
|---|---|---|---|
| refused punches with unknown width or throat (83) | +83 tools usable | **S per maker** (data) | drawn sections off the makers' published drawings (tools/bendsim/section_from_drawing.py; done for 8 Trumpf punches, which unlocked 3 refused goosenecks). Amada, Wilson, WILA, Mate first |
| partial punches (73 envelopes) | real collision answers, fewer false refusals | S per maker (data) | the same tool; the before/after on OW200/S: flanges hit the envelope 7-16 mm deep, the real tool misses them |
| refused dies (53) | +53 dies | S-M (data) | shoulder radius, height and width from the drawings (Amada 21 needs all three) |
| bump bending | large radii on any press | **M** (planner + one rule) | n sub-steps along a zone, a closed-form facet-deviation check |
| hems | the commonest refused feature | M-L | pre-bend + flatten (kinematics near 180 deg, a contact rule, flattening dies) |
| Z / offset | joggles | M | a two-bend step coupled through the stroke |
| bottoming | precise angles on thick stock | S-M | a mode with its own angle and force rules, cited force table |

Recommendation: data first (the drawn-sections tool across makers: +83 punches, +53 dies, real
bodies for 73 more), then bump bending, then hems.
