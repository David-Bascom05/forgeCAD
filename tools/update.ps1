# Build latest main in isolation and install it; never touches the user's checkout.
#   forgecad update   (or: powershell -ExecutionPolicy Bypass -File forgecad-update.ps1 [-Root DIR])
param([string]$Root = $(Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
$repository = if ($env:FORGECAD_REPOSITORY) { $env:FORGECAD_REPOSITORY } else { 'https://code.commonty.org/david/ForgeCAD.git' }
$checkout = Join-Path ([System.IO.Path]::GetTempPath()) ("forgecad-update." + [guid]::NewGuid().ToString('N'))
try {
    git clone --depth 1 --single-branch --branch main -- $repository (Join-Path $checkout 'source')
    if ($LASTEXITCODE -ne 0) { throw 'git clone failed' }
    Set-Location (Join-Path $checkout 'source')
    git log -1 --format='Installing ForgeCAD main %h (%cs)'
    & (Join-Path $checkout 'source/tools/install.ps1') -Root $Root
    Write-Host "`nUpdated ForgeCAD in $(Join-Path $Root 'bin')."
} finally {
    Set-Location $env:USERPROFILE
    if (Test-Path $checkout) { Remove-Item -Recurse -Force $checkout }
}
