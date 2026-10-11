# Opens each .docx in Microsoft Word, fills in the table of contents, saves it and exports a
# PDF with bookmarks for every heading. Requires Microsoft Word (Windows).
# Usage: powershell -File to-pdf.ps1 <file.docx>...
param([Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)][string[]]$Files)
$ErrorActionPreference = "Stop"

$word = New-Object -ComObject Word.Application
$word.Visible = $false
$word.DisplayAlerts = 0
try {
    foreach ($f in $Files) {
        $docx = (Resolve-Path $f).Path
        $pdf = [IO.Path]::ChangeExtension($docx, ".pdf")
        $doc = $word.Documents.Open($docx, $false, $false, $false)
        try {
            # Twice: the first pass inserts the entries, the second fixes page numbers that
            # moved because the contents grew.
            for ($i = 0; $i -lt 2; $i++) {
                foreach ($toc in $doc.TablesOfContents) { $toc.Update() }
                $doc.Fields.Update() | Out-Null
                $doc.Repaginate()
            }
            $doc.Save()
            # 17 = PDF, 0 = optimise for print, 0 = whole document, 0 = content only,
            # 1 = bookmarks from headings.
            $doc.ExportAsFixedFormat($pdf, 17, $false, 0, 0, 1, 1, 0, $true, $true, 1, $true, $true, $false)
            Write-Output ("{0} -> {1} pages" -f (Split-Path $pdf -Leaf), $doc.ComputeStatistics(2))
        } finally {
            $doc.Close($false)
        }
    }
} finally {
    $word.Quit()
    [void][Runtime.InteropServices.Marshal]::ReleaseComObject($word)
}
