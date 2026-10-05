# FAQ

## Using it

### How do I scan?
Press <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Q</kbd> (change it in *Settings › Shortcuts*) or
left-click the tray icon. Every code on the screen gets a label. Press <kbd>Enter</kbd> or click
the label to open it. <kbd>Tab</kbd> moves to the next code, <kbd>C</kbd> copies, <kbd>Esc</kbd>
closes. If a code is not found, drag a rectangle around it.

### Why didn't the link open right away?
Links with warnings (for example `http://`, short links or international domains) always show
the details first so you can check the real site. Only clean links on your *trusted sites*
list can open without a click (*Settings › Scanning*).

### Why does it say "Short link — the real destination is hidden"?
Services like bit.ly hide where a link really goes. QRcode tool does not look the link up
online (that would tell the service what you scanned). Open it only if you trust the source.

### Can it read codes from a video or a game?
Yes — the screen is frozen when you press the shortcut, so moving content is fine.

### Where is the main window?
There is none on purpose. Use the tray icon (right-click for the menu), the shortcut, the
floating toolbar (tray › *Floating toolbar*) or the small bubble.

### How do I stop it from starting with Windows?
*Settings › General › Start with the computer*. You can also disable it in
Task Manager › Startup apps.

### Can I use it from the right-click menu?
On Windows, right-click the desktop for **QRcode tool Scan screen**, **Scan selection**,
**Scan with camera** and **Toolbar**. Right-click a picture (PNG, JPEG, BMP, GIF or WebP)
and choose **QRcode tool Open image**. Turn the entries off in
*Settings › General › Right-click menu*. On Windows 11 they are under **Show more options**.
Uninstalling removes them.

### Is my data sent anywhere?
No. Decoding and link checks happen on your computer. The only network request is the
optional update check to GitHub. See [PRIVACY.md](PRIVACY.md).

## Windows

### Windows SmartScreen
Early versions are not code-signed yet, so Windows may show *"Windows protected your PC"*.
Click **More info → Run anyway**. Code signing (SignPath Foundation for open source projects)
and a Microsoft Store version are planned. You can check the SHA-256 checksum of the download
against the one in the release notes:

```powershell
Get-FileHash .\QRcodeTool-win-x64-Setup.exe -Algorithm SHA256
```

### My antivirus flags it
Unsigned new programs that register global shortcuts, read the screen and start with Windows
are sometimes flagged by heuristics. QRcode tool uses the standard `RegisterHotKey` API (it does
not hook the keyboard) and is open source, so you can build it yourself. Please report false
positives to your antivirus vendor and let us know in an issue.

### The shortcut doesn't work
Another app may already use it — Settings shows *"Already used by another app"*. Pick another
one in *Settings › Shortcuts*. Some games and remote-desktop clients capture all keys while
focused.

### Where are the settings and logs?
*Settings › About › Open the settings folder / Open the log folder*. Settings:
`%APPDATA%\Lrniey666\qrcode-tool\config`, logs: `%LOCALAPPDATA%\Lrniey666\qrcode-tool\data\logs`.

### Does it work on Windows 7/8.1?
No. Windows 10 21H2 or newer is required.

## macOS

### Nothing happens / the screenshot is black
Allow QRcode tool in *System Settings › Privacy & Security › Screen Recording*, then try again.
macOS may ask again after updates.

### "The app can't be opened because it is from an unidentified developer"
Right-click the app and choose **Open** once. Notarization is planned.

## Linux

### Wayland: the shortcut does nothing
Wayland does not allow apps to grab global shortcuts. Add a custom shortcut in your desktop
settings that runs `qrcode-tool scan-screen` (the running instance receives the command).

### I don't see the tray icon
GNOME needs the *AppIndicator and KStatusNotifierItem Support* extension.
