#!/bin/sh
set -eu
if [ "${1:-}" != compile ]; then
    apk add --no-cache build-base linux-headers meson ninja pkgconf bash perl su-exec
    exec su-exec "$CJ_BUILD_UID:$CJ_BUILD_GID" /bin/sh /build.sh compile
fi
cd /work
export XDG_CACHE_HOME=/work/.cache
mkdir -p prefix/include/sys prefix/lib/pkgconfig output
make -C "libcap-$LIBCAP_VERSION/libcap" BUILD_CC=gcc CC=gcc GOLANG=no libcap.a
cp "libcap-$LIBCAP_VERSION/libcap/libcap.a" prefix/lib/
cp "libcap-$LIBCAP_VERSION/libcap/include/sys/capability.h" prefix/include/sys/
cat > prefix/lib/pkgconfig/libcap.pc <<EOF
prefix=/work/prefix
libdir=\${prefix}/lib
includedir=\${prefix}/include
Name: libcap
Description: pinned static POSIX capabilities
Version: $LIBCAP_VERSION
Libs: -L\${libdir} -lcap
Cflags: -I\${includedir}
EOF
export PKG_CONFIG_PATH=/work/prefix/lib/pkgconfig
export CFLAGS='-O2 -fstack-protector-strong -D_FORTIFY_SOURCE=2'
export LDFLAGS=-static
meson setup build "bubblewrap-$BWRAP_VERSION" --buildtype=release \
    -Dselinux=disabled -Dman=disabled -Dtests=false -Dprefer_static=true
meson compile -C build
cp build/bwrap output/bwrap
strip output/bwrap
if readelf -l output/bwrap | grep -q INTERP; then
    printf '%s\n' 'The bundled helper must not depend on a dynamic loader.' >&2
    exit 1
fi
