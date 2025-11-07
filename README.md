# book2pdf

A CLI utility to convert documentation websites into PDF files for offline reading.

**⚠️ Clunky Alpha Software** - Basic functionality works but expect stability issues.
Documentations with multiple pages may take up all your ram 🤯. Be safe.

## Issues

In case a specific website does not scrape correctly, please create an issue or a PR (if you're inclined).
Either way it will be tested and fixed. This is all early so expect lots of changes.

## Requirements

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Chrome/Chromium browser** - Must be installed and accessible in PATH
- **Internet connection** - For website scraping

## Supported Documentation Formats

book2pdf automatically detects and supports the following documentation platforms:

### GitBook
- **Versions**: v2.x, v3.x, v4.x
- **Format**: gitbook
- **Auto-detection**: Based on GitBook-specific selectors and navigation patterns

### Docusaurus
- **Versions**: v1.x, v2.x, v3.x, v4.x
- **Format**: docusaurus  
- **Auto-detection**: Detects Docusaurus metadata and navigation structure

### MkDocs
- **Versions**: Material theme, ReadTheDocs theme, Standard theme
- **Format**: mkdocs
- **Auto-detection**: Supports multiple MkDocs themes with automatic theme detection

### mdBook
- **Versions**: All versions (theme-agnostic)
- **Format**: mdbook
- **Auto-detection**: Detects mdBook generator meta tag and sidebar structure

### VitePress
- **Versions**: v1.x, v2.x+
- **Format**: vitepress
- **Auto-detection**: Detects VitePress structure and version-specific elements

### vocs
- **Versions**: v1.x+
- **Format**: vocs
- **Auto-detection**: Detects vocs data-vocs attribute and CSS classes

### Starlight
- **Versions**: v0.x
- **Format**: starlight  
- **Auto-detection**: Detects Starlight CSS layers and Astro-based components

### Sphinx
- **Versions**: v5.x, v6.x, v7.x, v8.x
- **Format**: sphinx
- **Auto-detection**: Detects Sphinx generator meta tags, themes (ReadTheDocs, Furo, PyData, Jupyter Book), and navigation patterns

To see all supported formats, run:
```bash
cargo run -- download --list
```

**Note**: Version detection is automatic and best-effort. Some versions may be detected as ranges (e.g., v2.x) when specific version cannot be determined.

## Installation

### Build from Source

```bash
git clone <your-repo-url>
cd book2pdf
cargo build --release
```

The binary will be available at `target/release/book2pdf`

### Run Directly (Development)

```bash
git clone <your-repo-url>
cd book2pdf
cargo run -- download https://docs.example.com
```

## Usage

Here is a concrete example:

```bash
cargo run -- download "https://claritychallenge.org/clarity_CEC1_doc/docs/intro"
```

### Main Command

CLI utility to turn published documentation into PDFs for offline reading
```
Usage: book2pdf <COMMAND>

Commands:
  download  Download and convert documentation website to a combined PDF (default behavior)
  merge     Merge existing PDF files into a single document
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Download Command

Download and convert documentation website to a combined PDF (default behavior)
```
Usage: book2pdf download [OPTIONS] <URL>

Arguments:
  <URL>  URL of the website to scrape

Options:
  -o, --outDir <OUT_DIR>   Output directory used to save files
  -v, --verbose            Enable verbose output (debug level)
  -d, --debug              Enable debug output (trace level)
      --no-combine         Don't combine PDFs into a single file (by default PDFs are combined)
  -p, --preserve-pages     Preserve individual page PDFs (by default they are deleted after combining)
  -q, --quiet              Enable quiet mode (errors only)
  -c, --config <CONFIG>    Path to configuration file
  -t, --timeout <TIMEOUT>  Request timeout in seconds
      --pages <PAGES>      Limit the number of pages to download
      --show-browser       Show browser window (headless by default)
  -s, --simulate           Simulate mode - execute everything but don't actually download or create files
  -h, --help               Print help
```

### Merge Command

Merge existing PDF files into a single document

```
Usage: book2pdf merge [OPTIONS]

Options:
      --dir <INPUT_DIR>       Directory containing PDF files to merge [default: output/pages]
  -v, --verbose               Enable verbose output (debug level)
  -d, --debug                 Enable debug output (trace level)
  -o, --output <OUTPUT_FILE>  Output file path for the merged PDF [default: merged.pdf]
  -q, --quiet                 Enable quiet mode (errors only)
  -c, --config <CONFIG>       Path to configuration file
  -h, --help                  Print help
```

## Configuration

book2pdf supports configuration files in TOML format. Create a `book2pdf.toml` file in your project directory or `~/.config/book2pdf/config.toml` for global settings.

See `book2pdf.toml.example` for all available options including browser settings, PDF formatting, logging levels, and more.

Configuration precedence: CLI arguments > config file > defaults

## Examples

### Basic Usage

```bash
# Download and convert a GitBook site (creates combined PDF by default)
./target/release/book2pdf download https://docs.example.com

# Or during development
cargo run -- download https://docs.example.com

# Download to custom directory
book2pdf download https://docs.example.com --outDir my-docs

# Keep individual page PDFs alongside combined PDF
book2pdf download https://docs.example.com --preserve-pages

# Don't combine - keep only individual page PDFs
book2pdf download https://docs.example.com --no-combine
```

### Advanced Usage

```bash
# Simulate download without creating files (dry-run)
book2pdf download https://docs.example.com --simulate

# Limit to first 10 pages
book2pdf download https://docs.example.com --pages 10

# Show browser window (useful for debugging)
book2pdf download https://docs.example.com --show-browser

# Use configuration file
book2pdf download https://docs.example.com --config my-config.toml

# Enable verbose logging
book2pdf download https://docs.example.com --verbose

# Enable debug logging
book2pdf download https://docs.example.com --debug

# Quiet mode (errors only)
book2pdf download https://docs.example.com --quiet
```

### Merge Existing PDFs

```bash
# Merge PDFs from default directory
book2pdf merge

# Merge from custom directory
book2pdf merge --dir my-pdfs --output combined-docs.pdf
```

## Development

```bash
# Check code
cargo check

# Run tests
cargo test

# Build release binary
cargo build --release

# Generate documentation
cargo doc --open
```



```
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::::::::::::::::  :::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::   -  ::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::🌐::::::::::::::::::::.       :::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::.:::::::::::::::::   -=-  .::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::-::::::::::::::          :::::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::-::::::::::::  =       ::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::.:::::::::      =    :::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::     ::::            :::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::.      ::: :   @  @  :::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::      ::@@%@+      :::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::        -@@++. @  ::::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::        @@@@@@@# :::::::::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::        @@@@@@@  ::::::::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::    @@@@@@@*    :::::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::   @@@@@@@=     :::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::    @@@@@@@    📜::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::     @@@@@@@:  ::::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::       @@@@@+   :::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::               :::::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::                :::::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::                #-:::::::::::::::::::::::::::::::
::::::::::::::::::::::::::::::::::::::::::::.                %:::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::-*%@@=::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
:::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::::
```
