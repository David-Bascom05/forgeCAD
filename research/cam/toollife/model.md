# Tool life: model, data, tracking

David asked: "Do we have a way to model or track tool wear?" This is the answer: a prediction
from cited data (`forge_tooling::life`), and a record of the physical tools
(`forge_tooling::instances`, `forgecad tool log`). Both are advisory: a tool-life warning never
blocks posting or certification.

## 1. Model

Taylor (1907), extended: v T^n f^a ap^b = C, so

    T(v, f, ap) = (C / (v f^a ap^b))^(1/n)      [min]

v cutting speed (m/min) at the engaged diameter, f the feed the row names (`fz`, `hex`, `hm`
mm, or `f` mm/rev), ap axial depth (mm). T is minutes to the row's wear criterion (e.g. VB 0.3
mm). With a = b = 0 it is the basic v T^n = C.

**Per sample, not per op.** A program's life use is the linear damage sum (Miner's rule
carried to tool wear):

    D = Σ_i Δt_i / T(v_i, f_i, ap_i)

over fixed-length samples of the posted path (`forge_cam::life::measure`, which replays the
path on the in-process stock with TOOLPATH-QUALITY's `engage::schedule_samples`). Each sample
has its own minutes (length / its feed), fz = F/(z n), peak chip fz·cos θ_max, mean chip over
the engaged span, depth ap, and v = π D_eff n / 1000 (D_eff for a ball at ap, FEEDS-SPEEDS'
`feeds::effective_diameter`). Air samples add nothing. D = 1 is the row's criterion.

Basis for linear accumulation: Abeni et al., Wear 560–561 (2025) 205607 and Procedia CIRP
(CMMO 2025) build cumulative wear from constant-speed curves (citing Miner, ASME J Appl Mech
67 (1945) A159) and find variable-speed turning lives of 21.26 vs 21.75, 19.35 vs 21.75 and
31.63 vs 28.50 min (actual vs predicted) for 1045 / PM4325: within ~10% for that pair only.
Elsewhere it is an assumption, stated as such.

**Reported per tool:** minutes in cut, D (None = "not estimated"), minutes outside the row's
fitted ranges (extrapolated), the op/move where D reaches the user's limit (`Limit { at,
start }`: the change fraction and the life already used from the tool's instance log), per-op
shares, minutes of plunges/ramps the sampler does not measure, canned cycles (not estimated).

**Check:** `life::check` warns when D > at − start, naming the change point and a sister tool
(another loaded tool of the same library record). Never an error.

**Unknown stays unknown.** A tool × material with no row → "not estimated". Rows match by
substrate (`Tool.material`), coating (the tool record's `coating` text, compared exactly: a
vendor's name such as "C3" is not taken to mean AlTiN), and the part's material by
forge-material's canonical name (`part(.., material: "...")`; an alias resolves to the
library's entry first, so "AISI 1045" is "1045 cold drawn" and does not match a row for
168–190 HB 1045). No group-level fallback. A row's `condition` field states when the source
does not give the work's condition (the Ti row: Albertelli et al. name Ti6Al4V without heat
treatment); it is printed with every estimate.

**UI API:** `forge_cam_lang::life::estimate(&Cam, rows, limits) -> LifeReport` keyed by op and
tool names (pure: no fs/clock/threads).

## 2. Data: `kernel/forge-tooling/data/life.toml`

Tool-side cutting table (per the one-materials-library rule, tool × material pairs live in the
tooling crate; no new key in materials.toml). Every row: n, C, a, b, feed, vb/criterion,
vc/f/ap ranges, `source`, `taken` (table/equation), `value` (the source's statement), `licence`.

| id | tool | work | n | C | range | source |
|---|---|---|---|---|---|---|
| ti64-carbide-alti-n-highfeed-vb03 | carbide AlTiN | Ti-6Al-4V | 0.336 | 204.4 | vc 50–125 | Albertelli et al. 2021, IJAMT, CC BY (Eq. 5 β → n, C) |
| h13-58hrc-pcbn-face | CBN | 1.2344 58 HRC | 0.969 | 5154 | 100–700 | Er & Aslan 2025, AMM 70(1), CC BY |
| h13-58hrc-carbide-cvd-face | carbide CVD | 1.2344 58 HRC | 1.005 | 2946 | 50–250 | same |
| d3-55hrc-pcbn-face | CBN | 1.2080 55 HRC | 1.077 | 3855 | 100–900 | same |
| d2-56hrc-pcbn-face | CBN | 1.2379 56 HRC | 0.986 | 2617 | 100–500 | same |
| p20-55hrc-ceramic-face | ceramic TiN | 1.2738 55 HRC | 0.493 | 2204 | 200–1000 | same |
| p20-55hrc-pcbn-face | CBN | 1.2738 55 HRC | 0.697 | 2250 | 100–500 | same |
| 1045-carbide-cvd-turning | carbide CVD (turning) | AISI 1045 | 0.362 | 1085 | 300–400 | Abeni et al. 2025, Wear |

Caveats kept in each row's `note`: the Ti row is a high-feed insert cutter (fz 0.7), not an end
mill; Er & Aslan's n ≈ 1 comes from pairwise slopes and their own graph fit differs (Table 5).
The hardened steels and 1045 are not yet in forge-material, so those rows cannot match a part
until the materials are added there (FEEDS-SPEEDS owns materials.toml).

Found but not representable yet (need an ae or rake term): Tatar et al., Heliyon 2020
(Ti-6Al-4V, ln T = 25.99 − 1.00 ln ae − 4.01 ln vc); Mohruni et al. 2007 (Ti-6Al4V,
T = 653.1 V^−3.509 fz^−3.548 γ^0.647).

**UNKNOWN (no open, readable source found):** HSS anything; carbide in aluminium, stainless,
cast iron, Inconel 718; steel milling with f/ap exponents; maker tool-life correction tables
(Sandvik/Walter/Kennametal/Seco/Iscar originals not readable openly). Mitsubishi's web page gives
"+20% vc → life ½, +50% → ⅕" (implied n ≈ 0.25–0.26, turning, generic) — not a row: no C.

## 3. Tracking: tool instances

A library kind of its own (forge-cam-lang `toolstore::ToolInstances` on forge-library, agreed with
MACHINE-BUILDER): one file per physical tool, `cam/tool-instances/<label>.toml`, at the user
level by default (`<config>/forgecad/cam/tool-instances/`: a crib outlives projects) or the
project's (`--project`). Never in the .fg, never in `cam/tools/`. An instance names its
library tool by id and is resolved through the tools catalog; values are not copied in.
Storage goes through `instances::Store` (toolstore::LevelStore on disk, MemStore in memory/browser).

    [[instance]] label, tool (library id), serial, note, state
    [[instance.run]] program, date, work, minutes, how = "run" | "posted"
    [[instance.run.bin]] v = [vc, fz, hex, hm, f_rev, ap, minutes]   # 2% log bins of the cut
    [[instance.log]] date, event = observe|worn|chipped|broken|regrind, vb (mm), note, after

Runs keep their cut as condition bins, so D can be recomputed against any row (cited or
calibrated). A life ends at worn/chipped/broken/regrind; the next starts fresh.

Mounts (which instance is each tool number of a job) are per-project UI/CLI state in
`.forgecad/cam-mounts.toml` (`instances::Mounts`, `[mounts] T1 = "label"`), shared by the CAM
workspace and `forgecad tool log mount|unmount`; `--use T1=LABEL` overrides for one run. Not in
the .fg (no `instance:` argument: what is in the spindle changes run to run).

CLI: `forgecad tool log add|list|observe|worn|chipped|broken|regrind|mount|unmount|predict|run|calibrate`
(interface/forge-cam-lang/src/toollog.rs).

## 4. Calibration

From lives that ended **worn** (chipped/broken are another failure mode and are not used), all
in one row's material: each life's own C under exponent n is the C that makes its D exactly 1,
C_i = (Σ t_k (v_k f^a ap^b)^(1/n))^n. The shop C is their geometric mean; spread = min–max and
the geometric SD. n is fitted (minimising the spread of ln C_i) only with ≥ 3 lives whose
time-weighted speeds span ≥ 20%; otherwise the cited n is kept. Shown as "calibrated from your N
logs: C = .. (range ..); cited: n = .., C = .. (source)". The cited row is never replaced.

## 5. Validation

See validation.md.

## 6. Entry load (separate from tool life)

The validation showed that breakage at entry was what actually failed, not flank wear. So there
is a second, separate check (`forge_tooling::entry`, data `kernel/forge-tooling/data/entry.toml`).
It is reported as "entry load (makers' guidance, not tool life)" and never folded into D.
`forge_cam::life::measure` finds the entries while replaying the stock.

The cited rules:
- **Feed at first contact.** A sample meeting stock after air may run at ≤ 50% of the op's
  largest cutting feed. Source: Helical Machining Guidebook p. 5, "reduced by at least 50%"; Sandvik
  Coromant, "Reduce feed to 50% until the cutter is fully engaged".
- **Straight plunge into stock.** Feed ≤ 50% of the op's largest cutting feed. Source: Garr General
  Solid Carbide Milling Guide p. 262, "drop feed by approximately 50%".
- **Ramp or helix angle.** At most 3° in ferrous work (ISO P, M, K, H) and 10° in non-ferrous work
  (N): the upper ends of the source's "1–3°" and "3–10°" starting ranges. Source: Helical p. 4. Ti
  and superalloys (S) fit neither class, so this check is reported "not checked".
- **Helix diameter.** At least 1.1 × D. Source: Helical p. 4, "programmed helix diameter
  >110–120% of tool diameter". We read the helix diameter as the diameter the helix cuts (2r + D).

**UNKNOWN (not used, no guess made):**
- A breakage threshold on chip thickness for small end mills.
- The Harvey Tool and Kyocera SGS chip loads. They are starting values, not limits.

**Not used for coefficients (second research round):**
- AISI 4140: Hubbard et al., IJERI 2015, n = 0.125 / 0.150. These are secondary constants derived
  from McClure and Groover, with no VB stated.
- Hardox 500: Majerik and Barenyi 2016. The paper's own arithmetic disagrees with its data, and the
  material is not in forge-material.
- Kantojärvi et al. 2023 (C45E milling, extended Taylor, open access) could not be fetched. It is
  worth retrieving.
- Aluminium, stainless and cast iron with carbide: still UNKNOWN.
