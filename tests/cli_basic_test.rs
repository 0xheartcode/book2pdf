use anyhow::Result;
use std::process::Command;

/// Test basic CLI functionality that should always work
/// These are simple, fast tests that validate core CLI operations

/// Test that help command works
#[test]
fn test_help_command() -> Result<()> {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()?;
    
    assert!(output.status.success(), "Help command should succeed");
    
    let stdout = String::from_utf8(output.stdout)?;
    
    // Should contain basic information
    assert!(stdout.contains("book2pdf"), "Should show program name");
    assert!(stdout.contains("download"), "Should show download command");
    assert!(stdout.contains("merge"), "Should show merge command");
    
    Ok(())
}

/// Test that version command works
#[test]
fn test_version_command() -> Result<()> {
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .output()?;
    
    assert!(output.status.success(), "Version command should succeed");
    
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("0.1.0"), "Should show version number");
    
    Ok(())
}

/// Test that list command shows all supported formats
#[test]
fn test_list_supported_formats() -> Result<()> {
    let output = Command::new("cargo")
        .args(&["run", "--", "download", "--list"])
        .output()?;
    
    assert!(output.status.success(), "List command should succeed");
    
    let stdout = String::from_utf8(output.stdout)?;
    
    // Should list all major supported formats
    let expected_formats = [
        "GitBook", "Docusaurus", "Sphinx", "mdBook", 
        "MkDocs", "VitePress", "Nextra", "Starlight", "vocs"
    ];
    
    for format in &expected_formats {
        assert!(stdout.contains(format), "Should list {} format", format);
    }
    
    Ok(())
}

/// Test simulate mode with a known documentation site
#[test]
fn test_simulate_mode() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--", "download",
            "https://docs.python.org/3/",
            "--out-dir", temp_dir.path().to_str().unwrap(),
            "--simulate",
            "--pages", "1"
        ])
        .output()?;
    
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    let combined_output = format!("{}\n{}", stdout, stderr);
    
    // Simulate mode should either succeed or fail gracefully
    if !output.status.success() {
        // If it fails, should be a meaningful error
        let error_lower = combined_output.to_lowercase();
        assert!(
            error_lower.contains("simulate") || 
            error_lower.contains("sphinx") ||
            error_lower.contains("timeout") ||
            error_lower.contains("network"),
            "Simulate failure should be meaningful: {}", combined_output
        );
    } else {
        // If it succeeds, should mention simulation or detection
        assert!(
            combined_output.to_lowercase().contains("simulate") ||
            combined_output.to_lowercase().contains("sphinx") ||
            combined_output.to_lowercase().contains("detected"),
            "Simulate success should show progress: {}", combined_output
        );
    }
    
    Ok(())
}

/// Test basic parameter validation
#[test]
fn test_parameter_validation() -> Result<()> {
    // Test missing URL
    let output = Command::new("cargo")
        .args(&["run", "--", "download"])
        .output()?;
    
    assert!(!output.status.success(), "Should fail without URL");
    
    // Test invalid timeout
    let output = Command::new("cargo")
        .args(&[
            "run", "--", "download",
            "https://example.com",
            "--timeout", "invalid",
            "--simulate"
        ])
        .output()?;
    
    assert!(!output.status.success(), "Should fail with invalid timeout");
    
    // Test invalid pages
    let output = Command::new("cargo")
        .args(&[
            "run", "--", "download", 
            "https://example.com",
            "--pages", "0",
            "--simulate"
        ])
        .output()?;
    
    assert!(!output.status.success(), "Should fail with pages=0");
    
    Ok(())
}

/// Test merge command basic functionality
#[tokio::test] 
async fn test_merge_command_basic() -> Result<()> {
    // Test merge functionality directly instead of through CLI to avoid cargo lock issues
    use book2pdf::PdfMerger;
    
    let temp_dir = tempfile::tempdir()?;
    let empty_dir = temp_dir.path().join("empty");
    std::fs::create_dir(&empty_dir)?;
    
    let output_file = temp_dir.path().join("merged.pdf");
    
    // Test the merge functionality directly
    let result = PdfMerger::merge_directory(
        empty_dir.to_str().unwrap(),
        output_file.to_str().unwrap()
    ).await;
    
    // Should fail gracefully when no PDF files found
    assert!(result.is_err(), "Should fail with empty directory");
    
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_msg.contains("no pdf files") || 
        error_msg.contains("error"),
        "Should give meaningful error: {}", error_msg
    );
    
    Ok(())
}

/// Test conflicting CLI arguments
#[test]
fn test_conflicting_arguments() -> Result<()> {
    // Test conflicting logging levels
    let output = Command::new("cargo")
        .args(&["run", "--", "--verbose", "--quiet", "download", "--list"])
        .output()?;
    
    assert!(!output.status.success(), "Should reject conflicting log levels");
    
    Ok(())
}

/// Test basic configuration handling
#[test]
fn test_config_handling() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let config_file = temp_dir.path().join("test_config.toml");
    
    // Create a simple valid config with all required fields
    std::fs::write(&config_file, r#"
[output]
folder = "custom_output"
combine_pdfs = true
preserve_pages = false

[browser]
timeout = 30.0
show_window = false
window_width = 1920
window_height = 1080

[pdf]
scale = 0.75
margin_top = 0.0
margin_right = 0.0
margin_bottom = 0.0
margin_left = 0.0

[scraping]
simulate = false

[logging]
level = "info"
suppress_browser_logs = true
"#)?;
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--", 
            "--config", config_file.to_str().unwrap(),
            "download", "--list"
        ])
        .output()?;
    
    // Should accept valid config file
    assert!(output.status.success(), "Should accept valid config");
    
    // Test with non-existent config file
    let fake_config = temp_dir.path().join("does_not_exist.toml");
    let output = Command::new("cargo")
        .args(&[
            "run", "--",
            "--config", fake_config.to_str().unwrap(), 
            "download", "--list"
        ])
        .output()?;
    
    // Should either succeed (using defaults) or fail gracefully
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        assert!(
            stderr.to_lowercase().contains("config") ||
            stderr.to_lowercase().contains("file"),
            "Config error should be clear: {}", stderr
        );
    }
    
    Ok(())
}