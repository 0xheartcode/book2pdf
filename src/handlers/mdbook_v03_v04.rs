use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for mdBook v0.3.x - v0.4.x documentation sites
pub struct MdBookV03V04Handler;

#[async_trait]
impl SiteDetector for MdBookV03V04Handler {
    async fn can_handle(&self, _url: &str, page: &Page) -> Result<ConfidenceLevel> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        let document = Html::parse_document(&content);
        
        // Must have mdBook comment
        if !content.contains("Book generated using mdBook") {
            return Ok(ConfidenceLevel::None);
        }
        
        // Check for v0.3.x - v0.4.x specific selectors
        let v03_v04_selectors = [
            "#sidebar",                    // Old sidebar ID
            ".sidebar-scrollbox",          // Old scrollbox class (not custom element)
            "#content",                    // Old content ID
            "#menu-bar",                   // Old menu bar
            "#sidebar-toggle",             // Old sidebar toggle
            ".sidebar"                     // Generic sidebar class (fallback)
        ];
        
        // Check for ABSENCE of v0.5+ elements (negative detection)
        let v05_selectors = [
            "#mdbook-sidebar",
            "mdbook-sidebar-scrollbox", 
            "#mdbook-content"
        ];
        
        let mut v03_v04_matches = 0;
        let mut v05_matches = 0;
        
        for selector_str in &v03_v04_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    v03_v04_matches += 1;
                    debug!("Found v0.3-v0.4 selector: {}", selector_str);
                }
            }
        }
        
        for selector_str in &v05_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    v05_matches += 1;
                    debug!("Found v0.5+ selector (negative for this handler): {}", selector_str);
                }
            }
        }
        
        // High confidence if we have v0.3-v0.4 elements AND no v0.5+ elements
        if v05_matches == 0 && v03_v04_matches >= 3 {
            Ok(ConfidenceLevel::Certain)
        } else if v05_matches == 0 && v03_v04_matches >= 2 {
            Ok(ConfidenceLevel::High)
        } else if v05_matches == 0 && v03_v04_matches >= 1 {
            // More lenient - if we have mdBook comment and at least one v0.3-v0.4 element
            Ok(ConfidenceLevel::Medium)
        } else {
            Ok(ConfidenceLevel::None)
        }
    }
    
    fn version(&self) -> &str {
        "v0.3-v0.4"
    }
    
    fn format_name(&self) -> &str {
        "mdbook-v0.3-v0.4"
    }
}

#[async_trait]
impl FormatHandler for MdBookV03V04Handler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        debug!("Expanding mdBook v0.3-v0.4 navigation...");
        
        page.evaluate(r#"
            (() => {
                // v0.3-v0.4 typically doesn't have complex collapsible navigation
                console.log('mdBook v0.3-v0.4 navigation expanded (no action needed)');
            })()
        "#).await.map_err(|e| anyhow!("Failed to expand navigation: {}", e))?;
        
        Ok(())
    }
    
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>> {
        debug!("Extracting links from mdBook v0.3-v0.4...");
        
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        let document = Html::parse_document(&content);
        
        // v0.3-v0.4 specific navigation selectors
        let link_selectors = [
            "#sidebar a",                    // Main sidebar
            ".sidebar-scrollbox a",          // Scrollbox area
            ".sidebar a",                    // Generic sidebar
            ".chapter a"                     // Chapter links
        ];
        
        let mut links = Vec::new();
        
        for selector_str in &link_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if let Ok(absolute_url) = base_url.join(href) {
                            let url_str = absolute_url.to_string();
                            
                            // Filter out non-content links
                            if !url_str.contains("print.html") &&
                               !url_str.contains("#") &&
                               !url_str.contains("mailto:") &&
                               !url_str.contains("javascript:") &&
                               !links.contains(&url_str) {
                                links.push(url_str);
                            }
                        }
                    }
                }
                
                if !links.is_empty() {
                    debug!("Found {} links using v0.3-v0.4 selector: {}", links.len(), selector_str);
                    break;
                }
            }
        }
        
        debug!("Extracted {} links from mdBook v0.3-v0.4", links.len());
        Ok(links)
    }
    
    async fn prepare_page(&self, page: &Page) -> Result<()> {
        debug!("Preparing mdBook v0.3-v0.4 page for PDF...");
        
        page.evaluate(r#"
            (() => {
                const style = document.createElement('style');
                style.textContent = `
                    /* Hide sidebar for PDF (v0.3-v0.4) */
                    #sidebar, .sidebar {
                        display: none !important;
                    }
                    
                    /* Hide navigation elements */
                    .nav-wrapper, .nav-chapters, .mobile-nav-chapters {
                        display: none !important;
                    }
                    
                    /* Hide controls (v0.3-v0.4) */
                    #sidebar-toggle, .sidebar-toggle,
                    #theme-toggle, .theme-toggle, .theme-popup,
                    #search-toggle, .search-toggle,
                    #menu-bar, .menu-bar {
                        display: none !important;
                    }
                    
                    /* Expand content to full width (v0.3-v0.4) */
                    #content, main, .content,
                    #page-wrapper, .page-wrapper {
                        margin-left: 0 !important;
                        max-width: none !important;
                        padding: 20px !important;
                        width: 100% !important;
                    }
                    
                    /* Ensure good print layout */
                    body {
                        background: white !important;
                        color: black !important;
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif !important;
                    }
                    
                    /* Style code blocks for readability */
                    pre, code {
                        background: #f8f8f8 !important;
                        color: #333 !important;
                        border: 1px solid #e1e1e1 !important;
                        padding: 8px !important;
                        font-size: 0.9em !important;
                    }
                    
                    /* Remove fixed positioning */
                    * {
                        position: static !important;
                    }
                    
                    /* Ensure headings are properly styled */
                    h1, h2, h3, h4, h5, h6 {
                        color: black !important;
                        margin-top: 1.5em !important;
                        margin-bottom: 0.5em !important;
                    }
                `;
                document.head.appendChild(style);
                
                console.log('mdBook v0.3-v0.4 page prepared for PDF');
            })()
        "#).await.map_err(|e| anyhow!("Failed to prepare page: {}", e))?;
        
        Ok(())
    }
    
    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let version_result = page.evaluate(r#"
            (() => {
                // Check generator meta tag
                const meta = document.querySelector('meta[name="generator"]');
                if (meta && meta.getAttribute('content')) {
                    const content = meta.getAttribute('content');
                    const match = content.match(/mdBook\s*v?([0-9\.]+)/i);
                    if (match) {
                        const version = match[1];
                        // Validate it's actually v0.3-v0.4 range
                        if (version.startsWith('0.3.') || version.startsWith('0.4.')) {
                            return `v${version}`;
                        }
                    }
                }
                
                // Check for mdBook comment with version
                const html = document.documentElement.outerHTML;
                const commentMatch = html.match(/Book generated using mdBook\s*v?([0-9\.]+)/i);
                if (commentMatch) {
                    const version = commentMatch[1];
                    if (version.startsWith('0.3.') || version.startsWith('0.4.')) {
                        return `v${version}`;
                    }
                }
                
                // v0.3-v0.4 indicator (structure-based detection)
                if (html.includes('Book generated using mdBook') && 
                    document.getElementById('sidebar') && 
                    !document.getElementById('mdbook-sidebar')) {
                    return 'v0.3.x-v0.4.x';
                }
                
                return null;
            })()
        "#).await;
        
        if let Ok(result) = version_result {
            if let Ok(version) = result.into_value::<String>() {
                if !version.is_empty() && version != "null" {
                    return Ok(Some(version));
                }
            }
        }
        
        Ok(None)
    }
    
    fn name(&self) -> &str {
        "mdBook Handler (v0.3.x-v0.4.x)"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v0.3.x", "v0.4.x"]
    }
}