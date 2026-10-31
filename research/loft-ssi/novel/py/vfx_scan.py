# Zero set of the exact F(s, sigma) for the volute vs the core shifted 8 up (vfx.fc): sign changes along
# sigma at fixed s, lifted to 3D and checked on both surfaces.
import numpy as np
from loft import Loft, volute_profiles, VOLUTE
from pencil import F, lift
CUP = Loft([(c + np.array([0, 0, 8.0]), x, y, r) for c, x, y, r in volute_profiles([11.67, 13.89, 16.11, 18.33, 20.56, 22.78, 25, 28.5, 32, 33])])
def on(L, p, s0):
    # distance from p to loft L by brute force near s0
    best = 1e9
    for s in np.linspace(s0-3, s0+3, 61):
        for a in np.linspace(0, 2*np.pi, 721):
            best = min(best, np.linalg.norm(L.point(s, a) - p))
    return best
if __name__ == '__main__':
    for s in [30.0, 74.83324003, 100.0, 300.0]:
        sig = np.linspace(s-20, s+20, 4001)
        f = np.array([F(VOLUTE, s, CUP, t) for t in sig])
        idx = np.where(np.sign(f[:-1]) != np.sign(f[1:]))[0]
        print(f's={s:.3f}: |F| range {np.abs(f).min():.3e}..{np.abs(f).max():.3e}, sign changes at sigma',
              [round(sig[i], 3) for i in idx])
        for i in idx[:4]:
            p, a1, a2 = lift(VOLUTE, s, CUP, sig[i])
            print('   lift', np.round(p, 3), 'angles', round(a1, 3), round(a2, 3))
