# macOS packaging

The release workflow builds `QRcode tool.app` from `Info.plist` (version substituted), ad-hoc
signs it and packs a `.pkg` with Velopack. Notarization is not set up yet (needs an Apple
Developer account); users open the app once with right-click › Open.

Permissions: Screen Recording (asked on the first scan) and Camera (`NSCameraUsageDescription`).
