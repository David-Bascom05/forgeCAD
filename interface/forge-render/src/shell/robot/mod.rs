//! The Robot environment's content (research/export/urdf-editor.md §6, research/ui/environments.md
//! §5): a robot document beside the part or assembly it is linked to. What the Environments
//! framework's adapter calls:
//! - `open_for(source)`: the source's companion robot document (`NAME_robot.fg` beside it),
//!   opened, or made linked to it;
//! - `groups(s, act)`: its ribbon, each tool's action made by `act(command)`;
//! - `Shell::robot_side(ui, r)`: its browser, the tree of links and joints;
//! - `Shell::robot_ui(ctx)`: its panel (drive, details, checks), once a frame;
//! - `command(s, c)`: one of its ribbon commands.
//!
//! The host (gui_robot.rs) runs the export when the panel is new or stale and moves the parts.

mod robot_ui;

pub use robot_ui::{RobotAct, RobotPanel, Sel, TreeOp};

use super::ribbon::{Group, Tool};
use super::{Action, Shell};
use std::path::Path;

/// The environment's state: the panel of the robot document open.
pub type RobotWs = RobotPanel;

/// The robot document that goes with a part or assembly: `NAME_robot.fg` beside it, opened if
/// it is there (it must be a robot document), else made: `robot` and a source linking to it. A
/// robot document itself is opened as it is.
pub fn open_for(source: &str) -> Result<RobotWs, String> {
    let p = Path::new(source);
    if forge_dirs::fs::read_to_string(p).is_ok_and(|t| forge_lang::robot::is_robot(&t)) {
        return Ok(RobotPanel::new(source));
    }
    let stem = p
        .file_stem()
        .ok_or("a file with a name")?
        .to_string_lossy()
        .into_owned();
    let name = p
        .file_name()
        .ok_or("a file with a name")?
        .to_string_lossy()
        .into_owned();
    let doc = p.with_file_name(format!("{stem}_robot.fg"));
    match forge_dirs::fs::read_to_string(&doc) {
        Ok(t) if forge_lang::robot::is_robot(&t) => {}
        Ok(_) => {
            return Err(format!("{} is there and is not a robot document", doc.display()));
        }
        Err(_) => {
            let text = format!(
                "# The robot of {name}: linked to it, with what the URDF needs on top\n# (research/export/urdf-editor.md).\nrobot\n\nsrc = source(\"{name}\")\n"
            );
            forge_dirs::fs::write(&doc, text).map_err(|e| format!("{}: {e}", doc.display()))?;
        }
    }
    let mut p = RobotPanel::new(&doc.to_string_lossy());
    p.source = Some(name);
    Ok(p)
}

/// Its ribbon commands.
pub const EXPORT: u16 = 1;
pub const UNDO: u16 = 2;
pub const HOME: u16 = 3;
pub const AXES: u16 = 4;
pub const COM: u16 = 5;
const TREE: [(u16, TreeOp); 6] = [
    (10, TreeOp::MakeRoot),
    (11, TreeOp::Drop),
    (12, TreeOp::Merge),
    (13, TreeOp::Freeze),
    (14, TreeOp::Frame),
    (15, TreeOp::Hull),
];

/// The environment's ribbon: Robot (export, undo, the export pose), Show (joint axes, centres of
/// mass), Structure (the tree's operations on the selection, off where they don't apply).
// called by the Environments framework's adapter (ui/environments, shell/env/robot.rs)
#[allow(dead_code)]
pub(in crate::shell) fn groups(s: &Shell, act: &dyn Fn(u16) -> Action) -> Vec<Group> {
    let Some(p) = &s.robot else { return vec![] };
    let tool = |name, icon, c: u16, on: bool, enabled: bool| Tool {
        name,
        icon,
        action: enabled.then(|| act(c)),
        on,
    };
    vec![
        Group {
            name: "Robot",
            big: vec![tool("Export URDF", "export", EXPORT, false, p.view.is_some())],
            small: vec![
                tool("Undo", "undo", UNDO, false, !p.undo.is_empty()),
                tool("Export pose", "home", HOME, false, !p.q.is_empty()),
            ],
            icons: vec![],
        },
        Group {
            name: "Show",
            big: vec![],
            small: vec![
                tool("Joint axes", "axis", AXES, p.show_axes, true),
                tool("Centres of mass", "point", COM, p.show_com, true),
            ],
            icons: vec![],
        },
        Group {
            name: "Structure",
            big: vec![],
            small: TREE
                .iter()
                .map(|&(c, op)| tool(op.label(), "jrevolute", c, false, p.tree_edit(op).is_ok()))
                .collect(),
            icons: vec![],
        },
    ]
}

/// One of the ribbon's commands.
pub fn command(s: &mut Shell, c: u16) {
    let Some(p) = &mut s.robot else { return };
    match c {
        EXPORT => p.out.push(RobotAct::Export),
        UNDO => p.out.push(RobotAct::Undo),
        HOME => p.home(),
        AXES => p.show_axes = !p.show_axes,
        COM => p.show_com = !p.show_com,
        _ => {
            if let Some(&(_, op)) = TREE.iter().find(|t| t.0 == c)
                && let Ok(e) = p.tree_edit(op)
            {
                p.out.push(RobotAct::Edit(e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm() -> Shell {
        let mut s = Shell::default();
        let mut p = RobotPanel::new("arm_robot.fg");
        p.source = Some("arm.fg".into());
        let view = forge_urdf::RobotView {
            root: "base".into(),
            links: ["base", "upper"]
                .map(|n| forge_urdf::ViewLink {
                    name: n.into(),
                    bodies: vec![n.into()],
                    mass: Some(1.0),
                    com: [0.0; 3],
                    visuals: vec![],
                    collisions: vec![],
                    dummy: false,
                })
                .to_vec(),
            joints: vec![forge_urdf::ViewJoint {
                name: "shoulder".into(),
                kind: "revolute",
                of: "shoulder".into(),
                parent: "base".into(),
                child: "upper".into(),
                origin: forge_math::Frame::WORLD,
                axis: Some([0.0, 0.0, 1.0]),
                limit: Some((-1.0, 1.0)),
                position: vec![0.5],
                mimic: None,
                effort: None,
                velocity: None,
                dynamics: None,
            }],
        };
        p.view = Some(view);
        s.robot = Some(p);
        s
    }

    #[test]
    fn the_ribbon_follows_the_selection() {
        let mut s = arm();
        let on = |s: &Shell, name: &str| {
            groups(s, &|_| Action::FitAll)
                .iter()
                .flat_map(|g| g.big.iter().chain(&g.small))
                .find(|t| t.name == name)
                .map(|t| t.action.is_some())
        };
        assert_eq!(on(&s, "Export URDF"), Some(true));
        assert_eq!(on(&s, "Make root"), Some(false), "nothing selected");
        s.robot.as_mut().unwrap().sel = Some(Sel::Link("upper".into()));
        assert_eq!(on(&s, "Make root"), Some(true));
        assert_eq!(on(&s, "Merge into parent"), Some(false), "the shoulder moves");
        assert_eq!(on(&s, "Drop"), Some(true));
        command(&mut s, 10);
        let out = &s.robot.as_ref().unwrap().out;
        assert_eq!(
            out[0],
            RobotAct::Edit(forge_urdf::robot::Edit::Put {
                call: "root".into(),
                text: "root(src.upper)".into()
            })
        );
        s.robot.as_mut().unwrap().sel = Some(Sel::Joint("shoulder".into()));
        command(&mut s, 13);
        assert_eq!(
            s.robot.as_ref().unwrap().out[1],
            RobotAct::Edit(forge_urdf::robot::Edit::Add {
                text: "freeze(src.shoulder, at: 28.64789deg)".into()
            })
        );
        command(&mut s, AXES);
        assert!(!s.robot.as_ref().unwrap().show_axes);
    }

    #[test]
    fn a_companion_robot_document_is_made_once() {
        let d = std::env::temp_dir().join(format!("fg-robot-open-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let src = d.join("arm.fg");
        std::fs::write(&src, "assembly\n").unwrap();
        let p = open_for(src.to_str().unwrap()).expect("made");
        let doc = d.join("arm_robot.fg");
        assert_eq!(p.file, doc.to_string_lossy());
        let text = std::fs::read_to_string(&doc).unwrap();
        assert!(forge_lang::robot::is_robot(&text) && text.contains("src = source(\"arm.fg\")"));
        // again: the same document, untouched
        std::fs::write(&doc, format!("{text}actuation(effort: 1, velocity: 1)\n")).unwrap();
        let q = open_for(src.to_str().unwrap()).expect("opened");
        assert_eq!(q.file, p.file);
        assert!(std::fs::read_to_string(&doc).unwrap().contains("actuation"));
        // the robot document itself opens as it is
        assert_eq!(
            open_for(doc.to_str().unwrap()).expect("itself").file,
            doc.to_string_lossy()
        );
    }
}
