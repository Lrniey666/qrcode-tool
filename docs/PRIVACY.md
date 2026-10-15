# Privacy Statement

*Last updated: 2026-10-05*

QRcode tool is designed to work entirely on your computer.

## What the app does **not** do

- No account, no sign-in.
- No telemetry, analytics, crash reporting or advertising.
- No online lookups of the links you scan (no reputation services, no short-link expansion).
- Screenshots and camera frames are kept in memory only while a scan window is open and are
  never saved or uploaded.

## What is stored on your computer

| Data | When | Where |
|---|---|---|
| Settings | always | `settings.toml` in the app's configuration folder |
| Scan history | **only if you turn it on** (off by default) | `history.jsonl` in the app's data folder, up to the number of entries you choose |
| Logs | always | app's log folder, 5 days; events and errors only, **never** scanned content |

Wi-Fi passwords and 2-step verification (OTP) keys are never written to history or logs and are
never copied to the clipboard automatically. You can clear the history at any time in
*Settings › Privacy & history*. Uninstalling removes the program; settings and history folders
can be deleted by hand (*Settings › About › Open the settings folder*).

## Network access

The only network connection the app makes on its own is the **update check**: it asks GitHub
(`github.com` / `api.github.com`) whether a newer release exists, at start-up and once a day.
GitHub receives the usual request data (such as your IP address) under
[GitHub's privacy statement](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement).
You can turn this off in *Settings › General › Check for updates automatically*.

Opening a link you scanned, connecting to a Wi-Fi network or writing an email happens only when
you choose to, through your browser or system apps.

## Permissions

- **Screen recording** (macOS, Wayland portals) — to scan the screen when you ask.
- **Camera** — only while the camera window is open.
- **Start at login** — on by default so the shortcut always works; can be turned off.

## Contact

Questions: open an issue at <https://github.com/naritaroad/qrcode-tool/issues>.
