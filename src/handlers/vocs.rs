use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for vocs documentation sites
pub struct VocsHandler;

#[async_trait]
impl SiteDetector for VocsHandler {
    async fn can_handle(&self, _url: &str, page: &Page) -> Result<ConfidenceLevel> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        let document = Html::parse_document(&content);
        
        // Primary vocs detection: data-vocs attribute on html element
        let has_data_vocs = content.contains("data-vocs") ||
                           content.contains("html data-vocs") ||
                           document.select(&Selector::parse("html[data-vocs]").unwrap()).next().is_some();
        
        // Check for vocs-specific CSS class patterns
        let vocs_selectors = [
            ".vocs_DocsLayout",                   // Main layout container
            ".vocs_Sidebar",                      // Sidebar component
            ".vocs_Content",                      // Content wrapper
            "#vocs-content",                      // Content area ID
            ".vocs_Sidebar_navigation",           // Navigation wrapper
            ".vocs_Sidebar_item",                 // Navigation items
            ".vocs_DesktopTopNav",               // Top navigation
            ".vocs_DocsLayout_content"           // Layout content area
        ];
        
        let mut vocs_class_matches = 0;
        
        for selector_str in &vocs_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    vocs_class_matches += 1;
                    debug!("Found vocs selector: {}", selector_str);
                }
            }
        }
        
        // Check for vocs-specific patterns in content
        let has_vocs_patterns = content.contains("vocs_") ||
                               content.contains("#vocs-content") ||
                               content.contains("data-vocs");
        
        // Confidence logic
        if has_data_vocs && vocs_class_matches >= 3 {
            Ok(ConfidenceLevel::Certain)
        } else if (has_data_vocs && vocs_class_matches >= 1) || (vocs_class_matches >= 3 && has_vocs_patterns) {
            Ok(ConfidenceLevel::High)
        } else if vocs_class_matches >= 2 || has_vocs_patterns {
            Ok(ConfidenceLevel::Medium)
        } else if vocs_class_matches >= 1 {
            Ok(ConfidenceLevel::Low)
        } else {
            Ok(ConfidenceLevel::None)
        }
    }
    
    fn version(&self) -> &str {
        "v1.x"
    }
    
    fn format_name(&self) -> &str {
        "vocs"
    }
}

#[async_trait]
impl FormatHandler for VocsHandler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        debug!("Expanding vocs navigation...");
        
        page.evaluate(r#"
            (() => {
                // Expand vocs navigation sections
                const collapsibleSections = document.querySelectorAll([
                    '.vocs_Sidebar_sectionHeader[role="button"]',
                    '.vocs_Sidebar_sectionHeader',
                    '[data-collapsed="true"]',
                    '[aria-expanded="false"]'
                ].join(', '));
                
                collapsibleSections.forEach(section => {
                    // Check if section is collapsed
                    if (section.getAttribute('aria-expanded') === 'false' ||
                        section.getAttribute('data-collapsed') === 'true' ||
                        section.closest('[data-collapsed="true"]')) {
                        
                        // Try to expand by clicking
                        section.click();
                        
                        // Also try to expand via data attributes
                        section.setAttribute('aria-expanded', 'true');
                        section.setAttribute('data-collapsed', 'false');
                        
                        // Find associated content and show it
                        const items = section.nextElementSibling;
                        if (items && items.classList.contains('vocs_Sidebar_items')) {
                            items.style.display = 'block';
                        }
                    }
                });
                
                // Also expand any other collapsible elements
                const toggles = document.querySelectorAll([
                    '.vocs_Sidebar_sectionCollapse',
                    '.vocs_Icon[data-collapsed]',
                    'button[aria-expanded="false"]'
                ].join(', '));
                
                toggles.forEach(toggle => {
                    if (toggle.getAttribute('aria-expanded') === 'false') {
                        toggle.click();
                    }
                });
                
                console.log('vocs navigation expanded');
            })()
        "#).await.map_err(|e| anyhow!("Failed to expand navigation: {e}"))?;
        
        Ok(())
    }
    
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>> {
        debug!("Extracting links from vocs...");
        
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        let document = Html::parse_document(&content);
        
        // vocs specific navigation selectors
        let link_selectors = [
            ".vocs_Sidebar_item",                 // Primary sidebar navigation items
            ".vocs_Sidebar_navigation a",         // All sidebar navigation links
            ".vocs_Sidebar_items a",              // Section items
            ".vocs_NavigationMenu_link",          // Top navigation links
            ".vocs_Sidebar a[href]",              // Any link in sidebar
            "nav a[href]",                        // Generic navigation links
            "aside a[href]"                       // Aside navigation links
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
                               !url_str.ends_with(".css") &&
                               !url_str.ends_with(".js") &&
                               !url_str.ends_with(".png") &&
                               !url_str.ends_with(".jpg") &&
                               !url_str.ends_with(".svg") &&
                               !links.contains(&url_str) {
                                links.push(url_str);
                            }
                        }
                    }
                }
                
                if !links.is_empty() {
                    debug!("Found {} links using vocs selector: {}", links.len(), selector_str);
                    break;
                }
            }
        }
        
        debug!("Extracted {} links from vocs", links.len());
        Ok(links)
    }
    
    async fn prepare_page(&self, page: &Page) -> Result<()> {
        debug!("Preparing vocs page for PDF...");
        
        page.evaluate(r#"
            (() => {
                const style = document.createElement('style');
                style.textContent = `
                    /* Hide vocs sidebar and navigation */
                    .vocs_Sidebar, .vocs_DesktopTopNav, .vocs_MobileTopNav,
                    .vocs_DocsLayout_gutterLeft, .vocs_DocsLayout_gutterTop,
                    .vocs_DocsLayout_gutterRight, .vocs_DocsLayout_gutterBottom,
                    .vocs_Outline, .vocs_TableOfContents {
                        display: none !important;
                    }
                    
                    /* Hide vocs controls and UI elements */
                    .vocs_Footer, .vocs_Socials, .vocs_AiCtaDropdown,
                    .vocs_ThemeToggle, .vocs_SearchButton, .vocs_SearchBox,
                    .vocs_NavigationMenu, .vocs_Banner, .vocs_Announcement,
                    .vocs_EditLink, .vocs_PrevNext, .vocs_Contributors {
                        display: none !important;
                    }
                    
                    /* Hide version selector and other dropdowns */
                    .vocs_NavigationMenu_trigger, .vocs_NavigationMenu_content,
                    .vocs_Dropdown, .vocs_Select, .vocs_CommandPalette {
                        display: none !important;
                    }
                    
                    /* Expand content to full width */
                    .vocs_DocsLayout_content, .vocs_Content,
                    #vocs-content, .vocs_DocsLayout,
                    .vocs_Container, .vocs_DocsLayout_main {
                        margin-left: 0 !important;
                        margin-right: 0 !important;
                        max-width: none !important;
                        padding: 20px !important;
                        width: 100% !important;
                        box-sizing: border-box !important;
                        grid-column: 1 / -1 !important;
                    }
                    
                    /* Reset grid layouts */
                    .vocs_DocsLayout {
                        display: block !important;
                        grid-template-columns: none !important;
                        grid-template-areas: none !important;
                    }
                    
                    /* Ensure main content area uses full width */
                    main, article, .vocs_Content_article,
                    .vocs_markdown, .markdown-body {
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
                    pre, code, .vocs_Code {
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
                    table, .vocs_Table {
                        border-collapse: collapse !important;
                        width: 100% !important;
                        margin: 1em 0 !important;
                    }
                    
                    table th, table td, .vocs_Table th, .vocs_Table td {
                        border: 1px solid #ddd !important;
                        padding: 8px !important;
                        text-align: left !important;
                    }
                    
                    table th, .vocs_Table th {
                        background-color: #f5f5f5 !important;
                        font-weight: bold !important;
                    }
                    
                    /* Hide any overlay elements */
                    .vocs_Modal, .vocs_Overlay, .vocs_Backdrop,
                    .overlay, .modal, .popup,
                    .banner, .notice, .alert {
                        display: none !important;
                    }
                    
                    /* Ensure proper spacing for content */
                    .vocs_Content > * {
                        margin-bottom: 1em !important;
                    }
                `;
                document.head.appendChild(style);
                
                console.log('vocs page prepared for PDF');
            })()
        "#).await.map_err(|e| anyhow!("Failed to prepare page: {e}"))?;
        
        Ok(())
    }
    
    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let version_result = page.evaluate(r#"
            (() => {
                // Check for version in top navigation
                const navTrigger = document.querySelector('.vocs_NavigationMenu_trigger');
                if (navTrigger && navTrigger.textContent) {
                    const triggerText = navTrigger.textContent.trim();
                    if (/^\d+\.\d+\.\d+$/.test(triggerText)) {
                        return `v${triggerText}`;
                    }
                }
                
                // Check generator meta tag
                const meta = document.querySelector('meta[name="generator"]');
                if (meta && meta.getAttribute('content')) {
                    const content = meta.getAttribute('content');
                    const match = content.match(/vocs\s*v?([0-9\.]+)/i);
                    if (match) {
                        return `v${match[1]}`;
                    }
                }
                
                // Check for version in window object or script tags
                if (typeof window !== 'undefined' && window.vocs) {
                    if (window.vocs.version) {
                        return `v${window.vocs.version}`;
                    }
                }
                
                // Check for version in page content or comments
                const html = document.documentElement.outerHTML;
                const versionMatch = html.match(/vocs\s*v?([0-9\.]+)/i);
                if (versionMatch) {
                    return `v${versionMatch[1]}`;
                }
                
                // Check for data-vocs attribute value
                const htmlEl = document.documentElement;
                const vocsData = htmlEl.getAttribute('data-vocs');
                if (vocsData && /^\d+\.\d+\.\d+$/.test(vocsData)) {
                    return `v${vocsData}`;
                }
                
                // Default to v1.x since current vocs versions are all v1
                if (htmlEl.hasAttribute('data-vocs') || 
                    document.querySelector('.vocs_DocsLayout')) {
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
        "vocs Handler"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v1.x+"]
    }
}