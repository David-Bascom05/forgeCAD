//! (fuzz comparison) the delete_faces tries per fuzz seed, with each run's source.
fn main() {
    let out = std::env::args().nth(1).unwrap();
    for &seed in forge_lang::fuzz::SEEDS.iter() {
        let r = forge_lang::fuzz::run(seed, 12);
        std::fs::write(format!("{out}/all{seed}.fg"), &r.src).unwrap();
        std::fs::write(format!("{out}/tried{seed}.txt"), format!("{:#?}", r.tried)).unwrap();
        let mut rf = String::new();
        for (w, src) in &r.refused {
            rf += &format!("=== {w}\n{src}\n");
        }
        std::fs::write(format!("{out}/refused{seed}.txt"), rf).unwrap();
        for (k, why) in &r.tried {
            if *k == "delete_faces" {
                println!("seed {seed}: delete_faces {:?}", why);
                std::fs::write(format!("{out}/seed{seed}.fg"), &r.src).unwrap();
            }
        }
    }
}
