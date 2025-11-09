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
}
