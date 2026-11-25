//! A STEP file's solids as the reader gives them (index, faces, box in mm): to write a
//! workholding model's groups (forge_workholding_builder::compile::Spec).
fn main() {
    let p = std::env::args().nth(1).expect("STEP");
    let im = forge_step::import(&std::fs::read(&p).expect("read"), &forge_step::Limits::default())
        .expect("import");
    for (i, s) in im.solids.iter().enumerate() {
        let mut b = forge_math::Box3::EMPTY;
        for f in s.body.face_ids() {
            b = b.hull(&s.body.face_box(f));
        }
        println!(
            "{i:3} faces {:4}  x {:?} {:?}  y {:?} {:?}  z {:?} {:?}  {}",
            s.body.face_ids().count(),
            b.x.lo,
            b.x.hi,
            b.y.lo,
            b.y.hi,
            b.z.lo,
            b.z.hi,
            s.name
        );
    }
}
