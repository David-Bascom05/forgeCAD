//! The browser's H.264 encoder (WebCodecs `VideoEncoder`, driven by web/app.js's `forge_video_*`):
//! frames in, encoded frames back, written as MP4 by `mp4::mux`. Nothing blocks: the encoder's
//! flush is a promise the recorder polls a frame at a time.

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_open)]
    fn open(w: u32, h: u32, fps: u32) -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_frame)]
    fn frame(id: u32, rgba: Uint8Array, w: u32, h: u32, k: u32);
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_backlog)]
    fn backlog(id: u32) -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_finish)]
    fn finish(id: u32);
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_state)]
    fn state(id: u32) -> i32;
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_error)]
    fn error(id: u32) -> String;
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_take)]
    fn take(id: u32) -> Uint8Array;
    #[wasm_bindgen(js_namespace = window, js_name = forge_video_cancel)]
    fn cancel(id: u32);
}

/// A video the browser is encoding.
pub struct Video {
    id: u32,
    /// Even sides, as H.264 has them.
    size: (u32, u32),
    fps: u32,
    pub frames: u32,
    pub path: String,
}

impl Video {
    /// None: this browser has no H.264 encoder.
    pub fn start(path: &str, size: (u32, u32), fps: u32) -> Option<Video> {
        let size = (size.0 + size.0 % 2, size.1 + size.1 % 2);
        let id = open(size.0, size.1, fps);
        (id != 0).then(|| Video {
            id,
            size,
            fps,
            frames: 0,
            path: path.into(),
        })
    }

    /// Whether the encoder can take another frame now (a few in hand at most).
    pub fn ready(&self) -> bool {
        backlog(self.id) < 4
    }

    pub fn push(&mut self, size: (u32, u32), rgba: &[u8]) -> Result<(), String> {
        if state(self.id) < 0 {
            return Err(format!("the browser's H.264 encoder: {}", error(self.id)));
        }
        let (s, px) = super::even(size, rgba);
        debug_assert_eq!(s, self.size);
        frame(self.id, Uint8Array::from(&px[..]), s.0, s.1, self.frames);
        self.frames += 1;
        Ok(())
    }

    /// Asks for what's still encoding; `done` then says when it's all back.
    pub fn flush(&self) {
        finish(self.id);
    }

    /// None while still encoding; then the MP4, or why not.
    pub fn done(&self) -> Option<Result<Vec<u8>, String>> {
        match state(self.id) {
            0 => None,
            1 => {
                let b = take(self.id).to_vec();
                Some(
                    super::unpack_frames(&b).and_then(|(samples, avcc)| {
                        super::mp4::mux(self.size, self.fps as f64, &samples, &avcc)
                    }),
                )
            }
            _ => Some(Err(format!("the browser's H.264 encoder: {}", error(self.id)))),
        }
    }

    pub fn cancel(self) {
        cancel(self.id);
    }
}
