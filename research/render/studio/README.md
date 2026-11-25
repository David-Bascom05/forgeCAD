# Render Studio renders

Made with `forge-studio` (interface/forge-render, `cargo build --release --bin forge-studio`), on
llvmpipe (software Vulkan, 4 CPU cores, about 2 M samples/s): no GPU. Lighting: Poly Haven HDRIs
(CC0), 1k, fetched from https://polyhaven.com (not in the repository yet; see studio.md §5).
Design: ../studio.md.

| File | What | Command (from interface/forge-render) |
|---|---|---|
| 01-crank-aluminium.png | bench crank, "Aluminium, machined", studio_small_09 (Sergej Majboroda) | `forge-studio crank --look "Aluminium, machined" --hdri studio_small_09_1k.hdr --size 1280x800 --spp 256 -o ..` |
| 02-rack-assembly.png | examples/rack18u/asm/rack_nopanels.fg, five finishes by part, photo_studio_loft_hall (Sergej Majboroda) | `--look "Aluminium, machined;Steel, black oxide;Aluminium, anodised black;Stainless steel, brushed;Powder coat" --pitch 15 --spp 256` |
| 03-tray-mixed-finishes.png | examples/sheetmetal/tray_asm.fg, bead-blasted aluminium, black oxide, powder coat | `--look "Aluminium, bead-blasted;Steel, black oxide;Powder coat;Stainless steel, brushed" --spp 256` |
| 04-appearance-swatches.png | twelve of the 36 built-in appearances on the crank, 400x250, 128 spp each, denoised | `--look NAME --size 400x250 --spp 128` |
| 05-turntable-brass.png | eight frames of `--turntable 8`, brass, empty_warehouse_01 (Sergej Majboroda) | `--look Brass --turntable 8 --size 320x200 --spp 64` |

Every picture is denoised (the studio's own variance-guided a-trous filter) unless it says not.
| 06-render-studio-environment.png | the Render Studio environment in the app (native, headless, llvmpipe), 1400x860: tray_asm.fg in bead-blasted aluminium and black oxide under the bundled HDRI, 255 samples denoised in the live view | `STUDIO_SHOTS=DIR cargo test --release --test studio_env shots` |
| 07-render-studio-environment-1024x700.png | the same at 1024x700 | the same |

08-web-render-studio.png: the Render Studio environment in the browser (WebGPU, SwiftShader), the flange at 135 samples, denoised.

09-render-progress.png, 10-render-result.png: Render Image answers at once (David: "no visible
feedback"): from the next frame a card over the view with the progress (percent, samples, elapsed,
an estimate of what's left) and Cancel, the button saying "Rendering…"; at the end the picture with
its file (Open folder, Copy path, Close) and "Render saved: PATH" in the status bar; a failure says
why (tests/studio_model.rs render_image_shows_its_progress_at_once_then_its_result). A file named
alone (render.png) goes beside the document.
