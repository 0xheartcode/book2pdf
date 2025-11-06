use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};

/// Handler for GitBook documentation sites
pub struct GitBookHandler;

#[async_trait]
impl SiteDetector for GitBookHandler {
    async fn can_handle(&self, _url: &str, page: &Page) -> Result<ConfidenceLevel> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;
        
        let document = Html::parse_document(&content);
        
        // Check for old GitBook format
        let old_format_selector = Selector::parse("body > .gitbook-root")
            .map_err(|e| anyhow!("Invalid selector: {}", e))?;
        if document.select(&old_format_selector).next().is_some() {
            return Ok(ConfidenceLevel::Certain);
        }
        
        // Check for new GitBook format selectors
        let new_format_selectors = [
            "body > div.scroll-nojump",
            "nav[role=\"navigation\"]",
            "a[href*=\"gitbook.io\"]",
        ];
        
        let mut confidence = ConfidenceLevel::None;
        
        for selector_str in &new_format_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    confidence = ConfidenceLevel::High;
                    break;
                }
            }
        }
        
        // Check body class for theme indication
        if confidence == ConfidenceLevel::None {
            let body_selector = Selector::parse("body")
                .map_err(|e| anyhow!("Invalid selector: {}", e))?;
            if let Some(body) = document.select(&body_selector).next() {
                if let Some(class) = body.value().attr("class") {
                    if class.contains("theme-") {
                        confidence = ConfidenceLevel::Medium;
                    }
                }
            }
        }
        
        Ok(confidence)
    }
    
    fn version(&self) -> &str {
        "auto"
    }
    
    fn format_name(&self) -> &str {
        "gitbook"
    }
}

#[async_trait]
impl FormatHandler for GitBookHandler {
    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        let js_code = r#"
            (async () => {
                // For old GitBook format - expand TOC menu items
                const oldFormatElements = document
                    .querySelectorAll('a[data-rnwrdesktop-fnigne="true"] > div[tabindex="0"]');

                for (let element of oldFormatElements) {
                    element.click();
                }
                
                // For new GitBook format - look for expandable navigation items
                const expandButtons = document.querySelectorAll([
                    'button[aria-expanded="false"]',
                    'button[data-state="closed"]',
                    '[role="button"][aria-expanded="false"]'
                ].join(', '));
                
                for (let button of expandButtons) {
                    button.click();
                }
                
                // Wait a bit for animations
                await new Promise(r => setTimeout(r, 1000));
            })();
        "#;

        page.evaluate(js_code)
            .await
            .map_err(|e| anyhow!("Failed to expand GitBook navigation: {}", e))?;

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
        
        // GitBook-specific navigation selectors
        let nav_selectors = [
            "nav[role=\"navigation\"] a[href^=\"/\"]",  // Main navigation
            ".gitbook-root nav a[href^=\"/\"]",        // Old GitBook nav
            "aside a[href^=\"/\"]",                     // Sidebar links
            "nav a[href^=\"/\"]",                       // General nav links
        ];
        
        // Collect navigation links in order
        for selector_str in &nav_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if href.starts_with('/') && !href.contains('#') && !href.contains("/assets/") {
                            if seen.insert(href.to_string()) {
                                links.push(href.to_string());
                            }
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
        
        debug!("GitBook handler collected {} unique links", full_urls.len());
        Ok(full_urls)
    }
    
    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let content = page.content().await?;
        
        // Lightweight version detection using existing content
        
        // Check for GitBook v3/v4 indicators
        if content.contains("gitbook-root") || content.contains("scroll-nojump") {
            return Ok(Some("v3+".to_string()));
        }
        
        // Check for older GitBook v2 indicators
        if content.contains(r#"class="book""#) || content.contains("gitbook-2") {
            return Ok(Some("v2.x".to_string()));
        }
        
        Ok(None)
    }

    async fn prepare_page(&self, page: &Page) -> Result<()> {
        let js_code = r#"
            // Expand all expandable sections
            const sectionsToExpand = document
                .querySelectorAll('div[aria-controls^="expandable-body-"]');

            for (let section of sectionsToExpand) {
                section.click();
            }

            // Remove redundant/interactive elements specific to GitBook
            const itemSelectorsToRemove = [
                'header + div[data-rnwrdesktop-hidden="true"]',
                '.gitbook-root header',
                '[data-testid="page-header"]',
                'button[aria-label="Copy link"]',
                '.copy-button',
                'nav[role="navigation"]',
                '.sidebar',
                '.page-navigator'
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
        "#;

        page.evaluate(js_code)
            .await
            .map_err(|e| anyhow!("Failed to prepare GitBook page: {}", e))?;

        Ok(())
    }
    
    fn name(&self) -> &str {
        "GitBook Handler"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v2.x", "v3.x", "v4.x"]
    }
}