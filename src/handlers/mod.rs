use anyhow::Result;
use async_trait::async_trait;
use chromiumoxide::Page;
use url::Url;

/// Confidence level for format detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidenceLevel {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Certain = 4,
}

/// Trait for detecting if a site can be handled by a specific format handler
#[async_trait]
pub trait SiteDetector: Send + Sync {
    /// Check if this detector can handle the given URL and page content
    async fn can_handle(&self, url: &str, page: &Page) -> Result<ConfidenceLevel>;
    
    /// Get the version identifier for this handler
    fn version(&self) -> &str;
    
    /// Get the format name (e.g., "gitbook", "docusaurus")
    fn format_name(&self) -> &str;
}

/// Trait for handling specific documentation formats
#[async_trait]
pub trait FormatHandler: SiteDetector + Send + Sync {
    /// Expand navigation menus to reveal all links
    async fn expand_navigation(&self, page: &Page) -> Result<()>;
    
    /// Extract all documentation links from the page
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>>;
    
    /// Prepare the page for PDF generation (remove interactive elements, etc.)
    async fn prepare_page(&self, page: &Page) -> Result<()>;
    
    /// Detect the specific version of this format (e.g., "v2.4.0")
    async fn detect_version(&self, page: &Page) -> Result<Option<String>>;
    
    /// Get a human-readable name for this handler
    fn name(&self) -> &str;
    
    /// Get supported versions info for listing
    fn supported_versions(&self) -> Vec<&str>;
}

/// Registry for managing format handlers
pub struct HandlersRegistry {
    handlers: Vec<Box<dyn FormatHandler>>,
}

impl HandlersRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    
    /// Register a new format handler
    pub fn register(&mut self, handler: Box<dyn FormatHandler>) {
        self.handlers.push(handler);
    }
    
    /// Detect the best format handler for the given URL and page
    pub async fn detect_format(&self, url: &str, page: &Page) -> Result<Option<&dyn FormatHandler>> {
        let mut best_handler: Option<&dyn FormatHandler> = None;
        let mut best_confidence = ConfidenceLevel::None;
        
        for handler in &self.handlers {
            match handler.can_handle(url, page).await {
                Ok(confidence) => {
                    if confidence > best_confidence {
                        best_confidence = confidence;
                        best_handler = Some(handler.as_ref());
                    }
                }
                Err(e) => {
                    tracing::warn!("Handler {} failed detection: {}", handler.name(), e);
                }
            }
        }
        
        Ok(best_handler)
    }
    
    /// Get all registered handlers
    pub fn handlers(&self) -> &[Box<dyn FormatHandler>] {
        &self.handlers
    }
    
    /// List all supported formats and versions
    pub fn list_supported_formats(&self) {
        println!("Supported Documentation Formats:");
        println!();
        
        for handler in &self.handlers {
            println!("{}", handler.name());
            println!("   Format: {}", handler.format_name());
            println!("   Supported Versions: {}", handler.supported_versions().join(", "));
            println!();
        }
        
        println!("   Note: Version detection is automatic and best-effort");
        println!("   Some versions may be detected as ranges (e.g., v2.x) when specific version cannot be determined");
    }
    
    /// Create a registry with default handlers
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        
        // Register built-in handlers
        registry.register(Box::new(gitbook::GitBookHandler));
        registry.register(Box::new(docusaurus::DocusaurusHandler));
        registry.register(Box::new(mkdocs::MkDocsHandler));
        registry.register(Box::new(mdbook_v03_v04::MdBookV03V04Handler));
        registry.register(Box::new(mdbook_v05::MdBookV05Handler));
        
        registry
    }
}

impl Default for HandlersRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

pub mod gitbook;
pub mod docusaurus;
pub mod mkdocs;
pub mod mdbook_v03_v04;
pub mod mdbook_v05;

pub use gitbook::GitBookHandler;
pub use docusaurus::DocusaurusHandler;
pub use mkdocs::MkDocsHandler;
pub use mdbook_v03_v04::MdBookV03V04Handler;
pub use mdbook_v05::MdBookV05Handler;
