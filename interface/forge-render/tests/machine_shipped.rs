//! A built-in machine with forgecad's own models (forge_machine_builder::models::shipped_dir):
//! the Haas VF-2 drawn from Haas's model in the Machine Builder when the user has none of their
//! own; in a clone whose models are Git LFS pointers, drawn from its envelopes with the status
//! saying how to get them. `FORGE_UI_SHOTS=DIR` keeps the pictures (1400 x 860).

use forge_render::gui::Headless;
use forge_render::shell::machine::MachineUi;
use std::path::Path;

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        if let Ok(png) = h.png() {
            std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

#[test]
fn the_vf2_is_drawn_from_haas_model_or_says_how_to_get_it() {
    let mut h = match Headless::new((1400.0, 860.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    let root = std::env::temp_dir().join(format!("fg-machine-shipped-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("project");
    let user = root.join("config/forgecad/cam/machines");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&user).unwrap();

    let mut m = MachineUi::new(Some(project.clone()));
    m.user = Some(user.clone());
    m.reload_catalog();
    m.pick_record("haas-vf-2").unwrap();
    match forge_machine_builder::models::shipped_dir("haas-vf-2") {
        Some(d) => {
            // downloaded: Haas's model, a mesh a body, read from forgecad's folder (nothing copied)
            assert_eq!(m.models_shipped.as_deref(), Some(d.as_path()));
            let bodies: Vec<&str> = m.models.iter().filter_map(|x| x.body.as_deref()).collect();
            for b in ["base", "column", "saddle", "table", "head"] {
                assert!(bodies.contains(&b), "{b} not in {bodies:?}");
            }
            for md in &m.models {
                assert!(m.model_tris(md).is_ok_and(|t| !t.pos.is_empty()), "{}", md.file);
            }
            assert!(!user.join("models/haas-vf-2").exists(), "nothing copied until changed");
            h.gui.shell.machine = Some(m);
            shot(&mut h, "shipped-01-haas-vf-2-real-model");
        }
        None => {
            // a clone's LFS pointers: the envelopes, and the way to get the model
            assert!(m.models.is_empty());
            if forge_machine_builder::models::not_downloaded("haas-vf-2").is_some() {
                assert_eq!(
                    m.message,
                    "Model not downloaded: drawn from its envelopes. To get it, run forgecad machine model fetch haas-vf-2"
                );
            }
            h.gui.shell.machine = Some(m);
            shot(&mut h, "shipped-00-haas-vf-2-not-downloaded");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
