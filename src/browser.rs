use chromiumoxide::browser::{Browser as CrBrowser, BrowserConfig as CrBrowserConfig};
use chromiumoxide::handler::viewport::Viewport;
use futures::StreamExt;

use crate::config::{BrowserBuilder, BrowserConfig};
use crate::error::{Error, Result};
use crate::page::Page;

/// The main entry point for controlling a browser instance.
pub struct AgenticBrowser {
    browser: CrBrowser,
    _handler_task: tokio::task::JoinHandle<()>,
}

impl AgenticBrowser {
    /// Create a new BrowserBuilder for configuring and launching a browser.
    pub fn builder() -> BrowserBuilder {
        BrowserBuilder::new()
    }

    /// Launch a browser instance with the given configuration.
    pub async fn launch(config: BrowserConfig) -> Result<Self> {
        let mut builder = CrBrowserConfig::builder();

        if config.headless {
            builder = builder.no_sandbox().arg("--headless=new");
        } else {
            builder = builder.with_head().no_sandbox();
        }

        if let Some(ref path) = config.chrome_path {
            builder = builder.chrome_executable(path);
        }

        builder = builder.viewport(Viewport {
            width: config.viewport_width,
            height: config.viewport_height,
            device_scale_factor: None,
            emulating_mobile: false,
            is_landscape: false,
            has_touch: false,
        });

        let cr_config = builder
            .build()
            .map_err(|e| Error::LaunchError(e.to_string()))?;

        let (browser, mut handler) = CrBrowser::launch(cr_config)
            .await
            .map_err(|e| Error::LaunchError(e.to_string()))?;

        let handler_task = tokio::spawn(async move {
            while let Some(_event) = handler.next().await {}
        });

        Ok(Self {
            browser,
            _handler_task: handler_task,
        })
    }

    /// Open a new page (tab) navigated to the given URL.
    pub async fn new_page(&self, url: &str) -> Result<Page> {
        let cr_page = self
            .browser
            .new_page(url)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;
        Ok(Page::new(cr_page))
    }

    /// Return all currently open pages (tabs).
    pub async fn pages(&self) -> Result<Vec<Page>> {
        let cr_pages = self.browser.pages().await.map_err(|e| Error::CdpError(e))?;
        Ok(cr_pages.into_iter().map(Page::new).collect())
    }
}
