use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use book2pdf::{Downloader, PdfMerger};
use std::path::PathBuf;
use std::process;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tokio::fs;

#[derive(Parser)]
#[command(name = "book2pdf")]
#[command(about = "CLI utility to turn a published GitBook website into a collection of PDFs for offline reading")]
#[command(version = "0.1.0")]
struct Args {
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
#[arg(short = 'o', long = "outDir", default_value = "output_book2pdf")]
        out_dir: String,

        /// Don't combine PDFs into a single file (by default PDFs are combined)
        #[arg(long = "no-combine")]
        no_combine: bool,

        /// Preserve individual page PDFs (by default they are deleted after combining)
        #[arg(short = 'p', long = "preserve-pages")]
        preserve_pages: bool,

        /// Request timeout in seconds
        #[arg(short = 't', long = "timeout", default_value = "30.0", value_parser = parse_timeout)]
        timeout: f64,
    },
    /// Merge existing PDF files into a single document
    Merge {
        /// Directory containing PDF files to merge
        #[arg(short = 'd', long = "dir", default_value = "output/pages")]
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

async fn merge_pdfs(input_dir: &str, output_file: &str) -> Result<()> {
    let input_path = PathBuf::from(input_dir);
    
    if !input_path.exists() {
        return Err(anyhow::anyhow!("Input directory '{}' does not exist", input_dir));
    }

    info!("Scanning directory: {}", input_dir.green());
    
    let mut entries = fs::read_dir(&input_path).await?;
    let mut pdf_files = Vec::new();
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(extension) = path.extension() {
            if extension == "pdf" {
                pdf_files.push(path);
            }
        }
    }
    
    if pdf_files.is_empty() {
        return Err(anyhow::anyhow!("No PDF files found in '{}'", input_dir));
    }
    
    // Sort by filename to maintain order (especially numbered files)
    pdf_files.sort();
    
    info!("Found {} PDF files to merge:", pdf_files.len());
    for (i, path) in pdf_files.iter().enumerate() {
        info!("  {}: {}", i + 1, path.file_name().unwrap().to_string_lossy().blue());
    }
    
    let mut merger = PdfMerger::new();
    
    for pdf_path in &pdf_files {
        info!("Adding: {}", pdf_path.display());
        if let Err(e) = merger.add_pdf(pdf_path).await {
            error!("Failed to add PDF {}: {}", pdf_path.display(), e);
        }
    }
    
    let output_path = PathBuf::from(output_file);
    merger.save(&output_path).await?;
    
    info!("Successfully merged {} PDFs into: {}", 
          pdf_files.len(), 
          output_path.display().to_string().green());
    
    Ok(())
}

#[tokio::main]
async fn main() {
    // Set up logging with chromiumoxide errors suppressed
    let filter = EnvFilter::from_default_env()
        .add_directive("chromiumoxide::conn=off".parse().unwrap())
        .add_directive("chromiumoxide::handler=off".parse().unwrap())
        .add_directive("book2pdf=info".parse().unwrap());
    
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    let args = Args::parse();

    let result = match args.command {
        Commands::Download { url, out_dir, no_combine, preserve_pages, timeout } => {
            let combine = !no_combine; // Invert the logic: combine by default
            let downloader = Downloader::new(out_dir, combine, preserve_pages, timeout);
            downloader.run(&url).await
        }
        Commands::Merge { input_dir, output_file } => {
            merge_pdfs(&input_dir, &output_file).await
        }
    };

    if let Err(e) = result {
        error!("{}", format!("Error: {}", e).red());
        process::exit(1);
    }
}