# Batch 4: units in the view's dimension field

Branch `ui/audit-units`, on top of `ui/audit-words3`. 1400x860 native, lavapipe, by the crawler
(the part's browser › extrude row › Select, which selects extrude1 and shows its depth arrow).

| # | Where | Problem (P) | Fix |
|---|---|---|---|
| 1 | A selected feature's in-view dimension field (extrude depth, …) | P2: a bare "12" | "12 mm", in the part's system (`ux::show`, Qty::Length; "0.4724 in" in an inch part); typed "0.5 in" or "1/8 in" is read, a bare number in the part's unit; a wrong entry says how to type one |

Unit test: `shell::fgdim::words_tests`.
