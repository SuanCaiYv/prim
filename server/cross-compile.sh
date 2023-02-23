#!/bin/zsh

# This script is used to cross-compile the project from macOS to Linux. Windows users can't use this script
# setup before run
#brew install FiloSottile/musl-cross/musl-cross
#ln -s /usr/local/bin/x86_64-linux-musl-gcc /usr/local/bin/musl-gcc
# append in ~/.cargo/config.toml
#[target.x86_64-unknown-linux-musl]
#linker = "x86_64-linux-musl-gcc"

cd scheduler && CROSS_COMPILE=x86_64-linux-musl- cargo build --release --target x86_64-unknown-linux-musl && cd ../
cd recorder && CROSS_COMPILE=x86_64-linux-musl- cargo build --release --target x86_64-unknown-linux-musl && cd ../
cd api && CROSS_COMPILE=x86_64-linux-musl- cargo build --release --target x86_64-unknown-linux-musl && cd ../
cd message && CROSS_COMPILE=x86_64-linux-musl- cargo build --release --target x86_64-unknown-linux-musl && cd ../