#!/usr/bin/env python3
"""Build a stable Pages root and independent AutoTrace XLSX LAB subdirectory."""
import json
import re
import shutil
import sys
from pathlib import Path

def replace_one(source, old, new):
    if source.count(old) != 1:
        raise ValueError(f"Expected exactly one {old[:70]!r}, found {source.count(old)}")
    return source.replace(old, new)

def build(stable, lab, destination):
    for root in (stable, lab):
        for name in ("index.html","app.js","sw.js","manifest.webmanifest"):
            if not (root/name).is_file():
                raise FileNotFoundError(root/name)
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(stable,destination)
    copy=destination/"lab"
    shutil.copytree(lab,copy)
    f=copy/"app.js"
    f.write_text(replace_one(f.read_text(encoding="utf-8"),
        "const DB_NAME = 'autotrace-v1';", "const DB_NAME = 'autotrace-lab-v1';"),encoding="utf-8")
    f=copy/"sw.js"
    code=f.read_text(encoding="utf-8")
    code=replace_one(code,"const PREFIX = 'autotrace-shell-';","const PREFIX = 'autotrace-lab-shell-';")
    # Keep the LAB cache version synchronized with the source PWA without
    # hardcoding a single release's cache number into the build pipeline.
    cache_pattern=r"(?m)^const CACHE = 'autotrace-shell-v([0-9]+)';$"
    if len(re.findall(cache_pattern,code))!=1:
        raise ValueError("Expected exactly one versioned AutoTrace cache declaration")
    code=re.sub(cache_pattern,lambda m:f"const CACHE = 'autotrace-lab-shell-v{m.group(1)}';",code)
    f.write_text(code,encoding="utf-8")
    f=copy/"index.html"
    code=f.read_text(encoding="utf-8")
    code=replace_one(code,"<title>AutoTrace · Gestión de taller</title>",
        "<title>AutoTrace LAB · Pruebas Excel</title>")
    code=replace_one(code,"<body>",
      '<body><div role="status" style="position:relative;z-index:20;text-align:center;padding:8px 12px;'
      'background:#fde8b7;color:#503b14;font:600 14px system-ui">'
      'AUTOTRACE LAB · VERSIÓN DE PRUEBA · Datos separados de AutoTrace estable'
      '</div>')
    f.write_text(code,encoding="utf-8")
    f=copy/"manifest.webmanifest"
    manifest=json.loads(f.read_text(encoding="utf-8"))
    manifest.update({"id":"./","name":"AutoTrace LAB · Pruebas Excel",
       "short_name":"AutoTrace LAB","description":"Laboratorio aislado de importación y exportación Excel."})
    f.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    if not (copy/"vendor/xlsx.mini.min.js").is_file():
        raise ValueError("Excel vendor was not copied")
    if (destination/"app.js").read_bytes()!=(stable/"app.js").read_bytes():
        raise ValueError("The stable application was modified")
    if "const DB_NAME = 'autotrace-lab-v1';" not in (copy/"app.js").read_text():
        raise ValueError("Lab IndexedDB namespace is missing")
    if "const PREFIX = 'autotrace-lab-shell-';" not in (copy/"sw.js").read_text():
        raise ValueError("Lab CacheStorage namespace is missing")
    print("PASS: root stable byte-identical; lab IndexedDB/cache separate.")

if __name__=="__main__":
    if len(sys.argv)!=4: raise SystemExit("Usage: build-pages-preview.py STABLE_SITE LAB_SITE OUT")
    build(*(Path(p) for p in sys.argv[1:]))
