use anyhow::Result;

/// Test individual handler detection logic
/// This tests the core detection functionality without needing a full browser
#[tokio::test]
async fn test_individual_handler_detection() -> Result<()> {
    use book2pdf::handlers::*;
    
    let sphinx_html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="generator" content="Sphinx v4.0.0">
            <link rel="stylesheet" href="_static/sphinx.css">
        </head>
        <body>
            <div class="sphinxsidebar">Content</div>
        </body>
        </html>
    "#;
    
    let sphinx_handler = sphinx::SphinxHandler;
    let result = sphinx_handler.can_handle("https://docs.python.org/3/", sphinx_html).await?;
    println!("Sphinx confidence: {:?}", result);
    assert!(result > book2pdf::ConfidenceLevel::Low, "Expected at least Medium confidence, got {:?}", result);
    
    let docusaurus_html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="generator" content="Docusaurus v2.0.0">
        </head>
        <body>
            <div class="navbar__logo">Test</div>
        </body>
        </html>
    "#;
    
    let docusaurus_handler = docusaurus::DocusaurusHandler;
    let result = docusaurus_handler.can_handle("https://docusaurus.io/docs", docusaurus_html).await?;
    println!("Docusaurus confidence: {:?}", result);
    assert!(result > book2pdf::ConfidenceLevel::Low, "Expected at least Medium confidence, got {:?}", result);
    
    let mdbook_html = r#"
        <!DOCTYPE html>
        <!-- Book generated using mdBook -->
        <html>
        <head>
            <meta name="generator" content="mdBook">
        </head>
        <body>
            <div id="sidebar-toggle"></div>
            <div id="sidebar"></div>
        </body>
        </html>
    "#;
    
    let mdbook_handler = mdbook_v03_v04::MdBookV03V04Handler;
    let result = mdbook_handler.can_handle("https://doc.rust-lang.org/book/", mdbook_html).await?;
    println!("mdBook confidence: {:?}", result);
    assert!(result > book2pdf::ConfidenceLevel::Low, "Expected at least Medium confidence, got {:?}", result);
    
    Ok(())
}

/// Test that unknown content gets low confidence
#[tokio::test]
async fn test_unknown_content_detection() -> Result<()> {
    use book2pdf::handlers::*;
    
    let generic_html = r#"
        <!DOCTYPE html>
        <html>
        <head><title>Generic Site</title></head>
        <body><h1>Hello World</h1></body>
        </html>
    "#;
    
    // Test that all handlers give low confidence for generic content
    let handlers: Vec<Box<dyn SiteDetector>> = vec![
        Box::new(sphinx::SphinxHandler),
        Box::new(docusaurus::DocusaurusHandler),
        Box::new(mdbook_v03_v04::MdBookV03V04Handler),
    ];
    
    for handler in handlers {
        let result = handler.can_handle("https://example.com", generic_html).await?;
        assert!(matches!(result, book2pdf::ConfidenceLevel::None | book2pdf::ConfidenceLevel::Low));
    }
    
    Ok(())
}

/// Test confidence scoring consistency
#[tokio::test]
async fn test_confidence_scoring_consistency() -> Result<()> {
    use book2pdf::handlers::*;
    use book2pdf::ConfidenceLevel;
    
    // Strong indicators should give higher confidence than weak ones
    let strong_sphinx = r#"
        <html>
        <head>
            <meta name="generator" content="Sphinx v4.0.0">
            <link rel="stylesheet" href="_static/sphinx.css">
        </head>
        <body>
            <div class="sphinxsidebar"></div>
            <div class="sphinxsidebarwrapper"></div>
        </body>
        </html>
    "#;
    
    let weak_sphinx = r#"
        <html>
        <head>
            <title>Some documentation</title>
        </head>
        <body>
            <div class="content">Built with Sphinx</div>
        </body>
        </html>
    "#;
    
    let sphinx_handler = sphinx::SphinxHandler;
    let strong_result = sphinx_handler.can_handle("https://docs.example.com", strong_sphinx).await?;
    let weak_result = sphinx_handler.can_handle("https://docs.example.com", weak_sphinx).await?;
    
    assert!(strong_result >= weak_result);
    assert!(matches!(strong_result, ConfidenceLevel::High | ConfidenceLevel::Certain));
    
    Ok(())
}

/// Test URL pattern matching
#[tokio::test]
async fn test_url_pattern_matching() -> Result<()> {
    use book2pdf::handlers::*;
    
    // Some handlers use URL patterns for detection
    let generic_html = "<html><body>Test</body></html>";
    
    // Test ReadTheDocs URL pattern (common for Sphinx)
    let sphinx_handler = sphinx::SphinxHandler;
    let readthedocs_result = sphinx_handler.can_handle(
        "https://myproject.readthedocs.io/en/latest/", 
        generic_html
    ).await?;
    
    let generic_result = sphinx_handler.can_handle(
        "https://example.com", 
        generic_html
    ).await?;
    
    // ReadTheDocs URL should get some confidence even with generic content
    assert!(readthedocs_result >= generic_result);
    
    Ok(())
}

/// Performance test - detection should be fast
#[tokio::test]
async fn test_detection_performance() -> Result<()> {
    use book2pdf::handlers::*;
    
    let test_html = r#"
        <html>
        <head>
            <meta name="generator" content="Sphinx v4.0.0">
        </head>
        <body>Content</body>
        </html>
    "#;
    
    let sphinx_handler = sphinx::SphinxHandler;
    
    let start = std::time::Instant::now();
    
    for _ in 0..100 {
        let _result = sphinx_handler.can_handle("https://docs.python.org/3/", test_html).await?;
    }
    
    let elapsed = start.elapsed();
    
    // Should be very fast (less than 100ms for 100 detections)
    assert!(elapsed.as_millis() < 100, "Detection too slow: {:?}", elapsed);
    
    Ok(())
}

/// Test edge cases and error handling
#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    use book2pdf::handlers::*;
    
    let sphinx_handler = sphinx::SphinxHandler;
    
    let result = sphinx_handler.can_handle("https://example.com", "").await?;
    assert!(matches!(result, book2pdf::ConfidenceLevel::None | book2pdf::ConfidenceLevel::Low));
    
    let malformed_html = "<html><head><meta name=\"generator\" content=\"Sphinx";
    let result = sphinx_handler.can_handle("https://example.com", malformed_html).await?;
    // Should not crash, should return some confidence level
    assert!(matches!(result, book2pdf::ConfidenceLevel::None | book2pdf::ConfidenceLevel::Low | book2pdf::ConfidenceLevel::Medium));
    
    // Test very large content (shouldn't crash)
    let large_html = format!("<html><body>{}</body></html>", "x".repeat(100000));
    let start = std::time::Instant::now();
    let _result = sphinx_handler.can_handle("https://example.com", &large_html).await?;
    let elapsed = start.elapsed();
    
    // Should handle large content reasonably fast
    assert!(elapsed.as_millis() < 1000, "Large content handling too slow");
    
    Ok(())
}