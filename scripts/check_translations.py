#!/usr/bin/env python3
"""Checks every app/translations/<lang>/LC_MESSAGES/qrcode-tool.po against the template.

- every msgid of the template exists in the .po (same msgctxt)
- no empty translations (warning only)
- `{}` placeholders match the source exactly in number

Exit code 1 on errors. No dependencies besides the Python standard library.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent / "app" / "translations"
ENTRY = re.compile(r'(?:^msgctxt "(?P<ctx>(?:[^"\\]|\\.)*)"\n)?^msgid (?P<id>(?:".*"\n)+)^msgstr (?P<str>(?:".*"\n?)+)', re.M)


def unquote(block: str) -> str:
    return "".join(re.findall(r'"((?:[^"\\]|\\.)*)"', block))


def parse(path: pathlib.Path) -> dict:
    text = path.read_text(encoding="utf-8")
    out = {}
    for m in ENTRY.finditer(text):
        msgid = unquote(m.group("id"))
        if msgid == "":
            continue
        out[(m.group("ctx") or "", msgid)] = unquote(m.group("str"))
    return out


def main() -> int:
    template = parse(ROOT / "qrcode-tool.pot")
    errors = 0
    for po in sorted(ROOT.glob("*/LC_MESSAGES/qrcode-tool.po")):
        lang = po.parts[-3]
        entries = parse(po)
        missing = [k for k in template if k not in entries]
        empty = [k for k, v in entries.items() if not v]
        bad = [k for k, v in entries.items() if v and v.count("{}") != k[1].count("{}")]
        status = "ok"
        if missing or bad:
            status = "ERROR"
            errors += 1
        print(f"{lang:6} {len(entries):4} entries  missing={len(missing)} empty={len(empty)} placeholder-mismatch={len(bad)}  {status}")
        for k in missing[:10]:
            print(f"    missing: [{k[0]}] {k[1]}")
        for k in bad[:10]:
            print(f"    placeholders differ: [{k[0]}] {k[1]}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
