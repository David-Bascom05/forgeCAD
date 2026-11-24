import sys
root=sys.argv[1]
p=root+'/kernel/forge-model/examples/bits.rs'
s=open(p).read()
if 'points_of' not in s:
    s=s.replace('''    let measures = std::env::args().nth(1).as_deref() == Some("measures");''','''    let measures = std::env::args().nth(1).as_deref() == Some("measures");
    let points = std::env::args().nth(1).as_deref() == Some("points");''')
    s=s.replace('''            if measures {''','''            if points {
                print!("{}", points_of(&name, path, r.as_ref().ok().into_iter()));
                continue;
            }
            if measures {''')
    s+='''
/// `bits points`: each body's validity and vertex, edge and face counts, then every vertex point
/// and edge range to the bit (for comparing two builds' results line by line).
pub fn points_of<'a>(name: &str, path: &str, bodies: impl Iterator<Item = &'a forge_topo::Body>) -> String {
    let mut out = String::new();
    let mut n = 0;
    for (k, b) in bodies.enumerate() {
        n += 1;
        let valid = forge_topo::check(b).is_empty();
        out += &format!(
            "{name} {path} body {k} {} V {} E {} F {}\\n",
            if valid { "valid" } else { "INVALID" },
            b.vertices.len(),
            b.edges.len(),
            b.faces.len()
        );
        for v in &b.vertices {
            out += &format!("{name} {path} v {:?} {:?} {:?}\\n", v.point.x, v.point.y, v.point.z);
        }
        for e in &b.edges {
            out += &format!("{name} {path} e {:?} {:?}\\n", e.range.lo, e.range.hi);
        }
    }
    if n == 0 {
        out += &format!("{name} {path} error\\n");
    }
    out
}
'''
    open(p,'w').write(s)
p=root+'/interface/forge-lang/examples/benchparts.rs'
s=open(p).read()
if '"points" =>' not in s:
    s=s.replace('''        "bits" => {
            for name in names() {''','''        "points" => {
            for name in names() {
                let (part, ids) = bench(&name);
                for (path, mode) in [("seq", "seq"), ("nary", "nary4")] {
                    match regen(&part, &ids, mode) {
                        Ok(solids) => {
                            for (k, b) in solids.iter().map(|s| &s.body).enumerate() {
                                let valid = forge_topo::check(b).is_empty();
                                println!(
                                    "{name} {path} body {k} {} V {} E {} F {}",
                                    if valid { "valid" } else { "INVALID" },
                                    b.vertices.len(),
                                    b.edges.len(),
                                    b.faces.len()
                                );
                                for v in &b.vertices {
                                    println!("{name} {path} v {:?} {:?} {:?}", v.point.x, v.point.y, v.point.z);
                                }
                                for e in &b.edges {
                                    println!("{name} {path} e {:?} {:?}", e.range.lo, e.range.hi);
                                }
                            }
                        }
                        Err(_) => println!("{name} {path} error"),
                    }
                }
            }
        }
        "bits" => {
            for name in names() {''')
    open(p,'w').write(s)
