"""First-order CCM loop estimate for the LM5146 in the Rev B six-fan example.

Use the Type III feedback network and the feed-forward modulator gain from the
LM5146 data sheet. Include a half-cycle sample delay. This approximation omits
the amplifier's finite bandwidth and switching parasitics.
"""
import cmath
import itertools
import math


def parallel(a, b):
    return a * b / (a + b)


def loop(frequency, capacitance, load_amps, esr, inductance):
    s = 2j * math.pi * frequency
    output = parallel(12 / load_amps, esr + 1 / (s * capacitance))
    plant = 15 * output / (s * inductance + 0.007 + output)
    feedback = parallel(4700 + 1 / (s * 22e-9), 1 / (s * 120e-12))
    input_z = parallel(21000, 680 + 1 / (s * 2.2e-9))
    return plant * feedback / input_z * cmath.exp(-s / (2 * 250000))


def crossover(corner):
    last = 100
    for step in range(1, 1001):
        hz = 100 * 1000 ** (step / 1000)
        if abs(loop(hz, *corner)) < 1 <= abs(loop(last, *corner)):
            return hz, 180 + math.degrees(cmath.phase(loop(hz, *corner)))
        last = hz
    raise ValueError(f'No crossover found for {corner}')


if __name__ == '__main__':
    corners = itertools.product((190e-6, 300e-6), (3, 12), (.004, .012), (6.56e-6, 9.84e-6))
    crosses, margins = zip(*(crossover(corner) for corner in corners))
    print(f'{len(crosses)} CCM corners, crossover {min(crosses)/1000:.1f}–{max(crosses)/1000:.1f} kHz, '
          f'phase margin {min(margins):.1f}–{max(margins):.1f} degrees')
