//! Photos and recordings of any environment (research/render/record.md): the muxers here are
//! plain functions of encoded frames, the same natively and in the browser; the encoding is the
//! bundled ffmpeg's (native, a subprocess) or the browser's own (WebCodecs).

#[cfg(not(target_arch = "wasm32"))]
pub mod ffmpeg;
pub mod mp4;
pub mod zip;
