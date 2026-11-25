"""Verification probe. Run with KiCad 9.0.2 pcbnew Python bindings and board file paths."""
from collections import defaultdict
from hashlib import sha256
from math import ceil
from pathlib import Path
import sys
import os
import pcbnew as pcb

GRID = pcb.FromMM(2.0)
MOVE = pcb.FromMM(0.25)
CLEAR = pcb.FromMM(0.20)


def cells(box):
    x0, y0, x1, y1 = box
    for x in range(x0 // GRID, x1 // GRID + 1):
        for y in range(y0 // GRID, y1 // GRID + 1):
            yield x, y


def overlaps(a, b):
    return a[0] <= b[2] and a[2] >= b[0] and a[1] <= b[3] and a[3] >= b[1]


def probe(path):
    board = pcb.LoadBoard(str(path))
    boxes, vias, index = [], [], defaultdict(list)
    for obj in [*board.GetTracks(), *board.GetPads()]:
        if not obj.IsOnLayer(pcb.F_Cu):
            continue
        rect = obj.GetBoundingBox()
        box = rect.GetLeft(), rect.GetTop(), rect.GetRight(), rect.GetBottom()
        key = len(boxes)
        if isinstance(obj, pcb.PCB_VIA):
            vias.append(key)
        boxes.append(box)
        for cell in cells(box):
            index[cell].append(key)
    if dest := os.getenv("VIA_FIXTURES"):
        rows = []
        for obj in board.GetTracks():
            if isinstance(obj, pcb.PCB_VIA) and obj.IsOnLayer(pcb.F_Cu) and obj.GetNetCode() > 0:
                pt = obj.GetPosition()
                rows.append((pt.x, pt.y, obj.GetWidth(), obj.GetNetCode()))
        Path(dest, path.stem + ".tsv").write_text("".join(
            f"{x} {y} {diam} {net}\n" for x, y, diam, net in sorted(rows)))
    counts = []
    for key in vias:
        x0, y0, x1, y1 = boxes[key]
        query = x0 - CLEAR, y0 - CLEAR, x1 + MOVE + CLEAR, y1 + CLEAR
        hits = {i for cell in cells(query) for i in index[cell]
                if i != key and overlaps(query, boxes[i])}
        full = {i for i, other in enumerate(boxes) if i != key and overlaps(query, other)}
        assert hits == full
        counts.append(len(hits))
    counts.sort()
    print(path.name, sha256(path.read_bytes()).hexdigest(), 'items', len(boxes),
          'vias', len(vias), 'median', counts[ceil(len(counts) / 2) - 1],
          'p95', counts[ceil(len(counts) * .95) - 1], 'max', max(counts))


for arg in sys.argv[1:]:
    probe(Path(arg))
