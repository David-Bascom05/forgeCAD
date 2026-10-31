#!/bin/sh
# Regenerates classify.ptx (needs nvcc; the driver JIT-compiles the PTX for whatever GPU it finds).
cd "$(dirname "$0")" && nvcc -ptx -arch=compute_90 -fmad=false -O3 classify.cu -o classify.ptx
