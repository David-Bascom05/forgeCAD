# Real tools, holders and collets in forge-tooling

GENERATED: `kernel/forge-tooling/data/{tools,holders,collets}/<vendor>.toml` come from
`tools/tooling/gen.py`, reading the restated facts in `raw/<vendor>.json` (each value with the
page or catalogue PDF it is from, the date read, and the source's own wording). Edit the facts or
the rules, re-run, then `cargo test -p forge-tooling -p forge-cam-import`.

Format: forge-tooling's `[[tool]]`, `[[holder]]` (profile = [z from the nose, diameter]), and the
new `[[collet]]`, `[[collet_nut]]`, `[[pull_stud]]`, `[[insert]]`, `[[turn_holder]]`, each with
`real = true` and `[[<kind>.cite]]` per value (src/cite.rs: published | estimate | unknown |
conflict | user). build.rs embeds every vendor file; the generic files stay first.

## Sources (read 2026-11-23; open pages and catalogue PDFs only, no sign-ups)

| vendor | what | from |
|---|---|---|
| Tormach | TTS / BT30 holders, ER collets and nuts, own-brand cutters | tormach.com product pages |
| Harvey Tool, Helical Solutions | end mills, ball, corner radius, chamfer, spot, drills; speeds & feeds | tool-details pages + the linked Speeds & Feeds PDFs |
| Lakeshore Carbide | end mills, chamfer, spot, drills, thread mills; speeds & feeds | product pages + General Purpose / drill / threadmill PDFs |
| OSG, Guhring | drills, spot drills, taps; cutting data | OSG List PDFs (res.cloudinary.com, linked from osgtool.com); Guhring full-line catalogue PDFs (guhring.com) |
| YG-1 | collected, NOT generated | yg1usa.com prints no units on most dimension columns: left out rather than guessed |
| Techniks, Rego-Fix, Schunk, Maritool, Shars | CAT40/BT30/BT40/HSK63A holders (ER, side-lock, shrink, hydraulic, drill chucks), ER collets and nuts, pull studs | their catalogue PDFs; Maritool per-item drawing PDFs |
| Sandvik Coromant, Kennametal | nothing usable | product pages are script-built; the data API needs sign-in; no open PDF on their own domains |
| Iscar, Lyndex | nothing | 403 to automated reads |
| Haas Tooling | nothing | haascnc.com bot challenge |

So the turning inserts, holders and bars have a format (`forge_tooling::turning`, CAM-TURN's
spec) but no maker data yet.

## Rules applied

- A record missing a value the format requires (dc, lcf, dcon, oal; a bull's corner radius; a
  chamfer/spot angle; a drill's point angle; a tap's pitch; a holder's gauge length and nose) is
  left out; `gen.py` prints the count and reason per vendor.
- Holder profiles: where the maker's drawing gives the steps (Schunk, Maritool), the rings are
  read from it and measured from the nose; the undimensioned part up to the gauge line is taken
  at the interface's flange diameter. Where only the nose is published (Techniks, Rego-Fix,
  Shars, Tormach TTS), everything behind the nose up to the gauge line is taken at the flange
  diameter: errs large, never small. Flange diameters: CAT40 63.5 and BT40 62.99 from Maritool
  drawings; HSK63A 63 (estimate); BT30 and TTS stand-ins (unknown, believed large).
- Cutting presets: at most one per ISO 513 group, the vendor's slotting row first; the vendors
  name materials, not groups, so the group is an estimate by the material's name, cited so;
  a range of fz keeps its smallest value.
- Inconsistencies found in the sources are kept in the raw facts (e.g. Rego-Fix ER20 nut thread
  M25x1.5 in the catalogue vs M24x1.5 on the pocket sheet; Tormach BT30 "Gage length is 60mm
  (1.97 inches)"; Lakeshore ".0003.0010" chip load); the generator never resolves them silently.

## Fusion import alignment

`interface/forge-cam-import/tests/align.rs`: Harvey 792208 imported from a Fusion library
(inch, as Harvey ships it) gives the same type, dc, lcf, dcon, oal and flutes as our record.
