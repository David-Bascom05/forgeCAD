# Onshape exports

Parasolid files Onshape wrote (each file's header says `APPL=Onshape`), taken unchanged from
public repositories under the MIT licence, except the `_native` files. Those are the same
exports rewritten by `research/parasolid/native.py` into the form Onshape's translation service
writes: no embedded schema, the schema named by number only.

| File | Source | Schema |
|---|---|---|
| `extrude.x_t` | github.com/Hk-Hacker-Harsh/Onshape-Mouse, `Top - Part 2 (Eye Red 1).x_t` | SCH_3700209_36001 |
| `extrude_native.x_t` | the same, rewritten | SCH_3700209_36001 |
| `skull.x_t` | github.com/Hk-Hacker-Harsh/Onshape-Mouse, `Top - Part 1 (White Skull).x_t` | SCH_3700209_36001 |
| `box.x_t` | github.com/SimScaleGmbH/simscale-python-sdk, `examples/fixtures/flow-around-sphere-v3.x_t` | SCH_3300229_32001 |
| `pipe_native.x_t` | the same repository, `examples/fixtures/pipe_junction_model_tutorial.x_t`, rewritten | SCH_3300229_32001 |
| `housing.x_t` | github.com/betterengineering/open-keyless, `assets/reader/housing/back/reader-housing-back.x_t` | SCH_3100249_31001 |

Licences (MIT; each repository's notice, with the MIT permission text):

- Onshape-Mouse: Copyright (c) 2025 Hk Hacker
- simscale-python-sdk: Copyright 2023 SimScale GmbH
- open-keyless: Copyright (c) 2019 Mark Spicer

> Permission is hereby granted, free of charge, to any person obtaining a copy of this software
> and associated documentation files (the "Software"), to deal in the Software without
> restriction, including without limitation the rights to use, copy, modify, merge, publish,
> distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
> Software is furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or
> substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
> BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
> NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
> DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
