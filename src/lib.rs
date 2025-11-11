slint::include_modules!();

pub mod data;
pub mod ui;

/// Application configuration structure
/// Holds basic configuration for the CSV Navigator application
#[derive(Debug, Clone)]
pub struct AppConfig {
    window_title: String,
    min_width: u32,
    min_height: u32,
}

impl AppConfig {
    /// Creates a new AppConfig with default values
    pub fn new() -> Self {
        Self {
            window_title: String::from("CSV Navigator"),
            min_width: 640,
            min_height: 480,
        }
    }

    /// Creates a new AppConfig with a custom title
    pub fn with_title(title: &str) -> Self {
        Self {
            window_title: String::from(title),
            min_width: 640,
            min_height: 480,
        }
    }

    /// Creates a new AppConfig with custom dimensions
    pub fn with_dimensions(width: u32, height: u32) -> Self {
        Self {
            window_title: String::from("CSV Navigator"),
            min_width: width,
            min_height: height,
        }
    }

    /// Returns the window title
    pub fn window_title(&self) -> &str {
        &self.window_title
    }

    /// Returns the minimum window width
    pub fn min_width(&self) -> u32 {
        self.min_width
    }

    /// Returns the minimum window height
    pub fn min_height(&self) -> u32 {
        self.min_height
    }

    /// Validates that the configuration is valid
    pub fn is_valid(&self) -> bool {
        !self.window_title.is_empty()
            && self.min_width > 0
            && self.min_height > 0
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates and initializes the application with a specific configuration
/// Returns a Result containing the AppWindow or a PlatformError
pub fn create_app_with_config(config: AppConfig) -> Result<AppWindow, slint::PlatformError> {
    // Validate configuration before attempting to create window
    if !config.is_valid() {
        // Return a more appropriate error for invalid configuration
        return Err(slint::PlatformError::Other(
            "Invalid application configuration: title must not be empty and dimensions must be positive".into()
        ));
    }

    let ui = AppWindow::new()?;

    // Apply configuration to the window
    ui.set_window_title(config.window_title().into());

    // Note: Slint's logical sizes are set in the .slint file
    // The min-width and min-height in our config serve as validation
    // In a production app, you could extend this to set initial window size

    Ok(ui)
}

/// Creates and initializes the application with default configuration
/// Returns a Result containing the AppWindow or a PlatformError
pub fn create_app() -> Result<AppWindow, slint::PlatformError> {
    create_app_with_config(AppConfig::new())
}

/// Runs the application from start to finish
/// This is the main entry point for the application
pub fn run_app() -> Result<(), slint::PlatformError> {
    let ui = create_app()?;

    // Set up UI callbacks for file operations, filtering, and sorting
    ui::setup_callbacks(&ui);

    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_default_values() {
        let config = AppConfig::new();
        assert_eq!(config.window_title(), "CSV Navigator");
        assert_eq!(config.min_width(), 640);
        assert_eq!(config.min_height(), 480);
    }

    #[test]
    fn test_app_config_is_valid() {
        let config = AppConfig::new();
        assert!(config.is_valid());
    }
}
