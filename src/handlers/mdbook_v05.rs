use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for mdBook v0.5.x+ documentation sites
pub struct MdBookV05Handler;

#[async_trait]
impl SiteDetector for MdBookV05Handler {
    async fn can_handle(&self, _url: &str, content: &str) -> Result<ConfidenceLevel> {
        let document = Html::parse_document(content);
        
        // Must have mdBook comment
        if !content.contains("Book generated using mdBook") {
            return Ok(ConfidenceLevel::None);
        }
        
        // Check for v0.5+ specific selectors (including hybrid patterns)
        let v05_selectors = [
            "#mdbook-sidebar",               // New sidebar ID
            "mdbook-sidebar-scrollbox",      // New custom element (key identifier!)
            "#mdbook-content",               // New content ID
            "#mdbook-menu-bar",              // New menu bar
            "#mdbook-sidebar-toggle",        // New sidebar toggle
            "#sidebar",                      // Hybrid: old ID with new elements
            "#content"                       // Hybrid: old ID with new elements
        ];
        
        let mut selector_matches = 0;
        
        for selector_str in &v05_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    selector_matches += 1;
                    debug!("Found v0.5+ selector: {}", selector_str);
                }
            }
        }
        
        // High confidence if we have modern v0.5+ elements
        match selector_matches {
            4..=5 => Ok(ConfidenceLevel::Certain),
            3 => Ok(ConfidenceLevel::High),
            2 => Ok(ConfidenceLevel::Medium),
            1 => Ok(ConfidenceLevel::Low),  // More lenient
            _ => Ok(ConfidenceLevel::None),
        }
    }
    
    fn version(&self) -> &str {
        "v0.5+"
    }
    
    fn format_name(&self) -> &str {
        "mdbook-v0.5+"
    }
}

#[async_trait]
impl FormatHandler for MdBookV05Handler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        debug!("Expanding mdBook v0.5+ navigation...");
        
        page.evaluate(r#"
            (() => {
                // v0.5+ has collapsible chapters
                const foldToggles = document.querySelectorAll('.chapter-fold-toggle');
                foldToggles.forEach(toggle => {
                    const li = toggle.closest('li');
                    if (li && !li.classList.contains('expanded')) {
                        toggle.click();
                    }
                });
                
                // Also expand any chapter items that might be collapsed
                const chapterItems = document.querySelectorAll('.chapter-item, .header-item');
                chapterItems.forEach(item => {
                    if (!item.classList.contains('expanded')) {
                        item.classList.add('expanded');
                    }
                });
                
                console.log('mdBook v0.5+ navigation expanded');
            })()
        "#).await.map_err(|e| anyhow!("Failed to expand navigation: {e}"))?;
        
        Ok(())
    }
    
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>> {
        debug!("Extracting links from mdBook v0.5+...");
        
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        let document = Html::parse_document(&content);
        
        // v0.5+ specific navigation selectors
        let link_selectors = [
            "#mdbook-sidebar a",                    // New sidebar
            "mdbook-sidebar-scrollbox a",           // New custom element
            ".sidebar a",                           // Generic sidebar
            ".chapter-item a, .chapter a",          // Modern chapter links
            "nav[aria-label='Table of contents'] a" // ARIA navigation
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
                    debug!("Found {} links using v0.5+ selector: {}", links.len(), selector_str);
                    break;
                }
            }
        }
        
        debug!("Extracted {} links from mdBook v0.5+", links.len());
        Ok(links)
    }
    
    async fn prepare_page(&self, page: &Page) -> Result<()> {
        debug!("Preparing mdBook v0.5+ page for PDF...");
        
        page.evaluate(r#"
            (() => {
                const style = document.createElement('style');
                style.textContent = `
                    /* Hide sidebar for PDF (v0.5+) */
                    #mdbook-sidebar, .sidebar,
                    mdbook-sidebar-scrollbox, .sidebar-scrollbox {
                        display: none !important;
                    }
                    
                    /* Hide navigation elements */
                    .nav-wrapper, .nav-chapters, .mobile-nav-chapters,
                    .nav-wide-wrapper {
                        display: none !important;
                    }
                    
                    /* Hide controls (v0.5+) */
                    #mdbook-sidebar-toggle, .sidebar-toggle,
                    #mdbook-theme-toggle, .theme-toggle, .theme-popup,
                    #mdbook-search-toggle, .search-toggle,
                    #mdbook-search-wrapper, .search-container,
                    #mdbook-searchbar-outer,
                    #mdbook-menu-bar, .menu-bar {
                        display: none !important;
                    }
                    
                    /* Expand content to full width (v0.5+) */
                    #mdbook-content, main, .content,
                    #mdbook-page-wrapper, .page-wrapper {
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
                    
                    /* Hide help popup and other overlays */
                    #mdbook-help-container, #mdbook-help-popup,
                    .mdbook-help-container, .mdbook-help-popup {
                        display: none !important;
                    }
                `;
                document.head.appendChild(style);
                
                console.log('mdBook v0.5+ page prepared for PDF');
            })()
        "#).await.map_err(|e| anyhow!("Failed to prepare page: {e}"))?;
        
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
                        // Validate it's actually v0.5+ range
                        if (version.startsWith('0.5.') || 
                            parseInt(version.split('.')[1]) >= 5) {
                            return `v${version}`;
                        }
                    }
                }
                
                // Check for mdBook comment with version
                const html = document.documentElement.outerHTML;
                const commentMatch = html.match(/Book generated using mdBook\s*v?([0-9\.]+)/i);
                if (commentMatch) {
                    const version = commentMatch[1];
                    if (version.startsWith('0.5.') || 
                        parseInt(version.split('.')[1]) >= 5) {
                        return `v${version}`;
                    }
                }
                
                // Look for version in JavaScript variables (v0.5+)
                if (typeof window !== 'undefined') {
                    if (window.mdbook_version) return `v${window.mdbook_version}`;
                    if (window.MDBOOK_VERSION) return `v${window.MDBOOK_VERSION}`;
                }
                
                // v0.5+ indicator (structure-based detection)
                if (html.includes('Book generated using mdBook') && 
                    document.getElementById('mdbook-sidebar')) {
                    return 'v0.5.x+';
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
        "mdBook Handler (v0.5.x+)"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v0.5.x+"]
    }
}