# git-lfs-rust-cgi-server

> [!WARNING]
> ***WORK-IN-PROGRESS***

## Deploy

> [!NOTE]
> If you can't cross compile on Windows, try WSL instead.

```bash
$ rustup target add x86_64-unknown-linux-musl
$ cargo build --target x86_64-unknown-linux-musl --release
$ ls target/x86_64-unknown-linux-musl/release/git-lfs-rust-cgi-server
# target/x86_64-unknown-linux-musl/release/git-lfs-rust-cgi-server

# then copy target/x86_64-unknown-linux-musl/release/git-lfs-rust-cgi-server into your server as git-lfs-rust.cgi
```

## Dev (test CGI on local)

```bash
$ mkdir cgi-bin
$ cargo build
$ cp target/debug/git-lfs-rust-cgi-server cgi-bin/git-lfs-rust.cgi
$ python3 -m http.server --cgi

# then, from other terminal window:
$ curl localhost:8000/cgi-bin/git-lfs-rust.cgi
```