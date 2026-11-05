# Bundled Linux sandbox components

Linux release binaries embed an unmodified Bubblewrap executable built with static libcap. Code Journal itself remains MIT licensed. The bundled components retain their upstream licenses and copyright notices.

- Bubblewrap 0.13.1: see `bubblewrap.txt`. Source: https://github.com/containers/bubblewrap/releases/tag/v0.13.1
- libcap 2.78: see `libcap.txt`. Source: https://kernel.org/pub/linux/libs/security/linux-privs/libcap2/libcap-2.78.tar.xz

Linux download archives include both complete upstream source archives, their checksums and the helper build recipe under `third-party/`. The pinned source manifest is `scripts/sandbox/sources.json` in the public CLI repository. When the embedded helper is extracted for use, its license notices are written alongside the executable in the private helper cache.

The static helper also uses the musl C runtime and GCC runtime support. Their upstream notices are included as `musl.txt`, `gcc-gpl-3.0.txt` and `gcc-runtime-exception.txt`. Source notices: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT?h=v1.2.5 and https://github.com/gcc-mirror/gcc/tree/releases/gcc-14.2.0. The GCC runtime exception is included with its accompanying license text.
