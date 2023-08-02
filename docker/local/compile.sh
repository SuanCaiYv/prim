#!/bin/zsh

cd ../../server || exit

cd scheduler && cargo build --release && cd ../
cd api && cargo build --release && cd ../
cd msglogger && cargo +nightly build --release && cd ../
cd message && cargo build --release && cd ../
cd seqnum && cargo +nightly build --release && cd ../