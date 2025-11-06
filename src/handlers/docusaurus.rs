use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for Docusaurus documentation sites
pub struct DocusaurusHandler;

#[async_trait]
impl SiteDetector for DocusaurusHandler {
    async fn can_handle(&self, _url: &str, page: &Page) -> Result<ConfidenceLevel> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        let document = Html::parse_document(&content);
        
        // Check for Docusaurus-specific elements
        let docusaurus_selectors = [
            "div#__docusaurus",
            "div.docusaurus-root",
            "nav.navbar--fixed-top",
            "div.navbar__logo",
            "script[src*=\"docusaurus\"]",
        ];
        
        for selector_str in &docusaurus_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    debug!("Detected Docusaurus site with selector: {}", selector_str);
                    return Ok(ConfidenceLevel::Certain);
                }
            }
        }
        
        // Check for Docusaurus in script content
        let script_selector = Selector::parse("script")
            .map_err(|e| anyhow!("Invalid selector: {e}"))?;
        for script in document.select(&script_selector) {
            let content = script.text().collect::<String>();
            if content.contains("docusaurus") || content.contains("__DOCUSAURUS__") {
                debug!("Detected Docusaurus site from script content");
                return Ok(ConfidenceLevel::High);
            }
        }
        
        Ok(ConfidenceLevel::None)
    }
    
    fn version(&self) -> &str {
        "auto"
    }
    
    fn format_name(&self) -> &str {
        "docusaurus"
    }
}

#[async_trait]
impl FormatHandler for DocusaurusHandler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        let js_code = r#"
            (async () => {
                // For Docusaurus - expand collapsible sidebar categories
                const docusaurusExpandables = document.querySelectorAll([
                    '.menu__list-item--collapsed > .menu__link',
                    '.menu__link--sublist[aria-expanded="false"]',
                    'button.menu__link--sublist',
                    '.theme-doc-sidebar-item-category button[aria-expanded="false"]',
                    '.menu__caret', // Docusaurus v2 caret
                    '[class*="collapsible"] button[aria-expanded="false"]'
                ].join(', '));
                
                for (let item of docusaurusExpandables) {
                    item.click();
                }
                
                // Also try to click on category headers directly
                const categoryHeaders = document.querySelectorAll('.menu__list-item--collapsed');
                for (let header of categoryHeaders) {
                    header.click();
                }
                
                // Wait a bit for animations
                await new Promise(r => setTimeout(r, 1000));
            })();
        "#;

        page.evaluate(js_code)
            .await
            .map_err(|e| anyhow!("Failed to expand Docusaurus navigation: {e}"))?;

        Ok(())
    }
    
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        let document = Html::parse_document(&content);
        let mut links = Vec::new();
        let mut seen = HashSet::new();
        
        // Docusaurus-specific navigation selectors
        let nav_selectors = [
            "nav.navbar a[href^=\"/\"]",                           // Navbar links
            ".menu a[href^=\"/\"]",                                // Docusaurus menu
            ".theme-doc-sidebar-menu a[href^=\"/\"]",              // Docusaurus sidebar
            ".sidebar a[href^=\"/\"]",                             // Sidebar links
            ".theme-doc-sidebar-item-link[href^=\"/\"]",           // Specific Docusaurus sidebar links
            "nav a[href^=\"/\"]",                                  // General nav links
        ];
        
        // Collect navigation links in order
        for selector_str in &nav_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if href.starts_with('/') && !href.contains('#') && !href.contains("/assets/")
                            && seen.insert(href.to_string()) {
                                links.push(href.to_string());
                            }
                    }
                }
            }
        }
        
        // Convert relative paths to full URLs
        let mut full_urls = Vec::new();
        for link in links {
            if let Ok(full_url) = base_url.join(&link) {
                full_urls.push(full_url.to_string());
            }
        }
        
        debug!("Docusaurus handler collected {} unique links", full_urls.len());
        Ok(full_urls)
    }
    
    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let content = page.content().await?;
        
        // Quick checks for version info - reuse content we already fetched
        
        // Check for meta generator tag - most accurate method
        if let Some(start) = content.find(r#"content="Docusaurus v"#) {
            if let Some(version_start) = content[start..].find("v") {
                if let Some(version_end) = content[start + version_start..].find('"') {
                    let version = &content[start + version_start..start + version_start + version_end];
                    return Ok(Some(version.to_string()));
                }
            }
        }
        
        // Also check for window.docusaurus object
        if let Some(start) = content.find("window.docusaurus") {
            if let Some(version_start) = content[start..start + 200].find(r#""version":""#) {
                let search_start = start + version_start + 11;
                if let Some(version_end) = content[search_start..search_start + 50].find('"') {
                    let version = &content[search_start..search_start + version_end];
                    return Ok(Some(format!("v{version}")));
                }
            }
        }
        
        // Check for common Docusaurus v2 indicators
        if content.contains("navbar__inner") || content.contains("theme-doc-sidebar-container") {
            return Ok(Some("v2.x".to_string()));
        }
        
        // Check for Docusaurus v1 indicators  
        if content.contains("fixedHeaderContainer") || content.contains("onPageNav") {
            return Ok(Some("v1.x".to_string()));
        }
        
        Ok(None)
    }

    async fn prepare_page(&self, page: &Page) -> Result<()> {
        let js_code = r#"
            // Remove Docusaurus-specific interactive elements
            const itemSelectorsToRemove = [
                'nav.navbar',
                '.sidebar',
                '.theme-doc-sidebar-container',
                '.navbar',
                '.navbar__inner',
                '.theme-edit-this-page',
                '.theme-last-updated',
                '.pagination-nav',
                '.theme-doc-footer',
                '.theme-doc-toc-mobile',
                'button[title="Toggle navigation bar"]',
                'button[aria-label="Toggle navigation bar"]',
                '.theme-code-block-highlighted-line'
            ];

            for (let selectorStr of itemSelectorsToRemove) {
                try {
                    const elements = document.querySelectorAll(selectorStr);
                    for (let element of elements) {
                        element.remove();
                    }
                } catch (e) {
                    console.log('Failed to remove elements with selector:', selectorStr);
                }
            }

            // Expand any collapsible content
            const collapsibleElements = document.querySelectorAll('[data-collapsible]');
            for (let element of collapsibleElements) {
                if (element.getAttribute('data-collapsible') === 'true') {
                    element.click();
                }
            }
        "#;

        page.evaluate(js_code)
            .await
            .map_err(|e| anyhow!("Failed to prepare Docusaurus page: {e}"))?;

        Ok(())
    }
    
    fn name(&self) -> &str {
        "Docusaurus Handler"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v1.x", "v2.x", "v3.x", "v4.x"]
    }
}