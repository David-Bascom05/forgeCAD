# Puts the ffmpeg ForgeCAD records video with (tools/ffmpeg/PIN: a public prebuilt GPL build, pinned
# by URL and sha256) in ROOT\bin beside forge-app.exe, with the build's licence and
# ffmpeg-THIRD-PARTY.md. Keeps what is there when the pinned build is already installed.
# Returns $true when installed or already there, $false (with the reason) otherwise; never throws.
#   powershell -File tools\fetch-ffmpeg.ps1 -Root DIR
param([Parameter(Mandatory = $true)][string]$Root)
$ErrorActionPreference = 'Stop'
try {
    $pin = @{}
    foreach ($line in Get-Content (Join-Path $PSScriptRoot 'ffmpeg\PIN')) {
        if ($line -match '^([A-Z0-9_]+)="?(.*?)"?$') { $pin[$Matches[1]] = $Matches[2] }
    }
    if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { Write-Warning "fetch-ffmpeg: no ffmpeg pinned for $env:PROCESSOR_ARCHITECTURE; recording uses your own (FORGE_FFMPEG or PATH)"; return $false }
    $p = 'WINDOWS_X86_64'
    $url = $pin["${p}_URL"]; $sha = $pin["${p}_SHA256"]; $version = $pin["${p}_VERSION"]
    if (-not $url -or -not $sha) { Write-Warning 'fetch-ffmpeg: no ffmpeg pinned for Windows'; return $false }
    $bin = Join-Path $Root 'bin'
    $stamp = Join-Path $bin 'ffmpeg.pin'
    if ((Test-Path (Join-Path $bin 'ffmpeg.exe')) -and (Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq $sha)) {
        Write-Host "fetch-ffmpeg: ffmpeg $version already installed"; return $true
    }
    $work = Join-Path ([IO.Path]::GetTempPath()) ("forgecad-ffmpeg-" + [Guid]::NewGuid())
    New-Item -ItemType Directory $work | Out-Null
    try {
        $zip = Join-Path $work 'a.zip'
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $zip
        $got = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
        if ($got -ne $sha) { Write-Warning "fetch-ffmpeg: $url`: sha256 $got, pinned $sha; not installed"; return $false }
        $x = Join-Path $work 'x'
        Expand-Archive $zip -DestinationPath $x
        $from = Join-Path $x $pin["${p}_BIN"]
        New-Item -ItemType Directory -Force $bin | Out-Null
        foreach ($f in 'ffmpeg.exe', 'ffprobe.exe') { Copy-Item (Join-Path $from $f) (Join-Path $bin $f) -Force }
        Copy-Item (Join-Path $x $pin["${p}_LICENSE"]) (Join-Path $bin 'ffmpeg-LICENSE.txt') -Force
        $note = @"
# ffmpeg, beside ForgeCAD

ffmpeg $version, a GPL build by $($pin["${p}_BUILDER"]), as published:
$url (sha256 $sha). Its licence is ffmpeg-LICENSE.txt, next to this file.

ForgeCAD runs it as a separate program (frames over a pipe) to write videos and links none of it.
The source of this version: https://ffmpeg.org/releases/ffmpeg-$version.tar.xz (and the builder's
page above for the libraries it was built with).
"@
        Set-Content (Join-Path $bin 'ffmpeg-THIRD-PARTY.md') $note
        Set-Content $stamp $sha
        Write-Host "fetch-ffmpeg: installed ffmpeg $version in $bin"
        return $true
    } finally { Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue }
} catch {
    Write-Warning "fetch-ffmpeg: $($_.Exception.Message)"
    return $false
}
