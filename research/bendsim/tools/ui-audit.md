# Bend-sim manual tooling flow: an honest audit

2026-11-24, BENDSIM-TOOLS, on bendsim/tools (from bendsim/next). David asked: "Does the UI have a
good way to set up or manually change the tools used in a sim?" The renderer is BENDSIM-ENV's;
UX-LEAD reviews. Read from `interface/forge-render/src/shell/bendsim.rs` and driven headless.

## What a user can do today

| want | how today | verdict |
|---|---|---|
| pick a press | Machine & Tools: press cards (maker, force, bed), or a stand-in set | good |
| choose which tools the planner may use | Tooling: a card per catalogue tool of the press's clamping system, checkboxes, "Only tools that fit this press and part" | good; the planner then picks among the ticked ones |
| see the stations | "Stations on the bed" strip | **only with a bending document** (`<part>_bend.fg`): a planned or staged program shows no strip until "Write program" |
| plan the stations | none in the UI (forge-lang `stations auto` exists; the Choice flag is there, no control) | **missing** |
| move a station | drag along the bed (5 mm snap) or type y; written to the document | good |
| change a station's stack lengths | type punch / die mm | good |
| change a station's punch or die | edit the document's text | **missing in the UI** |
| add / remove a station | edit the document's text | **missing in the UI** |
| lock a bend to a station | the document's `bend(.., at: sN)`, but every bend has one (the writer states them all), so a lock cannot be told from the planner's choice | **missing** |
| re-plan around manual choices | Suggest Order / Simulate Again with a document: re-plans from the press choice alone and **writes the document over**: moved stations, edited stacks and tools are lost | **broken for manual work** |

## What changed on bendsim/tools (kernel and forge-lang, mine)

- `stations auto` lays the program's tools out (stage.rs); a staged program's stations are written
  to the document as laid out, each bend at its own (`forge-bend-lang::document`).
- Locks: `Options.locks` (per bend, a station), `StepCtx::at_station`; forge-lang `lock B=N` /
  `Ask.locks`. A locked bend is made at its station with its tools; the order and the other bends'
  stations are still searched. Tests: kernel `a_locked_bend_is_made_at_its_station`, forge-lang
  `a_lock_keeps_a_bend_at_its_station`.
- With stations given (`Ask.stations`) and no sequence, the planner already searches the order
  and each bend's station among them: that is "re-plan around my stations".

## Proposed UI (BENDSIM-ENV to build or approve; UX-LEAD to review)

1. **Plan stations** button beside Suggest Order (Program group): redo with `stations_auto`.
   The strip shows for any program with stations, document or not.
2. **Re-plan keeps what the user set.** With a document: Ask.stations = the document's stations,
   Ask.locks = the user's locked bends, no sequence; write back only the order and the unlocked
   bends' `at:`. Stations the user did not touch may be re-laid-out only on "Plan stations".
3. **Per-station card** under the strip (it exists for y and lengths): add a punch and a die
   picker (the press's tools, the same cards as Tooling, "fits" filter), a remove button (refused
   while a locked bend uses it, saying which), and "+ Station" at the right end.
4. **Lock**: in the program list, each bend's station chip (s1, s2, ..) is a dropdown; picking one
   locks the bend there (a lock glyph); "unlock" returns it to the planner. In the document a lock
   needs a marker the writer does not set for planned bends: proposal `bend("rear", at: s2,
   lock: true)` (forge-bend-lang's closed argument list gains `lock`; its COVERAGE row with it).
5. **Why** on hover of a station: the reply's `stage.stations[].why` ("a gooseneck punch: every
   straight punch failed (punch_radius); the part clears it by 0.504 mm").

Kernel and forge-lang support for 1, 2 and 4 is in; 3 and 4's document marker are text and UI work.
