# Library builder (design note)

David: "we need tools to make custom ones of all this": machines, cutting tools / holders /
collets, workholding. One framework, three (then more) clients, so the user learns one editor.
The Machine Builder (research/cam/machine-builder.md, branch machines/builder) is the first
client and the reference for the behaviour; this note is how it generalises. Owner: MACHINE-
BUILDER. Data owners supply each kind's schema and validation: machines CAM-MACHINE
(kernel/forge-machine), tooling MACHINE-LIB-A (kernel/forge-tooling, tools/base), workholding
WORKHOLDING (kernel/forge-workholding).

## 1. What every kind shares

| piece | machines today | generic |
|---|---|---|
| levels | project `cam/machines/` → user `<config>/forgecad/cam/machines/` → built-in; earlier hides later; broken file listed, not fatal | same per kind folder: `cam/<folder>/` (machines, tools, holders, collets, inserts, turn_holders, workholding) |
| records | `[[machine]]` | `[[<kind>]]`, several per file, id unique per level |
| provenance | `[[machine.cite]]` path/status/source/taken/value/basis/note | `[[<kind>.cite]]`, statuses published / estimate / unknown / conflict / user (F2; tooling already has user) |
| UNKNOWN | cite `unknown` + stand-in until F1 | F1 for every kind: a required number may be absent when a cite `unknown` names it; consumers say "not checked" |
| new | template or clone; copied values cited unknown ("copied from X"); `real = false` | `derive(lib, from, id, overrides)` per kind (see §4) |
| writer | canonical, `parse(write(r)) == r`, byte-stable | same contract, tested over the whole built-in set of each kind |
| check | structure, home/travel collision, part in envelope, provenance | kind checks (owner's) + shared provenance + geometry checks the kind declares |
| CLI | `forgecad machine list/new/check` | `forgecad <kind> list/new/check`, same flags, same exit codes (0 ok, 1 problems, 2 usage) |
| app | Machine Builder | one Library Builder environment, one tab per kind |

## 2. The shared crate: interface/forge-library

Kernel-free (no speed band), wasm-clean, used by the CLI and forge-render.

```rust
pub trait Kind: 'static {
    type Record: Clone + PartialEq;
    const WORD: &'static str;        // "machine", "tool", "holder", ...
    const FOLDER: &'static str;      // "machines", "tools", ...
    fn builtin() -> Vec<(String /*file*/, String /*text*/)>;
    fn parse(text: &str) -> Result<Vec<Self::Record>, Refusal>;   // the owner's reader
    fn write(r: &Self::Record) -> Result<String, String>;         // canonical record text
    fn id(r: &Self::Record) -> &str;
    fn cites(r: &Self::Record) -> &[Cite];  fn cites_mut(..);
    fn required_paths(r: &Self::Record) -> Vec<String>;           // what `real` must cite
    fn schema(r: &Self::Record) -> Vec<Field>;                    // the form, see §3
    fn check(r: &Self::Record, ctx: &CheckCtx) -> Vec<Finding>;   // the owner's checks
    fn preview(r: &Self::Record, ctx: &PreviewCtx) -> Preview;    // §5
    fn templates() -> Vec<(&'static str, Self::Record)>;
}
pub struct Catalog<K>;              // levels, hides, errors (catalog.rs generalised)
pub fn derive<K>(cat, from, id, overrides) -> Result<K::Record, Refusal>
pub fn save<K>(record, parts, dir, level, overwrite) -> Result<PathBuf, String>  // reads back before writing
pub mod cli { pub fn run<K>(env, argv) -> Option<Out> }
pub fn generation() -> u64          // bumped on any save: pickers reload
```

`forge-machine-builder` becomes `impl Kind for Machine` plus the machine-only checks (travel
sweep, envelope). Its public API stays (CAM-UI already uses `catalog::Catalog::load`).

## 3. Field schema (drives the form)

```rust
pub struct Field {
    pub path: String,        // cite path: "joint.X.range", "dc", "preset.finish.vc", "profile"
    pub label: &'static str,
    pub ty: Ty,              // Num | Int | Vec(n) | Range | Text | Choice(&[..]) | Bool
                             // | Link(kind word) | Ref(path to another field) | Table(columns)
                             // | Profile (z,d rings) | Points(columns)
    pub unit: Unit,          // Length mm | Angle deg | Feed mm/min | AngFeed deg/min | Rpm | Power kW
                             // | Time s | Mass kg | Accel | Speed m/min | None
    pub need: Need,          // Required | Optional(&'static str /* what absence means */)
                             // | When(&'static str /* condition, e.g. "type = bull" */, bool)
    pub group: &'static str, // the form's section
    pub help: &'static str,
    pub cited: bool,         // shows a provenance chip and "?" (unknown)
}
```

The form is built from the schema: each field a row with its unit, a provenance chip (copied /
user / UNKNOWN / published / estimate / conflict) and "?" to mark it unknown; editing drops its
cites (the value is the user's). Repeating structures (joints, spindles, presets, profile rings,
power-curve points) are `Table`/`Points` rows with add/remove. Kinds with a tree (machine
joints) or a picture (tool profile) add a custom panel beside the schema form (§5). Units are
shown and typed in the user's units, stored in mm/deg (forge-units).

## 4. Clone / derive

`derive(cat, from, id, overrides: &[(path, value)])`:
- copies the record under `id`, `real = false`, `source = "derived from <from> (<level> <file>)"`;
- the source's cites are kept for values not overridden; values it did not cite are cited
  `unknown`, basis "copied from <from>: not this <kind>'s value"; overridden values get status
  `user` (F2) and lose the source's cites;
- carries part files (machine bodies, fixture parts) to `parts/<id>/` beside the new file;
- refuses an id already at the target level (other file).

To MACHINE-LIB-A: please make `forge_tooling::derive` exactly this shape
`pub fn derive(lib: &Library, kind: &str, from: &str, id: &str, overrides: &[(String, Val)]) ->
Result<Record, DeriveError>` (Val = forge_tooling::toml::Val), so the generic layer calls it; or
expose the record type and I do the copy generically through `Kind`.

## 5. Preview (live 3D)

`Preview` = a list of (triangles, placement link, colour role) using machine_mesh.rs's `Tris`:
- machine: bodies at the jogged pose (done: real parts or prims; live contact red/amber);
- tool + holder + collet/nut: revolved profiles about the spindle axis, drawn live as the rings
  are edited (a `Profile` field edits a polyline in a 2D panel beside the 3D view); the cutting
  part tinted; stick-out shown; the assembly placed on a chosen machine's spindle nose (to show
  max tool length / diameter);
- workholding: vise jaws, clamps, plates, chucks from their parametric shapes; custom fixtures
  from the user's `.fg` parts (attach, fit envelope, as machine bodies).

## 6. Checks wanted in forge-tooling (answer to MACHINE-LIB-A)

All of the proposed ones, in forge-tooling (they are facts about the data, CLI and app share
them): `re ≤ dc/2` (bull), stick-out ≥ lcf, nose ring ≥ nut OD, collet clamp range contains the
tool shank (dcon), insert vs its ISO 1832 code (ic, s, re), turn holder vs ISO 5608/6261 code.
The cross-library one — the assembly within the machine's max tool length and diameter
(spindle `max`) — goes in the builder (it reads two libraries): `forge_library::checks::fits(
tool, holder, spindle)`. Each finding: path, measured, limit, fix (as forge-lang lints do).

## 7. App: the Library Builder environment

Per ENVIRONMENTS (ui/environments @435cea4, `EnvId::MachineBuilder`): the environment's tabs are
the kinds (Machines first; Tools, Holders, Collets, Inserts, Workholding as they land), its
ribbon groups from `machine::groups(s)` (New, Clone, Save, Check, Close), its browser
`machine_side(ui, r)` = the library list (search, filter by kind / vendor, collapse by vendor:
the vendor libraries make it long), its panel the form + validation, its view the preview. The
Manage-tab button and `Action::MachineBuilder` go when the adapter lands. Pickers elsewhere
(CAM-UI's setup dialog, tool picker) read `Catalog::<K>::load` and reload on `generation()`.

## 8. Order of work

1. F4 `collide::check_box` on cam/machine (band) — machine travel proofs without the workaround.
2. forge-library crate: generalise catalog/write/derive/cli from forge-machine-builder; machines
   move onto it with their tests unchanged (byte-stable files, same CLI output).
3. Environment adapter hooks (`groups`, `machine_side`), list search/filter/collapse.
4. Tools/holders/collets client once tools/base's types are on main (schema above).
5. Workholding client once WORKHOLDING's schema is sent.
