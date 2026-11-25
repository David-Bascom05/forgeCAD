//! A machine or fixture model whose body is a surface model's shell that does not close (Haas
//! Tooling's 05-0994, 08-1865: a vise's parts so) is drawn by its faces, not left out.
use forge_machine_builder::model_files as mf;

#[test]
fn a_shell_that_does_not_close_is_drawn_by_its_faces() {
    let src = "sketch a(on: xy) {\n  r = rect(corner: (0, 0), size: (10, 10))\n}\ncube = extrude(a, 10)\n";
    let (run_, _) = forge_lang::run(
        src,
        forge_lang::Options::default(),
        forge_lang::build::Cache::default(),
    );
    assert!(run_.ok(), "{:?}", run_.errors.first());
    let text = forge_step::export(&run_.body.unwrap(), &forge_step::Options::default())
        .unwrap()
        .text;
    let sm = text
        .replace("CLOSED_SHELL(", "OPEN_SHELL(")
        .replace("MANIFOLD_SOLID_BREP(", "SHELL_BASED_SURFACE_MODEL(")
        .replace(
            "ADVANCED_BREP_SHAPE_REPRESENTATION(",
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION(",
        );
    let at = sm.find("SHELL_BASED_SURFACE_MODEL('',").unwrap() + 29;
    let end = at + sm[at..].find(')').unwrap();
    let sm = format!("{}({}){}", &sm[..at], &sm[at..end], &sm[end..]);
    // the whole cube reads as one solid, its six faces
    let whole = mf::read("cube.step", sm.as_bytes()).unwrap();
    assert_eq!(whole.len(), 1);
    // two of its faces left out (every two faces of a cube meet, so the hole is not flat): a
    // sheet of four faces, drawn
    let sh = sm.find("OPEN_SHELL('',(").unwrap() + 15;
    let second = sh + sm[sh..].find(',').unwrap() + 1;
    let third = second + sm[second..].find(',').unwrap() + 1;
    let fourth = third + sm[third..].find(',').unwrap() + 1;
    // the shell's first and third faces (an extrusion's are its ends, then its sides: an end and
    // a side)
    let open = format!("{}{}{}", &sm[..sh], &sm[second..third], &sm[fourth..]);
    let parts = mf::read("open.step", open.as_bytes()).unwrap();
    assert_eq!(parts.len(), 1, "the sheet, a piece");
    let p = &parts[0];
    let area: f64 = p
        .tri
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|k| p.pos[k as usize]);
            let (u, v) = (
                [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
            );
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
        })
        .sum();
    assert!((area - 400.0).abs() < 1e-9, "{area}");
}
