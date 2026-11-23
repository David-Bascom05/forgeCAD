# p00010: a concave round filled where a ball over the support's rim says air

2026-11-23. Reported by the fillet-spec session; reproduced here. Not fixed here: this branch's
brief excludes kernel/ changes.

## Repro

```
python3 -I research/fillets/oracle/fc_twin.py PIPE 10 1      # PIPE/p00010.fg and its .fc twin
```

Keep the part to the round: the bar (radius 17.7138, height 51.4445), the oblique boss (radius
11.5667) and `round = fillet(edges(between: (body, boss), crease: "concave"), 2.13853177)`; drop
everything from `sketch s3` on. Then:

```
target/release/examples/filletwhy P.fg                             # built
PER_FACE=200 RANDOMS=3000 target/release/examples/label_body OUT.jsonl P.fg
```

label_body (the body before as the obstacle) counts 14 points over, every one within 1.5 of the
bar's top (z = 51.4445), e.g. (3.7449, 17.3377, 51.3009), (5.7995, 18.7251, 49.9212). label_pipe
(the certified closing of the .fc twin) counts 295 over on the whole part. OCCT's own fillet of
the same edge, a valid body, is air at all of them, and is 1.3e-4 smaller in volume. At the same
part with r 3.2 the kernel builds and label_body counts none over (3 under elsewhere).

## Cause

`roll` (fillet.rs) switches a round onto a rim (`cliff`) only where a spring's foot runs off its
face. Here both springs stay on their faces: the round is exact for its two supports (the
spec session's `near`: spine on both offsets to 5e-15, pipe at r to 1e-15). But a ball resting on
the bar's top rim and on the boss, in air, comes nearer than r to the round's arc: the closing is
air there and the round fills it. No check asks whether a ball on a support's rim reaches into the
round when the springs stay on.

## The check proposed (not applied)

In `roll`, after `cliff` returns nothing and nothing switched: for each support face's line or
circle edges (other than the rounded edge) within 2r of the round, build the blend between that
rim's tube and the other support (as `cliff` does), at centres along the round's spine. Where its
ball touches the rim within it and the other face within it, is not stopped by the face beyond the
rim (`blocked_beyond`), and comes nearer than r (less a margin of 1e-3 r) to a point of the
round's arc, refuse with Overflow, so the cliff retries take the edge. Whether those then build the
right shape (the round rolling over the rim) is the spec's construction to decide. Its test: the
truncated p00010 above, with label_body counting none over, and the census sets re-run for new
refusals.

Same pattern, per the spec session: p00133 (11 of 11 points) and p00164 (40 of 40).
