# Judging a STEP read against OCCT: when the truth is uncertain

`rate.py` holds each file's read against OCCT 8.0.1 (`tools/occt/step_read.py`). A file matches
when it is valid, not warned, and its summed volume and area are within 1e-3 of the truth's.
Two rules take a file out of the count instead, as **uncertain**: it is counted neither as
matching nor as not, and neither better nor worse in a comparison (`rate.py --compare`).

## The truth's own measures disagree

This is the importer's truth rule (research/import/truth.md on claude/repo-review-mbdrlj-import;
tools/find-parts/recreate/truth_rule.py `judged`).
- OCCT gives a volume three ways: its default integration, its integration at a stated precision,
  and Gauss–Kronrod (`step_read.py`'s `rule`).
- The Gauss–Kronrod volume is the truth.
- The area is taken from whichever measure's volume agrees with Gauss–Kronrod's to 1e-4, a tenth of
  the tolerance judged by.
- When neither agrees, or the two that agree give areas more than 1e-4 apart, OCCT cannot say what
  the file holds, and the file is uncertain.

## Gaps: the file's own

A file's edges may miss their faces. Each edge carries the distance it was measured to lie off them
(`record_deviations`, the read's `maxtol` for the largest). The solid such a file describes is then
only defined to within that distance.

**Derivation.**
- Let ε be the largest gap.
- Any closed surface a reader may take through the file's faces and edges lies within ε of every
  other one, pointwise along the boundary.
- Two solids whose boundaries are within ε of each other differ in volume by at most the volume of a
  shell of thickness ε about the boundary. To first order in ε that is ε·A, A being the boundary's
  area.
- So the volume itself is known to |ΔV| ≤ ε·A, and relatively to |ΔV|/V ≤ ε·A/V.

**Rule.** A read whose volume differs from the truth's by no more than `maxtol · area / volume`,
relatively, does not mismatch and does not match: it is uncertain within the file's own gaps.
- The bound is computed from each run's own read: its maxtol, area and volume.
- In a comparison of two runs, a file uncertain in either is counted neither way.
- A file with no tolerant edge has maxtol 0, and the rule never applies to it.

**Where it bites.** WHUCAD's CATIA files, among others: parts a few millimetres across whose edges
miss their faces by 1e-4 mm, so ε·A/V reaches 1e-3 to 1e-2. On the 3146 files of
research/import/robustness.md §2, 70 of the 85 mismatches with tolerant edges fall within their own
band. Healing those files with the heal limit at the stated uncertainty, instead of 20 times it,
changes no file's volume by a digit: the mismatch is the file's, not the reader's.
