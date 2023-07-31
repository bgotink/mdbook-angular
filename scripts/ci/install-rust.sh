#!/usr/bin/env bash
# Install/update rust.
# The first argument should be the toolchain to install.

set -ex
if [ -z "$1" ]
then
    echo "First parameter must be toolchain to install."
    exit 1
fi
TOOLCHAIN="$1"

rustup set profile minimal
rustup component remove --toolchain=$TOOLCHAIN rust-docs || echo "already removed"
rustup update --no-self-update $TOOLCHAIN
if [ -n "$2" ]
then
    TARGET="$2"
    HOST=$(rustc -Vv | grep ^host: | sed -e "s/host: //g")
    if [ "$HOST" != "$TARGET" ]
    then
        rustup component add llvm-tools-preview --toolchain=$TOOLCHAIN
        rustup component add rust-std-$TARGET --toolchain=$TOOLCHAIN
    fi

    case "$TARGET" in
        aarch64-unknown-linux-musl)
            MUSL_DOWNLOAD=https://musl.cc/aarch64-linux-musl-cross.tgz
            ;;
        x86_64-unknown-linux-musl)
            MUSL_DOWNLOAD=https://musl.cc/x86_64-linux-musl-native.tgz
            ;;
    esac

    if [ -n "$MUSL_DOWNLOAD" ]
    then
        curl -SsL "$MUSL_DOWNLOAD" | sudo tar -xvzC /usr/local --strip-components 1
    fi
fi

rustup default $TOOLCHAIN
rustup -V
rustc -Vv
cargo -V
