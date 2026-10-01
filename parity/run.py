"""Run real Python/Rust differential tests without user data or credentials."""
import argparse
import os
import pathlib
import subprocess
import unittest
from runtime import ROOT, Runtime

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--pattern", default="test_*.py")
parser.add_argument("--database-tests", action="store_true", help="Run the full PostgreSQL suite in the same disposable database")
args = parser.parse_args()

reference = pathlib.Path(os.environ.get("CJ_PYTHON_REFERENCE", str(pathlib.Path.home() /
    "Developer/nahime/ai/skills/me-ai-code-journal/scripts/code-journal-cli")))
if not reference.is_file():
    raise SystemExit("Set CJ_PYTHON_REFERENCE to the read-only Python reference")
subprocess.run(["cargo", "build", "--manifest-path", str(ROOT / "apps/cli/Cargo.toml"), "--locked"], check=True)
from surface import check
check(reference)
with Runtime() as runtime:
    import support
    support.RUNTIME = runtime
    support.REFERENCE = reference
    suite = unittest.defaultTestLoader.discover(str(pathlib.Path(__file__).parent), pattern=args.pattern)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if args.database_tests and result.wasSuccessful():
        subprocess.run(["npm", "run", "test:db"], cwd=ROOT,
                       env=dict(os.environ, DB_DATABASE=runtime.name), check=True)
raise SystemExit(0 if result.wasSuccessful() else 1)
