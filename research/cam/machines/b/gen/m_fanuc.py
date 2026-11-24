from common import *
import arch

def robodrill(model, X, tx, kg, srcs):
    p = dict(name=f"fanuc-robodrill-{model}", travel=(X, 400, 330), table=(tx, 400 if tx == 650 else 410), nose=(150, 480),
             col=205 + 200 + 60, rapid=(mm_min(54), mm_min(54), mm_min(54)), feed=mm_min(30),
             spindle=dict(rpm_max=10000, iface="BT30 (BBT30 dual contact)"), maxtool=(250, 80),
             changer=dict(pockets=21, change_s=1.6, kg=3.0, c_change_s="cut to cut with a 2 kg tool, published (tool to tool 0.9)"),
             nose_r=80.0, head_h=400.0, col_d=400.0)
    m = arch.vmc_cframe(p)
    m.update(
        name=p["name"], control="fanuc", coolants=["flood", "through"],
        source=" | ".join(srcs), taken=TAKEN,
        header=[f"FANUC ROBODRILL {model.replace('a-', 'alpha-').upper()} Plus: BT30 drill/tap/mill centre, FANUC 31i-B5 Plus.",
                f"X {X} Y 400 Z 330, table {tx} wide (300 kg), 10,000 rpm (14.2 kW 1-min, 4 kW cont, 80 N.m max),",
                "21 tools, cut to cut 1.6 s."],
        note=note(
            "architecture assumed: X/Y table, Z head on a fixed column (FANUC's pages do not say which unit carries which axis: UNKNOWN)",
            "spindle centre to column UNKNOWN: put at the least that clears the table at full Y (estimate)",
            "nose to table 150-480 is the standard column (the high column HC100 option gives 250-580)",
            "spindle power published as ratings only (1-min 14.2 kW, cont 4.0 kW, 80 N.m max, 13.6 N.m cont): no curve, no power check",
            "24,000 rpm spindle and the ADV variant (cut to cut 1.3 s) are options",
            "accel UNKNOWN; overall W/D/H only in a drawing not read (UNKNOWN); machine " + str(kg) + " kg",
            CONV_VMC, ENV),
    )
    return m

MACHINES = [
    robodrill("a-d21mib5", 500, 650, 2000, ["https://www.fanucamerica.com/products/robodrill/a-d21mib5-plus",
        "https://www.fanucamerica.com/uploads/files/data-sheets/robodrill-a-d21mib5-plus.pdf",
        "https://www.fanucamerica.com/uploads/files/data-sheets/robodrill-alphadib-plus-series-brochure.pdf"]),
    robodrill("a-d21lib5", 700, 850, 2100, ["https://www.fanucamerica.com/products/robodrill/a-d21lib5-plus",
        "https://www.fanucamerica.com/uploads/files/data-sheets/robodrill-alphadib-plus-series-brochure.pdf"]),
]
