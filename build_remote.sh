#!/bin/bash
cd "$(dirname "$0")"
cargo build --features log --target x86_64-unknown-linux-musl --release && \
    cp target/x86_64-unknown-linux-musl/release/git-lfs-rust-cgi-server target/x86_64-unknown-linux-musl/release/git-lfs-rust.cg