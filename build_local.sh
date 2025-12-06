#!/bin/bash
cd "$(dirname "$0")"
mkdir -p cgi-bin
cargo build --features log && \ 
cp target/debug/git-lfs-rust-cgi-server cgi-bin/git-lfs-rust.cgi
