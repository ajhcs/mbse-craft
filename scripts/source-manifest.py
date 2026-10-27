#!/usr/bin/env python3
"""Hash exact tracked build inputs with portable relative names; no writes."""
import hashlib, json, pathlib, subprocess
root = pathlib.Path(__file__).resolve().parent.parent
paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=root).decode().split("\0")
paths = sorted(p for p in paths if p and (p in ("Cargo.toml", "Cargo.lock") or p.startswith(("crates/", "vendor/"))))
h = hashlib.sha256(); records = []
for name in paths:
    data = (root / name).read_bytes(); encoded = name.encode()
    h.update(len(encoded).to_bytes(8,"little")); h.update(encoded)
    h.update(len(data).to_bytes(8,"little")); h.update(data)
    records.append({"path":name,"bytes":len(data),"sha256":hashlib.sha256(data).hexdigest()})
print(json.dumps({"scope":"Tracked build manifests, crate source/tests/fixtures and selected vendored source/notices; excludes documentation and target outputs", "source_sha256":h.hexdigest(),"files":records},indent=2))
