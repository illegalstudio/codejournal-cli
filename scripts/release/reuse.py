#!/usr/bin/env python3
"""Resume tap publication from an immutable release at the same source commit."""
import json
import os
import pathlib
import subprocess
import sys

repository = "illegalstudio/codejournal-cli"
version, sha, directory = sys.argv[1:]
tag = f"v{version}"
release = subprocess.run(
    ["gh", "release", "view", tag, "--repo", repository, "--json", "isDraft"],
    capture_output=True, text=True,
)
reused = False
if release.returncode == 0 and not json.loads(release.stdout)["isDraft"]:
    ref = json.loads(subprocess.check_output(["gh", "api", f"repos/{repository}/git/ref/tags/{tag}"], text=True))
    if ref["object"]["type"] != "commit" or ref["object"]["sha"] != sha:
        raise ValueError("published release belongs to a different source commit; bump the version")
    subprocess.run(["gh", "release", "download", tag, "--repo", repository,
                    "--pattern", "codejournal-cli-*.tar.gz", "--pattern", "codejournal-cli-*.zip",
                    "--dir", directory, "--clobber"], check=True)
    reused = True
with pathlib.Path(os.environ["GITHUB_OUTPUT"]).open("a") as output:
    output.write(f"reused={str(reused).lower()}\n")
