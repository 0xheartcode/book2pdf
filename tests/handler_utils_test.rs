use scraper::Html;
use book2pdf::handlers::utils::*;

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
fn test_has_any_css_selectors() {
    let html = r#"<html><body><div id="test" class="example">Hello</div></body></html>"#;
    let document = Html::parse_document(html);
    
    let selectors = ["#nonexistent", "#test", ".another"];
    assert!(has_any_css_selectors(&document, &selectors));
    
    let no_match = ["#missing", ".absent"];
    assert!(!has_any_css_selectors(&document, &no_match));
}

#[test]
fn test_contains_all_keywords() {
    let content = "Built with Sphinx documentation generator";
    
    assert!(contains_all_keywords(content, &["Built with", "Sphinx"]));
    assert!(!contains_all_keywords(content, &["Built with", "Docusaurus"]));
    assert!(contains_all_keywords(content, &["documentation"]));
    assert!(contains_all_keywords(content, &[])); // Empty array should return true
}

#[test]
fn test_contains_any_keywords() {
    let content = "Built with Sphinx documentation generator";
    
    assert!(contains_any_keywords(content, &["Sphinx", "Docusaurus"]));
    assert!(contains_any_keywords(content, &["MkDocs", "documentation"]));
    assert!(!contains_any_keywords(content, &["VitePress", "Nextra"]));
    assert!(!contains_any_keywords(content, &[])); // Empty array should return false
}

#[test]
fn test_has_script_keywords() {
    let html = r#"
        <html>
        <head>
            <script>window.docusaurus = {version: "2.0"};</script>
        </head>
        </html>
    "#;
    let document = Html::parse_document(html);
    
    assert!(has_script_keywords(&document, &["docusaurus"]));
    assert!(has_script_keywords(&document, &["window.docusaurus"]));
    assert!(!has_script_keywords(&document, &["sphinx"]));
}

#[test]
fn test_count_matching_selectors() {
    let html = r#"<html><body><div id="test" class="example">Hello</div><p>Text</p></body></html>"#;
    let document = Html::parse_document(html);
    
    let selectors = ["#test", ".example", "#nonexistent", "p"];
    assert_eq!(count_matching_selectors(&document, &selectors), 3);
    
    let none_match = ["#missing", ".absent"];
    assert_eq!(count_matching_selectors(&document, &none_match), 0);
    
    let all_match = ["div", "p", "body"];
    assert_eq!(count_matching_selectors(&document, &all_match), 3);
}

#[test]
fn test_find_first_matching_selector() {
    let html = r#"<html><body><div id="test" class="example">Hello</div></body></html>"#;
    let document = Html::parse_document(html);
    
    let selectors = ["#missing", "#test", ".example"];
    assert_eq!(find_first_matching_selector(&document, &selectors), Some("#test"));
    
    let no_match = ["#absent", ".missing"];
    assert_eq!(find_first_matching_selector(&document, &no_match), None);
    
    let first_match = [".example", "#test"];
    assert_eq!(find_first_matching_selector(&document, &first_match), Some(".example"));
}

#[test]
fn test_extract_css_classes() {
    let html = r#"
        <html>
        <body>
            <div class="navbar navbar-brand main">Content 1</div>
            <span class="sidebar">Content 2</span>
            <p class="text-center bold">Content 3</p>
        </body>
        </html>
    "#;
    let document = Html::parse_document(html);
    let classes = extract_css_classes(&document);
    
    assert!(classes.contains("navbar"));
    assert!(classes.contains("navbar-brand"));
    assert!(classes.contains("main"));
    assert!(classes.contains("sidebar"));
    assert!(classes.contains("text-center"));
    assert!(classes.contains("bold"));
    assert!(!classes.contains("nonexistent"));
}

#[test]
fn test_has_class_prefix() {
    let html = r#"
        <html>
        <body>
            <div class="nx-button nx-primary">Nextra Button</div>
            <span class="md-nav md-sidebar">Material Design</span>
            <p class="normal-class">Regular content</p>
        </body>
        </html>
    "#;
    let document = Html::parse_document(html);
    
    assert!(has_class_prefix(&document, "nx-"));
    assert!(has_class_prefix(&document, "md-"));
    assert!(!has_class_prefix(&document, "vp-"));
    assert!(!has_class_prefix(&document, "sphinx-"));
}

#[test]
fn test_count_text_indicators() {
    let content = "Built with Sphinx documentation generator using Python and reStructuredText";
    
    let indicators = ["Sphinx", "Python", "documentation", "missing"];
    assert_eq!(count_text_indicators(content, &indicators), 3);
    
    let no_indicators = ["VitePress", "Nextra", "absent"];
    assert_eq!(count_text_indicators(content, &no_indicators), 0);
    
    let all_indicators = ["Built", "with"];
    assert_eq!(count_text_indicators(content, &all_indicators), 2);
}

#[test]
fn test_try_parse_selector() {
    // Valid selectors
    assert!(try_parse_selector("#test").is_some());
    assert!(try_parse_selector(".example").is_some());
    assert!(try_parse_selector("div > p").is_some());
    
    // Invalid selectors
    assert!(try_parse_selector("").is_none());
    assert!(try_parse_selector("#").is_none());
    assert!(try_parse_selector(">>invalid<<").is_none());
}