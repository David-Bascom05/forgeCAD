# Fillet oracle census (step 0)

2026-11-22. Every fillet the Tier A oracle (verify/fillet-oracle) can reach, on four sets, labelled
against the oracle. Where the oracle says wrong, OCCT witnessed it twice: OCCT's point classifier
on our bodies, and OCCT's own fillet of the same body before. CADFS and WHUCAD parts are also held
against their source's STEP (Parasolid via Onshape for CADFS, CATIA V5 for WHUCAD).
Kernel as of 1a1f132 (its kernel/ differs from fe8e0f1b by clippy edits only). No kernel code changed.

## Headline

Labels per fillet feature, 2026-11-23: FreeCAD, WHUCAD and CADFS relabelled with the convex rule
mended (fault 2, below; label_body at fcc5b59, 7112559's stacks for 00154651). **Final** is after the
witnesses (OCCT's classifier, OCCT's own fillet, the source STEP); the oracle's own label before
them is in brackets where it differs. The pipe set is unchanged: its rounds are all concave.

| set | fillets | right | wrong | unsure | refused | unjudged | invalid |
|---|---|---|---|---|---|---|---|
| pipe set (certified obstacle) | 200 | 161 (147) | 8 (17) | 0 (5) | 31 | 0 | 0 |
| FreeCAD library | 576 | 507 | 0 | 0 | 45 | 24 | 0 |
| WHUCAD, 1 in 7 of the hand-drawn parts | 381 | 301 (300) | 0 (1) | 0 | 50 | 29 | 1 |
| CADFS pilot, 1 in 200 | 326 | 249 (246) | 0 (3) | 0 | 43 | 30 | 4 |

The first pass (2026-11-22, the old convex rule), for comparison: FreeCAD 474 right and 33 wrong,
WHUCAD 281 and 20, CADFS 226 and 23, all those wrongs the oracle's. With the rule mended the
oracle's own wrongs fall to 0, 1 and 3, and the witnesses overrule each of those 4 (below).

- **Wrong** that survives: the pipe set's 8 fillets only (below). On FreeCAD, WHUCAD and CADFS no
  fillet is wrong and none is unsure.
- **Unjudged**: the fillet rounds convex and concave edges together, and label_body judges one kind at a time.
- **Refused**: the kernel refused the fillet. Causes below.

## Two faults in the oracle, found by the census

1. **forge-bool's `Index` misclassifies points** in both directions. label_pipe and label_body ask
   `Index` which side of our result a sample lies on. On the pipe set, OCCT's `BRepClass3d_SolidClassifier`
   on our own STEP export and a winding number on our tessellation overrule `Index` at every counted
   point of 14 parts (p00006 p00007 p00023 p00056 p00071 p00080 p00094 p00128 p00137 p00167 p00168
   p00170 p00190 p00199). That includes all 198 points the 5 `unsure` parts "lost": OCCT and the winding
   number put them inside, and our volumes match OCCT's to 1e-8, so no material was lost. p00080's
   "material past a rim" (HANDOFF) is this too: its points stand 4.8 from any face, and OCCT puts them outside.
   The reverse (Index agreeing with the oracle where OCCT disagrees, hiding a wrong) was measured on
   33 of the pipe set's parts, 989,308 points (research/fillets/oracle/occt_all.py, stopped for time):
   220 Index-vs-OCCT disagreements, 199 of them p00023's, and 7 points where only OCCT disagrees. The
   cluster I checked (p00011's 3) is 0.0025 from a new face, and the mesh winding number sides with Index, so it is
   the STEP file's surface fit. Nothing hidden was found. On FreeCAD (33 wrong), WHUCAD (20) and CADFS (23)
   OCCT agrees with `Index` at every counted point: that fault is not what made them wrong.
   The 305 disagreeing points of the 14 parts, with each side's answer, are in
   oracle/index-misclass.tsv (regenerate the parts with fc_twin.py PIPE 0 200; their sha1 is listed).
2. **A ball wider than the wall.** On a convex round the oracle keeps only what balls of radius r
   *lying inside the body before* cover. A CAD fillet needs the ball to touch only the two faces, so
   where the body is thinner than 2r near the edge the oracle removes material no fillet removes.
   On all 33 FreeCAD and all 20 WHUCAD first-pass wrongs, and the 10 CADFS ones OCCT's fillet does not settle, every counted point sampled lies where the
   ball touching the nearest face does not fit inside the body before (its centre is often outside the
   body: plates thinner than r). Example, FreeCAD 02dd381d.Body (restated in my own terms): a 30-thick
   floor, a hole of radius 100 in it, its rim rounded at r 20. Our round is the exact torus of major
   radius 120, tube 20, centred 20 below the floor, which is what any rolling-ball fillet gives. The
   oracle wants material taken off the round because no ball of 20 fits in a 30-thick floor.
   OCCT's own fillet of the same edges on the body before (research/fillets/oracle/occt_refillet.py) is
   valid and equal to ours in volume to 1e-15 on 32 of FreeCAD's 33, 1.7e-8 on the 33rd. It puts none of
   the counted points on the oracle's side. On WHUCAD it is equal (to 2.2e-7 or better) on 12 of the 20.

### Fault 2 mended (fcc5b59)

label_body's convex rule now lets the ball out of the body: a witness is clear when it lies in the
body (the old rule), or when it lies wholly on the material side of the surfaces of the asked-for
edge nearest the point (and the picked edges meeting it at a vertex) and crosses no face whose
normal at its nearest point looks the way a contact's does. It may leave through the back of a
wall, not through the faces beside the ones it rests on. Farther than 3r from every picked edge the
body is kept. Rerun on the three sets (856 parts):
- every first-pass wrong of FreeCAD (33) and all but one of WHUCAD's (19 of 20) and CADFS's (21 of
  23) are now right; the unsure ones are gone with them;
- 4 fillets the oracle still calls wrong, each overruled: WHUCAD 100000603 f8 and CADFS 00807159
  f10 are concave (the closing rule, unchanged; OCCT's fillet is ours to 1e-16 and 1.8e-8), CADFS
  00033108 f21 matches Parasolid's STEP at 1e-4 (OCCT refuses it), CADFS 00740860 f6 is new;
- **one regression: CADFS 00740860 f6** (convex, r 0.127), right under the old rule and wrong (35
  under) under the new one. The kernel is right: OCCT's own fillet equals it to 4e-16 and the part
  matches Parasolid's STEP to 1e-14. The new rule admits a ball there that is not clear: it is too
  lenient on that part, and the case is the first to look at when the spec's obstacle replaces it.
- sensitivity: judging real rounds against r ±10% on five parts, the new rule flags the same 12 of
  14 convex fillets as the old, with the error's sign now right (`R_SCALE`).
- concave rounds still use the closing of the body's air; by the spec session's P3 framing that may
  over-constrain them too (100000603, 00807159).

## Wrong, by cause

**Pipe set, 8 (kernel), all concave rounds, all `over` against the certified closing**: p00010 p00029 p00072 p00111 p00124 p00133
p00164 p00188. Judged against the certified closing of the .fc twin, `Index` confirmed by OCCT at the
counted points (295, 328, 1, 60, 6, 66, 117, 347). OCCT's own fillet of the body before, ours − OCCT in volume:
p00010 +1.3e-4 (OCCT also on the oracle's side at the 2 points label_body counts), p00029 +6.4e-6,
p00072 +2.8e-7, p00133 +5.4e-6, p00164 +1.6e-7 (1 point, OCCT on the oracle's side); p00124 −1.8e-5;
p00111 −9.2e-6 with OCCT's result invalid. p00188 has no OCCT reference: its STEP export is invalid
for OCCT, and it has both convexities. The kernel-obstacle label (label_body) finds only p00010's and
p00164's disagreements: the other six show only against the certified obstacle, with its denser
sampling. Where on each part the excess lies is not yet sorted.

(The FreeCAD, WHUCAD and CADFS paragraphs below are the first pass's, under the old convex rule;
the rerun above supersedes their counts.)

**FreeCAD, 0.** All 33 first-pass wrongs (all convex, all `over`) are the oracle's (fault 2).

**WHUCAD, 0 wrong, 8 unsure.** All 20 first-pass wrongs were fault 2 at every point. On 12 OCCT's fillet equals ours.
On the other 8, OCCT is no reference: it refuses 4 (100000380, 100000515, 100001614, 100002029), gives an
invalid body on 1 (100002114), and differs in volume on 3 (100001997 −1.1e-4, 100002615 +7.6e-4,
100004323 +5.4e-6) without putting any counted point on the oracle's side. These stay unsure until
the oracle is mended. WHUCAD's one invalid (100000678, feature 12, r 1/128) fails the topology check.

**CADFS, 0 wrong, 3 unsure.** On 13 of the 23 first-pass wrongs OCCT's fillet equals ours (to
1.8e-8). The other 10 are fault 2 at every counted point. Of those, 7 parts match Parasolid's STEP to 1e-4 in volume and
area (00033108 00110487 00274674 00645924 00801110 00912781 00972891), so the industry reference
says right where OCCT refused or differed. The other 3 stay unsure: 00057004 (OCCT refuses; a mitre the
README lists as open), 00273783 (OCCT +3.8e-4) and 00918074 (OCCT −1.8e-6). The part stops at a later
refusal in all three, so there is no STEP to compare. The 4 invalid fillets (00153157 f10, 00731664 f64,
00984443 f10 and f11) agree with the oracle at every judged point, and the parts match the STEP at
1e-4. They fail only the topology check, so the fault is in the kernel's topology, not its shape.
00154651 overflowed label_body's 2 MiB worker stack (exit 134) and counted as unjudged; with 7112559's
64 MiB stacks it is refused (Crossed), as the rerun counts it.

## Refused, by cause (the kernel's error)

| set | Overflow | Corner | Mitre | NoBlend | Crossed |
|---|---|---|---|---|---|
| pipe set | 15 | 4 | 12 | | |
| FreeCAD | 13 | 32 | | | |
| WHUCAD | 32 | 7 | 1 | 9 | 1 |
| CADFS | 27 | 2 | 2 | 7 | 3, and 1 FreeEnd |

FreeCAD's by geometry (research/fillets/oracle/freecad/refusals-by-cause.txt, unchanged by this rerun):
30 are one model's seam-split cone-to-sphere rims, with the strip face shorter than r (10 copies × 3
bodies), 7 a ball wider than a short or thin face, 8 a spring over a sharp rim.

## Against the source's STEP

Match means volume and area within the tolerance (README: the pilot's 1e-3 hid a wrong corner, so 1e-4 is the rule). Parts with a fillet:

| set | parts | match 1e-3 | match 1e-4 | off 1e-3 / 1e-4 | refused | fail | untranslated | no STEP |
|---|---|---|---|---|---|---|---|---|
| WHUCAD (CATIA) | 308 | 182 | 141 | 29 / 70 | 65 | 6 | 25 | 1 |
| CADFS (Parasolid) | 476 | 166 | 164 | 13 / 15 | 57 | 2 | 238 | 0 |

- **WHUCAD at 1e-4 is not a test.** The 41 parts that match at 1e-3 and not at 1e-4 are off by 1e-5 to
  9e-4. That is the spread the dataset's 256-level quantisation of its sketch values gives, so CATIA's
  STEP is not the part we built. 4 sampled STEPs are 404 at the source (100000290, 100001593,
  100003558, 100004047); none of those parts got as far as a comparison.
- **CADFS at 1e-4 is.** Only 2 fillet parts move from match to off (00010966 1.4e-4 in volume,
  00722722 1.2e-4). Parts without a fillet are off about as often as parts with one (65 of 1036
  translated, against 15 of 238), so most "off" is not the fillet.
- Oracle label against the STEP, per part:
  WHUCAD at 1e-3: of the parts the oracle calls right, 156 match and 20 are off (wrong elsewhere in the
  part, or quantised); 23 unjudged match and 5 are off. Of the 8 unsure, 3 match, 4 are off and 1 was not built. CADFS at 1e-4: of the parts the oracle calls right, 133 match, 12 are off and 4 were not
  built (a later feature refused). Of the 10 parts it still calls wrong, the 7 the STEP can judge all match,
  and so do the 3 with an invalid fillet.

## How it was staged (for a rerun)

- Pipe set: research/fillets/oracle/pipe_run.sh. pipe-labels.jsonl holds the first pass, and
  pipe-census.jsonl the final label with its reason.
- FreeCAD: the 245 library files of oracle/freecad/fc_files.txt, sha1 checked against map.tsv,
  translated with the library run. That reproduces the README's 677 bodies {match 484, untranslated
  130, refused 53, fail 5, wrong 5}. 352 parts hold a fillet.
- WHUCAD: the gitee repository's hand-drawn parts (folders 0100 to 0103). 2229 of their 4509 parts
  have a fillet. I took every 7th (319), read the HDF5 with h5py, and fetched each STEP from gitee
  (whose resets need retries). tools/find-parts/recreate/run_whucad.py.
- CADFS: Hugging Face dataset VladPyatov/CADFS at revision dd76a9b, public, so no Onshape key was
  needed. ids are sorted(unique_models.json)[::200], 1914 of them. Only those members of
  featurescript_fp.zip, sketch_raw.zip and step.zip were fetched, by HTTP range
  (research/fillets/oracle/rangezip.py). Then run_cadfs.py run, and step_truth.py on the 1181
  that built.
- Labels: `DUMP=DIR PER_FACE=40 RANDOMS=600 label_body OUT.jsonl LIST`. Then occt_body.py DIR (OCCT's
  classifier at the counted points) and occt_refillet.py DIR OUT.jsonl (OCCT's own fillet). Then
  census_tally.py OUT.jsonl OCCT.json REFILLET.jsonl [--ids], and census_vs_step.py for the STEP cross table.
  DUMP_ALL=1 writes every fillet's bodies, for occt_refillet on fillets the oracle passed.
- Per-fillet final labels, ids only: oracle/{freecad,whucad,cadfs}-census.txt (`id feature first final why`).
- No dataset file is in the repository. Geometry quoted here is restated.

## What step 1 should take from this

- The oracle's convex rule must not demand that the ball fit inside the body (mended in fcc5b59, with
  a heuristic for which neighbouring faces stop the ball, to be replaced by the spec's obstacle;
  00740860 shows it too lenient once).
- `Index` is not fit to be the oracle's classifier near new faces. A classifier with a certificate
  (or OCCT's, as here) is needed before a label counts.
- The only kernel wrongs found are the pipe set's 8 concave rounds, over the certified closing. They are 1.6e-7 to
  1.3e-4 off OCCT's own fillet in volume, mostly larger. Then come 5 invalid fillets (topology) and the 11 unsure.
