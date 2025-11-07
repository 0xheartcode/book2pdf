use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for Starlight (Astro-based) documentation sites
pub struct StarlightHandler;

#[async_trait]
impl SiteDetector for StarlightHandler {
    fn version(&self) -> &str {
        "v0.x"
    }
    
    fn format_name(&self) -> &str {
        "starlight"
    }
    async fn can_handle(&self, _url: &str, page: &Page) -> Result<ConfidenceLevel> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        let document = Html::parse_document(&content);
        
        // Primary Starlight detection patterns
        let has_starlight_css = content.contains("@layer starlight") ||
                               content.contains("starlight.components");
                               
        let has_starlight_js = content.contains("StarlightThemeProvider") ||
                              content.contains("starlight-theme-select") ||
                              content.contains("starlight-lang-select");
        
        // Check for Starlight-specific CSS class patterns
        let starlight_selectors = [
            ".starlight-aside",              // Starlight aside components
            ".sl-badge",                     // Starlight badges
            ".sl-link-card",                 // Starlight link cards
            ".sl-steps",                     // Starlight step lists
            "starlight-theme-select",        // Theme selector component
            "starlight-lang-select",         // Language selector component
            "[data-starlight-theme]",        // Theme data attribute
            ".sidebar",                      // Common sidebar
            "nav[aria-label*='Main']"        // Main navigation
        ];
        
        let mut starlight_class_matches = 0;
        
        for selector_str in &starlight_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    starlight_class_matches += 1;
                    debug!("Found Starlight selector: {}", selector_str);
                }
            }
        }
        
        // Check for Starlight meta patterns
        let has_starlight_meta = document.select(&Selector::parse(r#"meta[name="generator"]"#).unwrap())
            .any(|element| {
                if let Some(content) = element.value().attr("content") {
                    content.to_lowercase().contains("astro") || 
                    content.to_lowercase().contains("starlight")
                } else {
                    false
                }
            });
        
        // Check for CSS variables with --sl- prefix
        let has_starlight_vars = content.contains("--sl-") ||
                                content.contains("var(--sl-");
        
        // Confidence logic
        if (has_starlight_css || has_starlight_js) && starlight_class_matches >= 3 {
            Ok(ConfidenceLevel::Certain)
        } else if (has_starlight_css || has_starlight_js) && starlight_class_matches >= 1 {
            Ok(ConfidenceLevel::High)
        } else if starlight_class_matches >= 3 && (has_starlight_meta || has_starlight_vars) {
            Ok(ConfidenceLevel::High)
        } else if starlight_class_matches >= 2 || has_starlight_meta || has_starlight_vars {
            Ok(ConfidenceLevel::Medium)
        } else if starlight_class_matches >= 1 {
            Ok(ConfidenceLevel::Low)
        } else {
            Ok(ConfidenceLevel::None)
        }
    }
}

#[async_trait]
impl FormatHandler for StarlightHandler {
    fn name(&self) -> &str {
        "Starlight"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v0.x"]
    }

    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        // Try to extract Astro version from meta generator tag
        let document = Html::parse_document(&content);
        
        if let Some(meta) = document.select(&Selector::parse(r#"meta[name="generator"]"#).unwrap()).next() {
            if let Some(generator_content) = meta.value().attr("content") {
                if generator_content.to_lowercase().contains("astro") {
                    // Extract version if present (e.g., "Astro v4.0.0")
                    if let Some(version_start) = generator_content.find("v") {
                        if let Some(version_part) = generator_content.get(version_start..) {
                            if let Some(space_pos) = version_part.find(' ') {
                                return Ok(Some(format!("Astro {}", &version_part[..space_pos])));
                            } else {
                                return Ok(Some(format!("Astro {}", version_part)));
                            }
                        }
                    }
                    return Ok(Some("Astro (Starlight)".to_string()));
                }
            }
        }
        
        // Fallback to checking for Starlight patterns
        if content.contains("starlight") || content.contains("@layer starlight") {
            Ok(Some("Starlight (unknown version)".to_string()))
        } else {
            Ok(None)
        }
    }

    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        // Starlight typically doesn't require navigation expansion as it's static HTML
        // However, we'll try to expand any collapsible sections
        page.evaluate(r#"
            // Expand any collapsible navigation sections
            document.querySelectorAll('[aria-expanded="false"]').forEach(element => {
                try {
                    element.setAttribute('aria-expanded', 'true');
                    element.click();
                } catch (e) {
                    console.debug('Could not expand element:', e);
                }
            });
            
            // Expand any collapsed details elements
            document.querySelectorAll('details:not([open])').forEach(details => {
                details.open = true;
            });
            
            // Wait for any animations to complete
            new Promise(resolve => setTimeout(resolve, 500));
        "#).await.map_err(|e| anyhow!("Failed to expand navigation: {e}"))?;

        Ok(())
    }

    async fn extract_links(&self, page: &Page, _base_url: &Url) -> Result<Vec<String>> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;

        let document = Html::parse_document(&content);
        let mut links = Vec::new();
        let mut seen = HashSet::new();

        // Starlight navigation selectors (try multiple patterns)
        let nav_selectors = [
            "nav[aria-label*='Main'] a[href]",           // Main navigation
            ".sidebar a[href]",                           // Sidebar navigation
            "nav[aria-label*='Sidebar'] a[href]",        // Labeled sidebar
            ".sl-sidebar a[href]",                        // Starlight sidebar
            "[data-sidebar] a[href]",                     // Data attribute sidebar
            "aside nav a[href]",                          // Navigation in aside
            ".starlight-sidebar a[href]",                 // Starlight-specific
            ".navigation a[href]",                        // Generic navigation
            ".docs-navigation a[href]"                    // Docs navigation
        ];

        for selector_str in &nav_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        // Filter internal documentation links
                        if href.starts_with('/') && 
                           !href.contains('#') && 
                           !href.contains("/assets/") &&
                           !href.contains("/api/") &&
                           !href.ends_with(".pdf") &&
                           !href.ends_with(".zip") &&
                           !href.ends_with(".tar.gz") &&
                           seen.insert(href.to_string()) {
                            links.push(href.to_string());
                            debug!("Found Starlight link: {}", href);
                        }
                    }
                }
            }
        }

        // If no navigation links found, try content links as fallback
        if links.is_empty() {
            debug!("No navigation links found, trying content links as fallback");
            
            let content_selectors = [
                "main a[href]",
                "article a[href]",
                ".content a[href]",
                "#content a[href]"
            ];
            
            for selector_str in &content_selectors {
                if let Ok(selector) = Selector::parse(selector_str) {
                    for element in document.select(&selector) {
                        if let Some(href) = element.value().attr("href") {
                            if href.starts_with('/') && 
                               !href.contains('#') && 
                               !href.contains("/assets/") &&
                               seen.insert(href.to_string()) {
                                links.push(href.to_string());
                            }
                        }
                    }
                }
                
                // Limit fallback links to avoid too many
                if links.len() >= 10 {
                    break;
                }
            }
        }

        debug!("Extracted {} Starlight navigation links", links.len());
        Ok(links)
    }

    async fn prepare_page(&self, page: &Page) -> Result<()> {
        // Prepare Starlight page for PDF generation
        page.evaluate(r#"
            // Hide unnecessary elements for PDF
            const hideSelectors = [
                '.sl-theme-select',                    // Theme selector
                'starlight-theme-select',              // Theme selector component
                '.theme-toggle',                       // Theme toggle buttons
                'starlight-lang-select',               // Language selector
                '.language-select',                    // Language selection
                '[data-theme-toggle]',                 // Theme toggle elements
                '.search',                             // Search functionality
                '.search-dialog',                      // Search dialogs
                'nav[aria-label*="Breadcrumb"]',       // Breadcrumb navigation
                '.edit-page',                          // Edit page links
                '.github-link',                        // GitHub links
                '.social-links',                       // Social media links
                'footer nav'                           // Footer navigation
            ];

            hideSelectors.forEach(selector => {
                document.querySelectorAll(selector).forEach(element => {
                    element.style.display = 'none';
                });
            });

            // Expand all collapsible content for PDF
            document.querySelectorAll('details:not([open])').forEach(details => {
                details.open = true;
            });
            
            // Expand all tabs if present
            document.querySelectorAll('[role="tabpanel"]').forEach(panel => {
                panel.style.display = 'block';
                panel.removeAttribute('hidden');
            });

            // Ensure proper styling for print
            const style = document.createElement('style');
            style.textContent = `
                @media print {
                    .sidebar { display: none !important; }
                    .sl-theme-select { display: none !important; }
                    starlight-theme-select { display: none !important; }
                    main { margin: 0 !important; padding: 20px !important; }
                    article { max-width: none !important; }
                    .content { max-width: none !important; }
                    a { color: black !important; text-decoration: underline !important; }
                    pre { background: #f5f5f5 !important; border: 1px solid #ddd !important; }
                }
            `;
            document.head.appendChild(style);

            // Wait for any layout changes to complete
            new Promise(resolve => setTimeout(resolve, 500));
        "#).await.map_err(|e| anyhow!("Failed to prepare page: {e}"))?;

        Ok(())
    }
}