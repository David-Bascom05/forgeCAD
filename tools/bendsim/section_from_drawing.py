#!/usr/bin/env python3
"""A tool's section measured from a maker's dimensioned drawing (a page rendered to PNG).

The drawing's filled body (its uniform grey) and its outline are segmented, the largest region
holding a seed point taken, its boundary simplified to a polygon, and the polygon scaled and placed
from the drawing's own dimensions (each axis by a printed dimension), the tip (lowest point) is the origin, z up, x toward the throat side
(+x: the side the drawing's `--throat` says). The polygon is clipped at the clamping face
(`--height`, a published dimension). Every vertex is reported with how it was found:

  published  a printed dimension the vertex sits on (checked: printed vs measured, `--check`)
  measured   read off the drawing at its scale (the drawing is to scale: its printed dimensions
             agree with the measured ones within `--tol`, which the run reports)

Usage:
  pdftoppm -f PAGE -l PAGE -r 300 -png catalog.pdf page
  section_from_drawing.py page-PAGE.png --seed X Y --width MM --height MM
      [--throat right|left] [--check X0 X1 MM ...] [--eps MM] [--grow MM] [--fill GREY]

  Each axis is scaled by its own printed dimension (the body's overall width across, the tip to
  the clamping face): catalogue drawings are not always drawn at one scale both ways. The
  section is grown outward by `--grow` (0.5 mm), so it errs large.

Prints the section as the tooling TOML's `outline = [[x, z], ..]` (a closed polygon, mm, the tip
at the origin, counter-clockwise) and the checks. The catalogue itself is never copied: only the measured
numbers, with the page cited, go into the tooling data.
"""
import argparse
import math
import sys

import cv2
import numpy as np


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("png")
    ap.add_argument("--seed", nargs=2, type=float, default=None, help="a pixel inside the body (default: the largest region of the tool's grey)")
    ap.add_argument("--width", type=float, default=None,
                    help="the body's printed overall width across, mm: scales x by it (default: x at z's scale)")
    ap.add_argument("--height", type=float, required=True, help="tip to clamping face, mm")
    ap.add_argument("--throat", default="right", choices=["right", "left"])
    ap.add_argument("--check", nargs=3, type=float, action="append", default=[],
                    help="x0 x1 mm: a printed horizontal dimension between two pixel columns")
    ap.add_argument("--grow", type=float, default=0.5, help="outward margin, mm: the section errs large")
    ap.add_argument("--eps", type=float, default=0.6, help="simplification, mm")
    ap.add_argument("--fill", type=int, default=None, help="the tool's grey (default: the commonest grey in 180..205)")
    ap.add_argument("--axis", type=float, default=None,
                    help="the pixel column of the drawing's tool axis (the tang's centre line): x = 0; else the tip's lowest pixel")
    ap.add_argument("--overlay", default=None, help="write the drawing with the section drawn on it (red) to this PNG")
    ap.add_argument("--spike", type=float, default=2.0, help="drop out-and-back spikes narrower than this, mm")
    ap.add_argument("--tol", type=float, default=1.0)
    a = ap.parse_args()

    g = cv2.cvtColor(cv2.imread(a.png), cv2.COLOR_BGR2GRAY)
    if a.fill is None:
        v, c = np.unique(g, return_counts=True)
        band = [(n, x) for x, n in zip(v, c) if 180 <= x <= 205]
        a.fill = int(max(band)[1])
    body = (np.abs(g.astype(int) - a.fill) <= 4).astype(np.uint8)
    # the outline is the drawing's thick line; dimension and leader lines are thin: an opening
    # keeps only the thick one (thin lines touching the body would otherwise join it)
    dark = (g < 90).astype(np.uint8)
    outline = cv2.morphologyEx(dark, cv2.MORPH_OPEN, np.ones((6, 6), np.uint8))
    mask = cv2.morphologyEx(body | outline, cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
    n, lab = cv2.connectedComponents(mask)
    if a.seed is None:
        nb, lb = cv2.connectedComponents(body)
        sizes = np.bincount(lb.ravel())
        sizes[0] = 0
        big = int(np.argmax(sizes))
        yy, xx = np.nonzero(lb == big)
        i = len(yy) // 2
        a.seed = (float(xx[i]), float(yy[i]))
    sx, sy = int(a.seed[0]), int(a.seed[1])
    k = lab[sy, sx]
    if k == 0:
        sys.exit("the seed is not on the body")
    region = lab == k
    # holes inside the body (text, dimension lines across it) filled
    cs0, _ = cv2.findContours(region.astype(np.uint8), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    filled = np.zeros_like(g, dtype=np.uint8)
    cv2.drawContours(filled, [max(cs0, key=cv2.contourArea)], -1, 1, -1)
    region = filled.astype(bool)
    ys, xs = np.nonzero(region)
    # the outline's half width: the region runs to the line's outer edge, the drawing's edge is
    # its centre line
    row = outline[sy]
    dk = np.nonzero(row[: sx] > 0)[0]
    lw = 1 + (dk[-1] - dk[0] if len(dk) else 0)
    lw = min(lw, 20)
    half = 0.5 * lw
    tip_y = ys.max() - half
    # the clamping face: the bottom of the clamp drawn above the tool (its lighter grey), else
    # the row where the tang widens to the body
    clamp = np.abs(g.astype(int) - 222) <= 4
    cols = slice(xs.min(), xs.max() + 1)
    def run(row):
        # the longest solid run in a row (hatching is broken lines; the clamp is filled)
        best = cur = 0
        for v in row:
            cur = cur + 1 if v else 0
            best = max(best, cur)
        return best

    sub = clamp[:, cols]
    cand = np.nonzero(sub.sum(axis=1) > 0.25 * np.ptp(xs))[0]
    crow = np.array([y for y in cand if run(sub[y]) > 0.25 * np.ptp(xs)], dtype=int)
    crow = crow[crow < tip_y]
    if len(crow):
        face = crow.max() + lw
        how = "the clamp's lower edge"
    else:
        widths = [(y, np.ptp(np.nonzero(region[y])[0])) for y in range(ys.min(), ys.max()) if region[y].any()]
        face = next(y for (y, w), (_, w2) in zip(widths, widths[1:]) if w2 - w > 0.2 * np.ptp(xs)) + half
        how = "the tang's step"
    zpx = (tip_y - face) / a.height
    xpx = (xs.max() - xs.min() - 2 * half) / a.width if a.width else zpx
    if a.axis is None:
        # the tool's axis: the tang's middle just above the clamping face
        r = np.nonzero(region[int(face - 2 * lw)])[0]
        a.axis = 0.5 * (r.min() + r.max())
    print(f"# grey {a.fill}; clamping face from {how} (row {face:.0f}); axis column {a.axis:.1f}")
    print(f"# scales: x {xpx:.3f} px/mm, z {zpx:.3f} px/mm (printed height {a.height}); line {lw} px")
    print(f"# overall width across: {(xs.max() - xs.min() - 2 * half) / xpx:.1f} mm")
    grown = cv2.dilate(region.astype(np.uint8) * 255, cv2.getStructuringElement(
        cv2.MORPH_ELLIPSE, (2 * int(a.grow * xpx) + 1, 2 * int(a.grow * zpx) + 1)))
    cs, _ = cv2.findContours(grown, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    c = max(cs, key=cv2.contourArea)
    poly = cv2.approxPolyDP(c, a.eps * xpx, True)[:, 0, :].astype(float)
    tipc = c[:, 0, :][np.argmax(c[:, 0, 1])].astype(float)
    tip = (a.axis if a.axis is not None else tipc[0], tip_y)
    print(f"# the tip's lowest pixel is {(tipc[0] - tip[0]) / xpx:+.2f} mm from x = 0")
    sgn = 1.0 if a.throat == "right" else -1.0

    def to_mm(p):
        return (sgn * (p[0] - tip[0]) / xpx, (tip[1] - p[1]) / zpx)

    pts = [to_mm(p) for p in poly]
    # an arrowhead or leader touching the outline leaves a spike: out and straight back within
    # `--spike` mm along the edge. Dropped (it is the drawing's annotation, not the tool).
    changed = True
    while changed and len(pts) > 4:
        changed = False
        for i in range(len(pts)):
            a_, b_, c_ = pts[i - 1], pts[i], pts[(i + 1) % len(pts)]
            if math.hypot(c_[0] - a_[0], c_[1] - a_[1]) < a.spike and math.hypot(b_[0] - a_[0], b_[1] - a_[1]) > 1.0:
                del pts[i]
                changed = True
                break
    # clip at the clamping face: z <= height (the tang above it is the clamp's, not the tool's)
    out = []
    for i in range(len(pts)):
        p, q = pts[i], pts[(i + 1) % len(pts)]
        if p[1] <= a.height:
            out.append(p)
        if (p[1] - a.height) * (q[1] - a.height) < 0:
            t = (a.height - p[1]) / (q[1] - p[1])
            out.append((p[0] + t * (q[0] - p[0]), a.height))
    # counter-clockwise
    area = sum(out[i][0] * out[(i + 1) % len(out)][1] - out[(i + 1) % len(out)][0] * out[i][1] for i in range(len(out)))
    if area < 0:
        out.reverse()
    print(f"# tip at pixel ({tip[0]:.0f}, {tip[1]:.0f}); {len(out)} vertices; grown {a.grow} mm")
    worst = 0.0
    for cx0, cx1, cmm in a.check:
        got = abs(cx1 - cx0) / xpx
        worst = max(worst, abs(got - cmm))
        print(f"# check: printed {cmm} mm, measured {got:.2f} mm")
    if worst > a.tol:
        sys.exit(f"the drawing is not to scale within {a.tol} mm (worst {worst:.2f})")
    zmax = max(p[1] for p in out)
    print(f"# top of the section: z = {zmax:.2f} (height {a.height})")
    pts = [p for i, p in enumerate(out) if math.hypot(p[0] - out[i - 1][0], p[1] - out[i - 1][1]) > 1e-6]
    if a.overlay:
        im = cv2.imread(a.png)
        px = np.array([[tip[0] + sgn * x * xpx, tip[1] - z * zpx] for x, z in pts], dtype=np.int32)
        cv2.polylines(im, [px], True, (0, 0, 255), 4)
        y0, y1 = max(0, int(tip[1] - a.height * zpx) - 150), int(tip[1]) + 100
        im = im[y0:y1]
        cv2.imwrite(a.overlay, cv2.resize(im, (im.shape[1] // 3, im.shape[0] // 3)))
    print("outline = [" + ", ".join(f"[{x:.1f}, {z:.1f}]" for x, z in pts) + "]")


if __name__ == "__main__":
    main()
