#!/bin/zsh

# link: https://betterprogramming.pub/cross-compiling-rust-from-mac-to-linux-7fad5a454ab1

cd scheduler && CROSS_COMPILE=x86_64-unknown-linux-gnu- cargo build --release --target x86_64-unknown-linux-gnu && cd ../
cd seqnum && CROSS_COMPILE=x86_64-unknown-linux-gnu- cargo +nightly build --release --target x86_64-unknown-linux-gnu && cd ../
cd api && CROSS_COMPILE=x86_64-unknown-linux-gnu- cargo build --release --target x86_64-unknown-linux-gnu && cd ../
cd msglogger && CROSS_COMPILE=x86_64-unknown-linux-gnu- cargo +nightly build --release --target x86_64-unknown-linux-gnu && cd ../
cd message && CROSS_COMPILE=x86_64-unknown-linux-gnu- cargo build --release --target x86_64-unknown-linux-gnu && cd ../