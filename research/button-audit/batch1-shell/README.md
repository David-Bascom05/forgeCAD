# Batch 1: the shell (sketch-mode silence, Assemble, refusals, File menu)

Branch `ui/audit-shell` (on top of `ui/audit`, the crawler). Shots at 1400x860, native, lavapipe,
taken by the crawler (tests/ui_audit): `before-*` from main, `after-*` from this branch. No
layout changed, so no 480 px pair. An `after` of an item that's off shows its hover.

| # | Command | Problem (P) | Fix |
|---|---|---|---|
| 1 | File › Save while a sketch is open (and every command's reply while sketching) | P1 dead: the sketcher wrote "Fully constrained" over every other status, every frame | `Shell::state_status`: a standing state never writes over a command's reply; the reply shows ~4 s or until the next press |
| 2 | Browser › Delete / Move Up / Move Down while a sketch is open | P1 dead: refused silently | drawn off: "Finish or cancel the sketch first" |
| 3 | Browser › Delete / Move of a feature another reads | P2 raw text: "refused: StillRead: plate reads plan" | drawn off: "Used by sk2: delete or change sk2 first"; the error's code is out of every status, dialog note and Place/Sheet/Assemble line (~15 places), kept in the typed refusal for the agent and tests |
| 4 | Assemble › Pattern / Mirror / Copy with no part chosen | P1 dead (Copy said what Mirror said, so nothing changed) | off until a top-level part is chosen, saying so; the Assemble tab's tips are its own ("Copy components", not the 3D Model's "Copy whole bodies") |
| 5 | Assemble › Keep Position / Return to Design | P1 lie: off with "not built yet" | "drag a part first (Free Move or Free Rotate)" |
| 6 | File menu: Save, Export…, New Drawing… when off | P2: off with no reason; "..." three dots | the reason on hover ("Open or create a part first", in a drawing "File > Close Drawing first"); "…" one character |

Not in this batch (noted for later): a successful Move/Delete leaves the build's statistics in the
status rather than what was done (P3); Rollback and Suppress still act while a sketch is open;
the hover lists one dependent, not up to three, and naming it doesn't select it yet.
