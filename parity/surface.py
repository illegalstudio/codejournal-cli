"""Command inventory and exact identity of the independently evolving oracle."""
import hashlib
import importlib.util
import json
import subprocess
import sys
from runtime import ROOT


def check(reference):
    path = ROOT / "scripts/check-cli-surface.py"
    spec = importlib.util.spec_from_file_location("cli_surface", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    report = module.audit([sys.executable, str(reference)], [str(ROOT / "apps/cli/target/debug/cj")])
    report["reference_sha256"] = hashlib.sha256(reference.read_bytes()).hexdigest()
    commit = subprocess.run(["git", "-C", str(reference.parent), "rev-parse", "HEAD"], capture_output=True, text=True)
    report["reference_commit"] = commit.stdout.strip() if commit.returncode == 0 else None
    print(json.dumps({key: value for key, value in report.items() if key != "shared_commands"},
                     indent=2, sort_keys=True), flush=True)
    print(f"Shared command paths checked: {len(report['shared_commands'])}", flush=True)
    if report["missing_commands"] or report["missing_flags"]:
        raise SystemExit("Shared Python commands or flags are missing")
