# Rebuilds the user manuals in docs/manual (Word + PDF, English and Traditional Chinese).
#
# Requirements: Windows, Node.js 20+, Microsoft Word, .NET Framework 4 (csc.exe, part of Windows).
# Screenshots are rendered from the app's real .slint files with the Slint interpreter, so no
# Rust build is needed. Windows appear on screen for a minute while screenshots are taken.
#
# Usage: powershell -ExecutionPolicy Bypass -File docs\manual\build\build.ps1 [-SkipShots]
param([switch]$SkipShots)
$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here "..\..\..")).Path
$manual = (Resolve-Path (Join-Path $here "..")).Path
$langs = @("en", "zh-TW")

Push-Location $here
try {
    if (-not (Test-Path node_modules)) { npm ci --no-audit --no-fund }
    & "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /nologo /target:exe /r:System.Drawing.dll /out:capture.exe capture.cs
    if ($LASTEXITCODE) { throw "capture.exe failed to compile" }

    # 1.5x scale gives screenshots sharp enough for print.
    $env:SLINT_SCALE_FACTOR = "1.5"
    if (-not $SkipShots) {
        foreach ($l in $langs) {
            node bake.js $repo baked $l docscenes; if ($LASTEXITCODE) { throw "bake $l" }
            node shoot.js $repo $l "shots\$l"; if ($LASTEXITCODE) { throw "shoot $l" }
        }
        node art.js; if ($LASTEXITCODE) { throw "art" }
    }
    foreach ($l in $langs) {
        node figures.js $l; if ($LASTEXITCODE) { throw "figures $l" }
        node build-docx.js $l $manual; if ($LASTEXITCODE) { throw "docx $l" }
    }
    & powershell -NoProfile -ExecutionPolicy Bypass -File to-pdf.ps1 `
        (Join-Path $manual "QRcode-tool-User-Manual-en.docx") `
        (Join-Path $manual "QRcode-tool-User-Manual-zh-TW.docx")
    if ($LASTEXITCODE) { throw "pdf export" }
} finally {
    Pop-Location
}
