#!/usr/bin/env bash
# Regenerates the translation template from the .slint files and merges it into every .po.
# Requires: cargo install slint-tr-extractor; gettext (msgmerge).
set -euo pipefail
cd "$(dirname "$0")/../app"
find ui -name '*.slint' | sort | xargs slint-tr-extractor -d qrcode-tool -o translations/qrcode-tool.pot
for po in translations/*/LC_MESSAGES/qrcode-tool.po; do
  msgmerge --quiet --update --backup=none --no-fuzzy-matching "$po" translations/qrcode-tool.pot
done
python3 ../scripts/check_translations.py
