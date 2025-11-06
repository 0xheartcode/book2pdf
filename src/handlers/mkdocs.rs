use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for MkDocs documentation sites
pub struct MkDocsHandler;

#[async_trait]
impl SiteDetector for MkDocsHandler {
    async fn can_handle(&self, _url: &str, page: &Page) -> Result<ConfidenceLevel> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        // High confidence - explicit MkDocs mentions
        if content.contains("Material for MkDocs") || content.contains("Made with Material for MkDocs") {
            return Ok(ConfidenceLevel::Certain);
        }
        
        // High confidence - MkDocs-specific patterns
        if content.contains("mkdocs") && (content.contains("md-nav") || content.contains("md-main")) {
            return Ok(ConfidenceLevel::High);
        }
        
        // Medium confidence - Material theme CSS classes
        if content.contains("md-nav") && content.contains("md-sidebar") {
            return Ok(ConfidenceLevel::High);
        }
        
        // Medium confidence - ReadTheDocs theme on MkDocs
        if content.contains("wy-nav-side") && content.contains("wy-menu") {
            return Ok(ConfidenceLevel::Medium);
        }
        
        // Low confidence - basic MkDocs indicators
        if content.contains("md-main") || content.contains("md-content") {
            return Ok(ConfidenceLevel::Low);
        }
        
        Ok(ConfidenceLevel::None)
    }
    
    fn version(&self) -> &str {
        "auto"
    }
    
    fn format_name(&self) -> &str {
        "mkdocs"
    }
}

#[async_trait]
impl FormatHandler for MkDocsHandler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        let js_code = r#"
            (async () => {
                // For Material for MkDocs - expand collapsible nav items
                const materialExpandables = document.querySelectorAll([
                    '.md-nav__item--nested > .md-nav__link',
                    'input[type="checkbox"].md-nav__toggle',
                    '.md-nav__item .md-nav__toggle',
                    'label.md-nav__link[for]'
                ].join(', '));
                
                for (let item of materialExpandables) {
                    if (item.type === 'checkbox') {
                        item.checked = true;
                    } else {
                        item.click();
                    }
                }
                
                // For ReadTheDocs theme - expand toctree
                const rtdExpandables = document.querySelectorAll([
                    '.wy-menu .toctree-l1.current > a',
                    '.wy-menu .toctree-expand',
                    '.wy-menu .current .toctree-expand'
                ].join(', '));
                
                for (let item of rtdExpandables) {
                    item.click();
                }
                
                // For standard MkDocs theme - expand nav
                const standardExpandables = document.querySelectorAll([
                    '.nav-item.dropdown > a',
                    '.toctree .current > a'
                ].join(', '));
                
                for (let item of standardExpandables) {
                    item.click();
                }
                
                // Wait for animations
                await new Promise(r => setTimeout(r, 1000));
            })();
        "#;

        page.evaluate(js_code)
            .await
            .map_err(|e| anyhow!("Failed to expand MkDocs navigation: {}", e))?;

        Ok(())
    }
    
    async fn extract_links(&self, page: &Page, base_url: &Url) -> Result<Vec<String>> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        let document = Html::parse_document(&content);
        let mut links = Vec::new();
        let mut seen = HashSet::new();
        
        // MkDocs-specific navigation selectors (ordered by priority)
        let nav_selectors = [
            // Material for MkDocs theme - both absolute and relative links
            ".md-nav--primary .md-nav__link[href]",
            ".md-nav__list .md-nav__link[href]",
            ".md-sidebar .md-nav__link[href]",
            
            // ReadTheDocs theme
            ".wy-menu-vertical .toctree-l1 > a[href]",
            ".wy-menu-vertical a[href]",
            ".wy-nav-side a[href]",
            
            // Standard MkDocs theme
            ".nav a[href]",
            ".toctree a[href]",
            "nav ul a[href]",
            
            // Fallback - any navigation-like links
            "aside a[href]",
            ".sidebar a[href]",
            "nav a[href]",
        ];
        
        // Collect navigation links in order of priority
        for selector_str in &nav_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        // Accept both absolute and relative paths, but filter out unwanted ones
                        let is_valid_link = href.ends_with(".html") || 
                                          href.ends_with("/") ||
                                          (href.starts_with('/') && !href.contains('#') && !href.contains("/assets/") && !href.contains("/static/")) ||
                                          (!href.starts_with("http") && !href.starts_with("mailto:") && !href.starts_with("javascript:") && !href.contains('#'));
                        
                        if is_valid_link && seen.insert(href.to_string()) {
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
        
        debug!("MkDocs handler collected {} unique links", full_urls.len());
        Ok(full_urls)
    }
    
    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let content = page.content().await?;
        
        // Check for Material for MkDocs version
        if content.contains("Material for MkDocs") {
            // Try to extract version from meta tags or scripts
            if let Some(start) = content.find("material-") {
                if let Some(version_start) = content[start..start + 50].find("-") {
                    if let Some(version_end) = content[start + version_start + 1..start + version_start + 20].find(&['"', '\'', ' '][..]) {
                        let version = &content[start + version_start + 1..start + version_start + 1 + version_end];
                        if !version.is_empty() && version.chars().next().unwrap().is_ascii_digit() {
                            return Ok(Some(format!("Material v{}", version)));
                        }
                    }
                }
            }
            return Ok(Some("Material theme".to_string()));
        }
        
        // Check for ReadTheDocs theme
        if content.contains("wy-nav-side") || content.contains("readthedocs") {
            return Ok(Some("ReadTheDocs theme".to_string()));
        }
        
        // Check for standard MkDocs
        if content.contains("mkdocs") {
            return Ok(Some("Standard theme".to_string()));
        }
        
        Ok(None)
    }

    async fn prepare_page(&self, page: &Page) -> Result<()> {
        let js_code = r#"
            // Remove MkDocs-specific UI elements
            const itemSelectorsToRemove = [
                // Material for MkDocs theme
                '.md-header',
                '.md-sidebar',
                '.md-nav',
                '.md-search',
                '.md-footer',
                '.md-top',
                
                // ReadTheDocs theme
                '.wy-nav-side',
                '.wy-nav-top',
                '.wy-breadcrumbs',
                '.rst-footer-buttons',
                '.wy-form',
                
                // Standard theme
                '.navbar',
                '.nav-sidebar',
                '.source-links',
                
                // Common elements
                'nav',
                '.navigation',
                '.sidebar',
                '.search',
                '.edit-link',
                '.page-edit',
                'a[href*="edit"]',
                'a[href*="github.com"][href*="edit"]'
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

            // Expand any remaining collapsible content
            const collapsibleElements = document.querySelectorAll([
                'details',
                '.collapsible',
                '.admonition.collapsible'
            ].join(', '));
            
            for (let element of collapsibleElements) {
                if (element.tagName === 'DETAILS') {
                    element.open = true;
                } else {
                    element.click();
                }
            }
        "#;

        page.evaluate(js_code)
            .await
            .map_err(|e| anyhow!("Failed to prepare MkDocs page: {}", e))?;

        Ok(())
    }
    
    fn name(&self) -> &str {
        "MkDocs Handler"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["Material theme", "ReadTheDocs theme", "Standard theme"]
    }
}