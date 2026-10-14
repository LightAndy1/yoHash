# yoHash

yoHash is a command-line tool for calculating file checksums to verify file integrity. It supports hashing multiple files and configurable digest formatting.

## Installation

Build and install the command-line tool from a local source checkout:

```sh
cargo install --path crates/yohash-cli
```

Cargo installs the `yohash` executable into its configured binary directory.

## CLI Usage

```sh
yohash <file>
yohash <file> -a md5
yohash <file> -f lowercase
yohash <file1> <file2>
```

Replace the file placeholders with file paths. A single file prints only its digest; multiple files print each path with its digest. The default algorithm is SHA-256, and the default output format is uppercase hexadecimal. The long option names `--alg` and `--format` are also available.

## Supported Algorithms

- MD5
- SHA-256

## Building

From the workspace root, build all crates in release mode:

```sh
cargo build --workspace --release
```

The CLI executable is written to `target/release/yohash`.

## License

yoHash is licensed under the BSD-2-Clause license.
