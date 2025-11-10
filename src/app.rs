use gpui::*;
use crate::views::MainView;

/// Configuration for the CSV Navigator application
pub struct AppConfig {
    window_title: String,
    min_width: f64,
    min_height: f64,
}

impl AppConfig {
    /// Create a new AppConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom window title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = title.into();
        self
    }

    /// Set custom window dimensions
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.min_width = width as f64;
        self.min_height = height as f64;
        self
    }

    /// Check if the configuration is valid
    pub fn is_valid(&self) -> bool {
        self.min_width >= 320.0 && self.min_height >= 240.0
    }

    pub fn window_title(&self) -> &str {
        &self.window_title
    }

    pub fn min_width(&self) -> f64 {
        self.min_width
    }

    pub fn min_height(&self) -> f64 {
        self.min_height
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_title: "CSV Navigator".to_string(),
            min_width: 640.0,
            min_height: 480.0,
        }
    }
}

/// Initialize and run the CSV Navigator application with custom config
pub fn run_app_with_config(config: AppConfig) -> anyhow::Result<()> {
    if !config.is_valid() {
        anyhow::bail!("Invalid configuration: dimensions must be at least 320x240");
    }

    App::new().run(move |cx: &mut AppContext| {
        // Set up window options
        let bounds = Bounds::centered(
            None,
            size(px(config.min_width as f32), px(config.min_height as f32)),
            cx,
        );

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from(config.window_title.clone())),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |cx| {
                cx.new_view(|cx| MainView::new(cx))
            },
        )
        .expect("Failed to open window");
    });

    Ok(())
}

/// Initialize and run the CSV Navigator application with default config
pub fn run_app() -> anyhow::Result<()> {
    run_app_with_config(AppConfig::default())
}
