# book2pdf

A CLI utility to convert documentation websites into PDF files for offline reading.

**⚠️ Clunky Alpha Software** - Basic functionality works but expect stability issues.

## Requirements

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Chrome/Chromium browser** - Must be installed and accessible in PATH
- **Internet connection** - For website scraping

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

CLI utility to turn a published GitBook website into a collection of PDFs for offline reading
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
  -o, --outDir <OUT_DIR>   Output directory used to save files [default: output_book2pdf]
      --no-combine         Don't combine PDFs into a single file (by default PDFs are combined)
  -p, --preserve-pages     Preserve individual page PDFs (by default they are deleted after combining)
  -t, --timeout <TIMEOUT>  Request timeout in seconds [default: 30.0]
  -h, --help               Print help
```

### Merge Command

Merge existing PDF files into a single document

```
Usage: book2pdf merge [OPTIONS]

Options:
  -d, --dir <INPUT_DIR>       Directory containing PDF files to merge [default: output_book2pdf/pages]
  -o, --output <OUTPUT_FILE>  Output file path for the merged PDF [default: merged.pdf]
  -h, --help                  Print help
```

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
