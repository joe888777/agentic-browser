use crate::browser::AgenticBrowser;
use crate::error::Result;

pub struct BrowserConfig {
    pub headless: bool,
    pub stealth: bool,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub chrome_path: Option<String>,
    /// Proxy server URL, e.g. "http://host:port", "socks5://host:port",
    /// or with auth: "http://user:pass@host:port"
    pub proxy: Option<ProxyConfig>,
}

/// Proxy configuration.
#[derive(Clone)]
pub struct ProxyConfig {
    /// Proxy server URL (e.g. "http://host:port", "socks5://host:port")
    pub server: String,
    /// Optional username for proxy authentication
    pub username: Option<String>,
    /// Optional password for proxy authentication
    pub password: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            stealth: true,
            viewport_width: 1920,
            viewport_height: 1080,
            chrome_path: None,
            proxy: None,
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

    /// Set a proxy server (e.g. "http://host:port", "socks5://host:port").
    pub fn proxy(mut self, server: impl Into<String>) -> Self {
        self.config.proxy = Some(ProxyConfig {
            server: server.into(),
            username: None,
            password: None,
        });
        self
    }

    /// Set a proxy server with authentication.
    pub fn proxy_with_auth(
        mut self,
        server: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.config.proxy = Some(ProxyConfig {
            server: server.into(),
            username: Some(username.into()),
            password: Some(password.into()),
        });
        self
    }

    pub fn build_config(self) -> BrowserConfig {
        self.config
    }

    pub async fn build(self) -> Result<AgenticBrowser> {
        AgenticBrowser::launch(self.build_config()).await
    }
}
