# Environments: design note

David: "make it so that CAM and thermofold and all this stuff live as environments in an
Environments tab. We should also probably have a render studio, and there might be other stuff
that should live there."

## 1. What we have today (main e8150ca, interface/forge-render/src/shell)
- The ribbon's tabs are indices (`TABS` 0–4 + `SHEET_TAB`..`WELD_TAB` 5–10) and `tab_order()` is a
  hand-written mode switch: a flat pattern shows `[FLAT_TAB, 2, 3, 4]`, a sheet metal part puts
  Sheet Metal first, Thermoform appears once a part has a `thermoform` feature. Three things are
  environments in all but name, each wired its own way:
  - **Flat Pattern** (`sheet.flat_env`): replaces the tabs, has its own exit ("Go to Folded").
  - **Thermoform** (`THERMO_TAB`, `fg.thermo`): a tab that comes and goes with a feature.
  - **Bend simulation** (`sheet.bend_sim`): a floating window toggled from Sheet Metal.
- In flight, each adds its own field + tab + side-panel branch to `Shell`: CAM-UI/CAM-SIM
  (`CAM_TAB = 11`, `Shell.cam`, `Shell.camsim`, `cam_side`), Machine Builder (`Shell.machine`,
  `Action::MachineBuilder` in Manage). Robot/URDF (research/export/urdf-editor.md) will be a third.
  Without a framework every one of them edits `tab_order`, `groups`, `side`, and `docs.rs` again.
- Per-document state already exists: `docs.rs per_doc!` swaps a document's fields in and out.

## 2. What an environment is
A **mode of the window over the active document**, for a job that is not modelling it:
- **Entered** from the Environments ribbon tab (or a command, a deep link, an agent tool) with the
  current document and selection as its context.
- **Replaces the ribbon's tabs** with its own (plus Inspect / View / Manage, which every
  environment keeps), its own **browser** (left panel) if it wants one, and its own **viewport
  overlays** / hit-testing over the shared 3D view.
- **One at a time** per document. A strip under the ribbon says `ENVIRONMENT · Thermoform` with a
  **Finish Thermoform** button (and Esc on an empty selection does not exit: only Finish does).
- **State persists per document**: each document keeps which environment it is in and each
  environment's own state (`EnvState` in `ShellDoc`), so switching document tabs and back returns
  to where you were. Exiting keeps the environment's state (re-entering resumes) until the document
  closes.
- **Text stays the source of truth.** An environment never holds model data of its own: what it
  makes goes into the document as edit ops (thermoform features, sheet flats) or into a
  **companion document** linked to it — CAM's `machining` document and the URDF editor's `robot`
  document are already designed that way. An environment backed by a companion document opens or
  creates it (linked to the active part) and its Finish returns to the source document's tab.

## 3. The API
```rust
pub enum EnvId { Manufacture, Thermoform, FlatPattern, BendSim, MachineBuilder, Robot,
                 RenderStudio, Drawing, Simulation }

/// Zero-sized, in a static registry (`env::ALL`); their state lives in `Shell` (per document),
/// so a method borrows the shell whole with no aliasing.
pub trait Environment: Sync {
    fn id(&self) -> EnvId;
    /// May it be entered for this document? Err: why not (the tile's tooltip, the agent's error).
    fn available(&self, s: &Shell) -> Result<(), String>;
    fn enter(&self, s: &mut Shell) -> Result<(), String>;   // context = s.fg / selection
    fn exit(&self, s: &mut Shell);                          // Finish; state kept
    /// Its ribbon tabs, each a list of groups (the same `Group`/`Tool` the ribbon draws).
    fn tabs(&self, s: &Shell) -> Vec<(&'static str, Vec<Group>)>;
    /// Its browser, in the left panel; false: the document's own tree stays.
    fn browser(&self, s: &mut Shell, ui: &mut Ui, r: Rect) -> bool { false }
    /// Its windows and panels (egui), once a frame.
    fn panels(&self, s: &mut Shell, ctx: &egui::Context) {}
    /// What it draws over the view, and the clicks it takes there (true: taken).
    fn draw(&self, s: &Shell, p: &Painter, view: Rect) {}
    fn hit(&self, s: &mut Shell, at: Pos2) -> bool { false }
    /// Its command, from its ribbon (`Action::Env(EnvId, u16)` carries it: Action is Copy).
    fn command(&self, s: &mut Shell, c: u16) {}
}
```
- `Shell.env: EnvHost { active: Option<EnvId>, tab: usize }` is per-document (in `per_doc!`); the
  environments' own states live where they already do (`fg.thermo`, `sheet`, …) or in their
  owner's field, so migrating one moves no state. `tab_order()`, `groups()` and `side()` ask the
  active environment first; nothing else in the shell names an environment.
- New actions: `Action::EnterEnv(EnvId)`, `Action::ExitEnv`, `Action::Env(EnvId, u16)`.
- **Deep link / agents**: web `?open=part.fg&env=thermoform` (→ `app_env(name)`), native
  `--env thermoform`, agent app tool `environment {name | "exit"}` replying the entered
  environment or the `available()` refusal, and `state()` JSON gains `env`.

## 4. The list
Environments (the tab shows one tile each, in this order; one that can't be entered is drawn off
with its reason, as the ribbon does — no buttons that pretend):

| Environment | Backed by | Status |
|---|---|---|
| **Manufacture** (CAM setups/ops/posts + toolpath & stock simulation) | `machining` document | CAM-UI + CAM-SIM branches; adapter spec §5 |
| **Thermoforming** | part's `thermoform` features | on main: migrate (today's tab → env) |
| **Flat Pattern** | part's sheet | on main: migrate (today's flat_env) |
| **Bend Simulation** (press brake) | part's sheet, `query bendsim` | on main: migrate (window → env, sim drawn in the view) |
| **Machine Builder** (+ the shared tool / workholding library builder) | machine definition files | MACHINE-BUILDER branch; adapter spec §5 |
| **Robot (URDF)** | `robot` document | URDF worker, planned; adapter spec §5 |
| **Render Studio** | the document's look (+ a scene file later) | new: shell slot + the existing look/ground/shadow switches and Capture; content by another worker |
| **Drawing** | `.fgdwg` document | stays a document kind (own tab in the strip, own File > New Drawing); its tile = New Drawing from this part |
| **Simulation** (FEA / motion) | — | drawn off: "not yet: no structural or motion solver in the kernel" |

Design tabs stay: **3D Model, Sketch, Assemble, Sheet Metal, Weldments, Harness, Inspect, View,
Manage**. Arguments: sheet metal, weldments and harness author the part/assembly's own geometry
and text (features in the same `.fg`), so they are modelling, not a separate job — Fusion keeps
Sheet Metal in Design for the same reason. Flat Pattern moves because it already is a mode with an
exit. Machine Builder leaves Manage (it is a job with its own scene, not a setting).
Thermoforming moves although its features live in the part: its tab is a sequence of steps over a
finished shape (convert, mould, check, draft legend), the same shape as Sheet Metal → Flat Pattern;
"Convert to Thermoform" in 3D Model enters it.

## 5. Adapter spec for the in-flight owners (no edits on their branches)
Each owner keeps its own module; the integration (by ENVIRONMENTS, after their branch lands) is a
~50-line `impl Environment` in `shell/env/<name>.rs` delegating to what they already have:
- **CAM-UI / CAM-SIM**: keep `cam::groups(s)`, `cam_side(ui, r)`, `cam_ui(ctx)`, `camsim_ui(ctx)`
  as they are. Please expose `cam::open_for(part_path) -> Result<CamWs, String>` (open the part's
  machining document, or create one linked to it) so Enter works from a part; please don't add more
  `tab_order` / `side` branches — those become the adapter's `tabs()` / `browser()`. `CAM_TAB`
  becomes the environment's first tab.
- **MACHINE-BUILDER**: keep `MachineUi`, `machine_ui(ctx)`, `machine_frame()`; expose its ribbon
  groups as a fn `machine::groups(s)` (today the ribbon has only the Manage button) and its
  browser as `machine_side(ui, r)` if it has one. `Action::MachineBuilder` becomes
  `EnterEnv(MachineBuilder)`; the Manage button goes.
- **URDF editor**: build the robot UI as `shell/robot/` with `groups(s)`, `robot_side(ui, r)`,
  `robot_ui(ctx)` and `open_for(source) -> Result<RobotWs, String>` (the companion `robot`
  document, as CAM's). No tab constants.
- **Render Studio worker**: own `shell/env/render.rs`'s content; the slot, tile, enter/exit and a
  first ribbon (Visual Style, Ground, Shadows, Capture) land with the framework.
