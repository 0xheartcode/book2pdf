use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for VitePress v1.x documentation sites
pub struct VitePressV1Handler;

#[async_trait]
impl SiteDetector for VitePressV1Handler {
    async fn can_handle(&self, _url: &str, page: &Page) -> Result<ConfidenceLevel> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        let document = Html::parse_document(&content);
        
        // Check for VitePress indicators
        let has_vitepress_comment = content.contains("VitePress") || 
                                   content.contains("Powered by VitePress") ||
                                   content.contains("vitepress");
        
        if !has_vitepress_comment {
            return Ok(ConfidenceLevel::None);
        }
        
        // Check for v1.x specific selectors
        let v1_selectors = [
            ".sidebar",                           // Generic sidebar
            ".nav-links",                         // Navigation links
            ".sidebar-group",                     // Sidebar groups
            "#content.content",                   // Content with ID and class
            ".page-content",                      // Page content
            ".theme-container",                   // Theme container
            ".navbar",                            // Navigation bar
            ".sidebar-links"                      // Sidebar links
        ];
        
        // Check for ABSENCE of v2.x elements (negative detection)
        let v2_selectors = [
            ".VPContent",
            "#VPContent", 
            ".VPSidebar",
            ".VPDoc"
        ];
        
        let mut v1_matches = 0;
        let mut v2_matches = 0;
        
        for selector_str in &v1_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    v1_matches += 1;
                    debug!("Found VitePress v1.x selector: {}", selector_str);
                }
            }
        }
        
        for selector_str in &v2_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    v2_matches += 1;
                    debug!("Found VitePress v2.x selector (negative for this handler): {}", selector_str);
                }
            }
        }
        
        // High confidence if we have v1.x elements AND no v2.x elements
        if v2_matches == 0 && v1_matches >= 3 {
            Ok(ConfidenceLevel::Certain)
        } else if v2_matches == 0 && v1_matches >= 2 {
            Ok(ConfidenceLevel::High)
        } else if v2_matches == 0 && v1_matches >= 1 {
            Ok(ConfidenceLevel::Medium)
        } else {
            Ok(ConfidenceLevel::None)
        }
    }
    
    fn version(&self) -> &str {
        "v1.x"
    }
    
    fn format_name(&self) -> &str {
        "vitepress-v1.x"
    }
}

#[async_trait]
impl FormatHandler for VitePressV1Handler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        debug!("Expanding VitePress v1.x navigation...");
        
        page.evaluate(r#"
            (() => {
                // Expand VitePress v1.x navigation groups
                const toggles = document.querySelectorAll([
                    '.sidebar-group .toggle',
                    '.nav-item-toggle',
                    '.collapsible .toggle',
                    '[aria-expanded="false"]'
                ].join(', '));
                
                toggles.forEach(toggle => {
                    if (toggle.getAttribute('aria-expanded') === 'false' ||
                        toggle.closest('.collapsed')) {
                        toggle.click();
                    }
                });
                
                // Also expand any collapsible sidebar groups
                const groups = document.querySelectorAll([
                    '.sidebar-group.collapsible',
                    '.nav-group.collapsible'
                ].join(', '));
                
                groups.forEach(group => {
                    if (group.classList.contains('collapsed')) {
                        const trigger = group.querySelector('.toggle, button');
                        if (trigger) trigger.click();
                    }
                });
                
                console.log('VitePress v1.x navigation expanded');
            })()
        "#).await.map_err(|e| anyhow!("Failed to expand navigation: {}", e))?;
        
        Ok(())
    }
    
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>> {
        debug!("Extracting links from VitePress v1.x...");
        
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        let document = Html::parse_document(&content);
        
        // VitePress v1.x specific navigation selectors
        let link_selectors = [
            ".sidebar a",                         // Primary sidebar
            ".nav-links a",                       // Navigation links
            ".sidebar-group a",                   // Sidebar groups
            ".sidebar-links a",                   // Sidebar links
            ".nav a",                             // Generic navigation
            ".theme-container nav a",             // Theme container nav
            "[class*='sidebar'] a",               // Flexible sidebar selector
            "aside a"                             // Aside links
        ];
        
        let mut links = Vec::new();
        
        for selector_str in &link_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if let Ok(absolute_url) = base_url.join(href) {
                            let url_str = absolute_url.to_string();
                            
                            // Filter out non-content links
                            if !url_str.contains("#") &&
                               !url_str.contains("mailto:") &&
                               !url_str.contains("javascript:") &&
                               !url_str.contains("tel:") &&
                               !url_str.contains("?") &&
                               !links.contains(&url_str) {
                                links.push(url_str);
                            }
                        }
                    }
                }
                
                if !links.is_empty() {
                    debug!("Found {} links using VitePress v1.x selector: {}", links.len(), selector_str);
                    break;
                }
            }
        }
        
        debug!("Extracted {} links from VitePress v1.x", links.len());
        Ok(links)
    }
    
    async fn prepare_page(&self, page: &Page) -> Result<()> {
        debug!("Preparing VitePress v1.x page for PDF...");
        
        page.evaluate(r#"
            (() => {
                const style = document.createElement('style');
                style.textContent = `
                    /* Hide VitePress v1.x sidebar and navigation */
                    .sidebar, .nav-links, .navbar,
                    .theme-container .sidebar,
                    .sidebar-mask, .nav-wrapper {
                        display: none !important;
                    }
                    
                    /* Hide VitePress controls */
                    .theme-toggle, .nav-toggle,
                    .sidebar-toggle, .search-box,
                    .algolia-search, .social-links,
                    .edit-link, .prev-next {
                        display: none !important;
                    }
                    
                    /* Hide VitePress specific UI elements */
                    .footer, .page-nav,
                    .carbon-ads, .sponsor-wrapper,
                    .home-features, .hero {
                        display: none !important;
                    }
                    
                    /* Expand content to full width */
                    .theme-container, .page,
                    .content, #content,
                    .page-content, .custom-layout {
                        margin-left: 0 !important;
                        margin-right: 0 !important;
                        max-width: none !important;
                        padding: 20px !important;
                        width: 100% !important;
                        box-sizing: border-box !important;
                    }
                    
                    /* Ensure main content area uses full width */
                    main, .main, article,
                    .content-wrapper {
                        max-width: none !important;
                        margin: 0 !important;
                        padding: 20px !important;
                        width: 100% !important;
                    }
                    
                    /* Remove sidebar offset */
                    .theme-container.sidebar-open .page,
                    .theme-container.no-navbar .page {
                        padding-left: 0 !important;
                    }
                    
                    /* Ensure good print layout */
                    body {
                        background: white !important;
                        color: black !important;
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif !important;
                        margin: 0 !important;
                        padding: 0 !important;
                    }
                    
                    /* Style code blocks for readability */
                    pre, code, .code {
                        background: #f8f8f8 !important;
                        color: #333 !important;
                        border: 1px solid #e1e1e1 !important;
                        padding: 8px !important;
                        font-size: 0.9em !important;
                        border-radius: 4px !important;
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
                        font-weight: bold !important;
                    }
                    
                    /* Style tables for better readability */
                    table {
                        border-collapse: collapse !important;
                        width: 100% !important;
                        margin: 1em 0 !important;
                    }
                    
                    table th, table td {
                        border: 1px solid #ddd !important;
                        padding: 8px !important;
                        text-align: left !important;
                    }
                    
                    table th {
                        background-color: #f5f5f5 !important;
                        font-weight: bold !important;
                    }
                    
                    /* Hide any overlay elements */
                    .overlay, .modal, .popup,
                    .banner, .notice, .alert {
                        display: none !important;
                    }
                `;
                document.head.appendChild(style);
                
                console.log('VitePress v1.x page prepared for PDF');
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
                    const match = content.match(/VitePress\s*v?([0-9\.]+)/i);
                    if (match) {
                        const version = match[1];
                        // Validate it's actually v1.x range
                        if (version.startsWith('1.') || 
                            parseInt(version.split('.')[0]) === 1) {
                            return `v${version}`;
                        }
                    }
                }
                
                // Check for version in page content or comments
                const html = document.documentElement.outerHTML;
                const versionMatch = html.match(/VitePress\s*v?([0-9\.]+)/i) ||
                                    html.match(/vitepress[^\w]*v?([0-9\.]+)/i);
                if (versionMatch) {
                    const version = versionMatch[1];
                    if (version.startsWith('1.') || 
                        parseInt(version.split('.')[0]) === 1) {
                        return `v${version}`;
                    }
                }
                
                // v1.x structure-based detection
                if (document.querySelector('.theme-container') && 
                    document.querySelector('.sidebar') &&
                    !document.querySelector('.VPContent')) {
                    return 'v1.x';
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
        "VitePress Handler (v1.x)"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v1.x"]
    }
}