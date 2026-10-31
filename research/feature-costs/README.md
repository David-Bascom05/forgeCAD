# What each feature costs

The speed hunt's ranking: instructions:u per feature, by prefix difference, over the 15 bench
parts (`tools/bench-parts`), the suite, torture, the fillet corpus, the finishing and edit parts.
`table.txt` is the ranking; `seq.tsv` / `nary1.tsv` the raw per-feature counts, `post.tsv` mesh,
mass and check per part. Measured at main ad33439.

The cost of feature N is count(first N features) minus count(first N-1), so what a feature leaves
for later (a face box computed on first use, say) is charged to the feature that first needs it:
`extrude.dist.cut` tops the table by sum with a median of 0.43M, because a few cuts pay for the
fillet faces they meet. `centres-inclusive.txt` (function, inclusive, whole parts, frame-pointer
build, which runs about 1.25x the instructions: read percentages) and `centres-self.txt` (self,
release build, its only trustworthy column is the leaf function) show that side.

## Threads, with what they leave behind

Each part again with its thread lines replaced by a parameter (keeps feature numbers and names):
```
part     with      without   thread lines alone
frame    3383M     1913M     1432M
valve     686M       56M      575M
crank    1127M      854M      252M
jack      690M      483M      159M
gearbox  1054M      855M      124M
conrod    185M       16M       79M
sum      7125M     4178M     2621M
```
Subdivision evals in all 13 bench thread features, by curve x surface and axis relation
(`pairs.py`, with `pairs-instrument.py` applied temporarily): helix x fence 66.9%, circle x screw
coaxial 11.9%, helix x plane coaxial 5.2%, helix x cylinder coaxial 4.7%, helix x cone coaxial
3.6%, helix x plane off axis 3.2%, line x fence 2.9%, circle x fence 1.2%.

## How to rerun

```
cp research/feature-costs/featcost.rs interface/forge-agent/examples/      # not kept in examples
fbuild cargo build --release -p forge-agent --example featcost
B=$PWD/target/release/examples/featcost
for n in $(ls tools/bench-parts/*.fc | xargs -n1 basename | sed 's/\.fc$//'); do
  fbuild $B resolve $n $PWD/.scratch/fc/$n.fc; done
fbuild python3 research/feature-costs/measure.py $B $PWD/.scratch/fc seq.tsv seq 4     # under a minute
fbuild python3 research/feature-costs/measure.py $B $PWD/.scratch/fc post.tsv post 4
fbuild python3 research/feature-costs/measure.py $B $PWD/.scratch/fc nary1.tsv nary1 4 '^(bench|suite|torture)'
python3 research/feature-costs/rank.py seq.tsv post.tsv > table.txt
```
Function profile of one feature (N its index + 1), and whole parts, in a frame-pointer build:
```
CARGO_TARGET_DIR=$PWD/target-prof RUSTFLAGS="-C force-frame-pointers=yes" \
  CARGO_PROFILE_RELEASE_DEBUG=line-tables-only fbuild cargo build --release -p forge-agent --example featcost
python3 research/feature-costs/prof.py .scratch/fc/frame.fc 127 seq 40
python3 research/feature-costs/centres.py $PWD/target-prof/release/examples/featcost $PWD/.scratch/fc
```
