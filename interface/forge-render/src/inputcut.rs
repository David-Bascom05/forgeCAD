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

/// `rest` starts with a press of `button` where it was let go at `at` (moves within egui's click
/// distance before it): the second click of a double click.
fn second_click(rest: &[egui::Event], at: egui::Pos2, button: egui::PointerButton) -> bool {
    const NEAR: f32 = 6.0;
    for e in rest {
        match e {
            egui::Event::PointerMoved(p) if p.distance(at) <= NEAR => {}
            egui::Event::PointerButton {
                pressed: true,
                pos,
                button: b,
                ..
            } => {
                return *b == button && pos.distance(at) <= NEAR;
            }
            _ => return false,
        }
    }
    false
}

/// How many of `events` one frame takes: all of them, unless a mouse button and the keyboard
/// both come in them; then up to where the second of the two kinds starts (the rest is the next
/// frame's). `hold_buttons`: the frame before changed what's where (it asked for another pass at
/// once), and egui finds what a click hits from the frame before's places, so a click would hit
/// what was there, not what's there now: the frame takes what comes before the first button
/// (a tab clicked, then the button its panel shows: the button's click was lost).
pub fn frame_cut(events: &[egui::Event], hold_buttons: bool) -> usize {
    if hold_buttons && let Some(i) = events.iter().position(button) {
        return i;
    }
    let mut keys = false;
    let mut buttons = false;
    for (i, e) in events.iter().enumerate() {
        if (keyboard(e) && buttons) || (button(e) && keys) {
            return i;
        }
        // a frame ends with a button let go: egui takes a click where the pointer is at the
        // frame's end, so a move after it (to the next button) lost the click; and a second click
        // waits for what the first changes to be laid out (a tab, then the button its panel shows).
        // Not a double click's first click: egui counts the second as a double click only within
        // its delay of the first by the frames' clocks, and the next frame can come later than
        // that (frames paced to a busy GPU), so the two were read as two single clicks
        if let egui::Event::PointerButton {
            pressed: false,
            pos,
            button,
            ..
        } = e
            && !second_click(&events[i + 1..], *pos, *button)
        {
            return i + 1;
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
        assert_eq!(frame_cut(&typed, false), 4);
        assert_eq!(frame_cut(&[press(true), press(false)], false), 2);
        // typed, then a click on another field, then typed into it: three frames
        let mut ev = vec![text("Jo"), text(" "), press(true), press(false), text("xy")];
        let mut frames = vec![];
        while !ev.is_empty() {
            let n = frame_cut(&ev, false);
            assert!(n > 0);
            frames.push(ev.drain(..n).count());
        }
        assert_eq!(frames, [2, 2, 1]);
        // a click, then a move to the next button: the move is the next frame's
        assert_eq!(
            frame_cut(
                &[
                    press(true),
                    press(false),
                    egui::Event::PointerMoved(egui::pos2(3.0, 3.0))
                ],
                false
            ),
            2
        );
        assert_eq!(frame_cut(&[], false), 0);
        // a frame whose layout just changed: the click waits for the next
        let moved_then_click = [
            egui::Event::PointerMoved(egui::pos2(5.0, 5.0)),
            press(true),
            press(false),
        ];
        assert_eq!(frame_cut(&moved_then_click, true), 1);
        assert_eq!(frame_cut(&moved_then_click, false), 3);
        // two clicks: one a frame
        let two = [
            press(true),
            press(false),
            egui::Event::PointerMoved(egui::pos2(9.0, 9.0)),
            press(true),
            press(false),
        ];
        assert_eq!(frame_cut(&two, false), 2);
        assert_eq!(frame_cut(&two[2..], false), 3);
    }

    #[test]
    fn a_double_click_is_one_frame() {
        // press, release, press, release at one place (the page's app_click, or a busy page's
        // queue): one frame, so egui reads the second as a double click whatever the frames'
        // clocks. Before: two frames, and a second frame more than egui's 0.3 s after the first
        // (frames paced to a busy GPU) made it a second single click: no op dialog opened
        let double = [
            press(true),
            press(false),
            press(true),
            press(false),
            egui::Event::PointerGone,
        ];
        // (both clicks in the frame; the pointer's leaving is the next's, as after any click)
        assert_eq!(frame_cut(&double, false), 4);
        // the second press a few pixels off, a small move between: still one
        let near = [
            press(true),
            press(false),
            egui::Event::PointerMoved(egui::pos2(3.0, 2.0)),
            egui::Event::PointerButton {
                pos: egui::pos2(3.0, 2.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            press(false),
        ];
        assert_eq!(frame_cut(&near, false), 5);
        // the other button, or typing after the click: cut as before
        let other = [
            press(true),
            press(false),
            egui::Event::PointerButton {
                pos: egui::pos2(1.0, 1.0),
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: Default::default(),
            },
        ];
        assert_eq!(frame_cut(&other, false), 2);
        assert_eq!(frame_cut(&[press(true), press(false), text("x")], false), 2);
    }
}
