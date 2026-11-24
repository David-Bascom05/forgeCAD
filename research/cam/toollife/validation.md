# Tool life: predicted vs measured

Reported as found. Short version: the open datasets do not validate the model yet. One has
the right material but the tool failed by breakage at high feed, which a flank-wear Taylor
model cannot predict. The other has one speed only and no matching row.

## 1. Malyer & Çaydaş, Ti-6Al-4V pocket entries

- **Source:** Zenodo record 23755091 v1.1 (the README's own citation DOI is
  10.5281/zenodo.23754085). Zip md5 `8736d0c37bdd2b99cde7a02fad2c3200`, checked.
- **Licence:** CC BY 4.0 (data and NC), MIT (code). No sign-up needed.
- **Setup:** Ø4 mm 4-flute AlTiCrN carbide (Eurocut E55-4x11x4x50) at 3 820 rpm, dry. That gives
  vc = π·4·3820/1000 = **48.0 m/min**. Feeds were 366/732/1098 mm/min (fz 0.024/0.048/0.072).
- **Measured:** `data/tool_entries.csv`, the completed entries out of 112.
- **Minutes in cut:** our own parser, `malyer_nclen.py`. It counts G1/G2/G3 length below the
  stock top (z = 0) divided by F, with the arc centres incremental. A program is one face: it has
  58 descents below z = 0, against the 56 the README implies (14 levels × 4 pockets). Minutes per
  entry use 56.
- **Row used:** `ti64-carbide-alti-n-highfeed-vb03` (n 0.336, C 204.4) treated as AlTiN. The
  measured tool's coating is AlTiCrN, so this is a stretch: the coating does not match exactly. vc
  = 48 is below the row's range of 50–125, so the estimate is extrapolated. The row's tool is a
  high-feed insert cutter, not a Ø4 end mill. Predicted T = (204.4/48.0)^(1/0.336) = **74.6 min**.

| Material | Entry | Feed (mm/min) | Measured | Minutes in cut | Predicted D |
|---|---|---|---|---|---|
| both | H, P, R | 366 | 112/112, tool survived (no wear reported) | 59.9–62.7 | 0.80–0.84 |
| both | H, P, R | 732 | 112/112, survived | 30.0–31.3 | 0.40–0.42 |
| annealed | H, P | 1098 | 112/112, survived | 20.0–20.8 | 0.27–0.28 |
| annealed | R | 1098 | **failed at 22** (3 runs) | 4.10 | **0.055** |
| aged | H | 1098 | **failed at 29** | 5.39 | **0.072** |
| aged | P | 1098 | **failed at 29** | 5.17 | **0.069** |
| aged | R | 1098 | **failed at 12** | 2.24 | **0.030** |

**Verdict.**
- The surviving runs are consistent with the prediction (D < 1). They are censored, though, and
  no flank wear was measured, so they do not test the model.
- All four failures happened at D = 0.03–0.07. That is 14–33 times earlier than predicted.
- The failures depend on feed and entry strategy. The V–T row has no feed term, and vc was the
  same in every run, so it predicts the same life at every feed.
- These were tool breakages at high feed in dry Ti. They lie outside what a flank-wear Taylor
  model describes.
- **Not validated.** The dataset shows what the model must not be used for: breakage risk from
  feed per tooth and entry.

## 2. NASA / UC Berkeley milling (Agogino & Goebel)

- **Source:** data.nasa.gov `mill.zip`.
- **Licence:** US Government work. No sign-up needed.
- **Setup:** face milling with KC710 inserts in cast iron and stainless J45. Every case ran at
  vc = 200 m/min.
- **Why it cannot validate:** with one speed, n cannot be identified. Our data has no row for KC710
  in either material, so the model answers "not estimated". That is correct, and it is all the
  dataset can check today. If a cited row for these pairs appears, the VB-vs-time curves can be
  compared with D(t).
- Not downloaded in this round.

## 3. PHM 2010 milling

- The PHM Society page states no licence. The IEEE DataPort copy needs a subscription, and its
  files are "not uploaded". The Hugging Face mirror is gated behind contact details. We recorded
  this and did not work around it (research/cam/benchmarks/datasets-gcode.md §3.1).
- Not used.

## 4. What would validate it

- Measured VB(t) with a known toolpath in a material and tool that have a row.
- The other route is the shop's own logs: `forgecad tool log worn LABEL --vb ..` after runs,
  which the calibration (model.md §4) fits and shows beside the cited row.
