#!/usr/bin/env python3
import hashlib
import os

for root, dirs, files in os.walk("src"):
    dirs[:] = [d for d in dirs if d not in ("__pycache__",)]
    for name in files:
        if not name.endswith(".py"):
            continue
        path = os.path.join(root, name)
        content = open(path, "rb").read()
        text = content.decode()
        if name.startswith("test_") or ">>>" in text:
            h = hashlib.sha256(content).hexdigest()
            print(f"{path} {h}")
