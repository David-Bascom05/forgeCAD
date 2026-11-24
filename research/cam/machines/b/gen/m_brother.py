from common import *
import arch

SRC_S = "https://machinetool.global.brother/en-eu/sx2/spec/index.aspx"

def speedio_s(model, X, tx, W, D, H, kg_machine):
    p = dict(name=f"brother-speedio-{model}", travel=(X, 400, 300), table=(tx, 400), nose=(180, 480),
             col=200 + 200 + 60, rapid=(mm_min(50), mm_min(50), mm_min(56)), feed=mm_min(30),
             accel=[None, None, 2.2 * G],
             spindle=dict(rpm_min=10, rpm_max=10000, iface="BT30"), maxtool=(250, 110),
             changer=dict(pockets=14, change_s=1.3, kg=3.0, c_change_s="chip to chip, published (tool to tool 0.7)"),
             overall=(W, D, H), nose_r=80.0, head_h=400.0, col_d=400.0)
    m = arch.vmc_cframe(p)
    m.update(
        name=p["name"], control="brother", coolants=["flood", "through"],
        source=SRC_S, taken=TAKEN,
        header=[f"Brother SPEEDIO {model.upper()}: BT30 compact drill/tap/mill centre (CNC-C00). Discontinued; successor",
                f"S{model[1:4]}Xd2. X {X} Y 400 Z 300, table {tx} x 400 (250 kg), 10,000 rpm (10-min 10.1 kW, cont 7.1 kW,",
                "40 N.m max), 14 tools (21 optional), chip to chip 1.3 s."],
        note=note(
            "architecture assumed: X table on a Y saddle, Z head on a fixed column (Brother's page does not say which unit carries which axis: UNKNOWN)",
            "spindle centre to column UNKNOWN: put at the least that clears the table at full Y (460, estimate)",
            "spindle power published as ratings only (10-min 10.1 kW, continuous 7.1 kW, 40 N.m max), no curve: no power check",
            "accel published for Z only (2.2 G); X/Y UNKNOWN",
            "max cutting feed 30 m/min only in high accuracy mode B; through-spindle coolant optional",
            f"overall W x D = required floor space {W} x {D}, H {H}; machine {kg_machine} kg",
            CONV_VMC, ENV, "the control's dialect (brother) is not in forge-post yet"),
    )
    return m

MACHINES = [
    speedio_s("s300x2", 300, 600, 1080, 2463, 2497, 2200),
    speedio_s("s500x2", 500, 600, 1560, 2223, 2497, 2250),
    speedio_s("s700x2", 700, 800, 2050, 2223, 2497, 2400),
]

SRC_M140 = "https://machinetool.global.brother/en-us/all-products/m140x2"
SRC_M200 = "https://machinetool.global.brother/en-eu/mx3/spec/index.aspx"

def speedio_m(model, src, c_motor_kw, kg_machine, tt, cc, W, D, H):
    p = dict(name=f"brother-speedio-{model}", travel=(200, 440, 305), tilt=("A", [-30, 120]),
             pivot_below_top=50.0, table_d=140.0, nose=(80.0, 385.0),
             rapid=(mm_min(50), mm_min(50), mm_min(50)), feed=mm_min(30),
             rapid_tilt=rpm_to_deg_min(60), rapid_c=rpm_to_deg_min(200),
             spindle=dict(rpm_min=10, rpm_max=10000, iface="BT30"), maxtool=(200, 80),
             changer=dict(pockets=22, change_s=cc, kg=3.0, c_change_s=f"chip to chip, published (tool to tool {tt})"),
             nose_r=80.0, head_h=400.0, table_t=60.0, cradle_t=80.0, cradle_side=40.0)
    m = arch.trunnion(p)
    nose_estimate(m, "spindle nose to table at A = 0 not published: 80..385 (estimate, note)")
    m.update(
        name=p["name"], control="brother", coolants=["flood", "through"], source=src, taken=TAKEN,
        header=[f"Brother SPEEDIO {model.upper()}: BT30 mill-turn (discontinued). A tilts the trunnion about X",
                "(-30..+120 deg, roller gear cam) and carries C, a direct-drive turning spindle on a 140 mm table",
                f"(2,000 rpm, 55 N.m, {c_motor_kw} kW motor); turning uses a stationary tool locked in the milling spindle."],
        note=note(
            "C is modelled as a rotary joint only: its turning (2,000 rpm, 55 N.m, 8.7 kW max) is a format gap (a turning spindle on a table rotary of a milling machine; mill-turn requires a diameter X), FORMAT-GAPS.md",
            "spindle nose to table top at A = 0 UNKNOWN: put at 80-385 (estimate); A's pivot 50 mm below the table top (estimate)",
            "architecture of X/Y/Z UNKNOWN (Brother does not say which unit carries which); the tool-to-part motion is the same either way",
            "spindle power published as ratings (10-min 10.1 kW, 40 N.m max): no curve, no power check",
            "table side load 40 kg, tail side 11 kg; max cutting feed 30 m/min only in high accuracy mode B",
            f"overall W x D = required floor space {W} x {D}, H {H}; machine {kg_machine} kg",
            CONV_VMC, ENV, "the control's dialect (brother) is not in forge-post yet"),
    )
    return m

MACHINES += [
    speedio_m("m140x2", SRC_M140, 4.2, 2712, 0.9, 1.4, 1280, 3829, 2603),
    speedio_m("m200x3", SRC_M200, 3.6, 2750, 0.8, 1.5, 1280, 3862, 2603),
]
