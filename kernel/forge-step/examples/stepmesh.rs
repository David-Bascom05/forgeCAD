//! One STEP file: each solid it reads to, meshed as the app meshes a file's body
//! (`tessellate_tolerant`, at the chord the reader's own check takes: 5e-4 of the body's
//! diagonal), as one line: `PATH solids S meshed M watertight W open O failed F`.
//! For comparing a corpus's meshes between two builds, file by file.
use forge_mesh::Quality;
use forge_mesh::tessellate::tessellate_tolerant;

fn main() {
    let path = std::env::args().nth(1).expect("a STEP file");
    let text = std::fs::read(&path).expect("read");
    let imp = match forge_step::import(&text, &forge_step::Limits::default()) {
        Ok(i) => i,
        Err(_) => {
            println!("{path} unread");
            return;
        }
    };
    let (mut meshed, mut tight, mut open, mut failed) = (0, 0, 0, 0);
    for s in &imp.solids {
        let q = Quality {
            chord: 5e-4 * s.body.faces_box().diagonal(),
            ..Quality::default()
        };
        match tessellate_tolerant(&s.body, &q) {
            Ok(m) => {
                meshed += 1;
                let o = m.open_edges().len();
                open += o;
                tight += (o == 0) as usize;
            }
            Err(e) => {
                eprintln!("{path}: {e:?}");
                failed += 1
            }
        }
    }
    println!(
        "{path} solids {} meshed {meshed} watertight {tight} open {open} failed {failed}",
        imp.solids.len()
    );
}
