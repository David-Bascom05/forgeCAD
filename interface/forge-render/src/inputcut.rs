//! Where a frame's input stops: a click and typing never share a frame. egui moves the keyboard
//! focus between frames (a press away from a field takes it from that field at the frame's
//! start; the field clicked takes it at the frame's end), so keys that came in the same frame as
//! a click reached no field and were lost. In the page a frame waits for whatever ran before it
//! (a rebuild on the page's one thread), and everything typed and clicked meanwhile came in one
//! frame: "Jo Do", a click on the next field, "xy" kept only "Jo ".

fn keyboard(e: &egui::Event) -> bool {
    matches!(
        e,
        egui::Event::Key { .. }
            | egui::Event::Text(_)
            | egui::Event::Paste(_)
            | egui::Event::Copy
            | egui::Event::Cut
            | egui::Event::Ime(_)
    )
}

fn button(e: &egui::Event) -> bool {
    matches!(e, egui::Event::PointerButton { .. })
}

/// How many of `events` one frame takes: all of them, unless a mouse button and the keyboard
/// both come in them; then up to where the second of the two kinds starts (the rest is the next
/// frame's).
pub fn frame_cut(events: &[egui::Event]) -> usize {
    let mut keys = false;
    let mut buttons = false;
    for (i, e) in events.iter().enumerate() {
        if (keyboard(e) && buttons) || (button(e) && keys) {
            return i;
        }
        keys |= keyboard(e);
        buttons |= button(e);
    }
    events.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(down: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(1.0, 1.0),
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: Default::default(),
        }
    }

    fn text(s: &str) -> egui::Event {
        egui::Event::Text(s.into())
    }

    #[test]
    fn a_click_and_typing_go_to_separate_frames() {
        // typing alone, a click alone (its press and release together), moves: one frame
        let typed = [
            text("a"),
            text("b"),
            egui::Event::PointerMoved(egui::pos2(2.0, 2.0)),
            text("c"),
        ];
        assert_eq!(frame_cut(&typed), 4);
        assert_eq!(frame_cut(&[press(true), press(false)]), 2);
        // typed, then a click on another field, then typed into it: three frames
        let mut ev = vec![text("Jo"), text(" "), press(true), press(false), text("xy")];
        let mut frames = vec![];
        while !ev.is_empty() {
            let n = frame_cut(&ev);
            assert!(n > 0);
            frames.push(ev.drain(..n).count());
        }
        assert_eq!(frames, [2, 2, 1]);
        assert_eq!(frame_cut(&[]), 0);
    }
}
