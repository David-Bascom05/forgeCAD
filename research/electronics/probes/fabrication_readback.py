"""Check the fabrication-controller ZIP independently. Verification-only: Gerbonara and Shapely.

Usage: python fabrication_readback.py controller.zip
"""
import csv
import io
import json
import sys
import warnings
from zipfile import ZipFile

from gerbonara import ExcellonFile, GerberFile
from gerbonara.utils import MM
from shapely.geometry import LineString, Point, Polygon
from shapely.ops import unary_union


def geometry(obj):
    pieces = []
    for prim in obj.to_primitives(MM):
        assert prim.polarity_dark
        if hasattr(prim, 'outline'):
            assert not any(prim.arc_centers)
            pieces.append(Polygon(prim.outline))
        elif hasattr(prim, 'x1'):
            pieces.append(LineString([(prim.x1, prim.y1), (prim.x2, prim.y2)]).buffer(prim.width / 2))
        else:
            pieces.append(Point(prim.x, prim.y).buffer(prim.r))
    return unary_union(pieces)


def check(archive):
    warnings.simplefilter('error', SyntaxWarning)
    with ZipFile(archive) as files:
        plots = {n: GerberFile.from_string(files.read(n).decode()) for n in files.namelist() if n.endswith('.gbr')}
        drills = {n: ExcellonFile.from_string(files.read(n).decode()) for n in files.namelist() if n.endswith('.drl')}
        assert len(plots) == 11 and len(drills) == 2
        assert json.loads(files.read('review.json'))['drc']['findings'] == []
        positions = list(csv.DictReader(io.StringIO(files.read('placements.csv').decode())))
        assert len(positions) == 3 and sum(p['Side'] == 'bottom' for p in positions) == 1

    def image(name):
        return unary_union([geometry(o) for o in plots[name].objects])

    for side, code in [('top', 'Top'), ('bottom', 'Bot')]:
        for film, function in [('mask', 'Soldermask'), ('paste', 'Paste'), ('silk', 'Legend')]:
            attrs = plots[f'{side}-{film}.gbr'].file_attrs
            assert attrs['.FileFunction'] == (function, code)
            assert attrs['.FilePolarity'] == (('Negative' if film == 'mask' else 'Positive'),)
    assert image('top-mask.gbr').covers(Point(18, 6))
    assert not image('bottom-mask.gbr').covers(Point(18, 6))
    assert not image('top-mask.gbr').covers(Point(18, 10))
    assert image('top-paste.gbr').bounds == (11.35, 5.55, 12.65, 10.45)
    assert not image('top-paste.gbr').covers(Point(4, 6))
    assert image('bottom-paste.gbr').covers(Point(24, 6))
    assert not image('top-silk.gbr').intersects(image('top-mask.gbr'))
    assert not image('bottom-silk.gbr').intersects(image('bottom-mask.gbr'))
    assert len(plots['profile.gbr'].objects) == 8
    assert plots['profile.gbr'].file_attrs['.FileFunction'] == ('Profile', 'NP')
    plated = [geometry(o) for o in drills['plated.drl'].objects]
    assert len(plated) == 4
    slots = [geometry(o) for o in drills['nonplated.drl'].objects]
    assert len(slots) == 1 and slots[0].bounds == (1.2, 1.2, 4.8, 2.8)

    # Recover conducting islands from copper after drilling. Join layers only at PTH walls.
    holes = unary_union(plated + slots)
    islands, layers = [], []
    for n in sorted(n for n in plots if n.startswith('copper-L')):
        copper = image(n).difference(holes)
        layer = list(copper.geoms) if copper.geom_type == 'MultiPolygon' else [copper]
        layers.append(range(len(islands), len(islands) + len(layer)))
        islands.extend(layer)
    labels = list(range(len(islands)))
    for hole in plated:
        contacts = {labels[i] for i, island in enumerate(islands) if island.distance(hole.boundary) < 1e-6}
        assert contacts
        labels = [min(contacts) if label in contacts else label for label in labels]

    def net(layer, x, y):
        return next(labels[i] for i in layers[layer] if islands[i].covers(Point(x, y)))

    for y in (6, 10):
        assert net(0, 4.65, y) == net(0, 12, y) == net(3, 24, y)
    assert net(0, 12, 6) != net(0, 12, 10)
    assert net(1, 14, 8) == net(0, 12, 10)
    assert not any(islands[i].covers(Point(18.45, 6)) for i in layers[1])
    assert len(set(labels)) == 2
    print('11 Gerbers, 2 drill files, 3 placements, 2 independently recovered nets')


if __name__ == '__main__':
    check(sys.argv[1])
