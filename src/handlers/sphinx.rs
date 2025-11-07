use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chromiumoxide::Page;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tracing::debug;
use url::Url;

use crate::handlers::{ConfidenceLevel, FormatHandler, SiteDetector};
use crate::handlers::utils;

/// Handler for Sphinx documentation sites (including Jupyter Book, ReadTheDocs themes, etc.)
pub struct SphinxHandler;

#[async_trait]
impl SiteDetector for SphinxHandler {
    fn version(&self) -> &str {
        "v5.x-8.x"
    }
    
    fn format_name(&self) -> &str {
        "sphinx"
    }
    
    async fn can_handle(&self, _url: &str, content: &str) -> Result<ConfidenceLevel> {
        let document = Html::parse_document(content);
        
        // Primary Sphinx detection patterns
        let has_sphinx_meta = utils::has_meta_generator(&document, "sphinx");
        
        // Check for Sphinx-specific text patterns
        let sphinx_footer_patterns = [
            ("Built with", "Sphinx"),
            ("Made with", "Sphinx"), 
            ("Created using Sphinx", ""),
            ("sphinx-doc.org", "")
        ];
        
        let has_sphinx_footer = sphinx_footer_patterns.iter().any(|(pattern1, pattern2)| {
            if pattern2.is_empty() {
                content.contains(pattern1)
            } else {
                utils::contains_all_keywords(content, &[pattern1, pattern2])
            }
        });
        
        // Check for common Sphinx themes and patterns
        let sphinx_indicators = [
            // Theme-specific indicators
            "sphinx_rtd_theme",           // ReadTheDocs theme
            "furo",                       // Furo theme  
            "pydata-sphinx-theme",        // PyData theme
            "book-theme",                 // Jupyter Book theme
            
            // Sphinx-generated content patterns
            "_static/",                   // Static assets directory
            "_sources/",                  // Source files directory
            "genindex.html",             // Generated index
            "search.html",               // Search functionality
            "py-modindex.html",          // Python module index
            
            // CSS/JS patterns
            "searchtools.js",            // Sphinx search
            "doctools.js",               // Sphinx doc tools
            "_static/js/theme.js",       // Theme JavaScript
            "sphinx-design",             // Sphinx design extension
            
            // Navigation patterns
            "toctree",                   // Table of contents tree
            "sphinxsidebar",             // Sphinx sidebar
            "document",                  // Sphinx document wrapper
            "bodywrapper"                // Sphinx body wrapper
        ];
        
        let sphinx_matches = utils::count_text_indicators(content, &sphinx_indicators);
        debug!("Found {} Sphinx text indicators", sphinx_matches);
        
        // Check for Sphinx-specific CSS classes and IDs
        let sphinx_selectors = [
            ".sphinxsidebar",            // Classic Sphinx sidebar
            ".document",                 // Document wrapper
            ".bodywrapper",              // Body wrapper
            ".toctree-wrapper",          // Table of contents
            ".rst-content",              // reStructuredText content (RTD theme)
            ".wy-nav-content",           // ReadTheDocs navigation
            ".bd-main",                  // PyData theme main content
            ".bd-sidebar",               // PyData theme sidebar
            "#searchbox",                // Search box
            ".genindex-jumpbox",         // Generated index jump box
            "[class*='sphinx']",         // Any class containing 'sphinx'
            ".version-dropdown",         // Version selector
            ".ethical-ad"                // ReadTheDocs ethical ads
        ];
        
        let css_matches = sphinx_selectors.iter()
            .filter(|&selector_str| utils::has_css_selector(&document, selector_str))
            .count();
        debug!("Found {} Sphinx CSS selectors", css_matches);
        
        // Check for Jupyter Book specific patterns
        let jupyter_book_keywords = ["thebe", "jupyter-book", "executable-book"];
        let is_jupyter_book = utils::contains_any_keywords(content, &jupyter_book_keywords);
        
        // Check for ReadTheDocs hosting
        let readthedocs_keywords = ["readthedocs.io", "Read the Docs", "rtd-footer-container"];
        let is_readthedocs = utils::contains_any_keywords(content, &readthedocs_keywords);
        
        // Confidence logic
        if has_sphinx_meta && (sphinx_matches >= 3 || css_matches >= 2) {
            Ok(ConfidenceLevel::Certain)
        } else if has_sphinx_footer && (sphinx_matches >= 2 || css_matches >= 2) {
            Ok(ConfidenceLevel::High)  
        } else if (sphinx_matches >= 4 || css_matches >= 3) && (is_readthedocs || is_jupyter_book) {
            Ok(ConfidenceLevel::High)
        } else if sphinx_matches >= 3 || css_matches >= 2 {
            Ok(ConfidenceLevel::Medium)
        } else if sphinx_matches >= 2 || css_matches >= 1 || has_sphinx_footer {
            Ok(ConfidenceLevel::Low)
        } else {
            Ok(ConfidenceLevel::None)
        }
    }
}

#[async_trait]
impl FormatHandler for SphinxHandler {
    fn name(&self) -> &str {
        "Sphinx"
    }
    
    fn supported_versions(&self) -> Vec<&str> {
        vec!["v5.x", "v6.x", "v7.x", "v8.x"]
    }

    async fn detect_version(&self, page: &Page) -> Result<Option<String>> {
        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {e}"))?;
        
        // Try to extract Sphinx version from meta generator tag
        let document = Html::parse_document(&content);
        
        if let Ok(selector) = Selector::parse(r#"meta[name="generator"]"#) {
            if let Some(meta) = document.select(&selector).next() {
                if let Some(generator_content) = meta.value().attr("content") {
                    if generator_content.to_lowercase().contains("sphinx") {
                        // Try to extract version (e.g., "Sphinx 4.5.0")
                        if let Some(version_start) = generator_content.find(char::is_numeric) {
                            if let Some(version_part) = generator_content.get(version_start..) {
                                if let Some(space_pos) = version_part.find(' ') {
                                    return Ok(Some(format!("v{}", &version_part[..space_pos])));
                                } else if let Some(end_pos) = version_part.find(|c: char| !c.is_ascii_alphanumeric() && c != '.') {
                                    return Ok(Some(format!("v{}", &version_part[..end_pos])));
                                }
                            }
                        }
                        return Ok(Some("Sphinx".to_string()));
                    }
                }
            }
        }
        
        // Check for theme-specific version info
        if content.contains("jupyter-book") || content.contains("thebe") {
            return Ok(Some("Jupyter Book".to_string()));
        }
        
        if content.contains("sphinx_rtd_theme") {
            return Ok(Some("Sphinx (ReadTheDocs theme)".to_string()));
        }
        
        if content.contains("furo") {
            return Ok(Some("Sphinx (Furo theme)".to_string()));
        }
        
        if content.contains("pydata-sphinx-theme") {
            return Ok(Some("Sphinx (PyData theme)".to_string()));
        }
        
        // Fallback
        if content.contains("sphinx") || content.contains("Sphinx") {
            Ok(Some("Sphinx (unknown version)".to_string()))
        } else {
            Ok(None)
        }
    }

    async fn expand_navigation(&self, page: &Page) -> Result<()> {
        // Expand Sphinx navigation - different strategies for different themes
        page.evaluate(r#"
            // Expand ReadTheDocs theme navigation
            document.querySelectorAll('.wy-menu-vertical .toctree-l1 > a').forEach(link => {
                try {
                    if (link.getAttribute('href') === '#') {
                        link.click();
                    }
                } catch (e) {
                    console.debug('Could not expand RTD nav item:', e);
                }
            });
            
            // Expand any collapsed sections
            document.querySelectorAll('[aria-expanded="false"]').forEach(element => {
                try {
                    element.setAttribute('aria-expanded', 'true');
                    element.click();
                } catch (e) {
                    console.debug('Could not expand element:', e);
                }
            });
            
            // Expand details elements (for modern themes)
            document.querySelectorAll('details:not([open])').forEach(details => {
                details.open = true;
            });
            
            // Expand Furo theme navigation
            document.querySelectorAll('.toctree-expand').forEach(expand => {
                try {
                    expand.click();
                } catch (e) {
                    console.debug('Could not expand Furo nav:', e);
                }
            });
            
            // Expand PyData theme navigation
            document.querySelectorAll('.bd-toc .collapsed').forEach(collapsed => {
                collapsed.classList.remove('collapsed');
            });
            
            // Wait for any navigation animations
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

        // Sphinx navigation selectors (multiple strategies for different themes)
        let nav_selectors = [
            // ReadTheDocs theme
            ".wy-menu-vertical a[href]",
            ".wy-nav-content-wrap .toctree-wrapper a[href]",
            
            // Classic Sphinx theme
            ".sphinxsidebar .toctree-wrapper a[href]",
            ".toctree-l1 a[href]",
            ".toctree-l2 a[href]",
            ".toctree-l3 a[href]",
            
            // Furo theme
            ".sidebar-tree a[href]",
            ".toc-tree a[href]",
            
            // PyData theme
            ".bd-toc a[href]",
            ".navbar-nav a[href]",
            
            // Jupyter Book theme
            ".bd-toc a[href]",
            ".toc a[href]",
            
            // Generic navigation patterns
            "nav a[href]",
            ".navigation a[href]",
            ".sidebar a[href]",
            ".toctree a[href]",
            ".contents a[href]",
            
            // Fallback to any documentation links
            "main a[href]",
            ".document a[href]",
            ".rst-content a[href]"
        ];

        for selector_str in &nav_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        // Filter for documentation content links
                        if self.is_valid_sphinx_link(href) && seen.insert(href.to_string()) {
                            links.push(href.to_string());
                            debug!("Found Sphinx link: {}", href);
                        }
                    }
                }
            }
            
            // Stop if we found enough links from navigation
            if links.len() >= 50 {
                break;
            }
        }

        debug!("Extracted {} Sphinx documentation links", links.len());
        Ok(links)
    }

    async fn prepare_page(&self, page: &Page) -> Result<()> {
        // Prepare Sphinx page for PDF generation
        page.evaluate(r#"
            // Hide unnecessary elements for PDF
            const hideSelectors = [
                // ReadTheDocs theme elements
                '.wy-nav-side',                    // Left sidebar
                '.wy-nav-top',                     // Top navigation
                '.rst-footer-buttons',             // Footer buttons
                '.ethical-ad',                     // ReadTheDocs ads
                '#search-results',                 // Search results
                
                // Classic Sphinx theme elements
                '.sphinxsidebar',                  // Classic sidebar
                '.related',                        // Related links
                '#searchbox',                      // Search box
                
                // Furo theme elements
                '.sidebar-drawer',                 // Furo sidebar
                '.announcement',                   // Announcements
                '.header-center',                  // Header navigation
                
                // PyData theme elements
                '.bd-header',                      // PyData header
                '.bd-sidebar',                     // PyData sidebar
                '.prev-next-area',                 // Previous/next navigation
                
                // Version selectors and edit links
                '.version-dropdown',               // Version selector
                '.edit-this-page',                 // Edit page links
                '.sourcelink',                     // Source links
                '.headerlink',                     // Header anchor links
                
                // Generic elements to hide
                '.search',                         // Search functionality
                '.navigation',                     // Navigation bars
                '.breadcrumb',                     // Breadcrumb navigation
                'nav',                            // All navigation elements
                'footer nav'                       // Footer navigation
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
            
            // Show all tabs content if present
            document.querySelectorAll('[role="tabpanel"]').forEach(panel => {
                panel.style.display = 'block';
                panel.removeAttribute('hidden');
            });
            
            // Remove interactive elements that don't work in PDF
            document.querySelectorAll('.thebe-launch-button, .thebe-status').forEach(element => {
                element.style.display = 'none';
            });

            // Ensure proper styling for print
            const style = document.createElement('style');
            style.textContent = `
                @media print {
                    .sidebar, .wy-nav-side, .sphinxsidebar { display: none !important; }
                    .wy-nav-content { margin-left: 0 !important; }
                    .rst-content { max-width: none !important; }
                    .document { margin: 0 !important; padding: 20px !important; }
                    .bodywrapper { margin: 0 !important; }
                    .body { max-width: none !important; }
                    a { color: black !important; text-decoration: underline !important; }
                    pre, code { background: #f5f5f5 !important; border: 1px solid #ddd !important; }
                    .highlight { background: #f5f5f5 !important; }
                    .admonition { border: 1px solid #ddd !important; background: #f9f9f9 !important; }
                }
            `;
            document.head.appendChild(style);

            // Wait for any layout changes to complete
            new Promise(resolve => setTimeout(resolve, 500));
        "#).await.map_err(|e| anyhow!("Failed to prepare page: {e}"))?;

        Ok(())
    }
}

impl SphinxHandler {
    /// Check if a link is a valid Sphinx documentation link
    fn is_valid_sphinx_link(&self, href: &str) -> bool {
        // Include relative links and exclude external/unwanted links
        if href.starts_with("http") && !href.contains("readthedocs.io") {
            return false;
        }
        
        // Exclude anchors, static files, and special pages
        if href.contains('#') ||
           href.contains("/_static/") ||
           href.contains("/_sources/") ||
           href.ends_with(".pdf") ||
           href.ends_with(".zip") ||
           href.ends_with(".tar.gz") ||
           href.contains("/search.html") ||
           href.contains("/genindex.html") ||
           href.contains("/py-modindex.html") ||
           href == "/" ||
           href.is_empty() {
            return false;
        }
        
        // Include relative documentation links
        href.starts_with('/') || href.ends_with(".html") || (!href.contains('.') && !href.contains("://"))
    }
}