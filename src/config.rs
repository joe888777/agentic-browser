use crate::browser::AgenticBrowser;
use crate::error::Result;

pub struct BrowserConfig {
    pub headless: bool,
    pub stealth: bool,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub chrome_path: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            stealth: true,
            viewport_width: 1920,
            viewport_height: 1080,
            chrome_path: None,
        }
    }
}

pub struct BrowserBuilder {
    config: BrowserConfig,
}

impl BrowserBuilder {
    pub fn new() -> Self {
        Self {
            config: BrowserConfig::default(),
        }
    }

    pub fn headless(mut self, headless: bool) -> Self {
        self.config.headless = headless;
        self
    }

    pub fn stealth(mut self, stealth: bool) -> Self {
        self.config.stealth = stealth;
        self
    }

    pub fn viewport(mut self, width: u32, height: u32) -> Self {
        self.config.viewport_width = width;
        self.config.viewport_height = height;
        self
    }

    pub fn chrome_path(mut self, path: impl Into<String>) -> Self {
        self.config.chrome_path = Some(path.into());
        self
    }

    pub fn build_config(self) -> BrowserConfig {
        self.config
    }

    pub async fn build(self) -> Result<AgenticBrowser> {
        AgenticBrowser::launch(self.build_config()).await
    }
}
