//! # book2pdf
//!
//! A CLI utility to convert documentation websites into PDF files for offline reading.
//!
//! **⚠️ Alpha Software** - Basic functionality works but expect stability issues.
//! Documentations with multiple pages may take up all your ram 🤯. Be safe.
//!
//! ## Supported Documentation Formats
//!
//! book2pdf automatically detects and supports the following documentation platforms:
//!
//! - **GitBook** (v2.x, v3.x, v4.x) - GitBook-based documentation sites
//! - **Docusaurus** (v1.x, v2.x, v3.x, v4.x) - Facebook's documentation platform
//! - **MkDocs** (Material, ReadTheDocs, Standard themes) - Python documentation generator
//! - **mdBook** (All versions) - Rust's documentation tool
//! - **VitePress** (v1.x, v2.x+) - Vue.js ecosystem documentation
//! - **vocs** (v1.x+) - Modern documentation framework
//! - **Starlight** (v0.x) - Astro-based documentation framework
//! - **Sphinx** (v5.x-8.x) - Python documentation generator with multiple themes
//!
//! ## Features
//!
//! - Automatic format detection with confidence scoring
//! - PDF generation with cover pages and site logos
//! - PDF merging capabilities with proper page ordering
//! - Memory-efficient browser automation
//! - Configurable output options
//!
//! ## Usage
//!
//! ```bash
//! # Download and convert any supported documentation site
//! book2pdf download https://docs.example.com
//!
//! # Keep individual page PDFs alongside combined PDF
//! book2pdf download https://docs.example.com --preserve-pages
//!
//! # Limit to first 10 pages
//! book2pdf download https://docs.example.com --pages 10
//! ```

mod downloader;
mod pdf_merger;
pub mod config;
pub mod handlers;

pub use downloader::Downloader;
pub use pdf_merger::PdfMerger;
pub use config::Config;
pub use handlers::{HandlersRegistry, FormatHandler, SiteDetector, ConfidenceLevel};
