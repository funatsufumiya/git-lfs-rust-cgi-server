# git-lfs-rust-cgi-server

> [!WARNING]
> ***WORK-IN-PROGRESS***

## Deploy

```bash
$ cargo build --target x86_64-unknown-linux-musl --release
$ ls target/x86_64-unknown-linux-musl/release/git-lfs-rust-cgi-server
# target/x86_64-unknown-linux-musl/release/git-lfs-rust-cgi-server

# then copy target/x86_64-unknown-linux-musl/release/git-lfs-rust-cgi-server into your server as git-lfs-rust.cgi
```
