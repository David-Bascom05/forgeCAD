//! spinner-u5-620's rotary axes, fitted from the RPTU/Spinner dataset's log
//! (tools/machines/spinner_u5_620_fit.py; Zenodo 15735480, CC BY 4.0): the record reproduces logged
//! TRAORI rows. Each row: the control's machine positions (MCS) X Y Z B C and the tool tip in the
//! table's frame (the log's WCS column, the SINUMERIK BCS); the tool's gauge length from the fit.
//! This record's X and Y are machine X and Y, its Z machine Z - 610 (its note).
use forge_machine::kin;
use forge_machine::library::builtin_named;

const ROWS: [(&str, [f64; 5], [f64; 3], f64); 4] = [
    // 2024-03-12 08:58:02, SCHAFT_D12_N_AP30 (gauge 250.1)
    (
        "d12 B90 C0",
        [-350.738, 193.17, 600.217, 90.0, 0.0],
        [301.097, 193.17, 400.231],
        250.1,
    ),
    // 2024-03-12 09:04:29
    (
        "d12 B90 C90",
        [-355.049, 199.622, 599.882, 90.0, 90.0],
        [-199.627, 300.755, 404.542],
        250.1,
    ),
    // 2024-03-12 09:37:34, ENTFRAESER_D8_N_90GRAD (gauge 137.0)
    (
        "deburr B90 C0",
        [-301.983, 114.526, 510.908, 90.0, 0.0],
        [324.786, 114.526, 351.477],
        137.0,
    ),
    // 2024-03-12 09:38:11
    (
        "deburr B90 C90",
        [-271.269, 31.412, 454.428, 90.0, 90.0],
        [-31.416, 268.3, 320.763],
        137.0,
    ),
];

#[test]
fn the_record_reproduces_the_logged_traori_rows() {
    let m = builtin_named("spinner-u5-620").unwrap();
    let names: Vec<&str> = m.joints.iter().map(|j| j.name.as_str()).collect();
    for (what, mcs, bcs, gauge) in ROWS {
        let mut q = vec![0.0; m.joints.len()];
        for (i, l) in ["X", "Y", "Z", "B", "C"].iter().enumerate() {
            let j = names.iter().position(|n| n == l).unwrap();
            q[j] = if *l == "Z" { mcs[i] - 610.0 } else { mcs[i] };
        }
        let (tip, ax) = kin::tip_in_part(&m, &q, "spindle", "table", gauge).unwrap();
        let gap = (0..3).map(|k| (tip[k] - bcs[k]).abs()).fold(0.0, f64::max);
        eprintln!("{what}: {gap:.3} mm");
        // the fit's residuals: rms 0.142, max 0.333 mm
        assert!(gap < 0.5, "{what}: tip {tip:?} vs logged {bcs:?} ({gap:.3} mm)");
        // the tool axis in the table's frame at B+90: +x (the table top faces -x)
        assert!(ax[0].abs() > 0.999 || ax[1].abs() > 0.999, "{what}: {ax:?}");
    }
}
