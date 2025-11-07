use scraper::{Html, Selector};
use std::collections::HashSet;

/// Common utilities for HTML detection patterns used across handlers

/// Check if a meta generator tag contains a specific framework name
/// Used by: Sphinx, Docusaurus, Nextra, Starlight, VitePress, mdBook handlers
pub fn has_meta_generator(document: &Html, framework: &str) -> bool {
    if let Ok(selector) = Selector::parse(META_GENERATOR_SELECTOR) {
        document.select(&selector).any(|element| {
            if let Some(generator_content) = element.value().attr("content") {
                generator_content.to_lowercase().contains(&framework.to_lowercase())
            } else {
                false
            }
        })
    } else {
        false
    }
}

/// Check if any of the provided CSS selectors exist in the document
/// Returns true if at least one selector matches an element
pub fn has_any_css_selectors(document: &Html, selectors: &[&str]) -> bool {
    selectors.iter().any(|selector_str| {
        if let Ok(selector) = Selector::parse(selector_str) {
            document.select(&selector).next().is_some()
        } else {
            false
        }
    })
}

/// Check if a single CSS selector exists in the document
/// Safe wrapper around selector parsing with error handling
pub fn has_css_selector(document: &Html, selector_str: &str) -> bool {
    if let Ok(selector) = Selector::parse(selector_str) {
        document.select(&selector).next().is_some()
    } else {
        false
    }
}

/// Extract text content from all script tags and check for keywords
/// Used by handlers that look for framework-specific JavaScript patterns
pub fn has_script_keywords(document: &Html, keywords: &[&str]) -> bool {
    if let Ok(script_selector) = Selector::parse(SCRIPT_SELECTOR) {
        document.select(&script_selector).any(|script| {
            let content = script.text().collect::<String>();
            keywords.iter().any(|keyword| content.contains(keyword))
        })
    } else {
        false
    }
}

/// Check if content contains all of the specified keywords
/// Useful for footer text detection and multi-pattern matching
pub fn contains_all_keywords(content: &str, keywords: &[&str]) -> bool {
    keywords.iter().all(|keyword| content.contains(keyword))
}

/// Check if content contains any of the specified keywords
/// Useful for theme detection and flexible pattern matching
pub fn contains_any_keywords(content: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| content.contains(keyword))
}

/// Extract all CSS class names from the document
/// Returns a HashSet for efficient lookups
pub fn extract_css_classes(document: &Html) -> HashSet<String> {
    let mut classes = HashSet::new();
    
    if let Ok(selector) = Selector::parse("*[class]") {
        for element in document.select(&selector) {
            if let Some(class_attr) = element.value().attr("class") {
                for class_name in class_attr.split_whitespace() {
                    classes.insert(class_name.to_string());
                }
            }
        }
    }
    
    classes
}

/// Check if document has any CSS classes with the specified prefix
/// Useful for framework-specific class detection (nx-, md-, etc.)
pub fn has_class_prefix(document: &Html, prefix: &str) -> bool {
    let classes = extract_css_classes(document);
    classes.iter().any(|class| class.starts_with(prefix))
}

/// Count how many of the provided indicators are present in the content
/// Useful for confidence scoring based on multiple pattern matches
pub fn count_text_indicators(content: &str, indicators: &[&str]) -> usize {
    indicators.iter()
        .filter(|indicator| content.contains(**indicator))
        .count()
}

/// Count how many of the provided CSS selectors match elements in the document
/// Returns the number of selectors that found at least one matching element
pub fn count_matching_selectors(document: &Html, selectors: &[&str]) -> usize {
    selectors.iter()
        .filter(|&selector_str| has_css_selector(document, selector_str))
        .count()
}

/// Find the first CSS selector from the list that matches an element in the document
/// Returns Some(selector_string) if a match is found, None if no selectors match
pub fn find_first_matching_selector<'a>(document: &Html, selectors: &'a [&'a str]) -> Option<&'a str> {
    selectors.iter()
        .find(|&selector_str| has_css_selector(document, selector_str))
        .copied()
}

/// Safe CSS selector parsing that returns an Option instead of panicking
/// Use when you need to handle selector parsing errors gracefully
pub fn try_parse_selector(selector_str: &str) -> Option<Selector> {
    Selector::parse(selector_str).ok()
}

// Common CSS selector constants to avoid string duplication
pub const META_GENERATOR_SELECTOR: &str = r#"meta[name="generator"]"#;
pub const SCRIPT_SELECTOR: &str = "script";
pub const HTML_DATA_SELECTOR: &str = "html[data-*]";

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_has_meta_generator() {
        let html = r#"<html><head><meta name="generator" content="Sphinx v4.0.0"></head></html>"#;
        let document = Html::parse_document(html);
        
        assert!(has_meta_generator(&document, "sphinx"));
        assert!(has_meta_generator(&document, "Sphinx"));
        assert!(!has_meta_generator(&document, "docusaurus"));
    }
    
    #[test]
    fn test_has_css_selector() {
        let html = r#"<html><body><div id="test" class="example">Hello</div></body></html>"#;
        let document = Html::parse_document(html);
        
        assert!(has_css_selector(&document, "#test"));
        assert!(has_css_selector(&document, ".example"));
        assert!(!has_css_selector(&document, "#nonexistent"));
    }
    
    #[test]
    fn test_contains_all_keywords() {
        let content = "Built with Sphinx documentation generator";
        
        assert!(contains_all_keywords(content, &["Built with", "Sphinx"]));
        assert!(!contains_all_keywords(content, &["Built with", "Docusaurus"]));
    }
    
    #[test]
    fn test_count_matching_selectors() {
        let html = r#"<html><body><div id="test" class="example">Hello</div><p>Text</p></body></html>"#;
        let document = Html::parse_document(html);
        
        let selectors = ["#test", ".example", "#nonexistent", "p"];
        assert_eq!(count_matching_selectors(&document, &selectors), 3);
        
        let none_match = ["#missing", ".absent"];
        assert_eq!(count_matching_selectors(&document, &none_match), 0);
    }
    
    #[test]
    fn test_find_first_matching_selector() {
        let html = r#"<html><body><div id="test" class="example">Hello</div></body></html>"#;
        let document = Html::parse_document(html);
        
        let selectors = ["#missing", "#test", ".example"];
        assert_eq!(find_first_matching_selector(&document, &selectors), Some("#test"));
        
        let no_match = ["#absent", ".missing"];
        assert_eq!(find_first_matching_selector(&document, &no_match), None);
    }
}