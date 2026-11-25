# Batch 2: words (UX-LEAD's P2s from batch 1's review)

Branch `ui/audit-words`, on top of `ui/audit-shell`. 1400x860 native, lavapipe, by the crawler.
`before-*` are batch 1's heads; `after-*` this branch's.

| # | Where | Problem (P) | Fix |
|---|---|---|---|
| 1 | Every "Saved / Wrote <path>" in the status bar | P2: the whole temp path | the bar says a file by its name ("Saved plate.fg"); the whole line on hover (`shell::brief`, one place; not units, ratios or links) |
| 2 | A tool's "Off: …" line in every tooltip | P2: red reads as an error | amber: guidance (tips.rs, shared) |
| 3 | Browser › Delete of a feature others read | P2: one reader named, nothing to click | "Used by sk2, sk3 and sk4: delete or change them first" (up to three, then "and N more"), and "Select sk2" beside it |
| 4 | Browser › Move Up / Down, Delete (succeeding) | P3: the build's statistics replaced what was done | "Move hole1 above pattern1: done · volume …": every build of that edit leads with its words |
