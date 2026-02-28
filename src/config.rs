use std::time::Duration;

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
    /// Default timeout for operations like `wait_for_selector` (default: 30s).
    pub default_timeout: Duration,
    /// Enable low-resource mode for constrained devices (Raspberry Pi, ARM SBCs).
    /// Adds aggressive memory/CPU reduction flags and limits JS heap size.
    pub low_resource: bool,
    /// Optional JS heap size limit in MB (used with low_resource mode).
    pub js_heap_size_mb: Option<u32>,
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
            default_timeout: Duration::from_secs(30),
            low_resource: false,
            js_heap_size_mb: None,
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

    /// Preset for Raspberry Pi and other ARM single-board computers.
    /// Sets headless mode, 1280x720 viewport, low-resource flags, 256MB JS heap,
    /// and auto-detects the Chromium binary path on Raspberry Pi OS.
    pub fn raspi(mut self) -> Self {
        self.config.headless = true;
        self.config.viewport_width = 1280;
        self.config.viewport_height = 720;
        self.config.low_resource = true;
        self.config.js_heap_size_mb = Some(256);
        self.config.default_timeout = Duration::from_secs(60);
        // Auto-detect Chromium on Raspberry Pi OS / Debian ARM
        if self.config.chrome_path.is_none() {
            for path in &[
                "/usr/bin/chromium-browser",
                "/usr/bin/chromium",
            ] {
                if std::path::Path::new(path).exists() {
                    self.config.chrome_path = Some(path.to_string());
                    break;
                }
            }
        }
        self
    }

    /// Enable low-resource mode for constrained devices.
    /// Adds aggressive memory/CPU reduction Chrome flags.
    pub fn low_resource(mut self, enabled: bool) -> Self {
        self.config.low_resource = enabled;
        self
    }

    /// Set the JS heap size limit in MB (e.g., 256 for Raspberry Pi).
    pub fn js_heap_size_mb(mut self, size: u32) -> Self {
        self.config.js_heap_size_mb = Some(size);
        self
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

    /// Set the default timeout for operations like `wait_for_selector`.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
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

impl Default for BrowserBuilder {
    fn default() -> Self {
        Self::new()
    }
}
