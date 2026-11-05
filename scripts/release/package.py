#!/usr/bin/env python3
"""Create one portable binary release archive."""
import argparse
import hashlib
import json
import pathlib
import subprocess
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
    helper = None
    sources = []
    if args.platform.startswith("linux"):
        helper = json.loads(subprocess.check_output([str(binary.resolve()), "__cj_bundled_bwrap_info"], text=True))
        if not helper["available"]:
            raise ValueError("Linux release binaries must embed the sandbox helper")
        expected_arch = "x86_64" if args.platform == "linux-x64" else "aarch64"
        if helper["architecture"] != expected_arch:
            raise ValueError("Linux release architecture mismatch")
        pinned = json.loads(pathlib.Path("scripts/sandbox/sources.json").read_text())
        if helper["version"] != pinned["bubblewrap"]["version"]:
            raise ValueError("Linux release helper version mismatch")
        for component in (pinned["bubblewrap"], pinned["libcap"]):
            source = pathlib.Path("target/bundled-bwrap/sources") / component["archive"]
            algorithm = "sha256" if "sha256" in component else "sha512"
            if hashlib.new(algorithm, source.read_bytes()).hexdigest() != component[algorithm]:
                raise ValueError("Bundled helper source checksum mismatch")
            sources.append(source)
    with tarfile.open(args.output / f"{name}.tar.gz", "w:gz") as archive:
        archive.add(binary, arcname="cj")
        archive.add("LICENSE", arcname="LICENSE")
        if helper:
            archive.add("licenses", arcname="third-party/licenses")
            for source in sources:
                archive.add(source, arcname=f"third-party/sources/{source.name}")
            for source in pathlib.Path("scripts/sandbox").iterdir():
                if source.is_file():
                    archive.add(source, arcname=f"third-party/build/{source.name}")
