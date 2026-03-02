use std::sync::Arc;

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

/// Chrome flags that improve performance without affecting functionality.
const PERF_ARGS: &[&str] = &[
    "disable-gpu",
    "disable-extensions",
    "metrics-recording-only",
    "mute-audio",
    "no-default-browser-check",
    "disable-client-side-phishing-detection",
    "disable-popup-blocking",
    "disable-prompt-on-repost",
];

/// Additional Chrome flags for low-resource environments (Raspberry Pi, ARM SBCs).
/// These aggressively reduce memory and CPU usage at the cost of some features.
const LOW_RESOURCE_ARGS: &[&str] = &[
    "single-process",
    "disable-dev-shm-usage",
    "disable-software-rasterizer",
    "disable-gpu-compositing",
    "disable-background-networking",
    "disable-background-timer-throttling",
    "disable-renderer-backgrounding",
    "disable-backgrounding-occluded-windows",
    "disable-ipc-flooding-protection",
    "disable-hang-monitor",
    "disable-sync",
    "disable-translate",
    "disable-domain-reliability",
    "disable-site-isolation-trials",
    "no-zygote",
];

/// The main entry point for controlling a browser instance.
pub struct AgenticBrowser {
    browser: CrBrowser,
    stealth: bool,
    proxy_auth: Option<(Arc<str>, Arc<str>)>,
    default_timeout: std::time::Duration,
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

        // Performance: add Chrome flags that reduce startup and load time
        for arg in PERF_ARGS {
            builder = builder.arg(*arg);
        }

        // Low-resource mode: add aggressive memory/CPU reduction flags
        if config.low_resource {
            for arg in LOW_RESOURCE_ARGS {
                builder = builder.arg(*arg);
            }
            // Limit JS heap size
            let heap_mb = config.js_heap_size_mb.unwrap_or(256);
            let js_flags = format!("--max-old-space-size={heap_mb}");
            builder = builder.arg(("js-flags", js_flags.as_str()));
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

        // Extract proxy auth credentials for later use with CDP (Arc to avoid per-event cloning)
        let proxy_auth = config.proxy.as_ref().and_then(|p| {
            match (&p.username, &p.password) {
                (Some(u), Some(p)) => Some((Arc::from(u.as_str()), Arc::from(p.as_str()))),
                _ => None,
            }
        });

        Ok(Self {
            browser,
            stealth: config.stealth,
            proxy_auth,
            default_timeout: config.default_timeout,
            _handler_task: handler_task,
        })
    }

    /// Open a new page (tab) navigated to the given URL.
    /// If stealth mode is enabled, anti-detection scripts are injected before navigation.
    /// If proxy auth is configured, it handles 407 challenges automatically.
    /// Stealth and proxy auth setup run in parallel for faster page creation.
    pub async fn new_page(&self, url: &str) -> Result<Page> {
        let cr_page = self
            .browser
            .new_page("about:blank")
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;

        // Run stealth injection and proxy auth setup in parallel
        let stealth_fut = async {
            if self.stealth {
                stealth::apply_stealth(&cr_page).await
            } else {
                Ok(())
            }
        };

        let proxy_fut = async {
            if let Some((ref username, ref password)) = self.proxy_auth {
                Self::setup_proxy_auth(&cr_page, username, password).await
            } else {
                Ok(())
            }
        };

        let (stealth_result, proxy_result) = tokio::join!(stealth_fut, proxy_fut);
        stealth_result?;
        proxy_result?;

        cr_page
            .goto(url)
            .await
            .map_err(|e| Error::NavigationError(e.to_string()))?;

        Ok(Page::new(cr_page, self.default_timeout))
    }

    /// Set up proxy authentication handlers for a page.
    async fn setup_proxy_auth(
        cr_page: &chromiumoxide::page::Page,
        username: &Arc<str>,
        password: &Arc<str>,
    ) -> Result<()> {
        // Set up event listeners BEFORE enabling fetch domain to avoid race condition
        let mut auth_events = cr_page
            .event_listener::<EventAuthRequired>()
            .await
            .map_err(|e| Error::LaunchError(format!("Failed to listen for auth events: {e}")))?;

        let mut pause_events = cr_page
            .event_listener::<EventRequestPaused>()
            .await
            .map_err(|e| Error::LaunchError(format!("Failed to listen for request paused events: {e}")))?;

        // Now enable fetch domain — listeners are already subscribed
        let enable_params = EnableParams::builder()
            .handle_auth_requests(true)
            .build();
        cr_page
            .execute(enable_params)
            .await
            .map_err(|e| Error::LaunchError(format!("Failed to enable fetch for proxy auth: {e}")))?;

        // Listen for auth challenges and respond with credentials
        let username = Arc::clone(username);
        let password = Arc::clone(password);
        let page_clone = cr_page.clone();

        tokio::spawn(async move {
            while let Some(event) = auth_events.next().await {
                let auth_response = match fetch::AuthChallengeResponse::builder()
                    .response(AuthChallengeResponseResponse::ProvideCredentials)
                    .username(username.as_ref())
                    .password(password.as_ref())
                    .build()
                {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Failed to build auth response: {e}");
                        continue;
                    }
                };
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
            while let Some(event) = pause_events.next().await {
                let params = fetch::ContinueRequestParams::new(event.request_id.clone());
                let _ = page_clone2.execute(params).await;
            }
        });

        Ok(())
    }

    /// Return all currently open pages (tabs).
    pub async fn pages(&self) -> Result<Vec<Page>> {
        let timeout = self.default_timeout;
        let cr_pages = self.browser.pages().await.map_err(|e| Error::CdpError(e))?;
        Ok(cr_pages.into_iter().map(|p| Page::new(p, timeout)).collect())
    }
}
