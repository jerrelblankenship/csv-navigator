/// Integration tests for basic application infrastructure
/// Following TDD: These tests define what we need before implementation

#[cfg(test)]
mod app_tests {
    use csv_navigator::AppConfig;

    #[test]
    fn test_app_config_can_be_created() {
        // Test that we can create an application configuration
        let config = AppConfig::new();
        assert_eq!(config.window_title(), "CSV Navigator");
    }

    #[test]
    fn test_app_config_has_minimum_dimensions() {
        // Test that the app has sensible minimum window dimensions
        let config = AppConfig::new();
        assert_eq!(config.min_width(), 640);
        assert_eq!(config.min_height(), 480);
    }

    #[test]
    fn test_app_config_is_runnable() {
        // Test that we can verify the app is in a runnable state
        let config = AppConfig::new();
        assert!(config.is_valid());
    }

    #[test]
    fn test_app_config_with_custom_title() {
        // Test creating config with custom title
        let config = AppConfig::with_title("My Custom App");
        assert_eq!(config.window_title(), "My Custom App");
        assert!(config.is_valid());
    }

    #[test]
    fn test_app_config_with_empty_title_is_invalid() {
        // Test that empty title makes config invalid
        let config = AppConfig::with_title("");
        assert!(!config.is_valid());
    }

    #[test]
    fn test_app_config_with_custom_dimensions() {
        // Test creating config with custom dimensions
        let config = AppConfig::with_dimensions(800, 600);
        assert_eq!(config.min_width(), 800);
        assert_eq!(config.min_height(), 600);
        assert!(config.is_valid());
    }

    #[test]
    fn test_app_config_with_zero_width_is_invalid() {
        // Test that zero width makes config invalid
        let config = AppConfig::with_dimensions(0, 480);
        assert!(!config.is_valid());
    }

    #[test]
    fn test_app_config_with_zero_height_is_invalid() {
        // Test that zero height makes config invalid
        let config = AppConfig::with_dimensions(640, 0);
        assert!(!config.is_valid());
    }
}
