//! Video natively: ffmpeg as a separate program (a pinned public build put beside the app by the
//! installers, research/render/record.md), the frames piped in as raw RGBA, H.264 out in MP4.
//! Nothing of ffmpeg is linked; without it, recordings are PNG frames and the caller says so.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

/// Where ffmpeg is: `FORGE_FFMPEG`, else beside the running app (where the installers put it),
/// else on the PATH.
pub fn locate() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if let Some(p) = std::env::var_os("FORGE_FFMPEG").map(PathBuf::from) {
        return p.is_file().then_some(p);
    }
    if let Some(p) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join(exe)))
        .filter(|p| p.is_file())
    {
        return Some(p);
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(exe))
            .find(|p| p.is_file())
    })
}

/// The H.264 encoder this ffmpeg has, best first: x264 (the bundled GPL builds carry it), then
/// openh264, then the platform's own, else MPEG-4 part 2.
pub fn encoder(ffmpeg: &std::path::Path) -> Result<&'static str, String> {
    let out = Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .map_err(|e| format!("{}: {e}", ffmpeg.display()))?;
    let list = String::from_utf8_lossy(&out.stdout);
    let has = |name: &str| list.lines().any(|l| l.split_whitespace().nth(1) == Some(name));
    for e in ["libx264", "libopenh264", "h264_videotoolbox", "h264_mf", "mpeg4"] {
        if has(e) {
            return Ok(e);
        }
    }
    Err(format!("{} has no H.264 or MPEG-4 encoder", ffmpeg.display()))
}

/// A video being written: ffmpeg running, taking frames on its standard input.
pub struct Video {
    child: Child,
    stdin: Option<ChildStdin>,
    size: (u32, u32),
    pub path: String,
    pub encoder: &'static str,
    pub frames: u32,
}

impl Video {
    /// Starts ffmpeg writing `path` (MP4) from `size` RGBA frames at `fps`.
    pub fn start(ffmpeg: &std::path::Path, path: &str, size: (u32, u32), fps: u32) -> Result<Video, String> {
        let encoder = encoder(ffmpeg)?;
        // H.264 wants even sides; ffmpeg pads by a pixel when they aren't
        let pad = "pad=ceil(iw/2)*2:ceil(ih/2)*2,format=yuv420p";
        let mut args: Vec<String> = [
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
        ]
        .map(String::from)
        .to_vec();
        args.extend(["-s".into(), format!("{}x{}", size.0, size.1)]);
        args.extend(["-r".into(), fps.to_string(), "-i".into(), "-".into()]);
        args.extend(["-vf".into(), pad.into(), "-c:v".into(), encoder.into()]);
        match encoder {
            // no B-frames and a key frame a second: every frame findable, any player
            "libx264" => args.extend(["-bf", "0", "-crf", "18"].map(String::from)),
            "libopenh264" => args.extend(["-b:v", "8M"].map(String::from)),
            "mpeg4" => args.extend(["-q:v", "3"].map(String::from)),
            _ => args.extend(["-b:v", "8M"].map(String::from)),
        }
        args.extend(["-g".into(), fps.to_string()]);
        args.extend(["-movflags", "+faststart", "-f", "mp4"].map(String::from));
        args.push(path.into());
        let mut child = Command::new(ffmpeg)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{}: {e}", ffmpeg.display()))?;
        let stdin = child.stdin.take();
        Ok(Video {
            child,
            stdin,
            size,
            path: path.into(),
            encoder,
            frames: 0,
        })
    }

    /// One frame, RGBA8 rows from the top, the size it started with.
    pub fn push(&mut self, rgba: &[u8]) -> Result<(), String> {
        if rgba.len() != (self.size.0 * self.size.1 * 4) as usize {
            return Err(format!(
                "a frame of {} bytes for {} x {}",
                rgba.len(),
                self.size.0,
                self.size.1
            ));
        }
        let s = self.stdin.as_mut().ok_or("ffmpeg has stopped")?;
        s.write_all(rgba)
            .map_err(|e| format!("ffmpeg stopped taking frames: {e}"))?;
        self.frames += 1;
        Ok(())
    }

    /// Ends the video and waits for ffmpeg to write it: the file, or ffmpeg's error.
    pub fn finish(mut self) -> Result<String, String> {
        drop(self.stdin.take());
        let out = self.child.wait_with_output().map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!(
                "ffmpeg: {}",
                String::from_utf8_lossy(&out.stderr)
                    .lines()
                    .last()
                    .unwrap_or("failed")
            ));
        }
        Ok(self.path)
    }

    /// Stops without a file.
    pub fn cancel(mut self) {
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.path);
    }
}

/// What to tell someone with no ffmpeg: the frames were kept, and how to make the video.
pub fn no_ffmpeg_note(frames_dir: &str, fps: u32) -> String {
    format!(
        "no ffmpeg found (install it with forgecad-update, or set FORGE_FFMPEG): the frames are in \
         {frames_dir}; make the video with: ffmpeg -framerate {fps} -i {frames_dir}/frame_%05d.png \
         -c:v libx264 -pix_fmt yuv420p video.mp4"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_few_frames_become_an_mp4() {
        let Some(ff) = locate() else {
            eprintln!("skipped: no ffmpeg");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fg-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.mp4").to_string_lossy().into_owned();
        let (w, h) = (65, 47);
        let mut v = Video::start(&ff, &path, (w, h), 10).unwrap();
        for k in 0..12u32 {
            let f: Vec<u8> = (0..w * h)
                .flat_map(|i| [(i * 3 + k * 20) as u8, (i / w * 5) as u8, (k * 21) as u8, 255])
                .collect();
            v.push(&f).unwrap();
        }
        assert!(v.push(&[0; 3]).is_err());
        let enc = v.encoder;
        let out = v.finish().unwrap();
        let b = std::fs::read(&out).unwrap();
        assert_eq!(&b[4..8], b"ftyp", "an MP4 by {enc}");
        // ffprobe, where it is beside ffmpeg, counts the frames
        let probe = ff.with_file_name(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
        if probe.is_file() {
            let o = Command::new(probe)
                .args(["-v", "error", "-count_frames", "-select_streams", "v:0"])
                .args([
                    "-show_entries",
                    "stream=nb_read_frames,codec_name",
                    "-of",
                    "csv=p=0",
                    &out,
                ])
                .output()
                .unwrap();
            let line = String::from_utf8_lossy(&o.stdout).trim().to_string();
            assert!(line.ends_with(",12"), "{line}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
