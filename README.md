# vzdv

![lang](https://img.shields.io/badge/lang-rust-orange)
![licensing](https://img.shields.io/badge/license-MIT_or_Apache_2.0-blue)
![status](https://img.shields.io/badge/project_status-in_dev-red)
![CI](https://github.com/Celeo/vzdv/actions/workflows/ci.yml/badge.svg)

New vZDV website. Completely in-dev and unfinished.

## Project goals

TBD

## Building

### Requirements

- Git
- A recent version of [Rust](https://www.rust-lang.org/tools/install)
- You'll likely need some openssl-dev packages on your system

### Steps

```sh
git clone https://github.com/Celeo/vzdv
cd vzdv
cargo build
```

This app follows all [Clippy](https://doc.rust-lang.org/clippy/) lints on _Nightly Rust_. You can use either both a stable and nightly toolchain, or just a nightly (probably; I use the dual setup). If using both, execute clippy with `cargo +nightly clippy`. You do not need this for _running_ the app, just developing on it.

## Running

From the project root, you can run `cargo run` to start the app. If you build and export a binary (`cargo b --release`, ...), just execute the binary.

You'll need to create a configuration file. A sample file is supplied [here](./site_config.sample.toml). You can put this file anywhere on the system and point to it with the `--config <path>` flag; if the file is in the same directory as the binary and named "site_config.toml", you do not need to supply that flag.

Additional CLI parameters can be found by running the app with the `--help` flag.

## Deploying

This app makes few assertions about how it should be ran. You can run it directly, put it into a systemd unit file, run in a Docker container, etc. You _will_ need to have this app behind some sort of reverse proxy that provides HTTPS, like [Caddy](https://caddyserver.com/).

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

## Contributing

This repo is happily FOSS, but isn't likely to accept contributions from others right now, given the specific and targeted use-case.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
