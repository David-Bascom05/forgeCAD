//! Photos of the view (research/render/record.md): the view drawn offscreen at the size asked,
//! from the same camera, read back without blocking (the browser can't wait), written as a PNG
//! (natively a file; in the browser the project's file, which the page offers to download).

use super::Gui;
use crate::render::{Offscreen, Overlay};
use crate::studio::gpu::{Pending, read_texture};

/// A photo under way: the picture coming back, where it goes.
pub struct PhotoJob {
    pending: Pending,
    path: String,
    size: (u32, u32),
}

/// What the last photo did: the file written, or why not.
pub type PhotoDone = Result<String, String>;

impl Gui {
    /// Draws the view at `size` and starts reading it back; `photo_poll` writes it to `path`.
    pub fn photo_start(&mut self, size: (u32, u32), path: &str) -> Result<(), String> {
        if size.0 == 0 || size.1 == 0 || size.0 > 8192 || size.1 > 8192 {
            return Err(format!(
                "a photo is 1 to 8192 pixels a side, not {} x {}",
                size.0, size.1
            ));
        }
        let Some(v) = &mut self.view else {
            return Err("nothing in view".into());
        };
        let was = v.size;
        let target = Offscreen::new(&self.device, size);
        v.renderer.resize(size);
        v.renderer
            .frame(&target.view, &mut v.scene, &v.nav.camera, Overlay::default());
        v.renderer.resize(was);
        v.dirty = true;
        self.photo = Some(PhotoJob {
            pending: read_texture(&self.device, &self.queue, &target.texture),
            path: path.to_string(),
            size,
        });
        Ok(())
    }

    /// A frame's look at the photo under way: Some when it finished (written, or why not).
    pub fn photo_poll(&mut self) -> Option<PhotoDone> {
        let job = self.photo.as_ref()?;
        let got = job.pending.take(&self.device)?;
        let job = self.photo.take()?;
        Some(got.and_then(|rgba| {
            // the view's texture is sRGB-encoded bytes with the scheme's alpha: a photo is opaque
            let mut rgba = rgba;
            for a in rgba.iter_mut().skip(3).step_by(4) {
                *a = 255;
            }
            crate::studio::save_png(&job.path, job.size, &rgba).map(|_| job.path.clone())
        }))
    }

    /// Whether a photo is under way (the window draws again until it's done).
    pub fn photo_busy(&self) -> bool {
        self.photo.is_some()
    }
}
