#!/usr/bin/env python3
import hashlib
import json
import subprocess
import tempfile

with tempfile.NamedTemporaryFile(suffix=".json") as f:
    subprocess.run(
        ["uv", "run", "pytest", "--tb=no", "-q", "--json-report", f"--json-report-file={f.name}"],
        capture_output=True,
    )
    report = json.load(f)

seen = set()
for test in report.get("tests", []):
    if test["outcome"] != "failed":
        continue
    path = test["nodeid"].split("::")[0]
    if path in seen:
        continue
    seen.add(path)
    content = open(path, "rb").read()
    h = hashlib.sha256(content).hexdigest()
    print(f"{path} {h}")
