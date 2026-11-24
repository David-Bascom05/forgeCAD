# Puts the LGPL ffmpeg ForgeCAD records video with (tools/ffmpeg/PIN, built by
# .github/workflows/ffmpeg.yml) in ROOT\bin beside forge-app.exe, with its licences and THIRD-PARTY
# note. Checks the pinned sha256; keeps what is there when the pinned build is already installed.
# Returns $true when installed or already there, $false (with the reason) otherwise; never throws.
#   powershell -File tools\fetch-ffmpeg.ps1 -Root DIR
param([Parameter(Mandatory = $true)][string]$Root)
$ErrorActionPreference = 'Stop'
try {
    $pin = @{}
    foreach ($line in Get-Content (Join-Path $PSScriptRoot 'ffmpeg\PIN')) {
        if ($line -match '^([A-Z0-9_]+)=(.*)$') { $pin[$Matches[1]] = $Matches[2] }
    }
    $releases = if ($env:FORGECAD_FFMPEG_RELEASES) { $env:FORGECAD_FFMPEG_RELEASES } else { 'https://github.com/David-Bascom05/forgeCAD/releases/download' }
    if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { Write-Warning "fetch-ffmpeg: no ForgeCAD ffmpeg build for $env:PROCESSOR_ARCHITECTURE"; return $false }
    $sha = $pin['ASSET_WINDOWS_X86_64_SHA256']
    $tag = "$($pin['FFMPEG_VERSION'])-$($pin['BUILD'])"
    $bin = Join-Path $Root 'bin'
    $stamp = Join-Path $bin 'ffmpeg.pin'
    if ((Test-Path (Join-Path $bin 'ffmpeg.exe')) -and (Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq "$tag-$sha")) {
        Write-Host "fetch-ffmpeg: ffmpeg $tag already installed"; return $true
    }
    if (-not $sha) { Write-Warning "fetch-ffmpeg: no ffmpeg $tag published for windows-x86_64 yet"; return $false }
    $name = "ffmpeg-$tag-windows-x86_64"
    $url = "$releases/ffmpeg-$tag/$name.zip"
    $work = Join-Path ([IO.Path]::GetTempPath()) ("forgecad-ffmpeg-" + [Guid]::NewGuid())
    New-Item -ItemType Directory $work | Out-Null
    try {
        $zip = Join-Path $work 'a.zip'
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $zip
        $got = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
        if ($got -ne $sha) { Write-Warning "fetch-ffmpeg: $url`: sha256 $got, pinned $sha; not installed"; return $false }
        Expand-Archive $zip -DestinationPath $work
        $s = Join-Path $work $name
        New-Item -ItemType Directory -Force $bin | Out-Null
        foreach ($f in 'ffmpeg.exe', 'ffprobe.exe') { Copy-Item (Join-Path $s "bin\$f") (Join-Path $bin $f) -Force }
        Copy-Item (Join-Path $s 'THIRD-PARTY.md') (Join-Path $bin 'ffmpeg-THIRD-PARTY.md') -Force
        foreach ($l in Get-ChildItem (Join-Path $s 'LICENSE.*')) { Copy-Item $l.FullName (Join-Path $bin "ffmpeg-$($l.Name)") -Force }
        Set-Content $stamp "$tag-$sha"
        Write-Host "fetch-ffmpeg: installed ffmpeg $tag (LGPL, with OpenH264) in $bin"
        return $true
    } finally { Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue }
} catch {
    Write-Warning "fetch-ffmpeg: $($_.Exception.Message)"
    return $false
}
