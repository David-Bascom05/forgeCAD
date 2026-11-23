/-!
# Numbers as text (kernel/forge-mesh/src/scene.rs `num`)

The text formats write a coordinate with `num`: to 1e-6 mm, trailing zeros dropped, never "-0";
an index as Rust's integer `Display`. What reading that text back gives is assumed here, not
proven: Rust's formatting and `str::parse` are std's.

* `parse_write`: a number written and parsed back is the number rounded (`round`, to the nearest
  1e-6 as `{:.6}` does). So a text file holds the rounded mesh exactly, not the mesh;
* `parseN_writeN`: an index written and parsed back is itself.

Neither text has whitespace in it, so a line splits back into the tokens it was written from.
-/

namespace MeshFiles

structure Text (F : Type) where
  write : F → String
  parse : String → Option F
  round : F → F
  parse_write : ∀ x, parse (write x) = some (round x)
  writeN : Nat → String
  parseN : String → Option Nat
  parseN_writeN : ∀ n, parseN (writeN n) = some n

end MeshFiles
