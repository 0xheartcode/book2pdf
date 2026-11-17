# Changelog

All notable changes to this project are documented in this file.

## [0.1.1] - 2026-11-17

### 🚀 Features

- Added Starlight format support
- Added Sphinx format support
- Added Nextra format support
- Added `--no-sandbox` flag and `[browser] no_sandbox` config option so the
  headless browser can launch on Ubuntu 23.10+, Docker, and CI runners where
  user namespaces are disabled
- Optimized handler detection
- Optimized memory usage during PDF merge

### 🐛 Bug Fixes

- Config files no longer require every field in a `[table]`; any omitted field
  now falls back to its default (fixes `missing field "supported_sites"` when a
  partial `[scraping]` table is supplied)
- `--version` now reports the actual crate version instead of a hardcoded
  `0.1.0`

### 🧪 Testing

- Initial test suite created
- Marked live browser/network integration tests as `#[ignore]` so the default
  suite is deterministic (run them with `cargo test -- --ignored`)

### 🧹 Housekeeping

- Migrated to the Rust 2024 edition
- Resolved all `cargo clippy` warnings and applied `cargo fmt`

## [0.1.0] - 2025-11-06

### 🚀 Features

- Initial release
- PDF generation from documentation sites via headless Chrome/Chromium
- Automatic format detection and dedicated handlers for GitBook, Docusaurus,
  MkDocs, mdBook, VitePress, and Vocs
- `download` and `merge` subcommands
- TOML configuration file support
