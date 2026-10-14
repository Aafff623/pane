"""Build optional font artifacts, never imported by the app bundler.

Run: python site/scripts/build-font-packs.py [version]
Sources and redistribution licenses are already tracked with the Ink skin.
The Chinese font is the existing UI subset, not a full CJK character set;
missing characters fall back to the app's system-font stack.
"""
import hashlib
import io
import json
from pathlib import Path
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "site/public/fonts"
SOURCE = ROOT / "src/assets/skins/ink-wash/fonts"
VERSION = sys.argv[1] if len(sys.argv) > 1 else "1.0.0"
if not VERSION or any(c not in "0123456789.-abcdefghijklmnopqrstuvwxyz" for c in VERSION):
    raise ValueError("Invalid version")

FONTS = [
    # No curated downloadable packs for now: the owner rejected the two
    # Google-font convenience picks (2026-10-14), and MV Boli — the
    # recommended font — ships with Windows and is read off the system font
    # list instead of being redistributed. Add an entry here when a pack is
    # actually wanted; the script then emits the zips and the catalog again.
]

OUT.mkdir(parents=True, exist_ok=True)
offers = []
for ident, name, family, filename, license_file, source_url, license_name in FONTS:
    files = {filename: (SOURCE / filename).read_bytes(), license_file: (SOURCE / license_file).read_bytes()}
    files["SOURCE.txt"] = (f"{name}\nUpstream: {source_url}\nLicense: {license_name}\nSource: existing Pane Ink skin assets; unmodified font bytes.\n").encode("utf-8")
    manifest = dict(id=ident, name=name, family=family, version=VERSION, min_app_version="0.8.2", entry=filename, license=license_file, faces=[dict(path=filename, weight=400, style="normal")], files=[dict(path=p, sha256=hashlib.sha256(data).hexdigest()) for p, data in files.items()])
    files["manifest.json"] = json.dumps(manifest, ensure_ascii=False, indent=2).encode("utf-8")
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path, data in sorted(files.items()):
            info = zipfile.ZipInfo(path, date_time=(2026, 10, 14, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data)
    data = buf.getvalue()
    zip_name = f"pane-font-{ident}-{VERSION}.zip"
    (OUT / zip_name).write_bytes(data)
    offers.append(dict(id=ident, name=name, family=family, version=VERSION, size=len(data), sha256=hashlib.sha256(data).hexdigest(), url=f"https://pane.threetwoa.live/fonts/{zip_name}", license=license_name, source=source_url))
    print(f"{zip_name}: {len(data)} bytes")
(OUT / "index.json").write_text(json.dumps(dict(packs=offers), ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
