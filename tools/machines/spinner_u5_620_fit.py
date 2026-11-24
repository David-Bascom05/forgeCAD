#!/usr/bin/env python3
"""Fits the Spinner U5-620's rotary axes (the RPTU/Spinner dataset's machine) from its 1 s log:
the senses of +B and +C, where the C axis stands, and the line the B axis lies on. These are
estimates fitted from logs, never published values; spinner.toml cites them so.

Data: Zenodo 15735480 data_v1_0_1.csv (CC BY 4.0, 78.7 MB; sha256
d7227badb6ae1a27339622b337207cc342f75acc5baa18538352228fd52be580 as fetched 2026-11-24), which
logs every axis's machine position (MCS) and a second position the dataset calls WCS. With no
transformation active the two are equal in every row (so it is the SINUMERIK basic coordinate
system, BCS); under TRAORI (the 3+2 program Coordinate_system_representation_Achskreuz, KOORDI_JEAN
in the log) they differ by the table's kinematics:

    MCS = R_B R_C BCS + t,   t = (I - R_B) b + R_B (I - R_C) c + L e_z

R_B, R_C the table's rotations, b a point on the B axis and c on the C axis (machine coordinates),
L the tool's length. Rows are grouped by (tool, B, C) with B and C held (whole degrees), only rows
where MCS and BCS differ (TRAORI on); each group's rotation and offset are fitted (Kabsch), groups
whose median residual exceeds 0.1 mm dropped (they mix states); then one least-squares fit over the
groups gives b_x + b_z (B axis rotation about y: only that sum and b_z - b_x + L are seen at B 90),
c_x, c_y, and b_z - b_x + L per tool.

What the log cannot give: B is only ever held at 0 and 90 under TRAORI, so the B axis's position
along the 45 degree line b_x + b_z = s is not separable from the tool lengths (not published);
the record takes the B and C axes as intersecting (b_x = c_x), an assumption.

    python3 -I tools/machines/spinner_u5_620_fit.py data_v1_0_1.csv
"""
import collections
import csv
import sys

import numpy as np


def kabsch(p, q):
    """R, t with q ≈ R p + t (R a proper rotation)."""
    pc, qc = p.mean(0), q.mean(0)
    u, _, vt = np.linalg.svd((p - pc).T @ (q - qc))
    d = np.sign(np.linalg.det(vt.T @ u.T))
    r = vt.T @ np.diag([1.0, 1.0, d]) @ u.T
    return r, qc - r @ pc


def ry(deg):
    a = np.radians(deg)
    return np.array([[np.cos(a), 0, np.sin(a)], [0, 1, 0], [-np.sin(a), 0, np.cos(a)]])


def rz(deg):
    a = np.radians(deg)
    return np.array([[np.cos(a), -np.sin(a), 0], [np.sin(a), np.cos(a), 0], [0, 0, 1]])


def main(path):
    groups = collections.defaultdict(list)
    lo, hi = [1e9] * 5, [-1e9] * 5
    equal_untransformed = [0, 0]
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            try:
                m = [float(row[f"{a}-axis_position_MCS"]) for a in "XYZBC"]
                w = [float(row[f"{a}-axis_position_WCS"]) for a in "XYZBC"]
            except ValueError:
                continue
            for i in range(5):
                lo[i], hi[i] = min(lo[i], m[i]), max(hi[i], m[i])
            diff = max(abs(m[i] - w[i]) for i in range(3))
            if abs(m[3]) < 1e-9:
                equal_untransformed[0] += 1
                equal_untransformed[1] += diff == 0.0
            if diff <= 0.05:
                continue
            b, c = m[3], m[4]
            if abs(b - round(b)) > 1e-9 or abs(c - round(c)) > 1e-9:
                continue
            groups[(row["Tool_ID"], round(b), round(c) % 360)].append((m[:3], w[:3]))
    print("machine positions seen (MCS): " + ", ".join(
        f"{a} {lo[i]:.3f}..{hi[i]:.3f}" for i, a in enumerate("XYZBC")))
    print(f"rows at B 0: {equal_untransformed[0]}, MCS = BCS exactly in {equal_untransformed[1]}")
    clean = []
    for k, v in sorted(groups.items()):
        if len(v) < 50:
            continue
        mm = np.array([a for a, _ in v])
        ww = np.array([b for _, b in v])
        r, t = kabsch(ww, mm)
        res = np.linalg.norm(mm - (ww @ r.T + t), axis=1)
        med = float(np.median(res))
        ok = med <= 0.1
        print(f"{k}: {len(v)} rows, residual median {med:.4f} p95 {np.percentile(res, 95):.4f} mm"
              f"{'' if ok else '  (dropped: mixes states)'}")
        if ok:
            clean.append((k, r, t, med))
    # the senses: every clean group's rotation against R_y(-B) R_z(-C) and R_y(+B) R_z(+C)
    for k, r, _, _ in clean:
        neg = np.abs(r - ry(-k[1]) @ rz(-k[2])).max()
        pos = np.abs(r - ry(k[1]) @ rz(k[2])).max()
        print(f"  {k}: |R - Ry(-B)Rz(-C)| {neg:.5f}, |R - Ry(+B)Rz(+C)| {pos:.5f}")
    # one least-squares fit: unknowns s = b_x + b_z, c_x, c_y, and per tool z_T = b_z - b_x + L_T
    tools = sorted({k[0] for k, *_ in clean})
    rows, rhs = [], []
    for k, _, t, _ in clean:
        rb, rc = ry(-k[1]), rz(-k[2])
        cm = rb @ (np.eye(3) - rc)  # acts on c = (c_x, c_y, 0)
        ib = np.eye(3) - rb  # acts on b = (b_x, 0, b_z): x row b_x + b_z, z row b_z - b_x
        for i in range(3):
            row = np.zeros(3 + len(tools))
            if i == 0:
                row[0] = ib[0, 0]  # 1: b_x + b_z enters as s
            row[1], row[2] = cm[i, 0], cm[i, 1]
            if i == 2:
                row[3 + tools.index(k[0])] = 1.0
            rows.append(row)
            rhs.append(t[i])
    a, y = np.array(rows), np.array(rhs)
    sol, *_ = np.linalg.lstsq(a, y, rcond=None)
    res = y - a @ sol
    s, cx, cy = sol[:3]
    print(f"b_x + b_z = {s:.3f}, C axis at (c_x, c_y) = ({cx:.3f}, {cy:.3f}); "
          f"residual rms {np.sqrt(np.mean(res ** 2)):.3f} max {np.abs(res).max():.3f} mm")
    for i, tname in enumerate(tools):
        print(f"  b_z - b_x + L for {tname}: {sol[3 + i]:.3f}")
    print(f"if B and C intersect (b_x = c_x): B axis at z = {s - cx:.3f} (machine coordinates); "
          f"tool lengths then: " + ", ".join(
              f"{tname} {sol[3 + i] - (s - cx) + cx:.1f}" for i, tname in enumerate(tools)))


if __name__ == "__main__":
    main(sys.argv[1])
