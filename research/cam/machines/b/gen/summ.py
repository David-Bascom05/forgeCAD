import json,sys,glob,os
for f in sys.argv[1:]:
    d=json.load(open(f))
    def g(*ks):
        x=d
        for k in ks:
            if not isinstance(x,dict): return None
            x=x.get(k)
        return x
    print("==",os.path.basename(f), d.get("kind"), "|", (d.get("architecture") or "")[:300])
    print("  src:", d.get("source"), "sec:", d.get("source_secondary"))
    print("  ctrl:", g("control","standard"), "| trav:", d.get("travel_mm"), "| rot:", json.dumps(d.get("rotary")))
    print("  nose-table:", d.get("spindle_nose_to_table_mm"), "col:", d.get("spindle_center_to_column_mm"), "pal-top:", d.get("spindle_center_to_pallet_top_mm"), "table:", d.get("table_mm"), d.get("table_load_kg"), "rotdia:", d.get("rotary_table_dia_mm"), "pallet:", d.get("pallet"))
    print("  lathe: swing", d.get("swing_over_bed_mm"), "maxdia", d.get("max_turning_dia_mm"), "maxlen", d.get("max_turning_length_mm"), "chuck", d.get("chuck_size_in"), "bar", d.get("bar_capacity_mm"), "nose", d.get("spindle_nose_type"), "turret", d.get("turret_stations"), d.get("turret_index_s"))
    print("  spindle:", json.dumps(d.get("spindle")))
    print("  sub:", json.dumps(d.get("sub_spindle")), "live:", json.dumps(d.get("live_tool")), "mh:", json.dumps(d.get("milling_head")))
    print("  rapid:", d.get("rapid_m_min"), "feed:", d.get("max_cutting_feed_m_min"), "acc:", d.get("accel_g_or_m_s2"))
    print("  changer:", json.dumps(d.get("changer")))
    print("  overall:", d.get("overall_mm"), d.get("weight_kg"), "tblh:", d.get("table_height_from_floor_mm"))
    print("  notes:", (d.get("notes") or "")[:600])
