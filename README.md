# git-lfs-rust-cgi-server

## Usage

place `git-lfs-rust.cgi` into your server, then set `.lfsconfig` like this:

```toml
[lfs]
url = "https://your-server/git-lfs-rust.cgi/owner/repo"
```

then, your server works as LFS server!

> [!WARNING]
> Currently don't have any credential support.<br>
> Please consider basic auth or something, and **DO NOT USE FOR SOMETHING IMPORTANT** (just use for hobby).

## Deploy (Install)

Please consider to use [Releases](https://github.com/funatsufumiya/git-lfs-rust-cgi-server/releases) prebuilt binaries first.

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
$ curl http://localhost:8000/cgi-bin/git-lfs-rust.cgi
```
