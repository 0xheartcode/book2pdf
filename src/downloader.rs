use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams;
use chromiumoxide::{Browser, BrowserConfig};
use colored::*;
use futures_util::StreamExt;
use scraper::{Html, Selector};
use slug::slugify;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::PdfMerger;

#[derive(Debug, Clone)]
pub struct PdfOptions {
    pub scale: f64,
    pub margin_top: f64,
    pub margin_right: f64,
    pub margin_bottom: f64,
    pub margin_left: f64,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            scale: 0.75,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
        }
    }
}

pub struct Downloader {
    out_dir: String,
    combine: bool,
    preserve_pages: bool,
    _timeout: Duration,
    pdf_options: PdfOptions,
}

impl Downloader {
    pub fn new(out_dir: String, combine: bool, preserve_pages: bool, timeout_seconds: f64) -> Self {
        Self {
            out_dir,
            combine,
            preserve_pages,
            _timeout: Duration::from_secs_f64(timeout_seconds),
            pdf_options: PdfOptions::default(),
        }
    }

    pub async fn run(&self, target_url: &str) -> Result<()> {
        info!("Visiting \"{}\"", target_url.green());

        let config = BrowserConfig::builder()
            .with_head()
            .window_size(1920, 1080)  // Larger viewport for better rendering
            .build()
            .map_err(|e| anyhow!("Failed to create browser config: {}", e))?;

        let (mut browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| anyhow!("Failed to launch browser: {}", e))?;

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if let Err(err) = h {
                    // Only log if it's not a common websocket deserialization error
                    let err_str = err.to_string();
                    if !err_str.contains("data did not match any variant") && 
                       !err_str.contains("untagged enum Message") {
                        error!("Browser handler error: {}", err);
                    } else {
                        debug!("Chrome protocol message ignored: {}", err);
                    }
                }
            }
        });

        let result = self.run_internal(&browser, target_url).await;

        browser.close().await.ok();
        handle.abort();

        result
    }

    async fn run_internal(&self, browser: &Browser, target_url: &str) -> Result<()> {
        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| anyhow!("Failed to create new page: {}", e))?;

        page.goto(target_url)
            .await
            .map_err(|e| anyhow!("Failed to navigate to {}: {}", target_url, e))?;

        page.wait_for_navigation()
            .await
            .map_err(|e| anyhow!("Failed to wait for navigation: {}", e))?;

        // Wait for the page to fully load
        tokio::time::sleep(Duration::from_millis(3000)).await;

        // Navigate to a documentation page first to ensure sidebar is loaded
        if target_url.ends_with('/') || target_url.ends_with(".com") || target_url.ends_with(".app") {
            // Try to find the first documentation link
            let first_doc_link = page.evaluate(r#"
                const links = document.querySelectorAll('a[href^="/"]');
                for (let link of links) {
                    const href = link.getAttribute('href');
                    if (href && href !== '/' && !href.includes('#') && !href.includes('assets')) {
                        return link.href;
                    }
                }
                return null;
            "#).await.ok();

            if let Some(result) = first_doc_link {
                if let Ok(doc_link) = result.into_value::<String>() {
                    info!("Navigating to documentation page to load sidebar: {}", doc_link.green());
                    page.goto(&doc_link)
                        .await
                        .map_err(|e| anyhow!("Failed to navigate to doc page: {}", e))?;
                    tokio::time::sleep(Duration::from_millis(2000)).await;
                }
            }
        }

        self.expand_menu_links(&page).await?;

        tokio::time::sleep(Duration::from_millis(2000)).await;

        let content = page
            .content()
            .await
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;

        let document = Html::parse_document(&content);

        if !self.is_supported_documentation_site(&document) {
            return Err(anyhow!("Not a supported documentation website (GitBook or Docusaurus)"));
        }

        let links = self.collect_links(&document);
        debug!("Links collected: {:?}", links);

        // Create output directory structure
        let pages_dir = PathBuf::from(&self.out_dir).join("pages");
        fs::create_dir_all(&pages_dir)
            .await
            .map_err(|e| anyhow!("Failed to create pages directory: {}", e))?;

        let mut pdf_paths = Vec::new();

        // Create cover page with logo first
        if let Ok(cover_path) = self.create_cover_page(browser, target_url).await {
            pdf_paths.push(cover_path);
        }

        // Use links in the order they were collected (navigation order) 
        // Start index from 2 since cover page takes index 1
        for (index, href) in links.iter().enumerate() {
            if let Ok(path) = self.download_link(browser, target_url, href, index + 2).await {
                pdf_paths.push(path);
            }
        }

        if self.combine && !pdf_paths.is_empty() {
            let _combined_path = self.combine_all_pdfs(target_url, &pdf_paths).await?;
            
            // Delete individual pages unless preserve_pages is set
            if !self.preserve_pages {
                info!("Cleaning up individual page files...");
                for pdf_path in &pdf_paths {
                    if let Err(e) = fs::remove_file(pdf_path).await {
                        warn!("Failed to remove {}: {}", pdf_path.display(), e);
                    }
                }
                
                // Remove pages directory if empty
                if let Ok(mut entries) = fs::read_dir(&pages_dir).await {
                    if entries.next_entry().await?.is_none() {
                        let _ = fs::remove_dir(&pages_dir).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn create_cover_page(&self, browser: &Browser, target_url: &str) -> Result<PathBuf> {
        info!("Creating cover page with website logo...");

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| anyhow!("Failed to create cover page: {}", e))?;

        // Go to the main site to extract logo and title
        page.goto(target_url)
            .await
            .map_err(|e| anyhow!("Failed to navigate to {}: {}", target_url, e))?;

        page.wait_for_navigation()
            .await
            .map_err(|e| anyhow!("Failed to wait for navigation: {}", e))?;

        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Extract site title and logo
        let site_info = page.evaluate(r#"
            (() => {
                // Try to find logo
                const logoSelectors = [
                    'img[alt*="logo" i]',
                    'img[src*="logo" i]',
                    'img[class*="logo" i]',
                    '.navbar__logo img',
                    '.navbar-brand img',
                    'header img',
                    '.header img'
                ];
                
                let logoSrc = null;
                for (const selector of logoSelectors) {
                    const img = document.querySelector(selector);
                    if (img && img.src) {
                        logoSrc = img.src;
                        break;
                    }
                }
                
                // Get site title
                const title = document.title || 
                             document.querySelector('h1')?.textContent || 
                             document.querySelector('.navbar-brand')?.textContent ||
                             'Documentation';
                
                return {
                    title: title.trim(),
                    logo: logoSrc,
                    url: window.location.href
                };
            })()
        "#).await.map_err(|e| anyhow!("Failed to extract site info: {}", e))?;

        let site_data: serde_json::Value = site_info.into_value()
            .map_err(|e| anyhow!("Failed to parse site info: {}", e))?;

        let title = site_data["title"].as_str().unwrap_or("Documentation");
        let logo_url = site_data["logo"].as_str();
        let site_url = site_data["url"].as_str().unwrap_or(target_url);

        // Create HTML cover page
        let logo_html = if let Some(logo) = logo_url {
            format!(r#"<img src="{}" alt="Logo" style="max-width: 300px; max-height: 200px; margin-bottom: 30px;">"#, logo)
        } else {
            String::new()
        };

        let cover_html = format!(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                <title>Cover Page</title>
                <style>
                    body {{
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                        display: flex;
                        flex-direction: column;
                        align-items: center;
                        justify-content: center;
                        height: 100vh;
                        margin: 0;
                        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                        color: white;
                        text-align: center;
                    }}
                    .container {{
                        background: rgba(255, 255, 255, 0.1);
                        backdrop-filter: blur(10px);
                        border-radius: 20px;
                        padding: 60px;
                        box-shadow: 0 8px 32px rgba(31, 38, 135, 0.37);
                        border: 1px solid rgba(255, 255, 255, 0.18);
                    }}
                    h1 {{
                        font-size: 3em;
                        margin: 20px 0;
                        font-weight: 300;
                        text-shadow: 2px 2px 4px rgba(0,0,0,0.3);
                    }}
                    .subtitle {{
                        font-size: 1.2em;
                        opacity: 0.9;
                        margin-bottom: 20px;
                    }}
                    .url {{
                        font-size: 0.9em;
                        opacity: 0.7;
                        font-family: monospace;
                        background: rgba(0,0,0,0.2);
                        padding: 10px 20px;
                        border-radius: 10px;
                        margin-top: 30px;
                    }}
                    .generated {{
                        position: absolute;
                        bottom: 30px;
                        right: 30px;
                        font-size: 0.8em;
                        opacity: 0.6;
                    }}
                </style>
            </head>
            <body>
                <div class="container">
                    {}
                    <h1>{}</h1>
                    <div class="subtitle">Documentation Export</div>
                    <div class="url">{}</div>
                </div>
                <div class="generated">Generated with book2pdf</div>
            </body>
            </html>
        "#, logo_html, title, site_url);

        // Set the HTML content
        page.set_content(&cover_html).await
            .map_err(|e| anyhow!("Failed to set cover page content: {}", e))?;

        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Generate PDF
        let cover_filename = "01_cover.pdf";
        let cover_path = PathBuf::from(&self.out_dir).join("pages").join(cover_filename);

        if let Some(parent) = cover_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow!("Failed to create directory: {}", e))?;
        }

        let params = PrintToPdfParams {
            scale: Some(self.pdf_options.scale),
            margin_top: Some(self.pdf_options.margin_top),
            margin_right: Some(self.pdf_options.margin_right),
            margin_bottom: Some(self.pdf_options.margin_bottom),
            margin_left: Some(self.pdf_options.margin_left),
            ..Default::default()
        };

        let pdf_data = page
            .pdf(params)
            .await
            .map_err(|e| anyhow!("Failed to generate cover PDF: {}", e))?;

        fs::write(&cover_path, pdf_data)
            .await
            .map_err(|e| anyhow!("Failed to write cover PDF: {}", e))?;

        info!("Cover page created: {}", cover_path.display().to_string().blue());
        Ok(cover_path)
    }

    async fn expand_menu_links(&self, page: &chromiumoxide::Page) -> Result<()> {
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
            .map_err(|e| anyhow!("Failed to expand menu links: {}", e))?;

        Ok(())
    }

    async fn prepare_page(&self, page: &chromiumoxide::Page) -> Result<()> {
        let js_code = r#"
            // Expand all expandable sections
            const sectionsToExpand = document
                .querySelectorAll('div[aria-controls^="expandable-body-"]');

            for (let section of sectionsToExpand) {
                section.click();
            }

            // Remove redundant/interactive elements
            const itemSelectorsToRemove = [
                'header + div[data-rnwrdesktop-hidden="true"]',
                'div[aria-label^="Search"]',
                'div[aria-label="Page actions"]',
            ];
            const itemsToRemove = document
                .querySelectorAll(itemSelectorsToRemove.join(', '));

            for (let item of itemsToRemove) {
                item.remove();
            }

            // Turn relative timestamps into absolute ones
            const lastModifiedEl = document
                .querySelector('div[dir="auto"] > span[aria-label]');

            if (lastModifiedEl) {
                lastModifiedEl.innerText = lastModifiedEl.getAttribute('aria-label');
            }
        "#;

        page.evaluate(js_code)
            .await
            .map_err(|e| anyhow!("Failed to prepare page: {}", e))?;

        Ok(())
    }

    async fn download_link(&self, browser: &Browser, target_url: &str, href: &str, index: usize) -> Result<PathBuf> {
        let slug = self.href_to_slug(href);

        if slug.is_empty() {
            warn!("Empty slug, ignoring \"{}\"", href.green());
            return Err(anyhow!("Empty slug"));
        }

        let filename = format!("{:02}_{}.pdf", index, slug);
        let out_path = PathBuf::from(&self.out_dir).join("pages").join(filename);

        let url = Url::parse(target_url)?
            .join(href)
            .map_err(|e| anyhow!("Failed to join URL: {}", e))?;

        self.download_page(browser, &url, &out_path).await?;

        Ok(out_path)
    }

    async fn download_page(&self, browser: &Browser, url: &Url, path: &Path) -> Result<()> {
        info!("Downloading \"{}\" into \"{}\"", url.to_string().green(), path.display().to_string().blue());

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| anyhow!("Failed to create new page: {}", e))?;

        page.goto(url.as_str())
            .await
            .map_err(|e| anyhow!("Failed to navigate to {}: {}", url, e))?;

        page.wait_for_navigation()
            .await
            .map_err(|e| anyhow!("Failed to wait for navigation: {}", e))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow!("Failed to create directory: {}", e))?;
        }

        self.prepare_page(&page).await?;

        let params = PrintToPdfParams {
            scale: Some(self.pdf_options.scale),
            margin_top: Some(self.pdf_options.margin_top),
            margin_right: Some(self.pdf_options.margin_right),
            margin_bottom: Some(self.pdf_options.margin_bottom),
            margin_left: Some(self.pdf_options.margin_left),
            ..Default::default()
        };

        let pdf_data = page
            .pdf(params)
            .await
            .map_err(|e| anyhow!("Failed to generate PDF: {}", e))?;

        fs::write(path, pdf_data)
            .await
            .map_err(|e| anyhow!("Failed to write PDF to {}: {}", path.display(), e))?;

        Ok(())
    }

    fn is_supported_documentation_site(&self, document: &Html) -> bool {
        // GitBook detection
        let old_format_selector = Selector::parse("body > .gitbook-root").unwrap();
        if document.select(&old_format_selector).next().is_some() {
            return true;
        }

        let new_format_selectors = [
            "body > div.scroll-nojump",
            "nav[role=\"navigation\"]",
            "a[href*=\"gitbook.io\"]",
        ];

        for selector_str in &new_format_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if document.select(&selector).next().is_some() {
                    return true;
                }
            }
        }

        let body_selector = Selector::parse("body").unwrap();
        if let Some(body) = document.select(&body_selector).next() {
            if let Some(class) = body.value().attr("class") {
                if class.contains("theme-") {
                    return true;
                }
            }
        }

        // Docusaurus detection
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
                    return true;
                }
            }
        }

        // Check for Docusaurus in script content
        let script_selector = Selector::parse("script").unwrap();
        for script in document.select(&script_selector) {
            let content = script.text().collect::<String>();
            if content.contains("docusaurus") || content.contains("__DOCUSAURUS__") {
                debug!("Detected Docusaurus site from script content");
                return true;
            }
        }

        false
    }

    fn collect_links(&self, document: &Html) -> Vec<String> {
        let mut links = Vec::new();
        let mut seen = HashSet::new();
        
        // Prioritize navigation order - collect from sidebar/nav first
        let nav_selectors = [
            "nav.navbar a[href^=\"/\"]",  // Navbar links
            "aside a[href^=\"/\"]",       // Sidebar links
            ".menu a[href^=\"/\"]",       // Docusaurus menu
            ".theme-doc-sidebar-menu a[href^=\"/\"]",  // Docusaurus sidebar
            "nav a[href^=\"/\"]",         // General nav links
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
        
        // Fallback: collect any remaining internal links
        let internal_selector = Selector::parse("a[href^=\"/\"]").unwrap();
        for element in document.select(&internal_selector) {
            if let Some(href) = element.value().attr("href") {
                if href.starts_with('/') && !href.contains('#') && !href.contains("/assets/") {
                    if seen.insert(href.to_string()) {
                        links.push(href.to_string());
                    }
                }
            }
        }
        
        debug!("Collected {} unique links in navigation order", links.len());
        links
    }

    fn href_to_slug(&self, href: &str) -> String {
        let mut slug = slugify(href);
        slug = slug.replace("/", "-").trim().to_string();

        if slug == "/" || slug.is_empty() {
            "index".to_string()
        } else {
            slug.trim_end_matches('-').to_string()
        }
    }

    async fn combine_all_pdfs(&self, target_url: &str, pdf_paths: &[PathBuf]) -> Result<PathBuf> {
        info!("Combining all PDFs into a single file...");

        let url = Url::parse(target_url)?;
        let domain_slug = slugify(&url.host_str().unwrap_or("gitbook").replace('.', "-"));
        let combined_path = PathBuf::from(&self.out_dir).join(format!("{}-combined.pdf", domain_slug));

        let mut merger = PdfMerger::new();
        
        // Use the paths in the order they were discovered/downloaded
        for pdf_path in pdf_paths {
            if let Err(e) = merger.add_pdf(pdf_path).await {
                warn!("Failed to add PDF {}: {}", pdf_path.display(), e);
            }
        }

        merger.save(&combined_path).await?;

        info!("Combined PDF saved to: {}", combined_path.display().to_string().blue());

        Ok(combined_path)
    }
}