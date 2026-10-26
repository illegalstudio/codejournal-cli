#!/usr/bin/env python3
"""Publish a release without replacing assets from a completed publication."""
import json
import pathlib
import re
import subprocess
import sys
import tempfile

REPOSITORY = "illegalstudio/codejournal-cli"


def gh(*args):
    return subprocess.check_output(["gh", *args], text=True)


def publish(version, sha, directory):
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("invalid stable version")
    tag = f"v{version}"
    existing = subprocess.run(
        ["gh", "release", "view", tag, "--repo", REPOSITORY, "--json", "isDraft"],
        capture_output=True, text=True,
    )
    if existing.returncode == 0:
        ref = json.loads(gh("api", f"repos/{REPOSITORY}/git/ref/tags/{tag}"))
        if ref["object"]["type"] != "commit" or ref["object"]["sha"] != sha:
            raise ValueError("existing release tag points to a different commit")
        if not json.loads(existing.stdout)["isDraft"]:
            with tempfile.TemporaryDirectory(prefix="cj-release-") as temporary:
                gh("release", "download", tag, "--repo", REPOSITORY, "--pattern", "SHA256SUMS", "--dir", temporary)
                if (pathlib.Path(temporary) / "SHA256SUMS").read_bytes() != (directory / "SHA256SUMS").read_bytes():
                    raise ValueError("published release assets are immutable; bump the version")
            print("The same release is already published; continuing tap publication.")
            return
    else:
        gh("release", "create", tag, "--repo", REPOSITORY, "--target", sha, "--draft",
           "--title", f"Code Journal CLI {version}", "--notes",
           "Install with `brew install illegalstudio/tap/codejournal-cli` or "
           f"`mise use -g github:illegalstudio/codejournal-cli@{version}`. "
           "Archives include the cj binary and MIT license. Verify downloads with SHA256SUMS; "
           "GitHub artifact attestations record build provenance.")
    gh("release", "upload", tag, *map(str, sorted(directory.iterdir())), "--repo", REPOSITORY, "--clobber")
    gh("release", "edit", tag, "--repo", REPOSITORY, "--draft=false", "--latest")


if __name__ == "__main__":
    publish(sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3]))
