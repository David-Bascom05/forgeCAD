//! `forgecad tool log`: instances added, a job logged on them with its predicted life use, wear
//! observations, and a refusal for a tool the job does not load.
use forge_cam_lang::toollog::{Env, run};
use std::path::Path;

fn argv(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

#[test]
fn log_a_job_on_a_physical_tool() {
    let tmp = forge_dirs::temp_dir().join(format!("forge-toollog-{}", forge_dirs::process_id()));
    let _ = forge_dirs::fs::remove_dir_all(&tmp);
    forge_dirs::fs::create_dir_all(&tmp).unwrap();
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined");
    forge_dirs::fs::copy(ex.join("flange.fg"), tmp.join("flange.fg")).unwrap();
    forge_dirs::fs::write(
        tmp.join("job.fg"),
        "machining\nf = part(\"flange.fg\", group: \"S\", material: \"Ti-6Al-4V\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"lakeshore-LC206014X\", t: 1, stickout: 30, holder: \"bt40-er32\", rpm: 4000)\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01)\nskim = facing(op1, tool: t1, to: 18.5, step: 1)\nprog = program(op1, post: \"fanuc\", number: 1)\n",
    )
    .unwrap();
    let env = Env {
        project: tmp.clone(),
        user_cam: None,
        today: "2026-11-24".into(),
    };
    let r = |s: &str| run(&env, &argv(s));
    let o = r("add EM6-a --tool lakeshore-LC206014X --serial 0042");
    assert_eq!(o.code, 0, "{}", o.err);
    assert_eq!(r("add EM6-a --tool lakeshore-LC206014X").code, 1);
    assert_eq!(r("add X --tool no-such-tool").code, 1);
    let o = r("run job.fg --use T1=EM6-a");
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(
        o.out.contains("% used") && o.out.contains("logged on EM6-a"),
        "{}",
        o.out
    );
    let o = r("observe EM6-a --vb 0.08 --note even");
    assert_eq!(o.code, 0, "{}", o.err);
    let o = r("list --json");
    assert!(
        o.out.contains("\"label\":\"EM6-a\"") && o.out.contains("\"programs\":1"),
        "{}",
        o.out
    );
    assert!(
        o.out.contains("\"row\":\"ti64-carbide-alti-n-highfeed-vb03\""),
        "{}",
        o.out
    );
    assert!(o.out.contains("\"last_vb\":0.08"), "{}", o.out);
    // the second run starts from what the first used: predict says so
    let p = r("predict job.fg --use T1=EM6-a");
    assert_eq!(p.code, 0, "{}", p.err);
    assert_eq!(r("run job.fg --use T2=EM6-a").code, 1);
    let o = r("worn EM6-a --vb 0.3");
    assert_eq!(o.code, 0, "{}", o.err);
    let c = r("calibrate");
    assert!(c.out.contains("calibrated from your 1 log"), "{}", c.out);
    assert!(tmp.join("cam/tool-instances.toml").exists());
    assert_eq!(r("frobnicate").code, 2);
    assert_eq!(r("").code, 2);
    let _ = forge_dirs::fs::remove_dir_all(&tmp);
}
