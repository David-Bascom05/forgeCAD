"""Verify the fan carrier against the released copper and drills. Pass the pcb-fab ZIP.
Uses the same verification-only Gerbonara and Shapely environment as fabrication_readback.py.
"""
import csv
import io
import json
import sys
from zipfile import ZipFile

from fabrication_readback import ExcellonFile, GerberFile, geometry, recover


def check(path):
    with ZipFile(path) as archive:
        def read(name):
            return archive.read(name).decode()

        plots = {n: GerberFile.from_string(read(n)) for n in archive.namelist() if n.startswith('copper-L')}
        assert len(plots) == 4
        assert not json.loads(read('review.json'))['drc']['findings']
        placements = list(csv.DictReader(io.StringIO(read('placements.csv'))))
        assert len(placements) == 47
        plated, nonplated = ([geometry(p) for p in ExcellonFile.from_string(read(name)).objects]
                             for name in ('plated.drl', 'nonplated.drl'))
        assert len(nonplated) == 4
    _, _, labels, at = recover(plots, plated, nonplated)

    # Sample a remaining copper land beside each bore. Centre samples work for SMT.
    def top(x, y):
        return at(0, x, y)

    gnd = top(74.8 + 1.4, 60.38)
    v12 = top(74.8 + 1.4, 75.62)
    primary = top(8 + 1.2, 68)
    vin = top(8 + 1.2, 75.62)
    assert primary == top(24 + 1, 68) == top(24 + 1, 60.38) == top(10 + .8, 86)
    assert vin == top(24 + 1, 75.62) == top(10 + .8, 93.5)
    for y in (72, 78.8):
        assert vin == top(15.05, y) and primary == top(17.95, y)
    assert v12 == top(91 + .8, 74.5) and gnd == top(91 + .8, 79.5)
    assert gnd == top(83.45, 67) == top(82.95, 62)
    assert v12 == top(80.55, 67) == top(81.05, 62)
    assert v12 == top(13 + .7, 34) == top(12.05, 29)
    assert gnd == top(15.54 + .7, 34) == top(13.95, 29) == top(21.95, 29)
    v5 = top(18.08 + .7, 34)
    assert v5 == top(20.05, 29)

    def header(column, upper=False):
        return top(8 + 2.54 * column + .7, 23.24 if upper else 8)

    v33 = header(14)
    assert v5 == header(0, True)
    assert gnd == header(0) == header(1, True) == header(14, True)
    assert v33 == header(2, True) == top(75.3, 12.365) == top(69.05, 11)
    assert gnd == top(75.3, 13.635) == top(80.7, 11.095) == top(80.7, 14.905) == top(70.95, 11)
    can_tx = top(75.3, 14.905)
    can_rx = top(75.3, 11.095)
    assert can_tx == header(22) and can_rx == header(21)
    can_h = top(86.16 + .9, 5)
    can_l = top(81.08 + .9, 5)
    term = top(93.54 + .7, 14)
    assert can_h == top(80.7, 13.635) == top(91 + .7, 14)
    assert can_l == top(80.7, 12.365) == top(92.95, 20)
    assert term == top(91.05, 20) and gnd == top(76 + .9, 5)
    nets = [gnd, v12, primary, vin, v5, v33, can_tx, can_rx, can_h, can_l, term]
    pwm_gpio = [2, 3, 4, 5, 6, 9]
    for i, pin in enumerate(pwm_gpio):
        y = 16 + i * 15
        feed = top(143 + .7, y)
        tach = top(143 + .7, y - 2.54)
        pwm = top(143 + .7, y - 5.08)
        gate = header(pin + 1)
        assert gnd == top(143 + .7, y + 2.54) == top(133, y - 6.95) == top(129.95, y - 6)
        assert feed == top(133.55, y) and v12 == top(128.45, y)
        assert tach == header(12 - i, True) == top(141.95, y - 8.5)
        assert pwm == top(135, y - 6) and v33 == top(140.05, y - 8.5)
        assert gate == top(133, y - 5.05) == top(128.05, y - 6)
        nets.extend([feed, tach, pwm, gate])
    assert len(set(nets)) == 35
    used_lower = {0, 14, 21, 22} | {p + 1 for p in pwm_gpio}
    used_upper = {0, 1, 2, 14} | {12 - i for i in range(6)}
    unused = [header(i, upper) for upper, used in [(False, used_lower), (True, used_upper)]
              for i in range(24) if i not in used]
    unused.append(top(74.8 + .9, 68))  # U1 trim floats at the nominal 12 V setting.
    assert len(set(nets + unused)) == len(nets) + len(unused) == len(set(labels))
    print(f'35 independent working nets, {len(unused)} isolated unused pins, '
          f'{len(plated)} plated and four nonplated bores, 47 placements')


if __name__ == '__main__':
    check(sys.argv[1])
