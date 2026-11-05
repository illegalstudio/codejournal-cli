#!/usr/bin/env python3
"""Build the pinned static Linux helper without changing host packages."""
import hashlib
import json
import os
import platform
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCES = json.loads((Path(__file__).with_name('sources.json')).read_text())
ARCH = platform.machine()
if platform.system() != 'Linux' or ARCH not in ('x86_64', 'aarch64'):
    raise SystemExit('The bundled helper is built on native Linux x64 or ARM64.')
OUTPUT = ROOT / 'target' / 'bundled-bwrap' / ARCH
OUTPUT.mkdir(parents=True, exist_ok=True)
cached_sources = OUTPUT.parent / 'sources'
cached_sources.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='helper-build-', dir=OUTPUT.parent) as temporary:
    work = Path(temporary)
    bubble = SOURCES['bubblewrap']
    subprocess.run(['gh', 'release', 'download', 'v'+bubble['version'], '--repo',
        'containers/bubblewrap', '--pattern', bubble['archive'], '--dir', str(work)], check=True)
    cap = SOURCES['libcap']
    with urllib.request.urlopen(cap['url'], timeout=60) as response:
        data = response.read(5 * 1024 * 1024)
    (work / cap['archive']).write_bytes(data)
    for component in (bubble, cap):
        archive = work / component['archive']
        algorithm = 'sha256' if 'sha256' in component else 'sha512'
        if hashlib.new(algorithm, archive.read_bytes()).hexdigest() != component[algorithm]:
            raise ValueError('Source checksum mismatch: '+component['archive'])
        shutil.copy2(archive, cached_sources / archive.name)
        with tarfile.open(archive) as source:
            source.extractall(work, filter='data')
    container_platform = 'linux/amd64' if ARCH == 'x86_64' else 'linux/arm64'
    command = ['docker', 'run', '--rm', '--platform', container_platform,
        '--mount', f'type=bind,source={work},target=/work',
        '--mount', f'type=bind,source={Path(__file__).with_name("container.sh")},target=/build.sh,readonly',
        '-e', f'CJ_BUILD_UID={os.getuid()}', '-e', f'CJ_BUILD_GID={os.getgid()}',
        '-e', f'BWRAP_VERSION={bubble["version"]}', '-e', f'LIBCAP_VERSION={cap["version"]}',
        SOURCES['builder_image'], '/bin/sh', '/build.sh']
    subprocess.run(command, check=True)
    binary = work / 'output/bwrap'
    data = binary.read_bytes()
    if data[:4] != b'\x7fELF':
        raise ValueError('Helper is not ELF')
    recipe = Path(__file__).read_bytes()+Path(__file__).with_name('container.sh').read_bytes()+Path(__file__).with_name('sources.json').read_bytes()
    metadata = {**SOURCES, 'architecture': ARCH, 'binary_sha256': hashlib.sha256(data).hexdigest(),
                'build_recipe_sha256': hashlib.sha256(recipe).hexdigest()}
    subprocess.run([str(binary), '--version'], check=True)
    shutil.copy2(binary, OUTPUT / 'bwrap')
    (OUTPUT / 'provenance.json').write_text(json.dumps(metadata, indent=2)+'\n')
    print('Bundled helper:', OUTPUT / 'bwrap')
