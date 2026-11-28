"""Recover pad connectivity on the four-layer fan controller using exported artwork.

This verification tool uses Gerbonara and Shapely 2.x.
"""
import argparse
import json
from collections import defaultdict
from pathlib import Path
from zipfile import ZipFile

from gerbonara import ExcellonFile, GerberFile
from gerbonara.utils import MM
from shapely import LineString, Point, Polygon, STRtree, union_all


def shape(object_):
    pieces = []
    for primitive in object_.to_primitives(MM):
        if not primitive.polarity_dark:
            raise ValueError("Expected positive copper or drill geometry")
        if hasattr(primitive, "outline"):
            if any(primitive.arc_centers):
                raise ValueError("Expected linear region boundaries")
            pieces.append(Polygon(primitive.outline))
        elif hasattr(primitive, "x1"):
            line = LineString([(primitive.x1, primitive.y1), (primitive.x2, primitive.y2)])
            pieces.append(line.buffer(primitive.width / 2))
        else:
            pieces.append(Point(primitive.x, primitive.y).buffer(primitive.r))
    return union_all(pieces)


def inspect(archive, pads):
    def drills(filename):
        program = ExcellonFile.from_string(archive.read(filename).decode())
        return [shape(obj) for obj in program.objects]

    plated = drills("plated.drl")
    mounting = drills("nonplated.drl")
    holes = union_all(plated + mounting)
    copper_files = sorted(f for f in archive.namelist() if f.startswith("copper-L") and f.endswith(".gbr"))
    if len(copper_files) != 4:
        raise ValueError("Expected four copper layers")

    islands, layer_numbers, indices = [], [], []
    for layer, filename in enumerate(copper_files, start=1):
        gerber = GerberFile.from_string(archive.read(filename).decode())
        copper = union_all([shape(obj) for obj in gerber.objects]).difference(holes)
        polygons = list(copper.geoms) if copper.geom_type == "MultiPolygon" else [copper]
        indices.append((STRtree(polygons), len(islands)))
        islands.extend(polygons)
        layer_numbers.extend([layer] * len(polygons))

    parent = list(range(len(islands)))

    def root(index):
        while index != parent[index]:
            parent[index] = parent[parent[index]]
            index = parent[index]
        return index

    # Only plated bore walls join distinct layers. Spatial indices are layer-local.
    for bore in plated:
        contacts = []
        for tree, offset in indices:
            hits = tree.query(bore.boundary, predicate="dwithin", distance=0.000002)
            contacts.extend(offset + int(hit) for hit in hits)
        if contacts:
            anchor = root(contacts[0])
            for island in contacts[1:]:
                parent[root(island)] = anchor

    def roots_at(x, y):
        hits = indices[0][0].query(Point(x, y), predicate="intersects")
        return {root(int(hit)) for hit in hits}

    by_net = defaultdict(lambda: defaultdict(list))
    nets_on_island = defaultdict(set)
    failures = []
    for pad in pads:
        x, y = pad["at_mm"]
        drill = pad["drill_mm"]
        copper = pad["copper"]
        if drill:
            # Sample the annulus rather than the empty bore.
            contacts = roots_at(x + (copper["width_mm"] + drill) / 4, y)
        else:
            # Multiple interior samples handle vias placed in SMD lands.
            xs, ys = zip(*copper["points_mm"])
            width, height = max(xs) - min(xs), max(ys) - min(ys)
            contacts = set()
            for dx in (-0.38, 0.0, 0.38):
                for dy in (-0.38, 0.0, 0.38):
                    contacts.update(roots_at(x + dx * width, y + dy * height))
        if len(contacts) != 1:
            failures.append(pad["id"])
            continue
        island = contacts.pop()
        by_net[pad["net"]][island].append(pad["id"])
        nets_on_island[island].add(pad["net"])

    floating = []
    for index, island in enumerate(islands):
        if root(index) not in nets_on_island:
            floating.append({
                "layer": layer_numbers[index],
                "area_mm2": round(island.area, 4),
                "bounds_mm": [round(value, 3) for value in island.bounds],
            })
    return {
        "pads": len(pads),
        "pad_sampling_failures": failures,
        "nets": len(by_net),
        "plated_bores": len(plated),
        "mounting_bores": len(mounting),
        "opens": {net: list(groups.values()) for net, groups in by_net.items() if len(groups) > 1},
        "shorts": [sorted(nets) for nets in nets_on_island.values() if len(nets) > 1],
        "copper_islands_without_pads": len({root(i) for i in range(len(islands))} - nets_on_island.keys()),
        "floating_copper": floating,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fabrication_zip", type=Path)
    parser.add_argument("pad_manifest", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    pads = json.loads(args.pad_manifest.read_text())["result"]
    with ZipFile(args.fabrication_zip) as archive:
        report = inspect(archive, pads)
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
