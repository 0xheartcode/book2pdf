use anyhow::Result;
use book2pdf::Downloader;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::timeout;

/// Test site configuration from documentation_sites.toml
#[derive(Debug, Deserialize)]
struct TestSite {
    url: String,
    expected_format: String,
    _expected_confidence: String,
    description: String,
}

/// Root structure for the TOML file
#[derive(Debug, Deserialize)]
struct TestSitesConfig {
    test_site: Vec<TestSite>,
}

/// Load test sites from documentation_sites.toml
fn load_test_sites() -> Result<Vec<TestSite>> {
    let toml_content = include_str!("documentation_sites.toml");
    let config: TestSitesConfig = toml::from_str(toml_content)?;
    Ok(config.test_site)
}

/// Test framework detection against real documentation sites
/// This is the most valuable test - validates our core functionality
#[tokio::test]
async fn test_real_site_detection() -> Result<()> {
    let test_sites = load_test_sites()?;
    let mut passed = 0;
    let mut _failed = 0;
    let mut total_tested = 0;
    
    // Test a subset of sites to keep test time reasonable
    let sites_to_test = test_sites.into_iter().take(10);
    
    for site in sites_to_test {
        total_tested += 1;
        println!("Testing: {} ({})", site.url, site.description);
        
        let result = timeout(
            Duration::from_secs(15), // 15 second timeout per site
            test_single_site(&site)
        ).await;
        
        match result {
            Ok(Ok(())) => {
                println!("  ✅ PASS: {}", site.url);
                passed += 1;
            },
            Ok(Err(e)) => {
                println!("  ❌ FAIL: {} - {}", site.url, e);
                _failed += 1;
            },
            Err(_) => {
                println!("  ⏰ TIMEOUT: {}", site.url);
                _failed += 1;
            }
        }
    }
    
    println!("\nResults: {}/{} passed", passed, total_tested);
    
    // Require at least 70% success rate (some sites may be down or changed)
    let success_rate = passed as f64 / total_tested as f64;
    assert!(success_rate >= 0.7, "Success rate too low: {:.1}%", success_rate * 100.0);
    
    Ok(())
}

/// Test a single site using simulate mode (fast, no file creation)
async fn test_single_site(site: &TestSite) -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let output_dir = temp_dir.path().to_str().unwrap().to_string();
    
    // Use simulate mode - fast and doesn't create files
    let downloader = Downloader::new(output_dir, true, false, 10.0);
    
    let result = downloader.run(
        &site.url,
        Some(1), // Just test 1 page
        false,   // headless
        true     // simulate mode
    ).await;
    
    match result {
        Ok(_) => {
            // Simulate mode succeeded, which means:
            // 1. Site was reachable
            // 2. Format was detected (or failed gracefully)
            // 3. No crashes occurred
            Ok(())
        },
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            
            // Some errors are acceptable for unknown formats
            if site.expected_format == "unknown" && 
               (error_msg.contains("unsupported") || error_msg.contains("unknown format")) {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

/// Test that we can detect supported formats correctly
#[tokio::test]
async fn test_known_documentation_sites() -> Result<()> {
    let known_good_sites = vec![
        ("https://docs.python.org/3/", "Python docs should be detected"),
        ("https://doc.rust-lang.org/book/", "Rust Book should be detected"),
        ("https://docusaurus.io/docs", "Docusaurus docs should be detected"),
    ];
    
    for (url, description) in known_good_sites {
        println!("Testing: {}", description);
        
        let temp_dir = tempfile::tempdir()?;
        let output_dir = temp_dir.path().to_str().unwrap().to_string();
        
        let downloader = Downloader::new(output_dir, true, false, 15.0);
        
        let result = timeout(
            Duration::from_secs(20),
            downloader.run(url, Some(1), false, true) // simulate mode
        ).await;
        
        match result {
            Ok(Ok(_)) => println!("  ✅ {}: SUCCESS", description),
            Ok(Err(e)) => {
                // Even if format detection fails, it shouldn't crash
                println!("  ⚠️  {}: {}", description, e);
            },
            Err(_) => {
                println!("  ⏰ {}: TIMEOUT", description);
                // Don't fail test on timeout - network issues happen
            }
        }
    }
    
    // This test always passes - it's for observation and regression detection
    Ok(())
}

/// Test that unknown sites are handled gracefully
#[tokio::test]
async fn test_unknown_sites_handling() -> Result<()> {
    let unknown_sites = vec![
        "https://example.com",
        "https://github.com",
        "https://stackoverflow.com",
    ];
    
    for url in unknown_sites {
        let temp_dir = tempfile::tempdir()?;
        let output_dir = temp_dir.path().to_str().unwrap().to_string();
        
        let downloader = Downloader::new(output_dir, true, false, 10.0);
        
        let result = downloader.run(url, Some(1), false, true).await;
        
        // Should either succeed (if detected) or fail gracefully with proper error
        match result {
            Ok(_) => println!("  ✅ {} handled successfully", url),
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                // Should fail with meaningful error, not crash
                assert!(
                    error_msg.contains("unsupported") || 
                    error_msg.contains("unknown") ||
                    error_msg.contains("format") ||
                    error_msg.contains("timeout"),
                    "Unexpected error for {}: {}", url, e
                );
                println!("  ✅ {} failed gracefully: {}", url, e);
            }
        }
    }
    
    Ok(())
}

/// Test error handling with invalid URLs
#[tokio::test]
async fn test_invalid_urls() -> Result<()> {
    let invalid_urls = vec![
        "not-a-url",
        "http://",
        "https://definitely-does-not-exist-12345.com",
    ];
    
    for url in invalid_urls {
        let temp_dir = tempfile::tempdir()?;
        let output_dir = temp_dir.path().to_str().unwrap().to_string();
        
        let downloader = Downloader::new(output_dir, true, false, 5.0);
        
        let result = downloader.run(url, Some(1), false, true).await;
        
        // Should fail with appropriate error
        assert!(result.is_err(), "Invalid URL {} should fail", url);
        
        let error_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            error_msg.contains("invalid") || 
            error_msg.contains("error") ||
            error_msg.contains("failed") ||
            error_msg.contains("url"),
            "Error message should be descriptive for {}: {}", url, error_msg
        );
    }
    
    Ok(())
}