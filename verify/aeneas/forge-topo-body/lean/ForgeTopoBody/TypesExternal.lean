/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeGeom

/-!
# The opaque types of forge-topo-body's translation (TypesExternal_Template.lean)

- `OnceLock<T>` (the `Cached` derived data: boxes, hulls, the point-in-face index) is what it
  holds: a value or none. Only the cache's own methods read it (`new`, `get_mut`, `take`).
- `FaceQuery` (the point-in-face index, `Arc<dyn Any>`): an opaque value. Nothing D8 states reads
  it.
-/

open Aeneas Aeneas.Std

def std.sync.once_lock.OnceLock (T : Type _) : Type _ := Option T

namespace forge_topo

/-- `forge_topo::body::FaceQuery`, `Arc<dyn Any + Send + Sync>`: an opaque value. -/
opaque body.FaceQuery : Type

end forge_topo
