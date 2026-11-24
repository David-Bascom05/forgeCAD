//! Each built-in's arguments are one list, `ARGS` (LANG-PROOFS' shape: the proof ledger and
//! CAM-UI's schema read it too). A name a built-in does not take was silently ignored (a typo
//! did nothing and said nothing); it is now `UnknownArg`, with the nearest name as the fix.
mod common;
use common::{dir, doc};

fn errors(body: &str) -> Vec<String> {
    forge_cam_lang::run(&doc(body), &dir()).errors
}

#[test]
fn a_misspelled_argument_is_refused_with_the_nearest() {
    let e = errors("top = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepovr: 4)\n");
    assert!(
        e.iter()
            .any(|e| e
                .contains("UnknownArg: `stepovr` is not an argument of `clear` (did you mean `stepover`?)")),
        "{e:?}"
    );
    let e = errors("skim = facing(op1, tool: t1, to: 18, step: 1, leave_axail: 0.5)\n");
    assert!(
        e.iter().any(|e| e.contains("(did you mean `leave_axial`?)")),
        "{e:?}"
    );
    // nothing near: the list
    let e = errors("skim = facing(op1, tool: t1, to: 18, step: 1, colour: 3)\n");
    assert!(
        e.iter()
            .any(|e| e
                .contains("`colour` is not an argument of `facing` (it takes: tool, to, step, leave_axial, vc, rpm")),
        "{e:?}"
    );
    // in an expression built-in, and a positional too many
    let e = errors("op2 = setup(f, stock: box(margin: (1, 1, 1), bellow: 2))\n");
    assert!(
        e.iter()
            .any(|e| e.contains("`bellow` is not an argument of `box` (did you mean `below`?)")),
        "{e:?}"
    );
    let e = errors("skim = facing(op1, t1, to: 18)\n");
    assert!(
        e.iter()
            .any(|e| e
                .contains("UnknownArg: `facing` takes 1 positional argument (argument 2 is one too many)")),
        "{e:?}"
    );
}

#[test]
fn every_built_in_has_its_list_and_every_shipped_document_passes() {
    for (name, args) in forge_cam_lang::ARGS {
        assert!(!args.is_empty(), "{name}");
        assert_eq!(forge_cam_lang::args_of(name), Some(*args));
    }
    for e in forge_dirs::fs::read_dir(dir()).unwrap() {
        let p = e.unwrap().path();
        if p.to_string_lossy().ends_with("_cam.fg") {
            let src = forge_dirs::fs::read_to_string(&p).unwrap();
            let cam = forge_cam_lang::run(&src, &dir());
            assert!(
                cam.errors.iter().all(|e| !e.contains("UnknownArg")),
                "{}: {:?}",
                p.display(),
                cam.errors
            );
        }
    }
}
