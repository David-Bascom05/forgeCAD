//! Prints every header variable. `dwgvars FILE`
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let buf = std::fs::read(&path).unwrap();
    let s = forge_dwg::file::sections(&buf).unwrap();
    let h = forge_dwg::header::read_header(&s.header, s.ver, s.maint).unwrap();
    for (n, v) in &h.vars {
        let v = match v {
            forge_dwg::header::Val::S(x) => format!("{:?}", x.text()),
            v => format!("{v:?}"),
        };
        println!("{n} = {v}");
    }
}
