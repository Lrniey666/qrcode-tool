# Contributing to QRcode tool

Thanks for helping! Bug reports, translations, documentation and code are all welcome.

## Ways to help

- **Report a bug** — open an [issue](https://github.com/naritaroad/qrcode-tool/issues/new/choose).
  Include your OS version, the app version (Settings › About) and, if possible, the log file
  (Settings › About › *Open the log folder*). Logs never contain scanned content.
- **A code isn't detected?** Attach the image (or a similar one without private data).
  Hard cases make the best test fixtures.
- **Translate** — see [docs/TRANSLATING.md](docs/TRANSLATING.md).
- **Security issues** — please report privately, see [SECURITY.md](SECURITY.md).

## Development setup

See [docs/BUILDING.md](docs/BUILDING.md) for the toolchain per platform. In short:

```bash
git clone https://github.com/naritaroad/qrcode-tool
cd qrcode-tool
cargo run              # debug build of the tray app
cargo test --workspace
```

Useful while developing:

```bash
RUST_LOG=debug cargo run                       # verbose log on stderr (debug builds)
cargo run -- decode path/to/image.png --json   # test decoding without the UI
cargo run -- scan-screen                       # talk to a running instance
```

## Pull requests

1. For larger changes, open an issue first so we can agree on the approach.
2. Keep the crate boundaries (see [AGENTS.md](AGENTS.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)):
   parsing and safety logic go into `qrtool-core` with unit tests; `app/` stays thin.
3. Before pushing:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   python3 scripts/check_translations.py
   ```
4. If you changed or added UI text, run `scripts/update-translations.sh` (or `.ps1`) and
   commit the updated `.pot`/`.po` files. English text only is fine — translators will follow.
5. Describe *what* and *why* in the PR. Screenshots help for UI changes (light and dark).

### Security-sensitive areas

Changes to `url_safety.rs`, `payload.rs`, `open.rs`, `wifi.rs` or the auto-open logic get an
extra careful review. The rules in [AGENTS.md](AGENTS.md#non-negotiable-rules-security--privacy)
must hold: only `http`/`https` is ever opened, nothing goes through a shell, no network calls
besides the update check, nothing sensitive is stored.

## Commit messages

Short imperative subject line (`Add Aztec code fixture`, `Fix pill position on 150 % displays`),
optionally followed by a blank line and details.

## License

By contributing you agree that your contributions are dual-licensed under
MIT OR Apache-2.0, like the rest of the project, without additional terms.
