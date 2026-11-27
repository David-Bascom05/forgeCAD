"""Recover physical fan-controller nets from pcb-fab Gerbers and drills.

Usage: python fan_controller_readback.py board.zip board-pads.json
Create the second file with: forgecad pcb fan-controller.fg pads --json
Gerbonara and Shapely are verification-only dependencies.
"""
import json
import sys
from collections import defaultdict
from zipfile import ZipFile

from fabrication_readback import ExcellonFile, GerberFile, geometry, recover


def check(archive, manifest):
    with ZipFile(archive) as files:
        plots = {n: GerberFile.from_string(files.read(n).decode())
                 for n in files.namelist() if n.startswith('copper-L')}
        assert len(plots) == 4
        assert not json.loads(files.read('review.json'))['drc']['findings']
        def drills(name):
            return [geometry(p) for p in ExcellonFile.from_string(files.read(name).decode()).objects]
        plated, mounting = drills('plated.drl'), drills('nonplated.drl')
    assert len(mounting) == 4
    _, _, islands, at = recover(plots, plated, mounting)
    with open(manifest) as source:
        pads = json.load(source)['result']
    terminals = {}
    expected = defaultdict(set)
    for pad in pads:
        x, y = pad['at_mm']
        if pad['drill_mm']:
            # Between bore and land edge, on the plated annulus.
            x += (pad['copper']['width_mm'] + pad['drill_mm']) / 4
        if pad['drill_mm']:
            island = at(0, x, y)
        else:
            # Sampling several interior points also handles capped via-in-pad lands.
            xs, ys = zip(*pad['copper']['points_mm'])
            samples = set()
            for a in (-.38, 0, .38):
                for b in (-.38, 0, .38):
                    try:
                        samples.add(at(0, x+a*(max(xs)-min(xs)), y+b*(max(ys)-min(ys))))
                    except StopIteration:
                        continue
            assert len(samples) == 1, (pad['id'], samples)
            island = samples.pop()
        terminals[pad['id']] = island
        expected[pad['net']].add(island)
    for name, nets in expected.items():
        assert len(nets) == 1, ('open circuit', name, nets)
    assert len({next(iter(v)) for v in expected.values()}) == len(expected), 'shorted nets'
    assert set(terminals.values()) == set(islands), 'unassigned copper island'

    def joined(*names):
        assert len({terminals[p] for p in names}) == 1, names
        return terminals[names[0]]

    joined('J1.1', 'F1.1')
    joined('F1.2', 'D1.1', 'D2.1', 'D2.2')
    joined('D2.3', 'R1.1', 'U1.2')
    joined('R1.2', 'Q1.2', 'U1.1')
    joined('Q1.3', 'Q2.2', 'U101.2')
    joined('Q2.3', 'Q3.2', 'L1.1', 'U4.19')
    joined('Q3.3', 'R20.2', 'R26.2')
    joined('R20.1', 'J1.2', 'U2.7', 'J2.3')
    joined('U2.33', 'U3.4')
    joined('U2.34', 'U3.1')
    joined('J2.1', 'U3.7', 'JP1.1', 'D5.1')
    joined('J2.2', 'U3.6', 'R8.2', 'D5.2')
    branch_outputs = []
    for j in range(6):
        u, fan, fet = (f'{prefix}{11+j}.' for prefix in ('U', 'J', 'Q'))
        base = 220 + j*20
        joined('L1.2', u+'1', u+'2', u+'3', u+'6')
        branch_outputs.append(joined(u+'18', u+'19', u+'20', fan+'2'))
        joined(fan+'1', 'J1.2', u+'9', u+'21')
        joined(fan+'3', f'U2.{[38,39,40,41,43,44][j]}')
        joined(fan+'4', fet+'3')
        joined(fet+'1', f'U2.{[17,18,19,20,28,29][j]}')
        joined(u+'13', f'U2.{[27,30,31,32,37,42][j]}')
        joined(u+'15', f'U2.{[1,2,3,24,25,26][j]}')
        joined(u+'14', f'R{base+8}.2')
        joined(f'R{base+8}.1', f'R{base+1}.1', f'U2.{11+j}')
        joined(u+'11', f'R{base}.1')
        assert sum(v == terminals[u+'12'] for v in terminals.values()) == 1, 'MODE must float'
    assert len(set(branch_outputs + [terminals['L1.2']])) == 7
    print(f'{len(pads)} pads, {len(expected)} distinct nets, {len(plated)} plated bores, '
          f'six independent protected outputs')


if __name__ == '__main__':
    check(*sys.argv[1:])
