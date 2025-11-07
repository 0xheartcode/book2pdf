use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};
use crate::handlers::utils;

/// Handler for Nextra documentation sites (Next.js-based documentation framework)
pub struct NextraHandler;

#[async_trait]
impl SiteDetector for NextraHandler {
    fn version(&self) -> &str {
        "v2.x-4.x"
    }
    
    fn format_name(&self) -> &str {
        "nextra"
    }
    
    async fn can_handle(&self, _url: &str, content: &str) -> Result<ConfidenceLevel> {
        let document = Html::parse_document(content);
        
        // Primary Nextra detection patterns
        let has_nextra_meta = utils::has_meta_generator(&document, "nextra");
        
        // Check for Next.js with Nextra-specific patterns
        let has_nextjs_meta = utils::has_meta_generator(&document, "Next.js");
        
        // Nextra-specific CSS patterns and classes
        let nextra_indicators = [
            // CSS Variables (stable across versions)
            "--nextra-navbar-height",
            "--nextra-primary-hue", 
            "--nextra-primary-saturation",
            "--nextra-content-width",
            
            // Class patterns (version-specific but detectable)
            "nextra-navbar",
            "nextra-content", 
            "nextra-sidebar",
            "nextra-toc",
            "nextra-border",
            "nextra-mask",
            "nextra-banner",
            
            // Version-specific class patterns
            "nx-",                    // v2.x prefix
            "x\\:",                   // v4.x prefix (escaped)
            
            // Component patterns
            "nextra-theme-",
            "nextra-docs",
            "nextra-blog",
            
            // File/asset patterns
            "_nextra/",
            "nextra.js",
            "nextra.css",
            
            // MDX patterns
            "mdx-content",
            "__nextra",
            
            // Navigation patterns
            "aria-label=\"table of contents\"",
            "data-nextra",
            
            // Search patterns
            "nextra-search",
            "flexsearch",
            "pagefind"
        ];
        
        let nextra_matches = utils::count_text_indicators(content, &nextra_indicators);
        debug!("Found {} Nextra text indicators", nextra_matches);
        
        // Check for Nextra-specific CSS selectors and structural patterns
        let nextra_selectors = [
            ".nextra-navbar",             // Main navbar
            ".nextra-sidebar",            // Sidebar navigation  
            ".nextra-content",            // Main content area
            ".nextra-toc",                // Table of contents
            ".nextra-border",             // Border utility
            "[class*='nextra-']",         // Any nextra-prefixed class
            "[style*='--nextra-']",       // CSS custom properties
            "header[class*='navbar']",    // Header navigation
            "nav[aria-label='table of contents']", // TOC navigation
            "main[class*='max-w']",       // Main content with max-width
            "[class*='nx-']",             // v2.x class prefix
            "[class*='x\\:']",            // v4.x class prefix
            "article[class*='min-h']",    // Content article
            ".mdx-content",               // MDX content wrapper
            "[data-nextra]",              // Nextra data attributes
            ".nextra-search",             // Search functionality
        ];
        
        let css_matches = nextra_selectors.iter()
            .filter(|&selector_str| utils::has_css_selector(&document, selector_str))
            .count();
        debug!("Found {} Nextra CSS selectors", css_matches);
        
        // Detect specific Nextra themes
        let has_docs_theme = content.contains("nextra-theme-docs") || 
                            content.contains("nextra/theme-docs");
        
        let has_blog_theme = content.contains("nextra-theme-blog") ||
                            content.contains("nextra/theme-blog");
        
        // Check for MDX-specific patterns (Nextra is MDX-based)
        let has_mdx_patterns = content.contains("mdxComponents") ||
                              content.contains("MDXContent") ||
                              content.contains("__nextra");
        
        // JavaScript bundle patterns
        let has_js_patterns = content.contains("nextra") && 
                             (content.contains("_buildManifest.js") || 
                              content.contains("_ssgManifest.js") ||
                              content.contains("chunks/"));
        
        // Confidence scoring logic
        if has_nextra_meta && (nextra_matches >= 3 || css_matches >= 2) {
            Ok(ConfidenceLevel::Certain)
        } else if (has_nextjs_meta && (has_docs_theme || has_blog_theme)) && 
                  (nextra_matches >= 4 || css_matches >= 3) {
            Ok(ConfidenceLevel::High)
        } else if has_nextjs_meta && has_mdx_patterns && 
                  (nextra_matches >= 3 || css_matches >= 2) {
            Ok(ConfidenceLevel::High)  
        } else if has_js_patterns && (nextra_matches >= 2 || css_matches >= 2) {
            Ok(ConfidenceLevel::Medium)
        } else if nextra_matches >= 3 || css_matches >= 2 {
            Ok(ConfidenceLevel::Medium)
        } else if nextra_matches >= 2 || css_matches >= 1 {
            Ok(ConfidenceLevel::Low)
        } else {
            Ok(ConfidenceLevel::None)
        }
    }
}

#[async_trait]
impl FormatHandler for NextraHandler {
    fn name(&self) -> &str {
        "Nextra"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v2.x", "v3.x", "v4.x"]
    }

    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        // Try to detect specific Nextra version
        if content.contains("x\\:") || content.contains("x:") {
            return Ok(Some("Nextra v4.x (App Router)".to_string()));
        }
        
        if content.contains("nx-") {
            return Ok(Some("Nextra v2.x (Pages Router)".to_string()));
        }
        
        // Check for theme information
        if content.contains("nextra-theme-docs") {
            if content.contains("app/") || content.contains("App Router") {
                return Ok(Some("Nextra v4.x (Docs Theme)".to_string()));
            } else {
                return Ok(Some("Nextra v3.x (Docs Theme)".to_string()));
            }
        }
        
        if content.contains("nextra-theme-blog") {
            return Ok(Some("Nextra (Blog Theme)".to_string()));
        }
        
        // Check for Next.js version indicators
        if content.contains("Next.js") {
            // Try to extract Next.js version which can indicate Nextra version
            let document = Html::parse_document(&content);
            if let Ok(selector) = Selector::parse(r#"meta[name="generator"]"#) {
                if let Some(meta) = document.select(&selector).next() {
                    if let Some(generator_content) = meta.value().attr("content") {
                        return Ok(Some(format!("Nextra ({})", generator_content)));
                    }
                }
            }
        }
        
        // Fallback detection
        Ok(Some("Nextra (version unknown)".to_string()))
    }

    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        // Expand Nextra navigation - works across multiple versions
        page.evaluate(r#"
            // Expand collapsed navigation sections
            document.querySelectorAll('[aria-expanded="false"]').forEach(element => {
                try {
                    element.setAttribute('aria-expanded', 'true');
                    element.click();
                } catch (e) {
                    console.debug('Could not expand element:', e);
                }
            });
            
            // Expand details elements (common in Nextra)
            document.querySelectorAll('details:not([open])').forEach(details => {
                details.open = true;
            });
            
            // Expand any collapsed sidebar sections
            document.querySelectorAll('.nextra-sidebar [data-collapsed="true"]').forEach(element => {
                element.setAttribute('data-collapsed', 'false');
            });
            
            // Click any toggle buttons in navigation
            document.querySelectorAll('.nextra-sidebar button[aria-expanded="false"]').forEach(button => {
                try {
                    button.click();
                } catch (e) {
                    console.debug('Could not click toggle button:', e);
                }
            });
            
            // Expand any nested navigation items (version-agnostic selectors)
            document.querySelectorAll('nav button[role="button"], .sidebar button[aria-expanded]').forEach(button => {
                try {
                    if (button.getAttribute('aria-expanded') === 'false') {
                        button.click();
                    }
                } catch (e) {
                    console.debug('Could not expand nav button:', e);
                }
            });
            
            // Expand mobile navigation if present
            document.querySelectorAll('[data-nextra-menu]').forEach(menu => {
                menu.style.display = 'block';
            });
            
            // Wait for any animations to complete
            new Promise(resolve => setTimeout(resolve, 1000));
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

        // Nextra navigation selectors (cross-version compatible)
        let nav_selectors = [
            // Nextra-specific selectors
            ".nextra-sidebar a[href]",
            ".nextra-navbar a[href]", 
            ".nextra-toc a[href]",
            
            // Semantic navigation selectors
            "header nav a[href]",
            "aside nav a[href]",
            "nav[aria-label='table of contents'] a[href]",
            "nav[aria-label='navigation'] a[href]",
            
            // Theme-specific selectors
            ".nextra-theme-docs nav a[href]",
            ".nextra-theme-blog nav a[href]",
            
            // Version-agnostic structural selectors
            "nav a[href]",
            ".sidebar a[href]",
            "[role='navigation'] a[href]",
            
            // Content area navigation
            "main nav a[href]",
            "article nav a[href]",
            
            // Class pattern-based selectors (works across versions)
            "[class*='nextra'] a[href]",
            "[class*='sidebar'] a[href]",
            "[class*='navbar'] a[href]",
            "[class*='toc'] a[href]",
            
            // CSS custom property-based targeting
            "[style*='--nextra'] a[href]",
            
            // Fallback to main content links
            "main a[href]",
            "article a[href]"
        ];

        for selector_str in &nav_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        // Filter for documentation content links
                        if self.is_valid_nextra_link(href) && seen.insert(href.to_string()) {
                            links.push(href.to_string());
                            debug!("Found Nextra link: {}", href);
                        }
                    }
                }
            }
            
            // Stop if we found enough links from navigation
            if links.len() >= 100 {
                break;
            }
        }

        debug!("Extracted {} Nextra documentation links", links.len());
        Ok(links)
    }

    async fn prepare_page(&self, page: &Page) -> Result<()> {
        // Prepare Nextra page for PDF generation
        page.evaluate(r#"
            // Hide unnecessary elements for PDF (cross-version compatible)
            const hideSelectors = [
                // Nextra-specific elements
                '.nextra-navbar',                  // Top navigation
                '.nextra-sidebar',                 // Left sidebar
                '.nextra-toc',                     // Table of contents sidebar
                '.nextra-banner',                  // Announcement banner
                '.nextra-search',                  // Search functionality
                
                // Generic navigation elements
                'header nav',                      // Header navigation
                'aside nav',                       // Sidebar navigation  
                'nav[aria-label="table of contents"]', // TOC navigation
                
                // Interactive elements
                'button[aria-expanded]',           // Toggle buttons
                '[role="button"]',                 // Button elements
                '.hamburger',                      // Mobile menu button
                '[data-nextra-menu]',             // Mobile menu
                
                // Theme elements
                '.theme-switch',                   // Dark/light mode toggle
                '.locale-switch',                  // Language switcher
                
                // Edit and source links
                '[title*="Edit this page"]',      // Edit page links
                '[href*="github.com"]',           // GitHub links
                '.edit-link',                      // Edit links
                '.source-link',                    // Source code links
                
                // Search and interactive features  
                '[data-search]',                   // Search components
                '.search-box',                     // Search input
                '[class*="search"]',               // Any search-related elements
                
                // Feedback and social elements
                '.feedback',                       // Feedback components
                '.social-links',                   // Social media links
                '[class*="social"]',               // Social elements
                
                // Version and deployment info
                '.version-info',                   // Version information
                '.build-info',                     // Build/deploy information
                
                // Advertisement and tracking
                '.ad',                            // Advertisements
                '[class*="track"]',               // Tracking elements
                
                // Print-specific hiding
                '.no-print',                      // Explicitly marked no-print
                '.print-hidden'                   // Print hidden elements
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
            
            // Show all tab panels content if present
            document.querySelectorAll('[role="tabpanel"]').forEach(panel => {
                panel.style.display = 'block';
                panel.removeAttribute('hidden');
                panel.setAttribute('aria-hidden', 'false');
            });
            
            // Ensure code blocks are fully visible
            document.querySelectorAll('pre, code').forEach(element => {
                element.style.whiteSpace = 'pre-wrap';
                element.style.overflow = 'visible';
            });

            // Add print-specific styles
            const style = document.createElement('style');
            style.textContent = `
                @media print {
                    /* Hide navigation and interactive elements */
                    .nextra-navbar, .nextra-sidebar, .nextra-toc,
                    header, aside, nav { display: none !important; }
                    
                    /* Expand main content */
                    main, article { 
                        margin: 0 !important; 
                        padding: 20px !important;
                        max-width: none !important;
                        width: 100% !important;
                    }
                    
                    /* Improve typography for print */
                    body { 
                        font-size: 12pt !important;
                        line-height: 1.5 !important;
                        color: black !important;
                        background: white !important;
                    }
                    
                    /* Style links for print */
                    a { 
                        color: black !important; 
                        text-decoration: underline !important; 
                    }
                    
                    /* Style code blocks */
                    pre, code { 
                        background: #f5f5f5 !important; 
                        border: 1px solid #ddd !important;
                        color: black !important;
                        page-break-inside: avoid;
                    }
                    
                    /* Style headings */
                    h1, h2, h3, h4, h5, h6 {
                        color: black !important;
                        page-break-after: avoid;
                    }
                    
                    /* Handle page breaks */
                    .nextra-content, article {
                        page-break-inside: avoid;
                    }
                    
                    /* Ensure images fit on page */
                    img {
                        max-width: 100% !important;
                        height: auto !important;
                    }
                }
            `;
            document.head.appendChild(style);

            // Wait for any layout changes to complete
            new Promise(resolve => setTimeout(resolve, 500));
        "#).await.map_err(|e| anyhow!("Failed to prepare page: {e}"))?;

        Ok(())
    }
}

impl NextraHandler {
    /// Check if a link is a valid Nextra documentation link
    fn is_valid_nextra_link(&self, href: &str) -> bool {
        // Include relative links and exclude external/unwanted links
        if href.starts_with("http") {
            let allowed_domains = [".dev", ".com", ".org", ".io"];
            if !allowed_domains.iter().any(|domain| href.contains(domain)) {
                return false;
            }
        }
        
        // Exclude anchors, static files, and special pages
        if href.contains('#') ||
           href.contains("/_next/") ||
           href.contains("/static/") ||
           href.contains("/__nextra/") ||
           href.ends_with(".pdf") ||
           href.ends_with(".zip") ||
           href.ends_with(".tar.gz") ||
           href.ends_with(".js") ||
           href.ends_with(".css") ||
           href.ends_with(".png") ||
           href.ends_with(".jpg") ||
           href.ends_with(".gif") ||
           href.ends_with(".svg") ||
           href.contains("/api/") ||
           href.contains("/search") ||
           href == "/" ||
           href.is_empty() {
            return false;
        }
        
        // Exclude social media and external tool links
        if href.contains("github.com") ||
           href.contains("twitter.com") ||
           href.contains("discord.") ||
           href.contains("mailto:") ||
           href.contains("tel:") {
            return false;
        }
        
        // Include relative documentation links and same-domain absolute links
        href.starts_with('/') || 
        href.ends_with(".html") || 
        href.ends_with(".mdx") ||
        (!href.contains('.') && !href.contains("://"))
    }
}