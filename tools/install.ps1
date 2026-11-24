# Install the native app, the CLI and speech on Windows, without administrator rights.
#   powershell -ExecutionPolicy Bypass -File tools\install.ps1 [-Root DIR]
# Needs Rust (rustup, the MSVC toolchain) and the Visual Studio Build Tools' C++ workload.
param([string]$Root = $(if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }))
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

function Install-Bin($path, $bin) {
    cargo install --locked --path $path --bin $bin --root $Root --force
    if ($LASTEXITCODE -ne 0) { throw "cargo install $bin failed" }
}

Install-Bin 'interface/forge-render' 'forge-app'
Install-Bin 'interface/forge-cli' 'forgecad'
if (Test-Path 'interface/forge-speech/Cargo.toml') { Install-Bin 'interface/forge-speech' 'forge-speech' }

$bin = Join-Path $Root 'bin'
Copy-Item 'tools/update.ps1' (Join-Path $bin 'forgecad-update.ps1') -Force
Copy-Item 'tools/setup-speech.ps1' (Join-Path $bin 'forgecad-setup-speech.ps1') -Force
# ffmpeg for recording video; a failed fetch leaves the install as it is
if ((Test-Path 'tools/fetch-ffmpeg.ps1') -and -not (& (Join-Path $repo 'tools/fetch-ffmpeg.ps1') -Root $Root)) {
    Write-Warning 'No ffmpeg installed: recording saves PNG frames until forgecad-update succeeds.'
}

# a Start menu entry for the app
$programs = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut((Join-Path $programs 'ForgeCAD.lnk'))
$link.TargetPath = Join-Path $bin 'forge-app.exe'
$link.WorkingDirectory = $env:USERPROFILE
$link.Save()

Write-Host "`nInstalled ForgeCAD in $bin, with a Start menu entry."
Write-Host "Run forgecad to open the app, or forgecad open FILE. Ensure $bin is on PATH."
