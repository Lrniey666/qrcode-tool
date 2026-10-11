# User manual · 使用說明書

The complete user manual for QRcode tool 1.0, with screenshots of every window in the language of the manual.

| Language | PDF | Word |
|---|---|---|
| English | [QRcode-tool-User-Manual-en.pdf](QRcode-tool-User-Manual-en.pdf) | [QRcode-tool-User-Manual-en.docx](QRcode-tool-User-Manual-en.docx) |
| 繁體中文 | [QRcode-tool-User-Manual-zh-TW.pdf](QRcode-tool-User-Manual-zh-TW.pdf) | [QRcode-tool-User-Manual-zh-TW.docx](QRcode-tool-User-Manual-zh-TW.docx) |

The same files are attached to every [GitHub release](https://github.com/Lrniey666/qrcode-tool/releases/latest).

**Contents:** introduction · installation and first start · scanning the screen · webcam, clipboard,
image files, right-click menu and command line · results and detail cards · link safety ·
floating toolbar (horizontal or vertical) and bubble · creating QR codes · settings ·
updating and uninstalling · troubleshooting · shortcuts and supported codes.

完整的 QRcode tool 1.0 使用說明書，內含各視窗以對應語言顯示的畫面。內容包括：認識 QRcode tool、
安裝與第一次使用、掃描螢幕上的條碼、其他掃描方式、掃描結果與詳細資料、連結安全檢查、
浮動工具列（橫式或直式）與泡泡、建立 QR Code、設定、更新與解除安裝、疑難排解，以及快速鍵與條碼格式附錄。

## Updating the manual

The text lives in [`build/content.en.js`](build/content.en.js) and
[`build/content.zh-TW.js`](build/content.zh-TW.js). The screenshots are not edited by hand: they
are rendered from the app's real `.slint` files (with the translations from `app/translations`)
by the Slint interpreter for Node.js, so they always match the current UI.

On Windows with Node.js 20+ and Microsoft Word installed:

```powershell
powershell -ExecutionPolicy Bypass -File docs\manual\build\build.ps1
```

This bakes the UI in each language, takes the screenshots (windows appear on screen for about a
minute), draws the cover, writes the `.docx` files and exports the PDFs through Word, which also
fills in the table of contents. Use `-SkipShots` to rebuild only the documents from existing
screenshots.

| Step | Script |
|---|---|
| Copy the UI with translations filled in | `bake.js` |
| Render windows with demo data and capture them | `shoot.js`, `capture.cs`, `docscenes/` |
| Cover and back-cover artwork (SVG) | `art.js` |
| Crop, round and shadow the figures | `figures.js` |
| Word document | `build-docx.js` |
| Table of contents and PDF export | `to-pdf.ps1` |
