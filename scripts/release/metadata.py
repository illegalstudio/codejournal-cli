#!/usr/bin/env python3
"""Generate checksums and package-manager metadata from the release archives."""
import hashlib
import json
import pathlib
import re
import sys

PLATFORMS = ["macos-arm64", "macos-x64", "linux-arm64", "linux-x64", "windows-x64"]
REPOSITORY = "illegalstudio/codejournal-cli"


def generate(version, directory):
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("release version must be a stable semantic version")
    digests = {}
    for platform in PLATFORMS:
        suffix = ".zip" if platform.startswith("windows") else ".tar.gz"
        name = f"codejournal-cli-v{version}-{platform}{suffix}"
        digests[name] = hashlib.sha256((directory / name).read_bytes()).hexdigest()
    base = f"https://github.com/{REPOSITORY}/releases/download/v{version}"
    formula = [
        "class CodejournalCli < Formula",
        '  desc "Project memory and work history for developers and coding agents"',
        '  homepage "https://codejournal.online"',
        f'  version "{version}"',
        '  license "MIT"',
    ]
    for os_name, asset_os in [("macos", "macos"), ("linux", "linux")]:
        formula.append(f"  on_{os_name} do")
        for arch, asset_arch in [("arm", "arm64"), ("intel", "x64")]:
            name = f"codejournal-cli-v{version}-{asset_os}-{asset_arch}.tar.gz"
            formula.extend([
                f"    on_{arch} do",
                f'      url "{base}/{name}"',
                f'      sha256 "{digests[name]}"',
                "    end",
            ])
        formula.append("  end")
    formula.extend([
        "  def install",
        '    bin.install "cj"',
        "  end",
        "  test do",
        '    assert_match version.to_s, shell_output("#{bin}/cj --version")',
        "  end",
        "end",
    ])
    (directory / "codejournal-cli.rb").write_text("\n".join(formula) + "\n")
    windows = f"codejournal-cli-v{version}-windows-x64.zip"
    scoop = {
        "version": version,
        "description": "Project memory and work history for developers and coding agents",
        "homepage": "https://codejournal.online",
        "license": "MIT",
        "architecture": {"64bit": {"url": f"{base}/{windows}", "hash": digests[windows]}},
        "bin": "cj.exe",
        "checkver": {"github": f"https://github.com/{REPOSITORY}"},
        "autoupdate": {"architecture": {"64bit": {
            "url": f"https://github.com/{REPOSITORY}/releases/download/v$version/codejournal-cli-v$version-windows-x64.zip",
            "hash": {"url": f"https://github.com/{REPOSITORY}/releases/download/v$version/SHA256SUMS"},
        }}},
    }
    (directory / "codejournal-cli.json").write_text(json.dumps(scoop, indent=2) + "\n")
    for name in ["codejournal-cli.rb", "codejournal-cli.json"]:
        digests[name] = hashlib.sha256((directory / name).read_bytes()).hexdigest()
    (directory / "SHA256SUMS").write_text("".join(f"{digest}  {name}\n" for name, digest in sorted(digests.items())))


if __name__ == "__main__":
    generate(sys.argv[1], pathlib.Path(sys.argv[2]))
