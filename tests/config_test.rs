use anyhow::Result;
use book2pdf::Config;
use std::path::PathBuf;
use tokio::fs;
use tempfile::NamedTempFile;

fn create_temp_file() -> Result<NamedTempFile> {
    Ok(tempfile::NamedTempFile::new()?)
}

#[tokio::test]
async fn test_default_config() {
    let config = Config::default();
    
    // Test default values
    assert_eq!(config.output.folder, "output_book2pdf");
    assert_eq!(config.output.combine_pdfs, true);
    assert_eq!(config.output.preserve_pages, false);
    assert_eq!(config.browser.show_window, false);
    assert_eq!(config.browser.timeout, 30.0);
    assert_eq!(config.logging.level, "info");
}

#[test]
fn test_load_nonexistent_config() -> Result<()> {
    let nonexistent_path = PathBuf::from("definitely_does_not_exist.toml");
    
    // Should return default config when file doesn't exist
    let config = Config::load(Some(&nonexistent_path))?;
    
    // Verify it's the default config
    assert_eq!(config.output.folder, "output_book2pdf");
    assert_eq!(config.browser.timeout, 30.0);
    
    Ok(())
}

#[tokio::test]
async fn test_load_valid_config() -> Result<()> {
    let temp_file = create_temp_file()?;
    
    let config_content = r#"
[output]
folder = "custom_output"
combine_pdfs = false
preserve_pages = true

[browser]
show_window = true
timeout = 60.0

[logging]
level = "debug"
"#;
    
    fs::write(temp_file.path(), config_content).await?;
    
    let config = Config::load(Some(temp_file.path()))?;
    
    // Verify custom values were loaded
    assert_eq!(config.output.folder, "custom_output");
    assert_eq!(config.output.combine_pdfs, false);
    assert_eq!(config.output.preserve_pages, true);
    assert_eq!(config.browser.show_window, true);
    assert_eq!(config.browser.timeout, 60.0);
    assert_eq!(config.logging.level, "debug");
    
    Ok(())
}

#[tokio::test]
async fn test_load_invalid_config() -> Result<()> {
    let temp_file = create_temp_file()?;
    
    // Invalid TOML content
    let invalid_content = r#"
[output
folder = "missing_bracket"
"#;
    
    fs::write(temp_file.path(), invalid_content).await?;
    
    let result = Config::load(Some(temp_file.path()));
    
    // Should fail to parse invalid TOML
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_save_and_load_config() -> Result<()> {
    let temp_file = create_temp_file()?;
    
    // Create a custom config
    let mut config = Config::default();
    config.output.folder = "test_folder".to_string();
    config.browser.timeout = 45.0;
    config.logging.level = "trace".to_string();
    
    // Save the config
    config.save(temp_file.path())?;
    
    // Load it back
    let loaded_config = Config::load(Some(temp_file.path()))?;
    
    // Verify values match
    assert_eq!(loaded_config.output.folder, "test_folder");
    assert_eq!(loaded_config.browser.timeout, 45.0);
    assert_eq!(loaded_config.logging.level, "trace");
    
    Ok(())
}

#[test]
fn test_config_find_file_logic() -> Result<()> {
    // Test the default config loading (no file specified)
    let config = Config::load(None)?;
    
    // Should return default config when no config file is found
    assert_eq!(config.output.folder, "output_book2pdf");
    
    Ok(())
}