# The Manufacture workspace (CAM UI), 2026-11-23

Branch `cam/ui` (off `claude/repo-review-mbdrlj-cam-lean`). Code: `interface/forge-render/src/shell/cam/`
(the workspace: `doc.rs` the document's text, `run.rs` a run of it, `schema.rs` the op forms,
`mod.rs` its state and commands, `ui.rs` its widgets) and `src/gui_cam.rs` (the host: open,
generate off the window's thread, write files, the view's extras). Screenshots: this folder,
taken from the browser build (`web/tests/cam.cjs`, swiftshader WebGPU).

## What it is

Opening a `.fg` file whose first statement is `machining` opens the Manufacture tab over its
part (the part is the view's model). The machining document's **text is the source of truth**,
as a part's is: every dialog, menu and ribbon button makes forge-lang's own edit ops on it
(`SetArg`, `AddFeature`, `DeleteFeature`, `MoveFeature`, `Suppress`/`Unsuppress`, `Rename`) and
nothing else, so the text keeps its comments, spacing and order, and every change is one an
agent or the CLI could make. What the workspace shows is `forge_cam_lang::run` of that text, read
from the reply `forgecad.cam/1` (the same JSON `forgecad cam --json` prints) and the run's paths.

| piece | Fusion's | here |
|---|---|---|
| setup dialog | operation type, machine, WCS orientation and origin, stock modes, fixture | machine (document's or library's), origin (stock top centre / corner, part origin, a point), placed-at work offset, fixture (document's soft jaws or a library vise, across x or y), continue from a setup (turned over), stock: part box plus offsets or a fixed box, heights tab (tol, clearance, clear, retract). Cylinder / tube / from-solid stock: not in the language yet (forge-cam-lang has `box` only) |
| browser | setups, folders, ops with a dirty mark, suppress, protect, patterns | setups with their ops and NC programs, each with its state: certified ✔, refused ✖, not posted !, stale ↻, generating …, suppressed ⊘; fold, select, double-click to edit, right-click: Edit, Generate, Move Up / Down, Show Stock After (`--at OP`, for the stock view), Suppress, Delete. Folders and patterns: not in the language |
| op dialogs | Tool, Geometry, Heights, Passes, Linking tabs | the same tabs, fields from `schema.rs`, one row per op built-in forge-cam-lang has: Face, 2D Clear, 2D Contour, Pocket, Chamfer (edge break), Engrave, Drill, Thread Mill. Hole and sketch geometry from the part's history (a list of its `hole()` features with d, depth, thread), or picked in the view (a face of the hole). Every field shows the built-in's default when left out, and a left-out field is never written. Feeds and speeds on the Tool tab, from the run (cutting data, chip thinning) |
| generate | per op, dirty state | Generate runs the whole document off the window's thread (Auto Generate after each edit, on by default); an edit makes that op and the later ops of its setup stale (a tool, part or machine edit: every op after it) |
| simulation | stock simulation, collisions, rest | the views are CAM-SIM's: this workspace hands them `CamWs::focus()` (below). In the view now: the setup's stock box, its work offset's axes, the selected op's posted path (a preview), a located check's witness |
| assurance | — | each op says how its path is known safe: proven by construction (its generator proven, Aeneas L4), checked on demand (Check Now), or, today for every op, checked at generate (interim: forge-cam-lang certifies the posted path on each run). **Check Now** (ribbon and the browser's menu) runs the checker again; until an on-demand checker of a run's paths is exposed it generates again |
| checks | collision warnings | each op's certificate (least cutter distance, pairs), its failed or undecided checks (rule, value vs limit, tool part vs what, phase, move, at): click one to locate its witness. The Checks window lists them all with G4's leftover regions |
| tool library | tools, holders, presets | the library's cutters and holders (built in plus the project's `cam/*.toml`), filtered; Add puts a `tool(..)` with the next free T number into the document |
| post process | post, program number, NC output | per program: the dialect (fanuc, haas, linuxcnc, grbl), the number, round-trip and certification, the NC text, Write NC File beside the document |
| setup sheet | HTML | `Setup Sheet` writes the run's HTML sheet beside the document |
| text | — | Document Text: the text itself, edited by hand, applied as one undo step |

## The interface for the simulation views (CAM-SIM)

`forge_render::shell::cam::Focus`, read with `shell.cam.as_ref().map(|w| w.focus())`:

```rust
pub struct Focus {
    pub setup: Option<String>,        // the setup in focus
    pub op: Option<String>,           // the op selected (None: the whole setup)
    pub locate: Option<Locate>,       // a check's witness to show: { op, mov, at: [f64; 3] (WCS), rule }
    pub stock_at: Option<String>,     // show the in-process stock after this op (`--at OP`)
    pub only_selected: bool,
}
```

The paths are the run's: `w.run.as_ref().unwrap().cam.ops[k].paths` (forge-cam's `OpPath`, WCS
coordinates; part frame = WCS + the setup's origin, for a setup not turned over). The workspace
redraws the view when `Focus` changes (`Gui::cam_key`).

## Native and browser

The same code on both: the workspace is shell code; the run goes through `plat::spawn` (a thread
natively, a Web Worker in the threaded browser build, inline in the single-threaded one).
forge-cam-lang reads its files through `forge_dirs::fs` (the page's in-memory tree in a browser)
and starts no clock unless `FORGE_CAM_TIME` is set (std's `Instant` panics in a browser).

## Tests

- `cargo test --lib shell::cam` (forge-render): the document's edits round-trip (set, unset, add,
  move, suppress and back to the same text, a setup edited in place), stale marking, and real
  runs of examples/machined/flange_cam.fg (all ops certified; a short stick-out refused).
- `tests/cam_ui.rs`: the app headless (skips without a GPU adapter): opens on Manufacture, the op
  dialog's edit in the text, post writes the NC file, the tool library adds a tool.
- `web/tests/cam.cjs OUT`: the browser build end to end, with screenshots (`FORGE_WEB_SHOTS`).
