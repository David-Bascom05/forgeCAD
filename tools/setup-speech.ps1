# Build the exact whisper.cpp ABI forge-speech uses, on Windows; models stay outside the repo.
#   forgecad-setup-speech.ps1 [cpu|cuda|vulkan]
# Needs git, CMake and the Visual Studio Build Tools' C++ workload (and the CUDA or Vulkan SDK
# for those builds).
param([ValidateSet('cpu', 'cuda', 'vulkan')][string]$Mode = 'cpu')
$ErrorActionPreference = 'Stop'
$cache = if ($env:XDG_CACHE_HOME) { $env:XDG_CACHE_HOME } else { $env:LOCALAPPDATA }
$speech = if ($env:FORGECAD_SPEECH_DIR) { $env:FORGECAD_SPEECH_DIR } else { Join-Path $cache 'forgecad\speech' }
New-Item -ItemType Directory -Force $speech | Out-Null
$build = Join-Path ([System.IO.Path]::GetTempPath()) ("forgecad-speech." + [guid]::NewGuid().ToString('N'))
$jobs = if ($env:FORGECAD_BUILD_JOBS) { $env:FORGECAD_BUILD_JOBS } else { 2 }
try {
    git clone --filter=blob:none --no-checkout https://github.com/ggml-org/whisper.cpp.git (Join-Path $build 'source')
    git -C (Join-Path $build 'source') checkout --detach 727d3e7666485e12450746b523bfb4e7f057cc5e
    if ($LASTEXITCODE -ne 0) { throw 'whisper.cpp checkout failed' }

    function Build-Libs($backend, $dest, $cuda, $vulkan) {
        $dir = Join-Path $build $backend
        cmake -S (Join-Path $build 'source') -B $dir -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=ON `
            -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=OFF "-DGGML_CUDA=$cuda" "-DGGML_VULKAN=$vulkan"
        if ($LASTEXITCODE -ne 0) { throw "cmake ($backend) failed" }
        cmake --build $dir --config Release --parallel $jobs
        if ($LASTEXITCODE -ne 0) { throw "build ($backend) failed" }
        New-Item -ItemType Directory -Force $dest | Out-Null
        Get-ChildItem -Recurse -Path $dir -Filter *.dll | Copy-Item -Destination $dest -Force
    }
    # a separate CPU build also works when GPU libraries cannot load
    Build-Libs 'cpu' (Join-Path $speech 'lib-cpu') 'OFF' 'OFF'
    if ($Mode -eq 'cuda') { Build-Libs 'cuda' (Join-Path $speech 'lib') 'ON' 'OFF' }
    if ($Mode -eq 'vulkan') { Build-Libs 'vulkan' (Join-Path $speech 'lib') 'OFF' 'ON' }

    function Get-Model($model, $sha) {
        $target = Join-Path $speech $model
        if ((Test-Path $target) -and ((Get-FileHash -Algorithm SHA256 $target).Hash -eq $sha.ToUpper())) { return }
        $tmp = Join-Path $build $model
        Invoke-WebRequest -UseBasicParsing "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$model" -OutFile $tmp
        if ((Get-FileHash -Algorithm SHA256 $tmp).Hash -ne $sha.ToUpper()) { throw "$model checksum mismatch" }
        Move-Item -Force $tmp $target
    }
    Get-Model 'ggml-small.en.bin' 'c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d'
    if ($Mode -ne 'cpu') {
        Get-Model 'ggml-large-v3-turbo-q5_0.bin' '394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2'
    }
    Write-Host "`nSpeech installed in $speech. Restart ForgeCAD to detect it."
} finally {
    if (Test-Path $build) { Remove-Item -Recurse -Force $build }
}
