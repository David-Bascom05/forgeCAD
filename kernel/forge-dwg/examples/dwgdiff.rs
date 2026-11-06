//! Shows where an object's re-encoding differs from the file's bits. `dwgdiff FILE HANDLE`
use forge_dwg::file::{r2000, sections};
use forge_dwg::object::{Classes, encode_object, read_object};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let buf = std::fs::read(&a[1]).unwrap();
    let want = u32::from_str_radix(a[2].trim_start_matches("0x"), 16).unwrap();
    let s = sections(&buf).unwrap();
    let classes = r2000::classes(&s.classes, s.ver, s.maint).unwrap();
    let list: Vec<(String, bool)> = classes.iter().map(|c| (c.dxf.text(), c.item == 0x1F2)).collect();
    let cl = Classes { list: &list };
    let map = r2000::read_map(&s.handles, 0, s.handles.len()).unwrap();
    let at = map.iter().find(|m| m.0 == want).unwrap().1;
    let o = read_object(&s.objects, at, s.ver, &cl, None).unwrap();
    println!("{:#?}", o.data);
    println!("{:?}", o.ent);
    let mut f = o.clone();
    f.raw = None;
    let (b, n) = encode_object(&f, s.ver).unwrap();
    let r = o.raw.unwrap();
    // past the MS (and R2010+ MC) sizes, which differ when the length does
    let skip = |b: &[u8]| {
        let (_, mut q) = forge_dwg::bits::read_ms(b, 0).unwrap();
        if s.ver >= forge_dwg::version::Version::R2010 {
            let (h, q2) = forge_dwg::bits::read_mc(b, q, false).unwrap();
            println!("handle stream {h} bits");
            q = q2;
        }
        q
    };
    let (sr, sb) = (skip(&r), skip(&b));
    let n = n - sb * 8;
    let bs = |b: &[u8]| b.iter().map(|x| format!("{x:08b}")).collect::<String>();
    let (x, y) = (bs(&r[sr..]), bs(&b[sb..]));
    let first = x.chars().zip(y.chars()).take(n).position(|(p, q)| p != q);
    println!(
        "orig {} bits, new {} bits ({n} meaningful), first difference at bit {first:?}",
        x.len(),
        y.len()
    );
    if let Some(k) = first {
        let lo = k.saturating_sub(24);
        println!("orig ..{}", &x[lo..(k + 64).min(x.len())]);
        println!("new  ..{}", &y[lo..(k + 64).min(y.len())]);
    }
}
