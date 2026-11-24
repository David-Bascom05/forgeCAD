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
| 06-render-studio-environment.png | the Render Studio environment in the app (branch render/studio-ui, on ui/environments): its ribbon, the browser (appearance per part, lighting, camera, output) and the view traced live, here 89 samples denoised on llvmpipe | `STUDIO_SHOTS=DIR cargo test --release --test studio_env` |
