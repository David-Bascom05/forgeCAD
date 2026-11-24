# kernel/forge-bool through Charon and Aeneas: what translates, what stops it, and the order

This is the TRANSLATE workstream of the shipped boolean's proof (verify/forge-bool-proof,
research/decide-once/old-path-statements.md §E). The goal is level L4: the shipped Rust of
`boolean.rs`'s `general` and everything it calls, translated to Lean so that the contracts D1–D9 are
proven about the real code. The gaps are listed in verify/forge-bool-proof/LEDGER.md ("L4 gaps").

## How it is translated

- **One translation per crate, in dependency order**: verify/aeneas/forge-math, then
  verify/aeneas/forge-geom, then verify/aeneas/forge-bool (forge-topo joins next). A crate's
  translate.conf names the crates below it in `DEPS`. A dependency's items keep exactly one
  definition, the dependency's own: the dependency's types and functions that Charon brings along
  are dropped from the crate's translation, and its Types.lean imports the dependency's package.
  - Why one per crate: Charon only sees the current crate's own MIR. For another crate it has the
    *optimized* MIR, which is inlined and lowered (a `vec![..]` there is `box_new_uninit`,
    `MaybeUninit` writes and raw pointers), and Aeneas cannot take it.
  - A dependency's *types* must still be visible to Charon (`--include` each type, never `_`, so
    the functions stay the dependency's). A field read of a type Charon kept opaque is Aeneas's
    "Unexpected error".
- **Opaque markers** (research/aeneas/SURVEY.md, "The dyn question"). `forge_geom::curve::ProcCurve`,
  `forge_geom::surface::Proc` (the `Arc<dyn …>` wrappers) and `forge_geom::intcurve::IntCurve` (a
  traced curve) are Lean `opaque` types. With them, `Curve`, `Surface` and `SurfaceSurface`
  translate as ordinary types. Each marker is an open L4 gap. A proof that reaches one says
  nothing about it, so the theorems cover the analytic cases.
- **The toolchain** is tools/aeneas/setup.sh's pins plus tools/aeneas/patches. Two patches came
  in for this crate:
  - aeneas-robust's `lower_loop_exits`: a `break`/`continue` to an enclosing loop gets an exit
    code. A loop whose early `return` (or `?`) Aeneas's `update_loops` cannot take gets the
    statements after its enclosing branch copied into the branch. The five translations made
    before it are byte-identical under it.
  - The bridge's library functions, defined as std defines them (AeneasF64/Core.lean, Std.lean):
    `f64::min`/`max` (with the open `±0` choice as `F64.zeroTie`), `next_up`/`next_down`, `ceil`,
    `i64::rem_euclid`, `[T; N]::map`, `IntoIter` for arrays, and `Iterator::all`/`any` on slices.
- **Overflow checks on** in every translation (`OVERFLOW_CHECKS` defaults to true). An integer
  overflow is a failure in the Lean, so a theorem that a function returns `ok` proves that it does
  not overflow.

## The survey (forge-bool, module by module, 2026-11-23)

`tools/aeneas/survey-modules.sh` with the opaque markers, forge-math's types included, and the
largest functions (`plan_once_in`, `split_face`, `nary_body`) kept opaque. Without that, Charon
needs more than 12 GB for boolean.rs and nary.rs. `research/aeneas/fn_survey.py` turns the logs
into one row per function.

| module | lines | Aeneas attempted | bodies failed | stops at |
|---|---|---|---|---|
| ssi.rs | 861 | 107 | 3 (the thread-local cache `MET`, `MET_MOST`) | none: D2 translates (below) |
| unify.rs | 1074 | 212 | 19 | forge-topo's types not yet included; closures that capture borrows |
| classify.rs | 1522 | 190 | 17 | `dyn Any` downcast of `Face::query` (G9); `Box<dyn Iterator>` in `edges_near`; closures |
| shape.rs | 798 | 140 | 2 | mixed recursion between types |
| imprint.rs | 2085 | 570 | 58 | Aeneas `Not_found`. Most failures are `imprint::column` (G8: out of release scope) |
| nary.rs | 3593 | 383 | 29 | Aeneas `Not_found`; `Batch` trait object (G10) |
| boolean.rs | 7374 | (Charon done) | | Aeneas out of memory at a 12 GB cap: translate it function group by function group |

### What stops it, by how much

1. **forge-topo's types** (`Body`, `Face`, `Edge`, …): the next crate to translate under `DEPS`,
   with `forge_topo::body::Cached` as one more opaque marker. Most unify, classify and boolean
   failures are "Could not find: type name: forge_topo::body::Face".
2. **Closures that capture a borrow** ("Could not match the contexts", "Can't end abstraction",
   "Joining region abstractions containing shared loans"). A narrow rewrite: a `fn` with the
   captures as arguments. It is bits-identical, but it must pass the exact callgrind gate.
3. **Iterator adaptors** (`map`, `filter`, `fold`, `zip`, `collect`, `count`): an index loop, or
   std's body in the bridge (as `all`, `any` and `sum` are) when one definition serves many call
   sites.
4. **Slice patterns** (`let [a, b] = x[..]`, `matches!(x[..], [..])`, 25 sites). Aeneas does not
   read a slice's length metadata, and Charon's name patterns cannot name `<[T]>::len`, so a tool
   fix is not cheap. The rewrite is the length and an index.
5. **Trait objects** (`dyn Any` in `Face::query`, `Box<dyn Iterator>`, `dyn Fn` names, `Batch`):
   each goes with its guess site (G9, G10) or becomes an enum.
6. **Thread-locals** (`CAREFUL`, `CHORDS`, `SCRATCH`, `UNDECIDED`, `MET`, …): FALLBACK is turning
   the retry ladder's into an argument (G2–G9). TRANSLATE waits for that PR before
   `plan_once_in`, `trim`, `crossings_near` and `meet_region`.
7. **Size**: `plan_once_in` (860 lines), `split_face` (490) and `nary_body` (2218) are single
   functions Charon and Aeneas take whole. They are translated by splitting into named steps, which
   also gives E.shape its `Pipeline` steps.

## D2, translated

`ssi::surfaces_in` and `ssi::closed_or_flush`, with forge-geom's `plane_surface`, `plane_plane`,
`planes_coincident` and `spindle_in`, translate with no Aeneas error and elaborate. Four functions
were rewritten in Aeneas's style first:
- `closed_or_flush`: the length and an index, not a slice pattern;
- `closed_form`: matches, not an `or_else` chain of closures;
- `nearly_flush`, `turned_flush`: one `corners` function, and index loops for `all`/`count`.

Each rewrite is bits-identical: the same float operations in the same order, and short-circuits
stop at the same corner. The open L4 gaps on this route are `searched` (G9's cache, reached only
when no closed form exists) and forge-geom's curved meets (`meet_coaxial`, `meet_ruled`,
`meet_spline`, `used_by`, `plane_cylinder`). None is reached by a pair of planes.

## Order from here

1. forge-topo's types and accessors (`Body::face`, `edge`, `coedge`, …) under `DEPS`. This
   unblocks unify, classify and boolean.
2. classify.rs, for FLAT-SITES: `Index::classify`, `point_in_face`, `in_planar_face`, `in_loop`,
   `plane_ray_crossings`, `ray_hits_box`, with `info` (G9) opaque until the cache is a typed table.
3. After FALLBACK's ladder PR: `plan_once_in`'s pair loop split into steps, `trim`,
   `crossings_near`, `meet_region` (D2–D4).
4. `imprint::meet`, `split_face`, `classify_specs`, `touches`, `pinched_*` (D3–D9), then
   `nary::meet3` (Dn).
