//! The page's input for egui: winit's canvas events as egui's (egui-winit is native only),
//! shared by every browser host.

use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

#[derive(Default)]
pub struct Input {
    pub events: Vec<egui::Event>,
    pub modifiers: egui::Modifiers,
    pub pointer: egui::Pos2,
}

impl Input {
    /// What egui is given for a frame of `size` device pixels.
    pub fn raw(&mut self, size: (u32, u32), ppp: f32, time: f64) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size.0 as f32 / ppp, size.1 as f32 / ppp),
            )),
            time: Some(time),
            focused: true,
            events: std::mem::take(&mut self.events),
            ..Default::default()
        }
    }

    /// A window event as egui's input; true if it wants a frame.
    pub fn event(&mut self, e: &WindowEvent, ppp: f32) -> bool {
        let m = self.modifiers;
        match e {
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = egui::pos2(position.x as f32 / ppp, position.y as f32 / ppp);
                self.events.push(egui::Event::PointerMoved(self.pointer));
            }
            WindowEvent::CursorLeft { .. } => self.events.push(egui::Event::PointerGone),
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    MouseButton::Left => egui::PointerButton::Primary,
                    MouseButton::Right => egui::PointerButton::Secondary,
                    MouseButton::Middle => egui::PointerButton::Middle,
                    MouseButton::Back => egui::PointerButton::Extra1,
                    MouseButton::Forward => egui::PointerButton::Extra2,
                    MouseButton::Other(_) => return false,
                };
                self.events.push(egui::Event::PointerButton {
                    pos: self.pointer,
                    button,
                    pressed: *state == ElementState::Pressed,
                    modifiers: m,
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (unit, delta) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (egui::MouseWheelUnit::Line, egui::vec2(*x, *y)),
                    MouseScrollDelta::PixelDelta(p) => (
                        egui::MouseWheelUnit::Point,
                        egui::vec2(p.x as f32 / ppp, p.y as f32 / ppp),
                    ),
                };
                self.events.push(egui::Event::MouseWheel {
                    unit,
                    delta,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: m,
                });
            }
            WindowEvent::ModifiersChanged(s) => {
                let s = s.state();
                self.modifiers = egui::Modifiers {
                    alt: s.alt_key(),
                    ctrl: s.control_key(),
                    shift: s.shift_key(),
                    mac_cmd: false,
                    command: s.control_key() || s.super_key(),
                };
                self.events.push(egui::Event::ModifiersChanged(self.modifiers));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let Some(key) = key_of(&event.logical_key) {
                    self.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: event.repeat,
                        modifiers: m,
                    });
                }
                if pressed
                    && !m.ctrl
                    && !m.command
                    && let Some(t) = &event.text
                    && !t.chars().any(char::is_control)
                {
                    self.events.push(egui::Event::Text(t.to_string()));
                }
            }
            WindowEvent::Focused(f) => self.events.push(egui::Event::WindowFocused(*f)),
            _ => return false,
        }
        true
    }
}

pub fn key_of(k: &Key) -> Option<egui::Key> {
    use egui::Key as K;
    Some(match k {
        Key::Named(n) => match n {
            NamedKey::Enter => K::Enter,
            NamedKey::Space => K::Space,
            NamedKey::Backspace => K::Backspace,
            NamedKey::Delete => K::Delete,
            NamedKey::Escape => K::Escape,
            NamedKey::Tab => K::Tab,
            NamedKey::ArrowUp => K::ArrowUp,
            NamedKey::ArrowDown => K::ArrowDown,
            NamedKey::ArrowLeft => K::ArrowLeft,
            NamedKey::ArrowRight => K::ArrowRight,
            NamedKey::Home => K::Home,
            NamedKey::End => K::End,
            NamedKey::PageUp => K::PageUp,
            NamedKey::PageDown => K::PageDown,
            NamedKey::F1 => K::F1,
            NamedKey::F2 => K::F2,
            NamedKey::F3 => K::F3,
            NamedKey::F4 => K::F4,
            NamedKey::F5 => K::F5,
            NamedKey::F6 => K::F6,
            NamedKey::F7 => K::F7,
            NamedKey::F8 => K::F8,
            NamedKey::F9 => K::F9,
            NamedKey::F10 => K::F10,
            NamedKey::F11 => K::F11,
            NamedKey::F12 => K::F12,
            _ => return None,
        },
        Key::Character(s) => return egui::Key::from_name(&s.to_uppercase()),
        _ => return None,
    })
}
