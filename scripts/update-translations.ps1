# Regenerates the translation template from the .slint files and merges it into every .po.
# Requires: cargo install slint-tr-extractor; gettext for Windows (msgmerge), e.g. `winget install mlocati.GetText`.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..\app")
$files = Get-ChildItem -Recurse -Filter *.slint ui | Sort-Object FullName | ForEach-Object { $_.FullName }
slint-tr-extractor -d qrcode-tool -o translations/qrcode-tool.pot @files
Get-ChildItem translations/*/LC_MESSAGES/qrcode-tool.po | ForEach-Object {
    msgmerge --quiet --update --backup=none --no-fuzzy-matching $_.FullName translations/qrcode-tool.pot
}
python ../scripts/check_translations.py
