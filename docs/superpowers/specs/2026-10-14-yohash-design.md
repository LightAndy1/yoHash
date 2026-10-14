# yoHash Design Spec

## Overview

yoHash is a multi-platform file integrity verification tool built in Rust. The primary UX is a context menu integration where users right-click files and select a hash algorithm to compute and copy the hash to the clipboard. Secondary interfaces include a CLI and GUI.

## Target Platforms

- Windows 10 and 11 (primary)
- macOS (current version + previous major)
- Linux (major distributions)

## Target Architectures

- x86_64 (all platforms)
- i686 (Windows only, for 32-bit systems)
- aarch64 (Windows on ARM, Apple Silicon, ARM Linux)

## License

BSD-2-Clause

## Rust Toolchain

Latest stable Rust. No MSRV policy — always track current stable.

## Code Style Conventions

- No comments in code (let the code speak for itself)
- Human-like commit messages: short, simple, junior-dev style
- No Conventional Commits format (no "feat:", "fix:", etc.)
- No emojis in commits
- Example commit messages: "added sha512 support", "fixed clipboard bug on win11", "updated readme"

## Architecture

Cargo workspace with four crates:

```
yoHash/
├── crates/
│   ├── yohash-core/        # Hash engine: multi-threaded I/O, algorithm registry, formatting
│   ├── yohash-cli/         # CLI binary: `yohash <file>`, `yohash <file> --alg sha256`
│   ├── yohash-gui/         # GUI binary: Tauri-based (web frontend, Rust backend)
│   └── yohash-win-ext/     # Windows shell extension DLL (IExplorerCommand COM server)
├── dist/
│   ├── msi/                # WiX/MSI installer definitions
│   └── dmg/                # macOS package definitions
├── .github/workflows/
│   ├── ci.yml              # Build + test on PR
│   └── release.yml         # Build all targets, create GitHub Release on tag push
├── Cargo.toml              # Workspace root
├── README.md               # Simple but complete (build instructions, usage)
└── LICENSE                 # BSD-2-Clause
```

### yohash-core

Pure library crate. No `main()`, no OS-specific code beyond file I/O. Exposes a `HashEngine` that takes file paths and returns computed hashes.

**Multi-threaded hashing strategy:**

1. **File I/O**: `memmap2` crate memory-maps files. OS handles paging, caching, prefetching. For files larger than available RAM, fall back to streaming chunks.

2. **Parallel hashing**: `rayon` work-stealing thread pool. Files split into chunks (4 MB each), each chunk hashed in parallel, results combined.

3. **Algorithm registry**: Trait-based design:

```rust
pub trait HashAlgorithm {
    fn name(&self) -> &'static str;
    fn hash_file(&self, path: &Path) -> Result<String, HashError>;
}
```

Adding new algorithms (SHA-512, BLAKE3, etc.) is just implementing this trait.

4. **Output formatting**: Separate step from hashing. `FormatConfig` (uppercase/lowercase/base64) applied after computation.

**Dependencies:**

- `md-5` (RustCrypto) for MD5
- `sha2` (RustCrypto) for SHA-256
- `rayon` for parallelism
- `memmap2` for memory-mapped I/O
- `thiserror` for error types

### yohash-cli

Binary crate wrapping `yohash-core`. Provides command-line interface:

```
yohash <file>                          # Default: SHA-256, uppercase hex
yohash <file> --alg md5                # Specify algorithm
yohash <file> --format lowercase       # Override format
yohash <file1> <file2> ...             # Multiple files
```

Output format for multiple files:
```
file1.txt: 5D41402ABC4B2A76B9719D911017C592
file2.txt: 2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824
```

### yohash-gui

Tauri-based GUI binary. Minimal web frontend (HTML/CSS/JS) with:

- File selection dialog
- Algorithm selection (dropdown)
- Display computed hash
- Copy to clipboard button
- Settings panel (default format: uppercase/lowercase/base64)

The GUI is secondary — the primary UX is the context menu.

### yohash-win-ext

Windows shell extension DLL (cdylib). Implements `IExplorerCommand` COM interface for Windows 10/11 context menu integration.

**Windows 11 new-style context menu:**

- Shows "yoHash" entry in the modern context menu
- Sub-menu with algorithms: "Copy MD5", "Copy SHA-256"
- Clicking an entry computes the hash and copies to clipboard

**Windows 10 fallback:**

- Same functionality via legacy context menu API
- Works on both Win10 and Win11

**Registration:**

- DLL registered via `regsvr32` during MSI installation
- Installer handles COM registration automatically

**Behavior:**

- Single file: Compute hash, copy to clipboard
- Multiple files: Compute hash for each file, copy formatted list to clipboard (one per line: `filename: HASH`)

**Clipboard format:**

- Uppercase hex by default (configurable via registry key or settings file)

### macOS Extension

Finder Sync Extension (separate app with `FIPS` capability). Similar UX to Windows:

- Right-click file in Finder → "yoHash" → sub-menu with algorithms
- Computes hash and copies to clipboard

### Linux Extension

Nautilus/Dolphin plugin (Python or C, depending on file manager). Similar UX:

- Right-click file → "yoHash" → sub-menu with algorithms
- Computes hash and copies to clipboard

## CI/CD

### GitHub Repository

- Public repo: `yoHash`
- Owner: user's GitHub account

### GitHub Actions Workflows

**ci.yml** — runs on every PR:

- Build all crates for all target triples
- Run tests
- Run `cargo fmt --check`
- Run `cargo clippy`

**release.yml** — runs on git tag push (e.g., `v1.2.3`):

- Build all crates for all target triples
- Create MSI installers (Windows x86, x64, ARM)
- Create portable ZIPs (Windows, macOS, Linux)
- Create GitHub Release with:
  - MSI installers
  - Portable ZIPs
  - Auto-generated release notes from commit messages

**Versioning:**

- Semantic versioning (semver)
- Git tags trigger releases (e.g., `v1.0.0`, `v1.2.3`)
- Tag format: `v{major}.{minor}.{patch}`

## Packaging

### Windows

- **MSI installer** (WiX toolset):
  - Installs `yohash.exe` (CLI)
  - Installs `yohash-gui.exe` (GUI)
  - Installs `yohash-win-ext.dll` (shell extension)
  - Registers shell extension via `regsvr32`
  - Adds to PATH (optional, user can deselect)
  - Creates Start Menu shortcuts (optional)

- **Portable ZIP**:
  - Contains all binaries
  - Includes `register-shell-ext.bat` for manual shell extension registration

### macOS

- **DMG package**:
  - Contains `yohash` (CLI) and `yoHash.app` (GUI)
  - Contains Finder Sync Extension
  - User drags to Applications

### Linux

- **tar.gz archive**:
  - Contains `yohash` binary
  - Contains installation script for Nautilus/Dolphin plugin
  - User extracts and runs install script

## Features (v1 Scope)

- Compute MD5 hash
- Compute SHA-256 hash
- Memory-mapped I/O for fast file reading
- Multi-threaded hashing with rayon
- Context menu integration (Windows 10/11, macOS, Linux)
- Copy hash to clipboard
- Uppercase hex output (default)
- CLI interface
- GUI interface (minimal)
- Multi-file support (per-file list output)
- GitHub Actions CI/CD
- MSI installer (Windows)

## Features (Out of v1 Scope)

- Hash verification / compare mode
- Additional algorithms (SHA-512, BLAKE3, etc.) — easy to add later via trait
- Batch processing (hash all files in a folder)
- Checksum file generation/parsing
- File size / date display
- Drag-and-drop (GUI)
- Localization / translations

## Performance Targets

- Hash a 1 GB file in under 5 seconds (on modern SSD)
- Context menu appears instantly (< 100ms)
- GUI launches in under 1 second
- CLI starts in under 50ms

## Error Handling

- File not found: Clear error message, exit code 1 (CLI), toast notification (GUI/context menu)
- Permission denied: Same as above
- File locked: Retry once after 100ms, then fail
- Clipboard locked: Retry once, then fail with error message
- Unsupported file type: Hash anyway (all files are just bytes)

## Testing Strategy

- Unit tests for `yohash-core` (algorithm correctness, formatting, error handling)
- Integration tests for `yohash-cli` (command-line parsing, output format)
- Manual testing for shell extensions (automated testing is difficult)
- CI runs all tests on every PR

## Security Considerations

- No network access (pure local tool)
- No telemetry
- No data collection
- Hash computation is deterministic and verifiable
- Shell extension has minimal privileges (only file read + clipboard write)

## Future Extensibility

The trait-based algorithm registry makes adding new hash algorithms trivial:

```rust
pub struct Sha512;
impl HashAlgorithm for Sha512 {
    fn name(&self) -> &'static str { "SHA-512" }
    fn hash_file(&self, path: &Path) -> Result<String, HashError> {
        // Implementation
    }
}
```

Adding a new algorithm:
1. Add dependency to `Cargo.toml` (e.g., `sha2` already includes SHA-512)
2. Implement `HashAlgorithm` trait
3. Register in algorithm list
4. Rebuild

No changes to CLI, GUI, or shell extension needed — they all query the registry.
