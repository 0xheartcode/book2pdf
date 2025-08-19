//! # book2pdf
//!
//! A CLI utility to convert GitBook documentation websites into PDF files.
//!
//! **⚠️ Alpha software** - Basic functionality works but expect stability issues.
//!
//! ## Current Features
//!
//! - GitBook website scraping and PDF conversion
//! - PDF merging capabilities  
//! - Basic CLI interface
//!
//! ## Usage
//!
//! ```bash
//! book2pdf download https://docs.gitbook.com --combine
//! ```

mod downloader;
mod pdf_merger;

pub use downloader::Downloader;
pub use pdf_merger::PdfMerger;