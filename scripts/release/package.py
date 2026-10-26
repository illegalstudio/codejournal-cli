#!/usr/bin/env python3
"""Create one portable binary release archive."""
import argparse
import pathlib
import tarfile
import tomllib
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument("target")
parser.add_argument("platform", choices=["linux-x64", "linux-arm64", "macos-x64", "macos-arm64", "windows-x64"])
parser.add_argument("--output", type=pathlib.Path, default=pathlib.Path("dist"))
args = parser.parse_args()
version = tomllib.loads(pathlib.Path("Cargo.toml").read_text())["package"]["version"]
windows = args.platform.startswith("windows")
binary = pathlib.Path("target") / args.target / "release" / ("cj.exe" if windows else "cj")
args.output.mkdir(parents=True, exist_ok=True)
name = f"codejournal-cli-v{version}-{args.platform}"
if windows:
    with zipfile.ZipFile(args.output / f"{name}.zip", "w", zipfile.ZIP_DEFLATED) as archive:
        archive.write(binary, "cj.exe")
        archive.write("LICENSE", "LICENSE")
else:
    with tarfile.open(args.output / f"{name}.tar.gz", "w:gz") as archive:
        archive.add(binary, arcname="cj")
        archive.add("LICENSE", arcname="LICENSE")
