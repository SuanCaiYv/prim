#!/bin/zsh

cd ../../server || exit

cd scheduler && cross build --release --target x86_64-unknown-linux-gnu && cd ../
cd api && cross build --release --target x86_64-unknown-linux-gnu && cd ../
cd msglogger && cross +nightly build --release --target x86_64-unknown-linux-gnu && cd ../
cd message && cross build --release --target x86_64-unknown-linux-gnu && cd ../
cd seqnum && cross +nightly build --release --target x86_64-unknown-linux-gnu && cd ../