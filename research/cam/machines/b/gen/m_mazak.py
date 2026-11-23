from common import *
import arch

def vcn(model, src, X, Y, Z, tx, ty, nose, rpm, iface, power, maxtool, changer, rapid, overall, kg, extra=""):
    p = dict(name=f"mazak-{model}", travel=(X, Y, Z), table=(tx, ty), nose=nose, col=ty / 2 + Y / 2 + 60,
             rapid=(rapid, rapid, rapid), feed=rapid, spindle=dict(rpm_max=rpm, iface=iface), maxtool=maxtool,
             changer=changer, overall=overall, nose_r=110.0)
    m = arch.vmc_cframe(p)
    for a in "XYZ":
        mark(m, a, "max_feed", UNK_FEED)
    m.update(name=p["name"], control="mazak-eia", coolants=["flood"], source=src, taken=TAKEN,
        header=[f"Mazak {model.upper()}: 40-taper VMC, MAZATROL SmoothG. X {X} Y {Y} Z {Z}, table {tx} x {ty},",
                f"{rpm} rpm {iface}, {changer['pockets']} tools."],
        note=note(extra,
            "architecture assumed: fixed column, X table on a Y saddle (Mazak's page does not say: UNKNOWN)",
            "spindle centre to column UNKNOWN: put at the least that clears the table at full Y (estimate)",
            "spindle power published as ratings only (" + power + "): no curve, no power check",
            "tool change time UNKNOWN", VACUOUS, f"overall {overall[0]} x {overall[1]} x {overall[2]}, {kg} kg",
            CONV_VMC, ENV,
            "the control also runs MAZATROL conversational; posted EIA/ISO G-code is the dialect (mazak-eia: not in forge-post yet)"))
    return m

def ch(pockets, kg):
    return dict(pockets=pockets, change_s=0.001, kg=kg, c_change_s=UNK_CHANGE)

MACHINES = [
    vcn("vcn-530c-l", "https://www.mazak.com/my-en/detail/china/vertical/lg-vn530cel/ | https://mazak.com/id-en/products/vcn",
        1050, 530, 510, 1300, 550, (150, 660), 12000, "BT40", "25% ED 18.5 kW, cont 7.5 kW",
        (350, 80), ch(30, 8), mm_min(42), (2950, 2973, 2808), 6800,
        "the China-built VCN-530C L: the only VCN-530C page Mazak still publishes (the 530C/530C HS are no longer listed)"),
    vcn("vcn-700", "https://www.mazak.com/uk-en/detail/uk/vertical/uk-vcn700/ | https://www.mazak.com/uk-en/products/vcn/",
        1300, 700, 650, 1500, 700, (150, 800), 15000, "CAT40 (spec page; the product page says BT40)",
        "10% ED 22 kW, cont 11 kW, 119.4 N.m max, 57.3 N.m cont", (350, 80), ch(30, 8), mm_min(42), (3400, 3901, 3050), 8500,
        "UK-built VCN-700; table load 1,800 kg"),
]

def variaxis(model, src, X, Y, Z, d, nose, maxlen, overall, kg):
    p = dict(name=f"mazak-variaxis-{model}", travel=(X, Y, Z), tilt=("A", [-120, 30]), pivot_below_top=150.0,
             table_d=d, nose=nose, rapid=(mm_min(60), mm_min(60), mm_min(56)), feed=mm_min(60),
             rapid_tilt=VAC_ROT, rapid_c=VAC_ROT, spindle=dict(rpm_max=18000, iface="CAT40"),
             maxtool=(maxlen, 101.6), changer=ch(30, 8.2), nose_r=110.0, cradle_side=80.0)
    m = arch.trunnion(p)
    for a in "XYZ":
        mark(m, a, "max_feed", UNK_FEED)
    for a in "AC":
        mark(m, a, "rapid", UNK_RAPID)
    m.update(name=p["name"], control="mazak-eia", coolants=["flood", "through"], source=src, taken=TAKEN,
        header=[f"Mazak VARIAXIS {model.upper()} NEO: 5-axis trunnion, MAZATROL SmoothAi. A tilts the cradle about X",
                f"(-120..+30 deg) and carries the C table ({d:g} dia, {'500' if d < 650 else '700'} kg). X {X:g} Y {Y:g} Z {Z:g}, 18,000 rpm CAT40."],
        note=note(
            "A's sign: Mazak publishes -120..+30; the direction of +A (which way the table tilts) is UNKNOWN; modelled right-handed about +X",
            "A's pivot below the table top UNKNOWN: 150 mm (estimate); C range and A/C speeds UNKNOWN",
            "architecture of X/Y/Z UNKNOWN (Mazak does not say which unit carries which); the tool-to-part motion is the same either way",
            "the spec page says CAT40 18,000 rpm 30 kW (40% ED) / 26 kW cont, 120 N.m max; the product summary says BT40 12,000 rpm: the spec page is used",
            "spindle power as ratings only: no curve, no power check; tool change time UNKNOWN; 2-pallet changer optional",
            "C's speed and turning: none (milling only)", VACUOUS,
            f"overall {overall[0]} x {overall[1]} x {overall[2]}" + (f", {kg} kg" if kg else " (the page's weight is a misprint)"),
            CONV_VMC, ENV, "mazak-eia is not in forge-post yet"))
    return m

MACHINES += [
    variaxis("i-600", "https://www.mazak.com/us-en/detail/japan/5axis/vri600nn/ | https://www.mazak.com/us-en/products/variaxis-i-neo/",
             508, 914.4, 508, 600, (76.2, 584.2), 304.8, (2235.2, 4445, 3225.8), 13000),
    variaxis("i-700", "https://www.mazak.com/us-en/detail/japan/5axis/vri700nn/ | https://www.mazak.com/us-en/products/variaxis-i-neo/",
             635, 1092.2, 609.6, 700, (101.6, 711.2), 355.6, (2387.6, 4445, 3454.4), None),
]

def hcn5000():
    p = dict(name="mazak-hcn-5000", travel=(736.6, 736.6, 812.8), pallet=500.0,
             spindle_above_pallet=(101.6, 838.2), nose_to_center=(100.0, 912.8),
             rapid=(mm_min(60), mm_min(60), mm_min(60)), feed=mm_min(60), rapid_b=VAC_ROT,
             spindle=dict(rpm_max=12000, iface="CAT40"), maxtool=(508, 101.6), changer=ch(40, 11.8),
             nose_r=120.0)
    m = arch.hmc(p)
    nose_estimate(m, "spindle nose to pallet centre not published: 100 at the end of Z (estimate)")
    for a in "XYZ":
        mark(m, a, "max_feed", UNK_FEED)
    mark(m, "B", "rapid", UNK_RAPID)
    m.update(name=p["name"], control="mazak-eia", coolants=["flood", "through"],
        source="https://www.mazak.com/us-en/detail/japan/horizontal/hn5000e/ | https://www.mazak.com/us-en/products/hcn/", taken=TAKEN,
        header=["Mazak HCN-5000: HMC, MAZATROL SmoothG, two 500 mm pallets. X 736.6 Y 736.6 Z 812.8, spindle 101.6-838.2",
                "above the pallet, 12,000 rpm CAT40 (40% ED 18.5 kW, cont 15 kW, 82 N.m), 40 tools, max work 800 dia x 1000."],
        note=note(
            "two-pallet changer: change time, pallet load and B index increment UNKNOWN; the APC is not in the format (FORMAT-GAPS B1): the pallet at the spindle is the part mount, kind vmc4",
            "spindle nose to pallet centre UNKNOWN: put at 100-912.8 (estimate, the Z stroke from 100)",
            "Z modelled on the head (X column, Y head, Z ram) because the format needs the length joint on the head side (FORMAT-GAPS B10); which unit Mazak moves on Z is UNKNOWN; the tool-to-part motion is the same either way",
            "B modelled as a full rotary; whether it indexes (1 deg) or contours is UNKNOWN",
            "power as ratings only: no curve, no power check", VACUOUS,
            "overall 2616.2 x 5765.8 x 2819.4, 11,430 kg",
            "joint 0 = mid X, Y 0 = spindle lowest (home at the top), Z 0 = spindle farthest from the pallet; machine zero UNKNOWN", ENV,
            "mazak-eia is not in forge-post yet"))
    return m

def integrex_i200():
    p = dict(name="mazak-integrex-i-200", stroke=(533.4, 203.2, 635.0), max_dia=609.6, max_len=584.2,
             chuck_d=8 * 25.4, b_range=[-30, 210], rapid=(mm_min(48), mm_min(40), mm_min(40)), feed=mm_min(48),
             rapid_b=VAC_ROT, rapid_c=VAC_ROT, swing=609.6,
             spindle=dict(rpm_max=5000, iface="(chuck 8 in)"), mill=dict(rpm_max=12000, iface="HSK-A63"),
             maxtool=(304.8, 101.6), changer=ch(36, 5.0))
    m = arch.mill_turn(p)
    for a in "XYZ":
        mark(m, a, "max_feed", UNK_FEED)
    for a in "BC":
        mark(m, a, "rapid", UNK_RAPID)
    m.update(name=p["name"], control="mazak-eia", coolants=["flood", "through"],
        source="https://www.mazak.com/us-en/detail/japan/integrex-h/i200nen/ | https://www.mazak.com/us-en/products/integrex-i-neo/", taken=TAKEN,
        header=["Mazak INTEGREX i-200 NEO: mill-turn, B-axis milling head (-30..+210 deg, 12,000 rpm HSK-A63, 40% ED",
                "18.5 kW), main spindle 5,000 rpm 8 in chuck (40% ED 22 kW, cont 15 kW), tailstock; X 533.4 Y 203.2 Z 635,",
                "max 609.6 dia x 584.2, 36 tools."],
        note=note(
            "the plain i-200 NEO has a tailstock, not a second spindle (the i-200S has one): no sub-spindle; the tailstock is not modelled",
            "B's zero and sign (which way +B turns the head) UNKNOWN: modelled with the tool along -x (radial) at B = 0, right-handed about +y",
            "C's speed and the chuck's length UNKNOWN; the head's size an estimate", CONV_LATHE,
            "power as ratings only: no curve, no power check", VACUOUS, "overall 3505.2 x 2743.2 x 2438.4, 10,780 kg", ENV,
            "mazak-eia (lathe) is not in forge-post yet"))
    return m

def qt200():
    p = dict(name="mazak-quick-turn-200", stroke=(203.2, 558.8), max_dia=355.6, max_len=508.0, chuck_d=8 * 25.4,
             swing=609.6, stations=12, rapid=(VAC_RAPID, VAC_RAPID), feed=VAC_RAPID,
             spindle=dict(rpm_max=5000, iface="(chuck 8 in, bar 76.2)"))
    m = arch.lathe(p)
    for a in "XZ":
        mark(m, a, "rapid", UNK_RAPID)
        mark(m, a, "max_feed", UNK_FEED)
    m["turrets"][0]["c_index_s"] = "UNKNOWN (unpublished): 0.001 s (must be > 0)"
    m.update(name=p["name"], control="mazak-eia-lathe", coolants=["flood"],
        source="https://www.mazak.com/us-en/detail/usa/turning/us-qt2002eky/ | https://www.mazak.com/us-en/products/quick-turn/", taken=TAKEN,
        header=["Mazak QUICK TURN 200 (US-built): 2-axis lathe, 8 in chuck, 76.2 mm bar, 5,000 rpm 26 kW (30-min),",
                "12-station turret; X 203.2 Z 558.8, swing 609.6, max 355.6 dia x 508."],
        note=note("rapids, feeds, turret index time, control model, tailstock, size and weight UNKNOWN (not on the page)",
            "the page prints a 21.13 in Y row on this non-Y model: not used", CONV_LATHE,
            "power as a 30-min rating only: no curve, no power check", VACUOUS, ENV,
            "the base frame is turned so x is the slide's X (the bed's slant is not modelled)",
            "mazak-eia-lathe is not in forge-post yet"))
    return m

MACHINES += [hcn5000(), integrex_i200(), qt200()]
