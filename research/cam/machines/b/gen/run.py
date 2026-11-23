import sys, importlib
from gen import write_vendor
ROOT = "/home/user/forgecad"
import m_batch2
B2 = {"dn-solutions": ("DN", "DN Solutions (formerly Doosan Machine Tools) machines"),
      "hyundai-wia": ("WIA", "Hyundai WIA machines"), "makino": ("MAKINO", "Makino machines"),
      "matsuura": ("MATSUURA", "Matsuura machines"), "kitamura": ("KITAMURA", "Kitamura machines"),
      "okuma": ("OKUMA", "Okuma machines"), "dmg-mori": ("DMG", "DMG MORI machines"),
      "hermle": ("HERMLE", "Hermle machines"), "romi": ("ROMI", "Romi machines"),
      "bridgeport": ("BRIDGEPORT", "Bridgeport (Hardinge group) machines"), "spinner": ("SPINNER", "Spinner machines")}
if len(sys.argv) > 1 and sys.argv[1] == "b2":
    for v, (attr, t) in B2.items():
        ms = getattr(m_batch2, attr)
        write_vendor(ROOT, v, t, ms)
        print(v, len(ms))
    sys.exit(0)
VENDORS = {  # module: (vendor slug, title)
    "m_brother": ("brother", "Brother SPEEDIO machines"),
    "m_mazak": ("mazak", "Mazak machines"),
    "m_fanuc": ("fanuc", "FANUC ROBODRILL machines"),
}
for mod in sys.argv[1:] or VENDORS:
    v, t = VENDORS[mod]
    ms = importlib.import_module(mod).MACHINES
    write_vendor(ROOT, v, t, ms)
    print(mod, len(ms))
