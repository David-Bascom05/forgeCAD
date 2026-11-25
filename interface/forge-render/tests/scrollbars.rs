//! Every scroll bar's handle shows against its track, in both themes (the UI audit: egui's solid
//! bars filled the handle with the track's own colour, so they were invisible). The colours
//! checked are the ones egui paints for the bar, with the app's theme applied, so no GPU is needed.

use egui::{Color32, Rect, Shape, epaint::RectShape};
use forge_render::shell::theme::{self, DARK, LIGHT, Theme};

fn lum(c: Color32) -> f32 {
    let ch = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
}

fn ratio(a: Color32, b: Color32) -> f32 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The fills drawn in the bar's strip at the area's right edge: its track, then its handle.
fn bar_fills(t: &Theme, solid: bool) -> Vec<(Rect, Color32)> {
    let ctx = egui::Context::default();
    theme::apply(&ctx, t);
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0))),
        ..Default::default()
    };
    let mut area = Rect::NOTHING;
    let mut out = None;
    // a few frames: a scroll area settles its bar after its first
    for _ in 0..3 {
        let mut o = ctx.run_ui(input.clone(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                if solid {
                    ui.spacing_mut().scroll = theme::solid_scroll();
                }
                let r = egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        for k in 0..60 {
                            ui.label(format!("row {k}"));
                        }
                    });
                area = r.inner_rect;
            });
        });
        // (no painter here to take the font atlas)
        o.textures_delta.clear();
        out = Some(o);
    }
    let strip = Rect::from_min_max(egui::pos2(area.right() - 14.0, area.top()), egui::pos2(area.right() + 20.0, area.bottom()));
    let o = out.unwrap();
    if std::env::var_os("SB_DEBUG").is_some() {
        eprintln!("area {area:?}");
        for c in &o.shapes {
            if let Shape::Rect(r) = &c.shape {
                eprintln!("  rect {:?} fill {:?}", r.rect, r.fill);
            } else {
                eprintln!("  other {:?}", std::mem::discriminant(&c.shape));
            }
        }
    }
    o
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Rect(RectShape { rect, fill, .. })
                if strip.contains_rect(*rect) && rect.width() < 20.0 =>
            {
                Some((*rect, *fill))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn every_scroll_bars_handle_shows_against_its_track_in_both_themes() {
    for (name, t) in [("dark", DARK), ("light", LIGHT)] {
        for solid in [true, false] {
            let fills = bar_fills(&t, solid);
            assert!(fills.len() >= 2, "{name} solid={solid}: no track and handle drawn: {fills:?}");
            // the track is the tallest; the handle is drawn over it
            let track = fills.iter().max_by(|a, b| a.0.height().total_cmp(&b.0.height())).unwrap();
            let handle = fills.iter().rev().find(|f| f.0 != track.0).unwrap();
            // a translucent handle is seen as blended over the track; a floating bar's track
            // is clear at rest, so its handle is seen over the panel
            let under = if track.1.a() == 255 {
                track.1
            } else {
                t.surface.blend(track.1)
            };
            let seen = under.blend(handle.1);
            let c = ratio(seen, under);
            eprintln!("{name} solid={solid}: {c:.1}:1");
            assert!(
                c >= 3.0,
                "{name} solid={solid}: handle {:?} on track {:?} is {c:.1}:1 (at least 3:1)",
                handle.1,
                track.1
            );
        }
    }
}
