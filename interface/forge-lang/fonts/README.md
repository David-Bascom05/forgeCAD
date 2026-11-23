# Shipped fonts

Embedded in forge-lang (`src/fonts.rs`) for sketch text, by family and style.

| file | family, style | version | source | licence |
|---|---|---|---|---|
| OpenSans-Regular.ttf, OpenSans-Bold.ttf | Open Sans, Regular and Bold | 1.10 (Steve Matteson, 2011; TrueType outlines) | npm `open-sans-fontface@1.4.0` (the Google Fonts release) | Apache License 2.0: LICENSE-OpenSans-Apache-2.0.txt |
| SourceSans3-Regular.otf | Source Sans 3, Regular | Adobe release branch (CFF outlines) | github.com/adobe-fonts/source-sans, release/OTF | SIL Open Font License 1.1: OFL-SourceSans3.md |

Open Sans 1.10 is the version Onshape sets sketch text in (`OpenSans-Regular.ttf`,
`OpenSans-Bold.ttf`): an imported part's glyphs are then the same curves. Both licences allow the
fonts to be embedded in and redistributed with software; the files are unmodified (Source Sans 3
carries the Reserved Font Name "Source", which only binds modified versions).

SHA-256:

```
91ce513fa6945f376012dc9f5b1eb3d020f3eb4a510a6efae6083a2a1f1d82a7  OpenSans-Bold.ttf
52bab2720136d9c479b90581eeb14ed7dd92f73dd11f76c08ce0eba13c2f2821  OpenSans-Regular.ttf
08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6  SourceSans3-Regular.otf
```
