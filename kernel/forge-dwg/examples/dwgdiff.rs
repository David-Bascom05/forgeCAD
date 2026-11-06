//! Shows where an object's re-encoding differs from the file's bits. `dwgdiff FILE HANDLE`
use forge_dwg::object::{Classes, read_object, write_object};
use forge_dwg::version::Version;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let buf = std::fs::read(&a[1]).unwrap();
    let want = u32::from_str_radix(a[2].trim_start_matches("0x"), 16).unwrap();
    let (map, list) = forge_dwg::file::r2000::read_shell(&buf).unwrap();
    let cl = Classes { list: &list };
    let at = map.iter().find(|m| m.0 == want).unwrap().1;
    let o = read_object(&buf, at, Version::R2000, &cl, None).unwrap();
    println!("{o:#?}");
    let mut f = o.clone();
    f.raw = None;
    let b = write_object(&f, Version::R2000).unwrap();
    let r = o.raw.unwrap();
    let bs = |b: &[u8]| b.iter().map(|x| format!("{x:08b}")).collect::<String>();
    let (x, y) = (bs(&r), bs(&b));
    let first = x.chars().zip(y.chars()).position(|(p, q)| p != q);
    println!("orig {} bits, new {} bits, first difference at bit {first:?}", x.len(), y.len());
    if let Some(k) = first {
        let lo = k.saturating_sub(16);
        println!("orig ..{}", &x[lo..(k + 48).min(x.len())]);
        println!("new  ..{}", &y[lo..(k + 48).min(y.len())]);
    }
}
