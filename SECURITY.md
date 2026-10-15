# Security Policy

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

Use GitHub's private reporting instead:
**[Report a vulnerability](https://github.com/naritaroad/qrcode-tool/security/advisories/new)**
(Security tab › *Report a vulnerability*).

Please include the app version, your OS, steps to reproduce and — if relevant — the QR code
or image that triggers the problem. You can expect a first answer within 7 days. Fixed issues
are credited in the release notes unless you prefer otherwise.

## Supported versions

Only the latest release receives security fixes. The app updates itself (installer version);
portable copies must be replaced manually.

## Scope

In scope, for example:

- A QR code that makes the app open something other than an `http`/`https` link, run a
  program, or pass data to a shell.
- A link that bypasses the warnings (e.g. a look-alike domain shown as safe, a disguised host
  shown with the wrong site name, bidi/zero-width tricks in the displayed text).
- Automatic opening of a link that should have required confirmation.
- Sensitive data (2-step verification secrets, Wi-Fi passwords) written to disk or logs.
- Crashes or hangs caused by crafted images or payloads.
- Problems in the update mechanism.

Out of scope: the content of websites a user decides to open, social engineering that the app
already warns about, and issues in third-party libraries that are already public (please report
those upstream; we will update quickly).

## How the app protects you

- Only `http` and `https` links can be opened; other schemes are shown and can be copied.
- Links never open automatically unless they are the only code found, have no warnings and the
  site is on the user's trusted list.
- Warnings for: unencrypted `http`, `user@host` disguises, international (punycode) domains,
  mixed alphabets, raw IP addresses, unusual ports, short links, very long or heavily encoded
  URLs, deep subdomain chains. All checks run offline.
- Displayed text is cleaned of control, bidi and zero-width characters.
- No telemetry, no online link lookups. The only network request is the optional update check.
