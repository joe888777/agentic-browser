use chromiumoxide::browser::{Browser as CrBrowser, BrowserConfig as CrBrowserConfig};
use chromiumoxide::cdp::browser_protocol::fetch::{
    self, AuthChallengeResponseResponse, ContinueWithAuthParams, EnableParams,
    EventAuthRequired, EventRequestPaused,
};
use chromiumoxide::handler::viewport::Viewport;
use futures::StreamExt;

use crate::config::{BrowserBuilder, BrowserConfig};
use crate::error::{Error, Result};
use crate::page::Page;
use crate::stealth;

/// The main entry point for controlling a browser instance.
pub struct AgenticBrowser {
    browser: CrBrowser,
    stealth: bool,
    proxy_auth: Option<(String, String)>,
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
            builder = builder.new_headless_mode().no_sandbox();
        } else {
            builder = builder.with_head().no_sandbox();
        }

        // Stealth: add anti-detection Chrome flags
        // chromiumoxide adds `--` prefix automatically, so keys must NOT include `--`
        if config.stealth {
            for arg in stealth::stealth_key_args() {
                builder = builder.arg(arg);
            }
            for arg in stealth::stealth_kv_args() {
                builder = builder.arg(arg);
            }
        }

        // Proxy: pass proxy-server flag to Chrome
        // Use tuple format: ("key", "value") -> --key=value
        if let Some(ref proxy) = config.proxy {
            builder = builder.arg(("proxy-server", proxy.server.as_str()));
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

        // Extract proxy auth credentials for later use with CDP
        let proxy_auth = config.proxy.as_ref().and_then(|p| {
            match (&p.username, &p.password) {
                (Some(u), Some(p)) => Some((u.clone(), p.clone())),
                _ => None,
            }
        });

        Ok(Self {
            browser,
            stealth: config.stealth,
            proxy_auth,
            _handler_task: handler_task,
        })
    }

    /// Open a new page (tab) navigated to the given URL.
    /// If stealth mode is enabled, anti-detection scripts are injected before navigation.
    /// If proxy auth is configured, it handles 407 challenges automatically.
    pub async fn new_page(&self, url: &str) -> Result<Page> {
        let cr_page = self
            .browser
            .new_page("about:blank")
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;

        // Inject stealth scripts BEFORE navigating to the target URL
        if self.stealth {
            stealth::apply_stealth(&cr_page).await?;
        }

        // Set up proxy authentication if credentials are provided
        if let Some((ref username, ref password)) = self.proxy_auth {
            let enable_params = EnableParams::builder()
                .handle_auth_requests(true)
                .build();
            cr_page
                .execute(enable_params)
                .await
                .map_err(|e| Error::LaunchError(format!("Failed to enable fetch for proxy auth: {e}")))?;

            // Listen for auth challenges and respond with credentials
            let username = username.clone();
            let password = password.clone();
            let page_clone = cr_page.clone();

            tokio::spawn(async move {
                let mut auth_events = page_clone
                    .event_listener::<EventAuthRequired>()
                    .await
                    .unwrap();
                while let Some(event) = auth_events.next().await {
                    let auth_response = fetch::AuthChallengeResponse::builder()
                        .response(AuthChallengeResponseResponse::ProvideCredentials)
                        .username(username.clone())
                        .password(password.clone())
                        .build()
                        .unwrap();
                    let params = ContinueWithAuthParams::new(
                        event.request_id.clone(),
                        auth_response,
                    );
                    let _ = page_clone.execute(params).await;
                }
            });

            // Also continue non-auth paused requests
            let page_clone2 = cr_page.clone();
            tokio::spawn(async move {
                let mut pause_events = page_clone2
                    .event_listener::<EventRequestPaused>()
                    .await
                    .unwrap();
                while let Some(event) = pause_events.next().await {
                    let params = fetch::ContinueRequestParams::new(event.request_id.clone());
                    let _ = page_clone2.execute(params).await;
                }
            });
        }

        cr_page
            .goto(url)
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
