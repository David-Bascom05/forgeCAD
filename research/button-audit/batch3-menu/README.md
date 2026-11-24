# Batch 3: the browser menu's reasons (UX-LEAD's P2s from batch 2's review)

Branch `ui/audit-words3`, on top of `ui/audit-words`. 1400x860 native, lavapipe, by the crawler.

| # | Where | Problem (P) | Fix |
|---|---|---|---|
| 1 | Browser menu: an item that's off | P2: its reason was a tooltip over the items below ("Select sk2" looked greyed under it) | the reason is a muted 11 px line under the item, inside the menu; no tooltip covers anything |
| 2 | Browser menu: "Select sk2" | P3: a click toggled the selection, and said nothing | it selects (never toggles off) and the status says "sk2 uses extrude1: change or delete it first" |
| 3 | The edit reply's stats ("volume … mm³ · … faces") | P2: not the train-6 format | nothing here: the reply leads `Built::summary`, which UI-BUGHUNT's ui/bughunt-3 rewrites with `ux::show` ("valid · 58.5 cm³ · 133 faces"); the reply takes it as soon as that is in main |

| 4 | Drawing › Manage › Update, Zoom All on an up-to-date, fitted sheet | P1 dead: nothing changed, nothing said | "Views updated from plate.fg", "The sheet fitted to the view"; the drawing's own status line now goes through the status layer (`state_status`), so no command's reply is written over |
| 5 | Draft, Split, Combine, Delete Face at a 480 px window | P1 crash: fgdim.rs:444 `clamp` on an empty range (the in-view dimension field in a view narrower than it) | the field kept at the view's left when it can't fit; found by the 480 px pass |

Found on the way (not in this batch): the in-view depth field of a selected extrude shows "12"
with no unit.
