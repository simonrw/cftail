#!/bin/bash

set -eou pipefail

main() {
    apt update && apt install -y \
        clang \
        cmake
        # libssl-dev \
        # lzma-dev \
        # libxml2-dev  \
        # libgmp-dev \
        # libmpc-dev \
        # libmpfr-dev \
        # zlib1g-dev

    rustup target add x86_64-apple-darwin

    test -d osxcross || git clone https://github.com/tpoechtrager/osxcross

    test -f osxcross/target/bin/x86_64-apple-darwin14-clang || {
        (cd osxcross
        wget -nc https://s3.dockerproject.org/darwin/v2/MacOSX10.10.sdk.tar.xz
        mv MacOSX10.10.sdk.tar.xz tarballs/
        UNATTENDED=yes OSX_VERSION_MIN=10.7 ./build.sh
        )
    }

    export PATH=${PATH}:$(pwd)/osxcross/target/bin

    mkdir -p .cargo
    cat <<EOF > .cargo/config
[target.x86_64-apple-darwin]
linker = "x86_64-apple-darwin14-clang"
ar = "x86_64-apple-darwin14-ar"
EOF

    cargo build --target x86_64-apple-darwin --release
}

main