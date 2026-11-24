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

Turning (`forge_tooling::turning`, CAM-TURN's spec), from makers' catalogue PDFs on their own
domains: Sumitomo (General Catalogue 2025-26 B/C/E), Tungaloy (GC 2023-2024 B/C/D), Ingersoll
(turning inserts / holders PDFs; inch), Korloy (Turning 25-26). 51 inserts, 60 holder and bar
records (a row listing both hands is one record per hand). Left out: Korloy's inserts (a corner
radius range per size, not per insert), Ingersoll's inch-only ANSI holders (no ISO code),
Ingersoll's `l` (not the ISO edge length: it falls as the radius grows). Not reachable openly:
Widia, Dormer Pramet, Seco, Walter (JavaScript-only pages), Hoffmann (429), MSC, RS,
Mitsubishi (403), Travers (406), Kyocera (502/404), Iscar flip pages (TLS refused); see
raw/blocked-*.json.

`kernel/forge-turn/tests/vendor_codes.rs` decodes every record's ISO code with forge-turn and
compares it with the stated dimensions: all agree, except Ingersoll's corner radii (printed in
inches: .016/.031 in = 0.406/0.787 mm against the code's 0.4/0.8), which carry `conflict` cites.
Codes forge-turn does not decode yet (trigon W inserts; left-hand holders and bars) are listed
by the test, not failed.

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
- Cutting presets (FEEDS-SPEEDS' spec): one per (ISO 513 group, class), named "<group> <class>",
  class from the maker's column header (slot | rough | finish; Lakeshore's general milling,
  drilling, chamfering and unlabeled rows are `general`; Harvey's "Max" chip-load ceiling is
  not an operation and is left out). vc, fz, ap and ae are the printed inch values converted
  exactly (SFM x 0.3048, IPT/IPR x 25.4, a depth as a multiple or percent of the diameter x dc);
  a printed range keeps its low end, as an estimate. Drills carry fz in mm/rev; a tap with vc
  only feeds its pitch per rev. The vendors name materials, not groups: the group is taken by
  name (copper alloys N; titanium, nickel and cobalt alloys S; H only for a steel or cast iron
  whose stated hardness starts at 45 HRC). "<group> <class>" is the group's representative, by
  FEEDS-SPEEDS' preference list (N wrought 6061/7075, then wrought/"Aluminums", then any aluminium; P 1018/1010/low carbon, then medium
  carbon, then 4140/alloy steel at most 30 HRC; M 304/316/austenitic/"soft"; K grey or "soft"/
  "medium" cast iron; S Ti-6Al-4V/titanium, then nickel; H the lowest stated hardness from 45
  HRC); with none of them in the source, the group's lowest-vc row, cited so. Every row is also
  kept as "<group> <class> <maker's material>", for an exact material match. A diameter on the
  boundary of two diameter columns (Lakeshore) keeps the column whose lower edge is the
  diameter (the low end of its range, which the preset takes, is the value for that diameter). "Up to" depths are the
  source's upper limit, noted.
  Helical's finishing column is a programmed feed at the stated RDOC: the preset sets ae to it
  and notes "programmed at ae".
- Turning presets (`[[insert.preset]]`, raw/turn-vc/<maker>.json): vc per ISO group and grade
  from the makers' grade tables (Sumitomo catalogue section A "Recommended Cutting Conditions",
  with f and ap; Tungaloy GC 2023-2024 insert section p.4; Korloy grade selection tables;
  Ingersoll 1144_Turning_Tech "Cutting data for ISO turning"). Named "<G> <chipbreaker> <grade>"
  (the first material the maker prints for the group) and "<...> <material>" for each; `fz` is
  the feed in mm/rev. A printed optimum/recommended/nominal value is the start value, else the
  range's low end. Korloy and Ingersoll print vc per grade only: matched to an insert by the
  grades its catalogue row marks (read from the rendered page), f and ap from the insert's own
  row, cited as estimates. A vc with no feed is held back. Turning records are now `real`.
- Shop-part tools (raw/real-tools/<maker>.json, found one by one in the makers' catalogues):
  Tungaloy TungCut blade CGP32-3D with SGM3-020 / DGM3-020 (GC F section: the feed is only a
  bar chart, so the inserts carry no preset; reach = half the printed CUTDIA, an estimate),
  Sumitomo WDX 260D4S32 and Tungaloy TDX260F25-4 26 mm 4xD indexable drills (sig 180, a flat
  bottom, estimate; Sumitomo's flute length is its neck length LUX, estimate), Guhring 5518
  (carbide) and 1131 (HSCO) drills at 5.5, 6 and 18.5 mm, and the Tungaloy MTEC0807C171.5ISO
  thread mill (pitch 1.5). Feeds tabulated per diameter column take the first (smaller) column.
- Reamers and counterbores (raw/real-tools/reamers-<maker>.json): Guhring HR 500 solid carbide H7
  reamers, Series 1686 (steel) and 7286 (aluminium), 3/4/5/6/8/10/12 mm (HR 500 catalogue 11/26
  p.8/14, cutting data p.26/29, cross-checked on Guhring's webshop item pages); Guhring Series 324
  DIN 373 counterbores (fine grade, fixed pilot) for M3-M12, Ø 6/8/10/11/15/18/20 (webshop item
  pages; the pilot Ø and screw size come from Guhring's DIN 4000 XML, whose codes are not
  labelled: in the note only). RUKO's DIN 373 counterbores (same sizes) print no cutting length,
  so no record is made from them; their pilot Ø agree with Guhring's. Reamer feeds are tabulated
  for Ø 3/4/6/8/10/12, not 5 mm: the 5 mm reamers carry no preset.
- Inconsistencies found in the sources are kept in the raw facts (e.g. Rego-Fix ER20 nut thread
  M25x1.5 in the catalogue vs M24x1.5 on the pocket sheet; Tormach BT30 "Gage length is 60mm
  (1.97 inches)"; Lakeshore ".0003.0010" chip load); the generator never resolves them silently.

## Fusion import alignment

`interface/forge-cam-import/tests/align.rs`: Harvey 792208 imported from a Fusion library
(inch, as Harvey ships it) gives the same type, dc, lcf, dcon, oal and flutes as our record.
