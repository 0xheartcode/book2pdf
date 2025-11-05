use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use book2pdf::{Downloader, PdfMerger, Config};
use std::path::PathBuf;
use std::process;
use tracing::error;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "book2pdf")]
#[command(about = "CLI utility to turn published documentation into PDFs for offline reading")]
#[command(version = "0.1.0")]
struct Args {
    /// Enable verbose output (debug level)
    #[arg(short = 'v', long = "verbose", global = true, conflicts_with_all = ["debug", "quiet"])]
    verbose: bool,

    /// Enable debug output (trace level)
    #[arg(short = 'd', long = "debug", global = true, conflicts_with_all = ["verbose", "quiet"])]
    debug: bool,

    /// Enable quiet mode (errors only)
    #[arg(short = 'q', long = "quiet", global = true, conflicts_with_all = ["verbose", "debug"])]
    quiet: bool,

    /// Path to configuration file
    #[arg(short = 'c', long = "config", global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download and convert documentation website to a combined PDF (default behavior)
    Download {
        /// URL of the website to scrape
        url: String,

        /// Output directory used to save files
        #[arg(short = 'o', long = "outDir")]
        out_dir: Option<String>,

        /// Don't combine PDFs into a single file (by default PDFs are combined)
        #[arg(long = "no-combine")]
        no_combine: bool,

        /// Preserve individual page PDFs (by default they are deleted after combining)
        #[arg(short = 'p', long = "preserve-pages")]
        preserve_pages: bool,

        /// Request timeout in seconds
        #[arg(short = 't', long = "timeout", value_parser = parse_timeout)]
        timeout: Option<f64>,

        /// Limit the number of pages to download
        #[arg(long = "pages", value_parser = parse_pages)]
        pages: Option<usize>,

        /// Show browser window (headless by default)
        #[arg(long = "show-browser")]
        show_browser: bool,

        /// Simulate mode - execute everything but don't actually download or create files
        #[arg(long = "simulate", short = 's')]
        simulate: bool,
    },
    /// Merge existing PDF files into a single document
    Merge {
        /// Directory containing PDF files to merge
        #[arg(long = "dir", default_value = "output/pages")]
        input_dir: String,

        /// Output file path for the merged PDF
        #[arg(short = 'o', long = "output", default_value = "merged.pdf")]
        output_file: String,
    },
}

fn parse_timeout(s: &str) -> Result<f64, String> {
    let value = s.parse::<f64>().map_err(|_| "Not a number.")?;
    if value < 0.0 {
        return Err("Must be zero or positive number.".to_string());
    }
    Ok(value)
}

fn parse_pages(s: &str) -> Result<usize, String> {
    let value = s.parse::<usize>().map_err(|_| "Not a valid number.")?;
    if value == 0 {
        return Err("Pages must be greater than 0.".to_string());
    }
    Ok(value)
}


#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Load configuration
    let config = match Config::load(args.config.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            process::exit(1);
        }
    };

    // Determine log level based on CLI flags (override config)
    let (book2pdf_level, global_level) = if args.debug {
        ("trace", "warn")
    } else if args.verbose {
        ("debug", "warn")
    } else if args.quiet {
        ("error", "error")
    } else {
        // Use config file value as default
        match config.logging.level.as_str() {
            "trace" => ("trace", "warn"),
            "debug" => ("debug", "warn"),
            "error" => ("error", "error"),
            _ => ("info", "warn"),
        }
    };

    // Set up logging with chromiumoxide errors suppressed
    let filter = EnvFilter::from_default_env()
        .add_directive("chromiumoxide::conn=off".parse().unwrap())
        .add_directive("chromiumoxide::handler=off".parse().unwrap())
        .add_directive(format!("book2pdf={}", book2pdf_level).parse().unwrap())
        .add_directive(global_level.parse().unwrap());
    
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    let result = match args.command {
        Commands::Download { url, out_dir, no_combine, preserve_pages, timeout, pages, show_browser, simulate } => {
            // Use CLI args or fallback to config values
            let output_dir = out_dir.unwrap_or(config.output.folder.clone());
            let combine = if no_combine { false } else { config.output.combine_pdfs };
            let preserve = if preserve_pages { true } else { config.output.preserve_pages };
            let timeout_val = timeout.unwrap_or(config.browser.timeout);
            let show_window = if show_browser { true } else { config.browser.show_window };
            
            // Debug logging to verify values
            tracing::debug!("Using output_dir: {}", output_dir);
            tracing::debug!("Using timeout: {}", timeout_val);
            tracing::debug!("Using combine: {}", combine);
            tracing::debug!("Using show_window: {}", show_window);
            tracing::debug!("Using simulate: {}", simulate);
            
            let downloader = Downloader::new(output_dir, combine, preserve, timeout_val)
                .with_pdf_config(&config.pdf);
            let simulate_mode = simulate || config.scraping.simulate;
            downloader.run(&url, pages.or(config.scraping.page_limit), show_window, simulate_mode).await
        }
        Commands::Merge { input_dir, output_file } => {
            PdfMerger::merge_directory(&input_dir, &output_file).await
        }
    };

    if let Err(e) = result {
        error!("{}", format!("Error: {}", e).red());
        process::exit(1);
    }
}
