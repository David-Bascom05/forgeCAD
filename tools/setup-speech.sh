#!/bin/sh
# Build the exact whisper.cpp ABI used by forge-speech; models stay outside the repo.
set -eu
mode=${1:-cpu}
case "$mode" in cpu|cuda|vulkan) ;; *) echo "usage: forgecad-setup-speech [cpu|cuda|vulkan]" >&2; exit 2;; esac
[ "$#" -le 1 ] || exit 2
speech_dir=${FORGECAD_SPEECH_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/forgecad/speech}
mkdir -p "$speech_dir"
speech_dir=$(CDPATH= cd -- "$speech_dir" && pwd)
build_dir=$(mktemp -d "${TMPDIR:-/tmp}/forgecad-speech.XXXXXXXX")
trap 'rm -rf -- "$build_dir"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
git clone --filter=blob:none --no-checkout https://github.com/ggml-org/whisper.cpp.git "$build_dir/source"
git -C "$build_dir/source" checkout --detach 727d3e7666485e12450746b523bfb4e7f057cc5e
build_libs() {
    backend=$1
    destination=$2
    cmake -S "$build_dir/source" -B "$build_dir/$backend" \
        -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=ON \
        -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=OFF \
        -DGGML_CUDA="$3" -DGGML_VULKAN="$4"
    cmake --build "$build_dir/$backend" --parallel "${FORGECAD_BUILD_JOBS:-2}"
    mkdir -p "$destination"
    find "$build_dir/$backend" -type f -name '*.so*' -exec cp -P {} "$destination/" \;
    find "$build_dir/$backend" -type l -name '*.so*' -exec cp -P {} "$destination/" \;
}
# A separate CPU build also works when GPU libraries cannot load.
build_libs cpu "$speech_dir/lib-cpu" OFF OFF
if [ "$mode" = cuda ]; then
    build_libs cuda "$speech_dir/lib" ON OFF
elif [ "$mode" = vulkan ]; then
    build_libs vulkan "$speech_dir/lib" OFF ON
fi
download() {
    model=$1
    checksum=$2
    target="$speech_dir/$model"
    if [ -f "$target" ] && printf '%s  %s\n' "$checksum" "$target" | sha256sum --check --status; then
        return
    fi
    curl --fail --location --retry 3 "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$model" -o "$build_dir/$model"
    printf '%s  %s\n' "$checksum" "$build_dir/$model" | sha256sum --check
    mv "$build_dir/$model" "$target"
}
download ggml-small.en.bin c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d
if [ "$mode" != cpu ]; then
    download ggml-large-v3-turbo-q5_0.bin 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2
fi
printf '\nSpeech installed in %s. Restart ForgeCAD to detect it.\n' "$speech_dir"
