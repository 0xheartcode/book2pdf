use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub browser: BrowserConfig,
    #[serde(default)]
    pub pdf: PdfConfig,
    #[serde(default)]
    pub scraping: ScrapingConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputConfig {
    pub folder: String,
    pub filename: Option<String>,
    pub combine_pdfs: bool,
    pub preserve_pages: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrowserConfig {
    pub show_window: bool,
    pub timeout: f64,
    pub window_width: u32,
    pub window_height: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfConfig {
    pub scale: f64,
    pub margin_top: f64,
    pub margin_right: f64,
    pub margin_bottom: f64,
    pub margin_left: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScrapingConfig {
    pub page_limit: Option<usize>,
    pub supported_sites: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub suppress_browser_logs: bool,
}

// Default implementations
impl Default for Config {
    fn default() -> Self {
        Self {
            output: OutputConfig::default(),
            browser: BrowserConfig::default(),
            pdf: PdfConfig::default(),
            scraping: ScrapingConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            folder: "output_book2pdf".to_string(),
            filename: None,
            combine_pdfs: true,
            preserve_pages: false,
        }
    }
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            show_window: false,
            timeout: 30.0,
            window_width: 1920,
            window_height: 1080,
        }
    }
}

impl Default for PdfConfig {
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

impl Default for ScrapingConfig {
    fn default() -> Self {
        Self {
            page_limit: None,
            supported_sites: vec!["gitbook".to_string(), "docusaurus".to_string()],
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            suppress_browser_logs: true,
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults if file doesn't exist
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let config_file = match config_path {
            Some(path) => path.to_path_buf(),
            None => Self::find_config_file(),
        };

        if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Return default config if no file found
            Ok(Config::default())
        }
    }

    /// Find config file in standard locations
    fn find_config_file() -> PathBuf {
        // First try current directory
        let current_dir = PathBuf::from("./book2pdf.toml");
        if current_dir.exists() {
            return current_dir;
        }

        // Then try user config directory
        if let Some(config_dir) = dirs::config_dir() {
            let user_config = config_dir.join("book2pdf").join("config.toml");
            if user_config.exists() {
                return user_config;
            }
        }

        // Default to current directory (even if it doesn't exist)
        current_dir
    }

    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }
}