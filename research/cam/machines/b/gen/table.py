import tomllib,glob
for f in sorted(glob.glob('/home/user/forgecad/kernel/forge-machine/data/machines/*.toml')):
    for m in tomllib.load(open(f,'rb'))['machine']:
        js=" ".join(f"{j['name']}{j['range']}r{j['rapid']/1000:g}f{j['max_feed']/1000:g}" for j in m['joint'])
        s=m['spindle'][0]; c=(m.get('changer') or [{}])[0]
        print(f"{m['name'][:34]:34s} {m['kind']:11s} {m['control']:14s} {js} | rpm{s['rpm']} max{s['max']} | tc{c.get('pockets')}/{c.get('change_s')}/{c.get('max_tool_kg')}")
