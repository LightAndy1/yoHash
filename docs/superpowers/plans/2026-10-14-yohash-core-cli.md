# yoHash Core + CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the yoHash Rust workspace with a testable `yohash-core` hash engine and a working `yohash-cli` binary.

**Architecture:** Cargo workspace with `crates/yohash-core` (pure library: hash algorithm trait + registry, output formatting, memory-mapped/streaming file I/O) and `crates/yohash-cli` (argument parsing + output, thin wrapper over the core). Parallelism is across files with `rayon`; each file is read and hashed sequentially through one hasher. New algorithms are added by implementing one trait and registering it.

**Tech Stack:** Rust (latest stable), `md-5`, `sha2`, `rayon`, `memmap2`, `thiserror`, `base64`, `clap` (CLI only), `assert_cmd`/`predicates` (CLI tests).

**Spec:** `docs/superpowers/specs/2026-10-14-yohash-design.md`

## Global Constraints

- License: BSD-2-Clause.
- Toolchain: latest stable Rust. No MSRV policy.
- No comments in code, anywhere, including tests.
- Commit messages: short, human, junior-dev style. No Conventional Commits prefixes ("feat:", "fix:"). No emojis. Examples: `added sha512 support`, `fixed clipboard bug on win11`.
- Default algorithm SHA-256; default output uppercase hex.
- No network access, no telemetry, no data collection.
- Performance: 1 GB file under 5 s; CLI process start under 50 ms.
- Multi-file output line format: `filename: HASH`, one per line.
- **Deviation from spec (approved design change):** the spec's "split into 4 MB chunks and combine" is only valid for tree hashes. For MD5/SHA-256 the plan hashes each file as one sequential stream and parallelizes across files instead.

## Review Focus

These are the input classes most likely to bite, each owned by a task's tests:

1. Empty (0-byte) file must return the well-known digest of empty input, not an error. (Task 6 and Task 7)
2. A file whose size exceeds the mmap threshold, and any mmap open failure, must transparently use the streaming path with an identical digest. (Task 6)
3. A missing file, a permission-denied file, and a directory passed as input must produce a clear error and CLI exit code 1, never a panic. (Task 2 and Task 8)
4. In multi-file mode a single failing file must not abort the batch, and output order must match input order. (Task 7 and Task 9)
5. Filenames containing spaces and non-ASCII characters must round-trip unchanged into `filename: HASH` output. (Task 9)

---

### Task 1: Workspace scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/yohash-core/Cargo.toml`
- Create: `crates/yohash-core/src/lib.rs`
- Create: `rustfmt.toml`
- Create: `.gitignore`
- Create: `LICENSE`
- Create: `README.md`

**Interfaces:**
- Consumes: nothing.
- Produces: a workspace that `cargo build` accepts, with member crates `yohash-core`.

- [ ] **Step 1: Create workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/yohash-core"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "BSD-2-Clause"
repository = "https://github.com/OWNER/yoHash"

[workspace.dependencies]
md-5 = "0.10"
sha2 = "0.10"
rayon = "1.10"
memmap2 = "0.9"
thiserror = "2"
base64 = "0.22"
```

Replace `OWNER` with the user's GitHub account when known.

- [ ] **Step 2: Create `crates/yohash-core/Cargo.toml`**

```toml
[package]
name = "yohash-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
md-5.workspace = true
sha2.workspace = true
rayon.workspace = true
memmap2.workspace = true
thiserror.workspace = true
base64.workspace = true
```

- [ ] **Step 3: Create `crates/yohash-core/src/lib.rs`**

```rust
pub mod algorithm;
pub mod engine;
pub mod error;
pub mod format;
```

Create empty `crates/yohash-core/src/error.rs`, `format.rs`, `algorithm.rs`, `engine.rs` so the module declarations resolve.

- [ ] **Step 4: Create `rustfmt.toml`**

```toml
edition = "2021"
```

- [ ] **Step 5: Create `.gitignore`**

```
/target
```

- [ ] **Step 6: Create `LICENSE` with the standard BSD-2-Clause text** (copyright `2026, the yoHash authors`).

- [ ] **Step 7: Create `README.md` with a one-line description** (`yoHash — multi-platform file integrity verification tool.`) and a `## Building` section listing `cargo build --release`. It will be completed in the last task.

- [ ] **Step 8: Verify the workspace builds**

Run: `cargo build`
Expected: compiles with no errors.

- [ ] **Step 9: Commit**

```bash
git init
git add Cargo.toml rustfmt.toml .gitignore LICENSE README.md crates/
git commit -m "started the yohash workspace"
```

---

### Task 2: Core error type

**Files:**
- Modify: `crates/yohash-core/src/error.rs`
- Test: inline `#[cfg(test)]` module in `crates/yohash-core/src/error.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `HashError` enum with variants `NotFound { path: PathBuf }`, `PermissionDenied { path: PathBuf }`, `NotAFile { path: PathBuf }`, `Io(std::io::Error)`, used by every later task.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn not_found_message_names_the_path() {
        let e = HashError::NotFound { path: PathBuf::from("a.txt") };
        assert!(e.to_string().contains("a.txt"));
    }

    #[test]
    fn not_a_file_message_names_the_path() {
        let e = HashError::NotAFile { path: PathBuf::from("dir") };
        assert!(e.to_string().contains("dir"));
    }

    #[test]
    fn io_error_is_convertible_from_std_io_error() {
        let src = std::io::Error::new(std::io::ErrorKind::Other, "boom");
        let e: HashError = src.into();
        assert!(matches!(e, HashError::Io(_)));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yohash-core error`
Expected: FAIL — `HashError` is not defined.

- [ ] **Step 3: Implement `HashError` in `crates/yohash-core/src/error.rs`**

Derive `Debug` and `thiserror::Error`. Messages: `NotFound` → `"{path}: file not found"`, `PermissionDenied` → `"{path}: permission denied"`, `NotAFile` → `"{path}: not a regular file"`, `Io` → `"{0}"`. Add `#[from]` on `Io`. `path` fields use `std::path::PathBuf`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p yohash-core error`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/yohash-core/src/error.rs
git commit -m "added the hash error type"
```

---

### Task 3: Output formatting

**Files:**
- Modify: `crates/yohash-core/src/format.rs`
- Test: inline `#[cfg(test)]` module in `crates/yohash-core/src/format.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `OutputFormat` enum (`UpperHex`, `LowerHex`, `Base64`) with `Default` = `UpperHex`; method `fn render(&self, digest: &[u8]) -> String`; associated `fn parse(s: &str) -> Option<OutputFormat>` accepting `upper`/`uppercase`/`upperhex`, `lower`/`lowercase`/`lowerhex`, `base64`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_uppercase_hex() {
        assert_eq!(OutputFormat::default(), OutputFormat::UpperHex);
    }

    #[test]
    fn renders_upper_hex() {
        assert_eq!(OutputFormat::UpperHex.render(&[0xde, 0xad, 0xbe, 0xef]), "DEADBEEF");
    }

    #[test]
    fn renders_lower_hex() {
        assert_eq!(OutputFormat::LowerHex.render(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn renders_base64() {
        assert_eq!(OutputFormat::Base64.render(&[0xde, 0xad, 0xbe, 0xef]), "3q2+7w==");
    }

    #[test]
    fn parses_aliases_case_insensitively() {
        assert_eq!(OutputFormat::parse("UPPER"), Some(OutputFormat::UpperHex));
        assert_eq!(OutputFormat::parse("lowerhex"), Some(OutputFormat::LowerHex));
        assert_eq!(OutputFormat::parse("Base64"), Some(OutputFormat::Base64));
        assert_eq!(OutputFormat::parse("nope"), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yohash-core format`
Expected: FAIL — `OutputFormat` is not defined.

- [ ] **Step 3: Implement `OutputFormat` in `crates/yohash-core/src/format.rs`**

Derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`. `UpperHex`/`LowerHex` render with manual `format!("{:02X}")`/`format!("{:02x}")` over bytes. `Base64` uses `base64::engine::general_purpose::STANDARD.encode`. `parse` lowercases input and matches the alias list.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p yohash-core format`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/yohash-core/src/format.rs
git commit -m "added output formatting"
```

---

### Task 4: Hash algorithm trait, MD5, SHA-256

**Files:**
- Modify: `crates/yohash-core/src/algorithm.rs`
- Test: inline `#[cfg(test)]` module in `crates/yohash-core/src/algorithm.rs`

**Interfaces:**
- Consumes: `HashError` from Task 2.
- Produces: `pub trait HashAlgorithm: Send + Sync { fn name(&self) -> &'static str; fn digest_reader(&self, reader: &mut dyn std::io::Read) -> Result<Vec<u8>, HashError>; }`; unit structs `Md5`, `Sha256`; fn `hash_with<D: sha2::digest::Digest>(reader: &mut dyn std::io::Read) -> Result<Vec<u8>, HashError>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn digest(alg: &dyn HashAlgorithm, data: &[u8]) -> String {
        let mut r = Cursor::new(data);
        alg.digest_reader(&mut r)
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn md5_empty_vector() {
        assert_eq!(digest(&Md5, b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn md5_abc_vector() {
        assert_eq!(digest(&Md5, b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn sha256_empty_vector() {
        assert_eq!(
            digest(&Sha256, b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc_vector() {
        assert_eq!(
            digest(&Sha256, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn names_are_stable() {
        assert_eq!(Md5.name(), "MD5");
        assert_eq!(Sha256.name(), "SHA-256");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yohash-core algorithm`
Expected: FAIL — `HashAlgorithm`, `Md5`, `Sha256` are not defined.

- [ ] **Step 3: Implement the trait and both algorithms in `crates/yohash-core/src/algorithm.rs`**

`hash_with` reads the reader in 64 KiB chunks into a `Vec<u8>`, feeds each chunk to `D::update`, and returns `D::finalize().to_vec()`. `Md5` uses `md5::Md5`; `Sha256` uses `sha2::Sha256`. Each `digest_reader` delegates to `hash_with::<D>`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p yohash-core algorithm`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/yohash-core/src/algorithm.rs
git commit -m "added md5 and sha256 support"
```

---

### Task 5: Algorithm registry

**Files:**
- Modify: `crates/yohash-core/src/algorithm.rs`
- Test: inline `#[cfg(test)]` module additions in `crates/yohash-core/src/algorithm.rs`

**Interfaces:**
- Consumes: `HashAlgorithm`, `Md5`, `Sha256` from Task 4.
- Produces: `pub fn algorithms() -> &'static [&'static dyn HashAlgorithm]`; `pub fn algorithm_by_name(name: &str) -> Option<&'static dyn HashAlgorithm>`; `pub const DEFAULT_ALGORITHM: &str = "SHA-256";`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn registry_lists_md5_and_sha256() {
    let names: Vec<&str> = algorithms().iter().map(|a| a.name()).collect();
    assert_eq!(names, vec!["MD5", "SHA-256"]);
}

#[test]
fn lookup_is_case_and_alias_insensitive() {
    assert_eq!(algorithm_by_name("sha-256").unwrap().name(), "SHA-256");
    assert_eq!(algorithm_by_name("sha256").unwrap().name(), "SHA-256");
    assert_eq!(algorithm_by_name("MD5").unwrap().name(), "MD5");
    assert!(algorithm_by_name("blake3").is_none());
}

#[test]
fn default_algorithm_resolves() {
    assert!(algorithm_by_name(DEFAULT_ALGORITHM).is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yohash-core registry`
Expected: FAIL — `algorithms`, `algorithm_by_name`, `DEFAULT_ALGORITHM` are not defined.

- [ ] **Step 3: Implement the registry in `crates/yohash-core/src/algorithm.rs`**

A `static ALGORITHMS: [&dyn HashAlgorithm; 2] = [&Md5, &Sha256];`. `algorithm_by_name` trims, lowercases, strips `-`, and compares against each algorithm's normalized name plus the alias `sha256`→SHA-256 (already covered by stripping `-`). Return the first match.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p yohash-core registry`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/yohash-core/src/algorithm.rs
git commit -m "added the algorithm registry"
```

---

### Task 6: Hash engine, single file

**Files:**
- Modify: `crates/yohash-core/src/engine.rs`
- Test: inline `#[cfg(test)]` module in `crates/yohash-core/src/engine.rs`

**Interfaces:**
- Consumes: `HashAlgorithm`, `algorithm_by_name`, `HashError`, `OutputFormat`.
- Produces: `pub struct HashEngine` with `pub fn new() -> Self` and `pub fn with_mmap_threshold(bytes: u64) -> Self`; `pub fn hash_file(&self, path: &Path, alg: &dyn HashAlgorithm, format: OutputFormat) -> Result<String, HashError>`; `pub fn hash_file_named(&self, path: &Path, alg_name: &str, format: OutputFormat) -> Result<String, HashError>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::{Md5, Sha256};
    use crate::format::OutputFormat;
    use std::io::Write;

    fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("yohash-test-{name}"));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn hashes_small_file_via_mmap() {
        let p = temp_file("small", b"abc");
        let e = HashEngine::new();
        assert_eq!(
            e.hash_file(&p, &Sha256, OutputFormat::LowerHex).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hashes_empty_file() {
        let p = temp_file("empty", b"");
        let e = HashEngine::new();
        assert_eq!(
            e.hash_file(&p, &Md5, OutputFormat::UpperHex).unwrap(),
            "D41D8CD98F00B204E9800998ECF8427E"
        );
    }

    #[test]
    fn streaming_path_matches_mmap_path() {
        let data: Vec<u8> = (0..1024u32).flat_map(|i| i.to_le_bytes()).collect();
        let p = temp_file("stream", &data);
        let mmap_engine = HashEngine::new();
        let stream_engine = HashEngine::with_mmap_threshold(0);
        assert_eq!(
            mmap_engine.hash_file(&p, &Sha256, OutputFormat::LowerHex).unwrap(),
            stream_engine.hash_file(&p, &Sha256, OutputFormat::LowerHex).unwrap()
        );
    }

    #[test]
    fn missing_file_is_a_clear_error() {
        let p = std::path::PathBuf::from("definitely-not-here-12345");
        let e = HashEngine::new();
        assert!(matches!(e.hash_file(&p, &Md5, OutputFormat::UpperHex), Err(HashError::NotFound { .. })));
    }

    #[test]
    fn directory_input_is_not_a_file_error() {
        let dir = std::env::temp_dir();
        let e = HashEngine::new();
        assert!(matches!(e.hash_file(&dir, &Md5, OutputFormat::UpperHex), Err(HashError::NotAFile { .. })));
    }

    #[test]
    fn named_lookup_works() {
        let p = temp_file("named", b"abc");
        let e = HashEngine::new();
        assert_eq!(
            e.hash_file_named(&p, "sha256", OutputFormat::LowerHex).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yohash-core engine`
Expected: FAIL — `HashEngine` is not defined.

- [ ] **Step 3: Implement `HashEngine` in `crates/yohash-core/src/engine.rs`**

`HashEngine` holds `mmap_threshold: u64` (default 1 GiB via `new`). `hash_file` order: `std::fs::metadata(path)` → map `ErrorKind::NotFound` to `HashError::NotFound`, `PermissionDenied` to `HashError::PermissionDenied`; if `!metadata.is_file()` return `NotAFile`. If `len <= mmap_threshold`, open the file and `memmap2::Mmap::map`; on map error, fall back to streaming. Streaming path opens the file and passes it as `&mut dyn Read`. Both paths call `alg.digest_reader` then `format.render`. `hash_file_named` looks up via `algorithm_by_name` and treats an unknown name as a new `HashError::Io` carrying `unknown algorithm: {name}`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p yohash-core engine`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/yohash-core/src/engine.rs
git commit -m "added the hash engine"
```

---

### Task 7: Multi-file parallel hashing

**Files:**
- Modify: `crates/yohash-core/src/engine.rs`
- Test: inline `#[cfg(test)]` module additions in `crates/yohash-core/src/engine.rs`

**Interfaces:**
- Consumes: `HashEngine`, `HashAlgorithm`, `OutputFormat`, `HashError`.
- Produces: `pub fn hash_files<'a>(&self, paths: &'a [PathBuf], alg: &(dyn HashAlgorithm + Sync), format: OutputFormat) -> Vec<(&'a PathBuf, Result<String, HashError>)>`, result order matching input order.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn multi_file_preserves_input_order() {
    let a = temp_file("order-a", b"a");
    let b = temp_file("order-b", b"b");
    let c = temp_file("order-c", b"c");
    let paths = vec![c.clone(), a.clone(), b.clone()];
    let e = HashEngine::new();
    let out = e.hash_files(&paths, &Sha256, OutputFormat::LowerHex);
    let order: Vec<&PathBuf> = out.iter().map(|(p, _)| *p).collect();
    assert_eq!(order, vec![&c, &a, &b]);
}

#[test]
fn one_failure_does_not_abort_the_batch() {
    let a = temp_file("batch-a", b"a");
    let missing = std::path::PathBuf::from("definitely-not-here-67890");
    let paths = vec![a.clone(), missing.clone()];
    let e = HashEngine::new();
    let out = e.hash_files(&paths, &Md5, OutputFormat::UpperHex);
    assert!(out[0].1.is_ok());
    assert!(matches!(out[1].1, Err(HashError::NotFound { .. })));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p yohash-core multi_file`
Expected: FAIL — `hash_files` is not defined.

- [ ] **Step 3: Implement `hash_files` in `crates/yohash-core/src/engine.rs`**

Use `paths.par_iter().map(|p| (p, self.hash_file(p, alg, format))).collect()`, which preserves index order for indexed parallel iterators (`Vec` source yields an indexed into-parallel iterator). Import `rayon::prelude::*`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p yohash-core multi_file`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/yohash-core/src/engine.rs
git commit -m "added parallel hashing of multiple files"
```

---

### Task 8: CLI binary

**Files:**
- Create: `crates/yohash-cli/Cargo.toml`
- Create: `crates/yohash-cli/src/main.rs`
- Modify: `Cargo.toml` (add `crates/yohash-cli` to members; add `clap` to workspace deps)

**Interfaces:**
- Consumes: `yohash_core::engine::{HashEngine}`, `yohash_core::algorithm::algorithm_by_name`, `yohash_core::format::OutputFormat`.
- Produces: binary `yohash` accepting positional `FILE...`, `-a/--alg <NAME>` (default `DEFAULT_ALGORITHM`), `-f/--format <upper|lower|base64>` (default upper). Single file prints only the hash; multiple files print `filename: HASH` per line. Any failure prints `yohash: <error>` to stderr and exits 1.

- [ ] **Step 1: Add `clap` and the CLI crate to the workspace**

In root `Cargo.toml` add `clap = { version = "4", features = ["derive"] }` under `[workspace.dependencies]` and add `"crates/yohash-cli"` to `members`.

Create `crates/yohash-cli/Cargo.toml`:

```toml
[package]
name = "yohash-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[[bin]]
name = "yohash"
path = "src/main.rs"

[dependencies]
yohash-core = { path = "../yohash-core" }
clap.workspace = true
```

- [ ] **Step 2: Write `crates/yohash-cli/src/main.rs`**

Define a `clap` derive `Args` with `files: Vec<PathBuf>` (required, at least one) and the two option fields. `main` returns `std::process::ExitCode`. Resolve the algorithm via `algorithm_by_name`; if unknown, print `yohash: unknown algorithm: <name>` to stderr and return exit code 1. Call `hash_files`; if exactly one input, print the single result; otherwise print `{}: {}` per entry using the input path's `display()`. Any `Err` entry prints `yohash: <error>` to stderr and sets a failure exit code while continuing the batch. Format option parses via `OutputFormat::parse`, falling back to `UpperHex` only if `None` is impossible (clap `value_parser` restricts to the three accepted aliases).

- [ ] **Step 3: Verify it builds and runs on a real file**

Run:
```
cargo build -p yohash-cli
cargo run -p yohash-cli -- README.md
```
Expected: a 64-character uppercase SHA-256 hex string, exit code 0.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/yohash-cli
git commit -m "added the yohash cli"
```

---

### Task 9: CLI integration tests

**Files:**
- Create: `crates/yohash-cli/tests/cli.rs`
- Modify: `crates/yohash-cli/Cargo.toml` (dev-dependencies)

**Interfaces:**
- Consumes: the `yohash` binary from Task 8.
- Produces: nothing consumed downstream.

- [ ] **Step 1: Add dev-dependencies to `crates/yohash-cli/Cargo.toml`**

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 2: Write the failing tests in `crates/yohash-cli/tests/cli.rs`**

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn cmd() -> Command {
    Command::cargo_bin("yohash").unwrap()
}

#[test]
fn single_file_prints_only_the_hash() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.txt");
    std::fs::File::create(&p).unwrap().write_all(b"abc").unwrap();
    cmd()
        .arg(&p)
        .assert()
        .success()
        .stdout(predicate::eq("BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD\n"));
}

#[test]
fn md5_lowercase_option() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.txt");
    std::fs::File::create(&p).unwrap().write_all(b"abc").unwrap();
    cmd()
        .args(["-a", "md5", "-f", "lower"])
        .arg(&p)
        .assert()
        .success()
        .stdout(predicate::eq("900150983cd24fb0d6963f7d28e17f72\n"));
}

#[test]
fn multiple_files_print_name_colon_hash_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("hello world.txt");
    let b = dir.path().join("caf\u{e9}.txt");
    std::fs::File::create(&a).unwrap().write_all(b"abc").unwrap();
    std::fs::File::create(&b).unwrap().write_all(b"abc").unwrap();
    let out = cmd().arg(&a).arg(&b).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with(&format!("{}: ", a.display())));
    assert!(lines[1].starts_with(&format!("{}: ", b.display())));
}

#[test]
fn missing_file_exits_one_with_message() {
    cmd()
        .arg("definitely-not-here-99999")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn one_bad_file_still_reports_the_good_one() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("good.txt");
    std::fs::File::create(&a).unwrap().write_all(b"abc").unwrap();
    let out = cmd().arg(&a).arg("definitely-not-here-88888").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains(a.to_str().unwrap()));
}

#[test]
fn unknown_algorithm_exits_one() {
    cmd().args(["-a", "blake3"]).arg("README.md").assert().failure().code(1);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p yohash-cli --test cli`
Expected: FAIL — some behaviors (ordering, exit codes, message text) not yet implemented.

- [ ] **Step 4: Adjust `main.rs` until the tests pass**

Ensure exit code 1 on unknown algorithm and on any errored entry; ensure the successful entries are still written to stdout before the failure exit code.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p yohash-cli --test cli`
Expected: PASS (6 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/yohash-cli
git commit -m "added cli tests"
```

---

### Task 10: CI workflow and README

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: `README.md`

**Interfaces:**
- Consumes: the two crates.
- Produces: `ci.yml` runnable on PRs.

- [ ] **Step 1: Create `.github/workflows/ci.yml`**

Trigger on `pull_request` and `push` to `main`. One `ubuntu-latest` job, steps: `actions/checkout@v4`, `dtolnay/rust-toolchain@stable` with `components: rustfmt, clippy`, then `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`.

- [ ] **Step 2: Complete `README.md`**

Sections: description, `## Installation` (build from source), `## CLI Usage` with the examples from the spec (`yohash <file>`, `-a md5`, `-f lowercase`, multiple files), `## Supported Algorithms` (MD5, SHA-256), `## Building`, `## License` (BSD-2-Clause).

- [ ] **Step 3: Verify locally**

Run: `cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test --workspace`
Expected: all pass with no warnings.

- [ ] **Step 4: Commit**

```bash
git add .github README.md
git commit -m "added ci and docs"
```

---

## Deferred to Later Plans

- Tauri GUI binary (`yohash-gui`)
- Windows shell extension (`yohash-win-ext`) and `regsvr32` registration
- macOS Finder Sync extension and DMG packaging
- Linux Nautilus/Dolphin plugin and tar.gz packaging
- MSI installer definitions and `release.yml` release pipeline
- Additional algorithms (SHA-512, BLAKE3), verification mode, batch mode, checksum files
