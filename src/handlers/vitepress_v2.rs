use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for VitePress v2.x+ documentation sites
pub struct VitePressV2Handler;

#[async_trait]
impl SiteDetector for VitePressV2Handler {
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
        
        let has_vite_indicators = content.contains("__VP_HASH_MAP__") ||
                                 content.contains("__VP_SITE_DATA__") ||
                                 content.contains("window.__VP");
        
        if !has_vitepress_comment && !has_vite_indicators {
            return Ok(ConfidenceLevel::None);
        }
        
        // Check for v2.x specific selectors
        let v2_selectors = [
            "#VPContent",                     // Main content container
            ".VPContent",                     // Content area class  
            ".VPSidebar",                     // Sidebar component
            ".VPDoc",                         // Document container
            "#VPSidebarNav",                  // Sidebar navigation
            ".vp-doc",                        // Document content
            "div[class*='Layout']",           // Layout wrapper
            ".nav[id*='VP']"                  // VitePress navigation
        ];
        
        let mut selector_matches = 0;
        
        for selector_str in &v2_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    selector_matches += 1;
                    debug!("Found VitePress v2.x selector: {}", selector_str);
                }
            }
        }
        
        // High confidence if we have VitePress indicators and v2.x structure
        match (has_vitepress_comment || has_vite_indicators, selector_matches) {
            (true, 3..=8) => Ok(ConfidenceLevel::Certain),
            (true, 2) => Ok(ConfidenceLevel::High),
            (true, 1) => Ok(ConfidenceLevel::Medium),
            (true, 0) => Ok(ConfidenceLevel::Low),
            _ => Ok(ConfidenceLevel::None),
        }
    }
    
    fn version(&self) -> &str {
        "v2.x"
    }
    
    fn format_name(&self) -> &str {
        "vitepress-v2.x"
    }
}

#[async_trait]
impl FormatHandler for VitePressV2Handler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        debug!("Expanding VitePress v2.x navigation...");
        
        page.evaluate(r#"
            (() => {
                // Expand VitePress v2.x navigation groups
                const toggles = document.querySelectorAll([
                    '.VPSidebarItem.collapsible .caret',
                    '.VPSidebar .group-toggle', 
                    '.sidebar-group .toggle',
                    '[data-vp-sidebar] .toggle',
                    '[aria-expanded="false"]'
                ].join(', '));
                
                toggles.forEach(toggle => {
                    if (toggle.getAttribute('aria-expanded') === 'false' ||
                        toggle.closest('.collapsed')) {
                        toggle.click();
                    }
                });
                
                // Also expand any collapsible sections
                const collapsibleGroups = document.querySelectorAll([
                    '.VPSidebarItem.collapsible',
                    '.sidebar-group.collapsible',
                    '.nav-group.collapsible'
                ].join(', '));
                
                collapsibleGroups.forEach(group => {
                    if (!group.classList.contains('open') && 
                        !group.classList.contains('expanded')) {
                        const trigger = group.querySelector('button, .toggle, .caret');
                        if (trigger) trigger.click();
                    }
                });
                
                console.log('VitePress v2.x navigation expanded');
            })()
        "#).await.map_err(|e| anyhow!("Failed to expand navigation: {}", e))?;
        
        Ok(())
    }
    
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>> {
        debug!("Extracting links from VitePress v2.x...");
        
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        let document = Html::parse_document(&content);
        
        // VitePress v2.x specific navigation selectors
        let link_selectors = [
            ".VPSidebar nav a",                    // Primary sidebar navigation
            "#VPSidebarNav a",                     // Sidebar with ID
            ".VPSidebarItem a",                    // Sidebar items
            ".sidebar-nav a",                      // Generic sidebar nav
            ".nav-links a",                        // Navigation links
            ".VPDoc nav a",                        // Document navigation
            "aside nav a",                         // Aside navigation
            "[class*='sidebar'] a",                // Flexible sidebar selector
            ".menu a",                             // Menu links
            "nav[aria-label*='nav'] a"            // ARIA labeled navigation
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
                    debug!("Found {} links using VitePress v2.x selector: {}", links.len(), selector_str);
                    break;
                }
            }
        }
        
        debug!("Extracted {} links from VitePress v2.x", links.len());
        Ok(links)
    }
    
    async fn prepare_page(&self, page: &Page) -> Result<()> {
        debug!("Preparing VitePress v2.x page for PDF...");
        
        page.evaluate(r#"
            (() => {
                const style = document.createElement('style');
                style.textContent = `
                    /* Hide VitePress v2.x sidebar and navigation */
                    .VPSidebar, .VPNav, .VPNavBar,
                    #VPContent .sidebar, #VPSidebarNav,
                    .nav-wrapper, .navbar, .nav-links {
                        display: none !important;
                    }
                    
                    /* Hide VitePress controls */
                    .VPSwitch, .VPButton, .theme-toggle,
                    .nav-toggle, .sidebar-toggle,
                    .search-box, .algolia-search,
                    .social-links, .edit-link,
                    .prev-next, .page-nav {
                        display: none !important;
                    }
                    
                    /* Hide VitePress specific UI elements */
                    .VPFooter, .footer,
                    .VPLocalNav, .local-nav,
                    .VPCarbonAds, .carbon-ads,
                    .sponsor-wrapper {
                        display: none !important;
                    }
                    
                    /* Expand content to full width */
                    .VPContent, .VPDoc, #VPContent,
                    .content-container, .page,
                    .Layout, div[class*='Layout'],
                    .vp-doc {
                        margin-left: 0 !important;
                        margin-right: 0 !important;
                        max-width: none !important;
                        padding: 20px !important;
                        width: 100% !important;
                        box-sizing: border-box !important;
                    }
                    
                    /* Ensure main content area uses full width */
                    main, .main, article,
                    .content, .page-content {
                        max-width: none !important;
                        margin: 0 !important;
                        padding: 20px !important;
                        width: 100% !important;
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
                
                console.log('VitePress v2.x page prepared for PDF');
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
                        return `v${match[1]}`;
                    }
                }
                
                // Check for VitePress site data
                if (typeof window !== 'undefined') {
                    if (window.__VP_SITE_DATA__ && window.__VP_SITE_DATA__.version) {
                        return `v${window.__VP_SITE_DATA__.version}`;
                    }
                    if (window.VITEPRESS_VERSION) {
                        return `v${window.VITEPRESS_VERSION}`;
                    }
                }
                
                // Check for version in page content or comments
                const html = document.documentElement.outerHTML;
                const versionMatch = html.match(/VitePress\s*v?([0-9\.]+)/i) ||
                                    html.match(/vitepress[^\w]*v?([0-9\.]+)/i);
                if (versionMatch) {
                    return `v${versionMatch[1]}`;
                }
                
                // v2.x structure-based detection
                if (document.querySelector('.VPContent') || 
                    document.querySelector('#VPContent') ||
                    (window.__VP_HASH_MAP__ && window.__VP_SITE_DATA__)) {
                    return 'v2.x+';
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
        "VitePress Handler (v2.x+)"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v2.x+"]
    }
}